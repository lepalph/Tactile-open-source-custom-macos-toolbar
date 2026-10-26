//! Sources de données (`[[source]]`) : une valeur JSON relue par HTTP, une commande ou un serveur MCP,
//! que les éléments `texte` (et les boutons qui ont une `source`) affichent par un gabarit.
//!
//! Chaque source a son fil, lancé une fois la barre affichée : il lit, envoie la valeur par un canal (relu au `tic`),
//! puis attend `toutes_les` secondes sur son canal d'arrêt. La barre remplacée lâche ce canal : le fil s'arrête,
//! et `tuer_tous` a déjà tué ses processus (`curl`, commande, serveur MCP), inscrits dans `Etat::processus`.

pub(crate) mod http;
pub(crate) mod mcp;

use crate::barre::{executer, Processus};
use crate::config::{CouleurSelon, Source, SymboleSelon};
use serde_json::Value;
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::time::Duration;

/// Intervalle minimum entre deux lectures, en secondes.
pub(crate) const MINIMUM: f64 = 5.0;
/// Intervalle sans `toutes_les`.
pub(crate) const DEFAUT: f64 = 60.0;
/// Attente maximale avant de relancer un serveur MCP qui meurt.
const ATTENTE_MAX: Duration = Duration::from_secs(300);
/// Délai d'une requête HTTP, d'une réponse MCP ou d'une commande.
pub(crate) const DELAI: Duration = Duration::from_secs(10);
/// Taille maximale d'une réponse ou d'une sortie.
pub(crate) const TAILLE_MAX: usize = 1 << 20;
/// Ce qu'affiche un champ absent.
pub(crate) const ABSENT: &str = "—";

/// Ce que renvoie le fil d'une source : son index dans `Barre::sources`, et sa valeur ou ce qui a cloché.
pub(crate) type Valeur = (usize, Result<Value, String>);

/// `https://…`, ou `http://` vers cette machine seulement.
pub(crate) fn url_permise(url: &str) -> bool {
    if url.starts_with("https://") {
        return true;
    }
    let Some(reste) = url.strip_prefix("http://") else { return false };
    ["localhost", "127.0.0.1", "[::1]"].iter().any(|hote| reste.strip_prefix(hote).is_some_and(|r| r.is_empty() || r.starts_with([':', '/', '?'])))
}

/// Une longue suite de caractères de clé (20 ou plus) sans `trousseau:` : sans doute une clé écrite en clair.
pub(crate) fn cle_en_clair(valeur: &str) -> bool {
    !valeur.contains("trousseau:")
        && valeur.split_whitespace().any(|mot| mot.len() >= 20 && mot.chars().all(|c| c.is_ascii_alphanumeric() || "-_.=+/".contains(c)))
}

/// Remplace chaque `trousseau:service` par le mot de passe générique `service` du trousseau ; les secrets lus
/// vont dans `secrets`, pour être masqués dans les erreurs.
pub(crate) fn deverrouiller(valeur: &str, secrets: &mut Vec<String>, lire: impl Fn(&str) -> Result<String, String>) -> Result<String, String> {
    let mut sortie = String::new();
    let mut reste = valeur;
    while let Some(debut) = reste.find("trousseau:") {
        sortie.push_str(&reste[..debut]);
        let apres = &reste[debut + "trousseau:".len()..];
        let fin = apres.find(char::is_whitespace).unwrap_or(apres.len());
        let secret = lire(&apres[..fin])?;
        sortie.push_str(&secret);
        secrets.push(secret);
        reste = &apres[fin..];
    }
    sortie.push_str(reste);
    Ok(sortie)
}

/// Le mot de passe générique `service` du trousseau (`security`, hors du fil principal).
pub(crate) fn lire_trousseau(service: &str, processus: &Processus) -> Result<String, String> {
    // 60 s : le trousseau peut demander l'autorisation à l'utilisateur.
    let sortie = executer(std::process::Command::new("/usr/bin/security").args(["find-generic-password", "-s", service, "-w"]), None, Duration::from_secs(60), processus);
    let secret = sortie.texte.trim_end_matches('\n');
    if !sortie.erreur.is_empty() || secret.is_empty() {
        return Err(format!("trousseau : aucun mot de passe pour le service « {service} »"));
    }
    Ok(secret.to_string())
}

/// Un secret ne sort jamais dans le journal ni dans un message.
pub(crate) fn masquer(texte: &str, secrets: &[String]) -> String {
    secrets.iter().filter(|s| !s.is_empty()).fold(texte.to_string(), |t, s| t.replace(s.as_str(), "•••"))
}

/// Sortie d'une commande, corps HTTP ou texte MCP : un objet ou une liste JSON, sinon le texte, sous `texte`.
pub(crate) fn valeur_de_texte(texte: &str) -> Value {
    let texte = texte.trim();
    match serde_json::from_str::<Value>(texte) {
        Ok(v @ (Value::Object(_) | Value::Array(_))) => v,
        _ => serde_json::json!({ "texte": texte }),
    }
}

/// La valeur au chemin `a.b.0.c` (un nombre indexe une liste) ; le chemin vide donne toute la valeur.
pub(crate) fn champ<'a>(valeur: &'a Value, chemin: &str) -> Option<&'a Value> {
    if chemin.is_empty() {
        return Some(valeur);
    }
    chemin.split('.').try_fold(valeur, |v, cle| match v {
        Value::Object(table) => table.get(cle),
        Value::Array(liste) => liste.get(cle.parse::<usize>().ok()?),
        _ => None,
    })
}

/// Un texte tel quel, un nombre ou un booléen écrit comme en JSON ; `null` compte comme absent.
fn en_texte(valeur: &Value) -> Option<String> {
    match valeur {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        autre => Some(autre.to_string()),
    }
}

/// Ce qu'un élément lié affiche, et les champs qu'il n'a pas trouvés.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Rendu {
    pub texte: String,
    pub symbole: Option<String>,
    pub couleur: Option<String>,
    pub manquants: Vec<String>,
}

/// Remplit `gabarit` (`{a.b}` → valeur, `—` si absente), et choisit symbole et couleur selon la valeur.
pub(crate) fn rendre(gabarit: &str, symbole: Option<&SymboleSelon>, couleur: Option<&CouleurSelon>, valeur: &Value) -> Rendu {
    let mut rendu = Rendu::default();
    let mut lire = |chemin: &str| {
        let v = champ(valeur, chemin).and_then(en_texte);
        if v.is_none() && !rendu.manquants.iter().any(|m| m == chemin) {
            rendu.manquants.push(chemin.to_string());
        }
        v
    };
    let mut reste = gabarit;
    let mut texte = String::new();
    while let Some(debut) = reste.find('{') {
        let Some(fin) = reste[debut..].find('}') else { break };
        texte.push_str(&reste[..debut]);
        texte.push_str(&lire(reste[debut + 1..debut + fin].trim()).unwrap_or_else(|| ABSENT.into()));
        reste = &reste[debut + fin + 1..];
    }
    texte.push_str(reste);
    if let Some(s) = symbole {
        rendu.symbole = lire(&s.champ).and_then(|cle| s.valeurs.get(&cle).cloned()).or_else(|| s.defaut.clone());
    }
    if let Some(c) = couleur {
        rendu.couleur = lire(&c.champ).and_then(|v| v.parse::<f64>().ok()).and_then(|n| c.couleurs.get(c.seuils.iter().filter(|&&s| n >= s).count()).cloned());
    }
    rendu.texte = texte;
    rendu
}

/// Lance un fil par source ; lâcher les `Sender` rendus les arrête.
pub(crate) fn demarrer(sources: &[Source], envoi: Sender<Valeur>, processus: &Processus) -> Vec<Sender<()>> {
    sources
        .iter()
        .enumerate()
        .map(|(k, source)| {
            let (arret, arrete) = channel::<()>();
            let (source, envoi, processus) = (source.clone(), envoi.clone(), processus.clone());
            std::thread::spawn(move || {
                let intervalle = Duration::from_secs_f64(source.toutes_les.unwrap_or(DEFAUT).max(MINIMUM));
                let premiere_attente = Duration::from_secs_f64(MINIMUM);
                let (mut client, mut attente, mut derniere) = (None, premiere_attente, String::new());
                loop {
                    let (resultat, relancer) = lire(&source, &mut client, &processus);
                    let pause = if relancer {
                        client = None;
                        let pause = attente;
                        attente = (attente * 2).min(ATTENTE_MAX);
                        pause
                    } else {
                        attente = premiere_attente;
                        intervalle
                    };
                    // Une fois par changement, pas à chaque lecture.
                    let erreur = resultat.as_ref().err().cloned().unwrap_or_default();
                    if erreur != derniere {
                        if !erreur.is_empty() {
                            eprintln!("tactile : source « {} » : {erreur}", source.nom);
                        }
                        derniere = erreur;
                    }
                    if envoi.send((k, resultat)).is_err() || arrete.recv_timeout(pause) != Err(RecvTimeoutError::Timeout) {
                        break;
                    }
                }
            });
            arret
        })
        .collect()
}

/// Une lecture de la source ; `true` : le serveur MCP est à relancer (après une attente croissante).
fn lire(source: &Source, client: &mut Option<mcp::Client>, processus: &Processus) -> (Result<Value, String>, bool) {
    let mut secrets = Vec::new();
    let entetes = |secrets: &mut Vec<String>| -> Result<Vec<(String, String)>, String> {
        source.entetes.iter().map(|(nom, v)| Ok((nom.clone(), deverrouiller(v, secrets, |s| lire_trousseau(s, processus))?))).collect()
    };
    let (resultat, relancer) = if let Some(url) = &source.http {
        let reponse = entetes(&mut secrets).and_then(|e| http::requete(url, &e, None, DELAI, processus));
        (reponse.and_then(|r| r.valeur()), false)
    } else if let Some(commande) = &source.commande {
        let sortie = executer(std::process::Command::new("/bin/zsh").args(["-c", commande]), None, DELAI, processus);
        let resultat = match sortie {
            s if s.texte.len() > TAILLE_MAX => Err("sortie de plus de 1 Mo".into()),
            s if !s.erreur.is_empty() && s.texte.trim().is_empty() => Err(s.erreur),
            s => Ok(valeur_de_texte(&s.texte)),
        };
        (resultat, false)
    } else if let Some(m) = &source.mcp {
        if client.is_none() {
            let nouveau = match (&m.commande, &m.url) {
                (Some(commande), _) => mcp::Client::stdio(commande, processus, DELAI),
                (None, Some(url)) => entetes(&mut secrets).and_then(|e| mcp::Client::http(url, e, processus, DELAI)),
                (None, None) => Err("mcp sans commande ni url".into()),
            };
            match nouveau {
                Ok(c) => *client = Some(c),
                Err(e) => return (Err(masquer(&e, &secrets)), true),
            }
        }
        let c = client.as_mut().expect("créé juste au-dessus");
        secrets.extend(c.secrets.iter().cloned());
        let arguments = m.arguments.clone().unwrap_or_else(|| serde_json::json!({}));
        match c.appeler(&m.outil, arguments) {
            Ok(v) => (Ok(v), false),
            Err(e) => (Err(e.to_string()), e.relancer()),
        }
    } else {
        (Err("ni http, ni commande, ni mcp".into()), false)
    };
    (resultat.map_err(|e| masquer(&e, &secrets)), relancer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn chemins_de_champ() {
        let v = json!({ "current": { "temperature_2m": 12.5, "weather_code": 3 }, "liste": [{ "b": "x" }, 4], "vide": null });
        assert_eq!(champ(&v, "current.temperature_2m"), Some(&json!(12.5)));
        assert_eq!(champ(&v, "liste.0.b"), Some(&json!("x")));
        assert_eq!(champ(&v, "liste.1"), Some(&json!(4)));
        assert_eq!(champ(&v, ""), Some(&v));
        for absent in ["current.pluie", "liste.2", "liste.x", "current.temperature_2m.c"] {
            assert_eq!(champ(&v, absent), None, "{absent}");
        }
    }

    #[test]
    fn gabarits() {
        let v = json!({ "a": { "b": 12.5, "s": "ok", "n": null, "v": true }, "l": [1, 2] });
        let r = rendre("{a.b} °C · {a.s} · {a.v} · {l.1}", None, None, &v);
        assert_eq!(r, Rendu { texte: "12.5 °C · ok · true · 2".into(), ..Rendu::default() });
        // Absent ou null : « — », et le chemin est signalé une fois.
        let r = rendre("{a.x}/{a.x}/{a.n}", None, None, &v);
        assert_eq!((r.texte.as_str(), r.manquants), ("—/—/—", vec!["a.x".to_string(), "a.n".to_string()]));
        // Une accolade seule reste telle quelle ; une sortie texte se lit par {texte}.
        assert_eq!(rendre("x { y", None, None, &v).texte, "x { y");
        assert_eq!(rendre("{texte} !", None, None, &valeur_de_texte("bonjour\n")).texte, "bonjour !");
        assert_eq!(valeur_de_texte(" [1] "), json!([1]));
        assert_eq!(valeur_de_texte("42"), json!({ "texte": "42" }));
    }

    #[test]
    fn symbole_et_couleur_selon_la_valeur() {
        let symbole = SymboleSelon { champ: "code".into(), valeurs: [("0".to_string(), "sun.max.fill".to_string()), ("3".into(), "cloud.fill".into())].into(), defaut: Some("cloud.sun.fill".into()) };
        let couleur = CouleurSelon { champ: "t".into(), seuils: vec![10.0, 25.0], couleurs: vec!["#0A84FF".into(), "#30D158".into(), "#FF9F0A".into()] };
        let rendu = |v: Value| { let r = rendre("", Some(&symbole), Some(&couleur), &v); (r.symbole, r.couleur) };
        assert_eq!(rendu(json!({ "code": 3, "t": 9.9 })), (Some("cloud.fill".into()), Some("#0A84FF".into())));
        assert_eq!(rendu(json!({ "code": "0", "t": 10 })), (Some("sun.max.fill".into()), Some("#30D158".into())));
        assert_eq!(rendu(json!({ "code": 61, "t": 30 })), (Some("cloud.sun.fill".into()), Some("#FF9F0A".into())));
        assert_eq!(rendu(json!({})), (Some("cloud.sun.fill".into()), None));
    }

    #[test]
    fn trousseau_et_secrets_masques() {
        let mut secrets = Vec::new();
        let lire = |s: &str| if s == "meteo" { Ok("s3cr3t-tres-long".to_string()) } else { Err(format!("trousseau : aucun mot de passe pour le service « {s} »")) };
        assert_eq!(deverrouiller("Bearer trousseau:meteo", &mut secrets, lire), Ok("Bearer s3cr3t-tres-long".into()));
        assert_eq!(deverrouiller("sans secret", &mut secrets, lire), Ok("sans secret".into()));
        assert!(deverrouiller("Bearer trousseau:autre", &mut secrets, lire).unwrap_err().contains("« autre »"));
        assert_eq!(masquer("401 : Bearer s3cr3t-tres-long refusé", &secrets), "401 : Bearer ••• refusé");
        // Le vrai trousseau : un service qui n'existe pas donne une erreur sans rien d'autre.
        let p = Processus::new(std::sync::Mutex::new(Some(Vec::new())));
        assert!(lire_trousseau("tactile-service-qui-nexiste-pas", &p).unwrap_err().starts_with("trousseau : aucun mot de passe"));
        assert!(cle_en_clair("Bearer sk_live_0123456789abcdefghij") && !cle_en_clair("Bearer trousseau:meteo") && !cle_en_clair("application/json"));
    }

    /// Le fil d'une source MCP de bout en bout : valeur affichée, serveur tué, `⚠ source`, relance, arrêt.
    #[test]
    fn source_mcp_affichee_tuee_puis_relancee() {
        let serveur = concat!(env!("CARGO_MANIFEST_DIR"), "/exemples/mcp/horloge-mcp.py");
        let texte = format!("[[source]]\nnom = \"h\"\ntoutes_les = 5\nmcp = {{ commande = \"python3 {serveur}\", outil = \"heure\" }}\n");
        let barre = crate::config::lire(&texte).unwrap();
        let p = Processus::new(std::sync::Mutex::new(Some(Vec::new())));
        let (envoi, valeurs) = channel();
        let arrets = demarrer(&barre.sources, envoi, &p);
        let afficher = |v: &Result<Value, String>| v.as_ref().map(|v| rendre("MCP {heure}", None, None, v).texte).unwrap_or_else(|_| "⚠ source".into());
        let premiere = valeurs.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(premiere.0, 0);
        assert!(afficher(&premiere.1).starts_with("MCP ") && afficher(&premiere.1).len() == 12, "{:?}", premiere.1);
        let pid = p.lock().unwrap().as_ref().unwrap()[0];
        crate::barre::tuer_groupe(pid);
        assert_eq!(afficher(&valeurs.recv_timeout(Duration::from_secs(8)).unwrap().1), "⚠ source");
        // Relancé 5 s plus tard, par un nouveau processus.
        let relance = valeurs.recv_timeout(Duration::from_secs(8)).unwrap();
        assert!(relance.1.is_ok(), "{:?}", relance.1);
        assert_ne!(p.lock().unwrap().as_ref().unwrap()[0], pid);
        // La barre remplacée : processus tués, fil arrêté (le canal se ferme).
        crate::barre::tuer_tous(&p);
        drop(arrets);
        assert!(valeurs.recv_timeout(Duration::from_secs(3)).is_err_and(|e| e == RecvTimeoutError::Disconnected));
    }

    #[test]
    fn seules_https_et_localhost() {
        for (url, permise) in [
            ("https://api.open-meteo.com/v1/forecast", true),
            ("http://localhost:8080/x", true),
            ("http://127.0.0.1/x", true),
            ("http://localhost", true),
            ("http://example.org/x", false),
            ("http://localhost.example.org/x", false),
            ("file:///etc/passwd", false),
            ("ftp://x", false),
        ] {
            assert_eq!(url_permise(url), permise, "{url}");
        }
    }
}
