//! « Ajouter un lien » : à partir d'une adresse, un bouton `{ url }` avec le titre du site et son icône.
//! Tout vient du site lui-même (jamais d'un service d'icônes tiers) : la page, puis `apple-touch-icon`, puis le favicon.
//! `/usr/bin/curl` comme `sources::http` : https seulement (ou http vers cette machine), 10 s, 1 Mo, redirections suivies
//! à la main (chacune revérifiée), une seule série de requêtes au moment du clic.

use super::icones;
use crate::barre::{controler_image, executer, Processus};
use crate::actions::Action;
use crate::config::Element;
use crate::sources::{http::guillemets, url_permise, DELAI, TAILLE_MAX};
use std::path::{Path, PathBuf};

const REDIRECTIONS: usize = 4;

fn suspecte(s: &str) -> bool {
    s.chars().any(|c| c.is_whitespace() || c.is_control() || c == '"' || c == '\\')
}

/// « youtube.com » → « `https://youtube.com` » ; refuse l'espace, les caractères de contrôle et ce que curl ne lirait pas.
pub(super) fn normaliser(saisie: &str) -> Result<String, String> {
    let s = saisie.trim();
    if s.is_empty() || suspecte(s) {
        return Err("écris une adresse sans espace, comme youtube.com".into());
    }
    let url = if s.contains("://") { s.to_string() } else { format!("https://{s}") };
    if !url_permise(&url) {
        return Err(format!("« {s} » refusée : seules les adresses https:// (et http://localhost) sont lues"));
    }
    Ok(url)
}

/// `scheme://autorité` d'une adresse.
fn origine(url: &str) -> &str {
    let debut = url.find("://").map_or(0, |i| i + 3);
    &url[..url[debut..].find(['/', '?', '#']).map_or(url.len(), |i| debut + i)]
}

/// L'hôte seul (sans identifiants, port ni crochets), en minuscules.
fn hote_brut(url: &str) -> String {
    let autorite = &origine(url)[url.find("://").map_or(0, |i| i + 3)..];
    let h = autorite.rsplit('@').next().unwrap_or(autorite).to_lowercase();
    match h.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or_default().to_string(),
        None => h.split(':').next().unwrap_or_default().to_string(),
    }
}

fn hote(url: &str) -> String {
    let h = hote_brut(url);
    h.strip_prefix("www.").unwrap_or(&h).to_string()
}

/// Cette machine : localhost et ses sous-domaines, 127.x.x.x, 0.0.0.0, ::1.
fn locale(url: &str) -> bool {
    let h = hote_brut(url);
    h == "localhost" || h.ends_with(".localhost") || h == "0.0.0.0" || h == "::1" || (h.starts_with("127.") && h.bytes().all(|o| o.is_ascii_digit() || o == b'.'))
}

/// `href` résolu contre la page `base` (absolu, `//hôte`, `/chemin` ou relatif).
fn resoudre(base: &str, href: &str) -> String {
    let href = href.trim();
    if href.contains("://") {
        href.to_string()
    } else if let Some(reste) = href.strip_prefix("//") {
        format!("{}://{reste}", base.split("://").next().unwrap_or("https"))
    } else if href.starts_with('/') {
        format!("{}{href}", origine(base))
    } else {
        let sans = base.split(['?', '#']).next().unwrap_or(base);
        let o = origine(sans).len();
        let dossier = &sans[..sans[o..].rfind('/').map_or(o, |i| o + i)];
        format!("{dossier}/{href}")
    }
}

/// Un curl : la réponse va dans `sortie`. Rend (statut, adresse de redirection éventuelle).
fn lire(url: &str, sortie: &Path, borner: bool, processus: &Processus) -> Result<(u16, Option<String>), String> {
    let mut config = format!(
        "url = {}\noutput = {}\nwrite-out = \"%{{http_code}} %{{redirect_url}}\"\nheader = \"User-Agent: Mozilla/5.0 (Macintosh) Tactile\"\n",
        guillemets(url),
        guillemets(&sortie.to_string_lossy())
    );
    if !borner {
        config.push_str("range = \"0-1048575\"\n");
    }
    let proto = if url.starts_with("https://") { "=https" } else { "=http" };
    let (secondes, max) = (DELAI.as_secs().to_string(), TAILLE_MAX.to_string());
    // --limit-rate : une page sans taille annoncée ne remplit pas le disque pendant les 10 s.
    let mut args = vec!["-q", "-sS", "--max-time", &secondes, "--limit-rate", "2M", "--proto", proto, "--config", "-"];
    if borner {
        args.extend(["--max-filesize", &max]);
    }
    let s = executer(std::process::Command::new("/usr/bin/curl").args(args), Some(&config), DELAI + std::time::Duration::from_secs(2), processus);
    if s.erreur.contains("(63)") {
        return Err("fichier de plus de 1 Mo".into());
    }
    // Une page coupée par le délai reste lisible (son début suffit) ; une icône coupée ne l'est pas.
    if !s.erreur.is_empty() && borner {
        return Err(s.erreur.replace("code 28, ", "").replace("curl: ", ""));
    }
    let (statut, suite) = s.texte.split_once(' ').unwrap_or((&s.texte, ""));
    let statut: u16 = statut.trim().parse().unwrap_or(0);
    if statut == 0 {
        return Err(if s.erreur.is_empty() { "le site ne répond pas".to_string() } else { s.erreur.replace("code 28, ", "").replace("curl: ", "") });
    }
    Ok((statut, Some(suite.trim().to_string()).filter(|r| !r.is_empty())))
}

/// Télécharge `url` dans `sortie` en suivant au plus `REDIRECTIONS` redirections ; rend l'adresse finale.
fn telecharger(url: &str, sortie: &Path, borner: bool, distant: bool, processus: &Processus) -> Result<String, String> {
    let mut url = url.to_string();
    for _ in 0..=REDIRECTIONS {
        if distant && locale(&url) {
            return Err("adresse locale refusée : un site distant ne peut pas en désigner une".into());
        }
        if !url_permise(&url) || suspecte(&url) {
            return Err(format!("« {} » refusée : seules les adresses https:// (et http://localhost) sont lues", url.chars().take(80).collect::<String>()));
        }
        let (statut, suite) = lire(&url, sortie, borner, processus)?;
        match (statut, suite) {
            (300..=399, Some(vers)) => url = vers,
            (200..=299, _) => return Ok(url),
            (s, _) => return Err(format!("le site répond HTTP {s}")),
        }
    }
    Err("trop de redirections".into())
}

/// La valeur de l'attribut `nom` dans le texte d'une balise.
fn attribut(balise: &str, nom: &str) -> Option<String> {
    let bas = balise.to_ascii_lowercase();
    let mut depuis = 0;
    while let Some(i) = bas[depuis..].find(nom) {
        let debut = depuis + i;
        depuis = debut + nom.len();
        let avant_ok = bas[..debut].ends_with(|c: char| c.is_whitespace() || c == '"' || c == '\'');
        let Some(reste) = balise[depuis..].trim_start().strip_prefix('=').filter(|_| avant_ok) else { continue };
        let reste = reste.trim_start();
        return Some(match reste.chars().next()? {
            q @ ('"' | '\'') => reste[1..].split(q).next()?.to_string(),
            _ => reste.split(|c: char| c.is_whitespace() || c == '>').next()?.to_string(),
        });
    }
    None
}

/// Les adresses d'icônes de la page, dans l'ordre de préférence : apple-touch-icon, icon, puis `/favicon.ico`.
fn icones_de(html: &str, base: &str) -> Vec<String> {
    let bas = html.to_ascii_lowercase();
    let (mut apple, mut simples) = (Vec::new(), Vec::new());
    let mut depuis = 0;
    while let Some(i) = bas[depuis..].find("<link") {
        let debut = depuis + i;
        let fin = bas[debut..].find('>').map_or(bas.len(), |f| debut + f);
        depuis = fin.max(debut + 5);
        let balise = &html[debut + 5..fin];
        let (Some(rel), Some(href)) = (attribut(balise, "rel"), attribut(balise, "href")) else { continue };
        let rel = rel.to_ascii_lowercase();
        let url = resoudre(base, &decoder(&href));
        if rel.split_whitespace().any(|r| r.starts_with("apple-touch-icon")) {
            apple.push(url);
        } else if rel.split_whitespace().any(|r| r == "icon") {
            simples.push(url);
        }
    }
    let mut tous = [apple, simples, vec![format!("{}/favicon.ico", origine(base))]].concat();
    tous.dedup();
    tous.truncate(4);
    tous
}

fn decoder(texte: &str) -> String {
    texte.replace("&quot;", "\"").replace("&#39;", "'").replace("&#x27;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// Le contenu de `<title>`, espaces réduits, entités courantes décodées, 24 caractères au plus.
fn titre(html: &str) -> Option<String> {
    let bas = html.to_ascii_lowercase();
    let debut = bas.find("<title")?;
    let debut = debut + bas[debut..].find('>')? + 1;
    let fin = debut + bas[debut..].find("</title")?;
    let t = decoder(&html[debut..fin]).split_whitespace().collect::<Vec<_>>().join(" ");
    let court: String = t.chars().take(24).collect();
    Some(if t.chars().count() > 24 { format!("{}…", court.trim_end()) } else { court }).filter(|t| !t.is_empty())
}

/// L'extension d'après les premiers octets : seuls les formats que `NSImage` lit sans surprise (pas de SVG venu du réseau).
fn extension(octets: &[u8]) -> Option<&'static str> {
    match octets {
        [0x89, b'P', b'N', b'G', ..] => Some("png"),
        [0xFF, 0xD8, 0xFF, ..] => Some("jpg"),
        [b'G', b'I', b'F', b'8', ..] => Some("gif"),
        [0, 0, 1, 0, ..] => Some("ico"),
        _ => None,
    }
}

/// Dossier de travail jetable, supprimé à la fin.
struct Temporaire(PathBuf);

impl Temporaire {
    fn nouveau() -> Result<Self, String> {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        let d = std::env::temp_dir().join(format!("tactile-lien-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&d).map_err(|e| format!("dossier temporaire : {e}"))?;
        Ok(Self(d))
    }
}

impl Drop for Temporaire {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Une icône du site, téléchargée, validée (`controler_image`, 1 Mo) puis copiée dans `icones/` : son chemin, ou `None`.
fn icone(url: &str, hote: &str, dossier: &Path, tmp: &Path, distant: bool, processus: &Processus) -> Option<PathBuf> {
    let brut = tmp.join("icone.bin");
    telecharger(url, &brut, true, distant, processus).ok()?;
    let octets = std::fs::read(&brut).ok().filter(|o| o.len() <= TAILLE_MAX)?;
    let fichier = tmp.join(format!("{hote}.{}", extension(&octets)?));
    std::fs::rename(&brut, &fichier).ok()?;
    controler_image(&fichier.to_string_lossy()).ok()?;
    icones::importer_en_png(&fichier, dossier).ok()
}

/// Le bouton du lien. Sans icône trouvée, le symbole `globe`.
pub(super) fn recuperer(saisie: &str, dossier: &Path, processus: &Processus) -> Result<Element, String> {
    let adresse = normaliser(saisie)?;
    // Tapée par l'utilisateur, une adresse locale est permise ; une page distante, elle, ne mène jamais à cette machine.
    let distant = !locale(&adresse);
    let tmp = Temporaire::nouveau()?;
    let page = tmp.0.join("page.html");
    let finale = telecharger(&adresse, &page, false, distant, processus)?;
    let octets: Vec<u8> = std::fs::read(&page).unwrap_or_default().into_iter().take(TAILLE_MAX).collect();
    let html = String::from_utf8_lossy(&octets).into_owned();
    let nom = hote(&finale);
    let image = icones_de(&html, &finale).iter().find_map(|u| icone(u, &nom, dossier, &tmp.0, distant, processus));
    Ok(Element::Bouton {
        texte: Some(titre(&html).unwrap_or(nom)),
        symbole: image.is_none().then(|| "globe".to_string()),
        image: image.map(|c| icones::avec_tilde(&c)),
        gabarit: false,
        couleur: None,
        couleur_texte: None,
        taille: None,
        affichage: None,
        largeur: None,
        action: Some(Action::Url(adresse)),
        appui_long: None,
        source: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};

    const ICNS: &str = "/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/GenericApplicationIcon.icns";

    /// Serveur local qui rend des octets : `reponse(chemin)` donne (statut, en-têtes en plus, corps).
    fn serveur(reponse: impl Fn(&str) -> (u16, String, Vec<u8>) + Send + Sync + 'static) -> String {
        let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = format!("http://127.0.0.1:{}", ecoute.local_addr().unwrap().port());
        let reponse = std::sync::Arc::new(reponse);
        std::thread::spawn(move || {
            for flux in ecoute.incoming().flatten() {
                let reponse = reponse.clone();
                std::thread::spawn(move || {
                    let mut lecteur = BufReader::new(flux.try_clone().unwrap());
                    let mut ligne = String::new();
                    lecteur.read_line(&mut ligne).unwrap();
                    let chemin = ligne.split_whitespace().nth(1).unwrap_or("/").to_string();
                    while lecteur.read_line(&mut ligne).is_ok_and(|n| n > 2) {}
                    let (statut, plus, corps) = reponse(&chemin);
                    let mut flux = flux;
                    let _ = write!(flux, "HTTP/1.1 {statut} X\r\n{plus}Content-Length: {}\r\nConnection: close\r\n\r\n", corps.len());
                    let _ = flux.write_all(&corps);
                });
            }
        });
        adresse
    }

    fn processus() -> Processus {
        Processus::new(std::sync::Mutex::new(Some(Vec::new())))
    }

    #[test]
    fn adresses_et_balises() {
        assert_eq!(normaliser(" youtube.com ").unwrap(), "https://youtube.com");
        assert_eq!(normaliser("http://localhost:3000/x").unwrap(), "http://localhost:3000/x");
        assert!(normaliser("http://example.org").unwrap_err().contains("refusée"));
        assert!(normaliser("a b").is_err() && normaliser("").is_err() && normaliser("a\"b").is_err());
        assert_eq!(hote("https://www.youtube.com:443/watch?v=1"), "youtube.com");
        assert_eq!(resoudre("https://a.fr/x/y.html?q=1", "i.png"), "https://a.fr/x/i.png");
        assert_eq!(resoudre("https://a.fr/x/y.html", "/i.png"), "https://a.fr/i.png");
        assert_eq!(resoudre("https://a.fr", "//cdn.b.fr/i.png"), "https://cdn.b.fr/i.png");
        let html = "<head><TITLE>\n  Ma  page &amp; moi \n</title><link rel='shortcut icon' href=/f.ico><link rel=\"apple-touch-icon\" sizes=\"180x180\" href=\"/t.png\"><link rel=stylesheet href=s.css>";
        assert_eq!(titre(html).unwrap(), "Ma page & moi");
        assert_eq!(icones_de(html, "https://a.fr/p"), ["https://a.fr/t.png", "https://a.fr/f.ico", "https://a.fr/favicon.ico"]);
        assert_eq!(titre(&format!("<title>{}</title>", "x".repeat(40))).unwrap().chars().count(), 25);
        assert_eq!(titre("<p>rien"), None);
    }

    /// Un site distant ne fait pas lire un service de cette machine : ni par redirection, ni par l'adresse d'une icône.
    #[test]
    fn un_site_distant_ne_mene_pas_au_local() {
        for local in ["http://localhost:3000/x", "https://localhost/x", "http://127.0.0.1:8080", "https://127.1.2.3/", "http://[::1]:80/", "https://app.localhost/", "http://0.0.0.0/", "https://user@localhost/"] {
            assert!(locale(local), "{local}");
        }
        for distant in ["https://youtube.com", "https://localhost.example.org/", "https://example.org/127.0.0.1", "https://notlocalhost.fr"] {
            assert!(!locale(distant), "{distant}");
        }
        let serveur = serveur(|chemin| match chemin {
            "/ancien" => (301, "Location: /\r\n".into(), vec![]),
            _ => (200, String::new(), b"x".to_vec()),
        });
        let (tmp, p) = (std::env::temp_dir().join(format!("tactile-distant-{}", std::process::id())), processus());
        std::fs::create_dir_all(&tmp).unwrap();
        // Départ local (l'utilisateur l'a tapé) : la redirection locale passe. Départ distant : le local est refusé.
        assert!(telecharger(&format!("{serveur}/ancien"), &tmp.join("a"), false, false, &p).is_ok());
        assert_eq!(telecharger(&format!("{serveur}/ancien"), &tmp.join("b"), false, true, &p).unwrap_err(), "adresse locale refusée : un site distant ne peut pas en désigner une");
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    #[cfg_attr(ci, ignore = "AppKit rendering into a bitmap context: no graphics session on a CI runner")]
    fn lien_avec_titre_icone_stable_et_repli_globe() {
        let d = std::env::temp_dir().join(format!("tactile-liens-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let png = std::fs::read(icones::importer(Path::new(ICNS), &d.join("src")).unwrap()).unwrap();
        let serveur = serveur(move |chemin| match chemin {
            "/" => (200, String::new(), "<title>Vidéos</title><link rel=\"apple-touch-icon\" href=\"/logo.png\">".into()),
            "/ancien" => (301, "Location: /\r\n".into(), vec![]),
            "/sans-icone" => (200, String::new(), b"<title>Sans</title>".to_vec()),
            "/logo.png" => (200, String::new(), png.clone()),
            "/gros" => (200, String::new(), b"<link rel=icon href=/enorme.png>".to_vec()),
            "/enorme.png" => (200, String::new(), [&png[..], &vec![0; TAILLE_MAX][..]].concat()),
            _ => (404, String::new(), vec![]),
        });
        let (dossier, p) = (d.join("icones"), processus());
        let Element::Bouton { texte, image, symbole, action, .. } = recuperer(&serveur, &dossier, &p).unwrap() else { panic!() };
        assert_eq!(texte.as_deref(), Some("Vidéos"));
        assert_eq!(symbole, None);
        assert_eq!(action, Some(Action::Url(serveur.clone())));
        let image = image.unwrap();
        let nom = image.rsplit('/').next().unwrap();
        assert!(nom.starts_with("127-0-0-1-") && nom.ends_with(".png") && nom.len() == "127-0-0-1-".len() + 8 + 4, "{nom}");
        // Même site, même contenu : même nom, rien de plus dans icones/, et la redirection est suivie.
        let Element::Bouton { image: encore, .. } = recuperer(&format!("{serveur}/ancien"), &dossier, &p).unwrap() else { panic!() };
        assert_eq!(encore.as_deref(), Some(image.as_str()));
        assert_eq!(std::fs::read_dir(&dossier).unwrap().count(), 1);
        // Pas d'icône (favicon 404) ou icône de plus de 1 Mo : le symbole globe, le titre reste.
        for chemin in ["sans-icone", "gros"] {
            let Element::Bouton { texte, image, symbole, .. } = recuperer(&format!("{serveur}/{chemin}"), &dossier, &p).unwrap() else { panic!() };
            assert_eq!((image, symbole.as_deref()), (None, Some("globe")), "{chemin}");
            assert!(texte.is_some());
        }
        assert_eq!(recuperer(&format!("{serveur}/absent"), &dossier, &p).unwrap_err(), "le site répond HTTP 404");
        assert!(recuperer("http://example.org", &dossier, &p).unwrap_err().contains("refusée"));
        assert_eq!(p.lock().unwrap().as_ref().unwrap().len(), 0, "aucun curl ne reste");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
