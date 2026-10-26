//! Requêtes HTTP par `/usr/bin/curl`, lancé comme un script : son groupe de processus est tué avec la barre.
//! L'adresse, les en-têtes (secrets compris) et le corps passent par l'entrée standard (`--config -`),
//! jamais par les arguments, que `ps` montre à tout le monde.

use super::{url_permise, TAILLE_MAX};
use crate::barre::{executer, Processus};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug)]
pub(crate) struct Reponse {
    pub statut: u16,
    /// Noms en minuscules.
    pub entetes: Vec<(String, String)>,
    pub corps: String,
}

impl Reponse {
    pub(crate) fn entete(&self, nom: &str) -> Option<&str> {
        self.entetes.iter().find(|(n, _)| n == nom).map(|(_, v)| v.as_str())
    }

    /// Le corps d'une réponse réussie, en valeur ; une erreur HTTP avec le début de son corps.
    pub(crate) fn valeur(self) -> Result<Value, String> {
        if self.statut >= 400 {
            return Err(format!("HTTP {} : {}", self.statut, self.corps.chars().take(120).collect::<String>().trim()));
        }
        Ok(super::valeur_de_texte(&self.corps))
    }
}

/// Une valeur entre guillemets pour le fichier de config de curl.
fn guillemets(texte: &str) -> String {
    format!("\"{}\"", texte.replace('\\', "\\\\").replace('"', "\\\""))
}

/// GET (ou POST avec `corps`) sur `url` : https, ou http vers cette machine ; `delai` au plus, 1 Mo au plus,
/// aucune redirection suivie.
pub(crate) fn requete(url: &str, entetes: &[(String, String)], corps: Option<&str>, delai: Duration, processus: &Processus) -> Result<Reponse, String> {
    if !url_permise(url) {
        return Err(format!("« {url} » refusée : seules https:// et http://localhost sont lues"));
    }
    let mut config = format!("url = {}\n", guillemets(url));
    for (nom, valeur) in entetes {
        if nom.contains(['\r', '\n', ':']) || valeur.contains(['\r', '\n']) {
            return Err(format!("en-tête « {} » invalide (retour à la ligne ou deux-points)", nom.replace(['\r', '\n'], " ")));
        }
        config.push_str(&format!("header = {}\n", guillemets(&format!("{nom}: {valeur}"))));
    }
    if let Some(corps) = corps {
        config.push_str(&format!("data-binary = {}\n", guillemets(corps)));
    }
    let proto = if url.starts_with("https://") { "=https" } else { "=http" };
    let secondes = format!("{}", delai.as_secs_f64());
    let max = TAILLE_MAX.to_string();
    // -q en premier : ignorer ~/.curlrc (il pourrait suivre les redirections ou passer par un proxy).
    let args = ["-q", "-sS", "-i", "--max-time", &secondes, "--max-filesize", &max, "--proto", proto, "--config", "-"];
    let sortie = executer(std::process::Command::new("/usr/bin/curl").args(args), Some(&config), delai + Duration::from_secs(2), processus);
    if sortie.erreur.contains("(63)") || sortie.texte.len() > TAILLE_MAX + 64 * 1024 {
        return Err("réponse de plus de 1 Mo".into());
    }
    if !sortie.erreur.is_empty() {
        return Err(sortie.erreur.replace("code 28, ", "").replace("curl: ", ""));
    }
    analyser(&sortie.texte)
}

/// Sortie de `curl -i` : blocs d'en-têtes (un 1xx ou un tunnel de proxy peut précéder le vrai), puis le corps.
fn analyser(brut: &str) -> Result<Reponse, String> {
    let mut reste = brut;
    loop {
        let (bloc, corps) = reste.split_once("\r\n\r\n").unwrap_or((reste, ""));
        let mut lignes = bloc.lines();
        let statut: u16 = lignes.next().and_then(|l| l.split_whitespace().nth(1)).and_then(|s| s.parse().ok()).ok_or("réponse HTTP illisible")?;
        let intermediaire = (100..200).contains(&statut) || bloc.contains("Connection established");
        if intermediaire && corps.starts_with("HTTP/") {
            reste = corps;
            continue;
        }
        let entetes = lignes.filter_map(|l| l.split_once(':')).map(|(n, v)| (n.trim().to_lowercase(), v.trim().to_string())).collect();
        if corps.len() > TAILLE_MAX {
            return Err("réponse de plus de 1 Mo".into());
        }
        return Ok(Reponse { statut, entetes, corps: corps.to_string() });
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::time::Instant;

    fn processus() -> Processus {
        Processus::new(std::sync::Mutex::new(Some(Vec::new())))
    }

    /// Petit serveur local : `reponse(chemin, en-têtes reçus, corps reçu)` donne (statut, type, corps).
    pub(crate) fn serveur(reponse: impl Fn(&str, &str, &str) -> (u16, &'static str, String) + Send + Sync + 'static) -> String {
        let ecoute = TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = format!("http://127.0.0.1:{}", ecoute.local_addr().unwrap().port());
        let reponse = std::sync::Arc::new(reponse);
        std::thread::spawn(move || {
            for flux in ecoute.incoming().flatten() {
                let reponse = reponse.clone();
                std::thread::spawn(move || {
                let mut lecteur = BufReader::new(flux.try_clone().unwrap());
                let mut entete = String::new();
                let mut ligne = String::new();
                while lecteur.read_line(&mut ligne).is_ok_and(|n| n > 2) {
                    entete.push_str(&ligne);
                    ligne.clear();
                }
                let longueur = entete.lines().find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|n| n.trim().parse::<usize>().unwrap_or(0))).unwrap_or(0);
                let mut corps = vec![0; longueur];
                let _ = lecteur.read_exact(&mut corps);
                let chemin = entete.split_whitespace().nth(1).unwrap_or("/").to_string();
                let (statut, genre, texte) = reponse(&chemin, &entete, &String::from_utf8_lossy(&corps));
                let mut flux = flux;
                let _ = write!(flux, "HTTP/1.1 {statut} X\r\nContent-Type: {genre}\r\nContent-Length: {}\r\nMcp-Session-Id: s1\r\nConnection: close\r\n\r\n{texte}", texte.len());
                });
            }
        });
        adresse
    }

    #[test]
    fn json_local_delai_taille_et_secret_masque() {
        let adresse = serveur(|chemin, entetes, _| match chemin {
            "/json" => (200, "application/json", r#"{"a":{"b":[1,{"c":"ok"}]}}"#.into()),
            "/lent" => {
                std::thread::sleep(Duration::from_secs(5));
                (200, "text/plain", "trop tard".into())
            }
            "/gros" => (200, "text/plain", "x".repeat(TAILLE_MAX + 10)),
            // Un serveur qui renvoie l'en-tête reçu dans son erreur : le secret ne doit pas sortir.
            _ => (401, "text/plain", entetes.lines().find(|l| l.starts_with("Authorization")).unwrap_or("").to_string()),
        });
        let p = processus();
        let v = requete(&format!("{adresse}/json"), &[], None, Duration::from_secs(5), &p).unwrap().valeur().unwrap();
        assert_eq!(super::super::champ(&v, "a.b.1.c"), Some(&Value::from("ok")));

        let debut = Instant::now();
        let erreur = requete(&format!("{adresse}/lent"), &[], None, Duration::from_secs(1), &p).unwrap_err();
        assert!(debut.elapsed() < Duration::from_secs(3), "{:?}", debut.elapsed());
        assert!(erreur.contains("timed out"), "{erreur}");

        assert_eq!(requete(&format!("{adresse}/gros"), &[], None, Duration::from_secs(5), &p).unwrap_err(), "réponse de plus de 1 Mo");

        let mut secrets = Vec::new();
        let valeur = super::super::deverrouiller("Bearer trousseau:essai", &mut secrets, |_| Ok("cle-secrete-0123456789".into())).unwrap();
        let entetes = [("Authorization".to_string(), valeur)];
        let erreur = requete(&format!("{adresse}/prive"), &entetes, None, Duration::from_secs(5), &p).unwrap().valeur().unwrap_err();
        assert!(erreur.contains("cle-secrete"), "le serveur renvoie bien l'en-tête : {erreur}");
        let masque = super::super::masquer(&erreur, &secrets);
        assert_eq!(masque, "HTTP 401 : Authorization: Bearer •••");
        // Aucun processus curl ne reste inscrit.
        assert_eq!(p.lock().unwrap().as_ref().unwrap().len(), 0);
    }

    #[test]
    fn http_non_local_refuse_sans_reseau() {
        let p = processus();
        let erreur = requete("http://example.org/", &[], None, Duration::from_secs(1), &p).unwrap_err();
        assert!(erreur.contains("refusée"), "{erreur}");
        assert!(requete("https://x/", &[("A\nB".into(), "v".into())], None, Duration::from_secs(1), &p).unwrap_err().contains("invalide"));
    }

    /// L'adresse vient d'un fichier de barre, peut-être importé : un retour à la ligne n'y ajoute pas d'option à curl
    /// (`output = …` écrirait la réponse où le fichier le veut).
    #[test]
    fn une_adresse_n_ajoute_pas_d_option_a_curl() {
        let adresse = serveur(|_, _, _| (200, "text/plain", "contenu choisi par le serveur".into()));
        let cible = std::env::temp_dir().join(format!("tactile-injection-{}", std::process::id()));
        let _ = std::fs::remove_file(&cible);
        let p = processus();
        for url in [format!("{adresse}/x\noutput = {}\n#", cible.display()), format!("{adresse}/x\routput = {}\r#", cible.display())] {
            let _ = requete(&url, &[], None, Duration::from_secs(5), &p);
            assert!(!cible.exists(), "curl a écrit {} pour {url:?}", cible.display());
        }
        let _ = std::fs::remove_file(&cible);
    }

    /// Une sortie de curl tronquée ou mutée : réponse ou erreur, jamais de panique.
    #[test]
    fn sortie_de_curl_abimee_sans_panique() {
        let bonne = "HTTP/1.1 100 Continue\r\n\r\nHTTP/2 200\r\ncontent-type: application/json\r\nmcp-session-id: s1\r\n\r\n{\"é\":[1,2]}";
        let mut h = crate::essais::Hasard::new(5);
        for k in 0..2000 {
            let mut car: Vec<char> = bonne.chars().collect();
            match h.jusqua(3) {
                0 => car.truncate(h.jusqua(car.len() + 1)),
                1 => {
                    let i = h.jusqua(car.len());
                    car[i] = *h.parmi(&['\r', '\n', ':', ' ', 'H', '9', 'é', '\0']);
                }
                _ => {
                    let i = h.jusqua(car.len());
                    car.insert(i, *h.parmi(&['\r', '\n', ':', ' ']));
                }
            }
            let brut: String = car.into_iter().collect();
            let _ = crate::essais::sans_panique(|| analyser(&brut)).unwrap_or_else(|p| panic!("cas {k} : {brut:?} : {p}"));
        }
        assert_eq!(analyser("").unwrap_err(), "réponse HTTP illisible");
        assert_eq!(analyser("HTTP/1.1 404 Not Found\r\nX: y").unwrap().statut, 404);
        let erreur = Reponse { statut: 500, entetes: vec![], corps: "é".repeat(500) }.valeur().unwrap_err();
        assert_eq!(erreur.chars().count(), "HTTP 500 : ".chars().count() + 120);
    }

    #[test]
    fn reponse_apres_un_100_continue() {
        let r = analyser("HTTP/1.1 100 Continue\r\n\r\nHTTP/2 200\r\ncontent-type: application/json\r\n\r\n{}").unwrap();
        assert_eq!((r.statut, r.entete("content-type"), r.corps.as_str()), (200, Some("application/json"), "{}"));
    }
}
