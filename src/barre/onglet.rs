//! Barre par site (`[[site]]`) : l'adresse de l'onglet actif du navigateur au premier plan.
//!
//! Lue par AppleScript (`NSAppleScript` compilé une fois par navigateur), sur un fil dédié, toutes les 0,5 s et
//! seulement tant qu'un navigateur connu est au premier plan et que la config a un `[[site]]`. Le fil ne garde que
//! « hôte/chemin » (sans schéma, identifiants, port, `?…` ni `#…`), en mémoire : rien n'est journalisé ni écrit.
//! Mesures et choix face à `osascript` : docs/ARCHITECTURE.md, « Barre par site ».

use super::{texte, Id};
use objc2::runtime::{AnyObject, Bool};
use objc2::{class, msg_send, sel};
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::Duration;

/// Bundle id, nom affiché, et vrai pour Safari (`current tab`) ; les autres sont des Chromium (`active tab`).
pub(crate) const NAVIGATEURS: [(&str, &str, bool); 7] = [
    ("com.apple.Safari", "Safari", true),
    ("com.google.Chrome", "Google Chrome", false),
    ("company.thebrowser.Browser", "Arc", false),
    ("com.brave.Browser", "Brave", false),
    ("com.microsoft.edgemac", "Microsoft Edge", false),
    ("com.vivaldi.Vivaldi", "Vivaldi", false),
    ("org.chromium.Chromium", "Chromium", false),
];

/// `errAEEventNotPermitted` : l'utilisateur a refusé l'Automatisation de ce navigateur à Tactile.
pub(crate) const REFUSE: isize = -1743;

/// Réglages Système, Confidentialité et sécurité, Automatisation : où lever le refus.
pub(super) const REGLAGES: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_Automation";

/// Lecture toutes les 0,5 s ; après une erreur (refus, navigateur qui ne répond pas), toutes les 5 s.
const PERIODE: Duration = Duration::from_millis(500);
const PERIODE_ERREUR: Duration = Duration::from_secs(5);

/// Ce que le fil a lu pour un navigateur : la page (« hôte/chemin »), aucune (pas de fenêtre, page locale), ou l'erreur AppleScript.
pub(super) type Lecture = Result<Option<String>, isize>;

static VERS_LE_FIL: Mutex<Option<Sender<Option<String>>>> = Mutex::new(None);
/// Autotest : la page est donnée à la main (`barre::recevoir_page`), le fil ne lit plus rien.
pub(super) static SIMULE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// La dernière lecture qui a changé, reprise par `ongletChange:` sur le fil principal.
static DERNIERE: Mutex<Option<(String, Lecture)>> = Mutex::new(None);

/// Le nom d'un navigateur connu (bundle id sans casse).
pub(crate) fn navigateur(bundle: &str) -> Option<&'static str> {
    NAVIGATEURS.iter().find(|n| n.0.eq_ignore_ascii_case(bundle)).map(|n| n.1)
}

/// `https://moi@www.YouTube.com:443/watch?v=x#t` → `www.youtube.com/watch` ; autre schéma que http(s) : `None`.
pub(crate) fn page(url: &str) -> Option<String> {
    let schema = url.find("://")?;
    if !["http", "https"].contains(&url[..schema].to_ascii_lowercase().as_str()) {
        return None;
    }
    let reste = &url[schema + 3..];
    let reste = &reste[..reste.find(['?', '#']).unwrap_or(reste.len())];
    let (autorite, chemin) = reste.split_once('/').unwrap_or((reste, ""));
    let hote = autorite.rsplit_once('@').map_or(autorite, |(_, h)| h);
    let hote = match hote.rsplit_once(':') {
        Some((h, port)) if !h.is_empty() && port.bytes().all(|o| o.is_ascii_digit()) => h,
        _ => hote,
    };
    (!hote.is_empty()).then(|| format!("{}/{chemin}", hote.to_lowercase()))
}

/// Suit ce navigateur (lecture tout de suite, puis toutes les 0,5 s), ou plus aucun (`None` : le fil dort sur son canal).
pub(super) fn suivre(bundle: Option<String>) {
    let bundle = bundle.filter(|_| !SIMULE.load(std::sync::atomic::Ordering::Relaxed));
    let mut place = VERS_LE_FIL.lock().unwrap_or_else(|e| e.into_inner());
    let envoi = place.get_or_insert_with(|| {
        let (envoi, reception) = channel();
        std::thread::Builder::new().name("tactile-onglet".into()).spawn(move || fil(reception)).expect("fil des onglets");
        envoi
    });
    let _ = envoi.send(bundle);
}

/// La lecture la plus récente (navigateur, lecture), une fois.
pub(super) fn derniere() -> Option<(String, Lecture)> {
    DERNIERE.lock().unwrap_or_else(|e| e.into_inner()).take()
}

fn fil(reception: Receiver<Option<String>>) {
    let mut scripts: HashMap<String, Id> = HashMap::new();
    let (mut suivi, mut derniere, mut periode) = (None::<String>, None::<Lecture>, PERIODE);
    let mut signales: Vec<(String, isize)> = Vec::new();
    loop {
        let message = match suivi {
            Some(_) => reception.recv_timeout(periode),
            None => reception.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match message {
            Ok(nouveau) => {
                // Seule la demande la plus récente compte.
                suivi = reception.try_iter().last().unwrap_or(nouveau);
                derniere = None;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let Some(bundle) = suivi.clone() else { continue };
        let lecture = objc2::rc::autoreleasepool(|_| lire(&bundle, &mut scripts));
        periode = if lecture.is_err() { PERIODE_ERREUR } else { PERIODE };
        if let Err(code) = lecture
            && !signales.contains(&(bundle.clone(), code))
        {
            signales.push((bundle.clone(), code));
            let nom = navigateur(&bundle).unwrap_or(&bundle);
            if code == REFUSE {
                eprintln!("tactile : macOS refuse à Tactile de lire l'onglet actif de {nom} (erreur {code}) : les barres de site y restent cachées ; autorise-le dans Réglages Système → Confidentialité et sécurité → Automatisation → Tactile");
            } else {
                eprintln!("tactile : onglet actif de {nom} illisible (erreur AppleScript {code}) : barres de site cachées, nouvel essai dans 5 s");
            }
        }
        if derniere.as_ref() != Some(&lecture) {
            derniere = Some(lecture.clone());
            *DERNIERE.lock().unwrap_or_else(|e| e.into_inner()) = Some((bundle, lecture));
            let cible = super::CIBLE_PARTAGEE.load(std::sync::atomic::Ordering::Acquire);
            if !cible.is_null() {
                unsafe { let _: () = msg_send![cible, performSelectorOnMainThread: sel!(ongletChange:), withObject: std::ptr::null_mut::<AnyObject>(), waitUntilDone: Bool::NO]; }
            }
        }
    }
}

/// Le script d'un navigateur. `is running` ne lance pas l'app (elle a pu quitter entre-temps) ; `with timeout` borne
/// l'attente à 1 s si le navigateur ne répond pas (-1712).
fn source(bundle: &str, safari: bool) -> String {
    let onglet = if safari { "current tab" } else { "active tab" };
    format!(
        "if application id \"{bundle}\" is running then\n\twith timeout of 1 second\n\t\ttell application id \"{bundle}\" to return URL of {onglet} of front window\n\tend timeout\nend if\nreturn \"\""
    )
}

/// Le numéro d'erreur d'un dictionnaire d'erreur de `NSAppleScript`.
fn numero(erreur: Id) -> isize {
    if erreur.is_null() {
        return 0;
    }
    unsafe {
        let n: Id = msg_send![erreur, objectForKey: &*texte("NSAppleScriptErrorNumber")];
        if n.is_null() { -1 } else { msg_send![n, integerValue] }
    }
}

/// Une lecture. La première fois pour un navigateur : l'autorisation d'abord, demandée sans délai (la question de
/// macOS attend la réponse ; dans le script, `with timeout of 1 second` la ferait expirer au bout d'une seconde, vu
/// dans le journal de tccd), puis le script est compilé (le navigateur est au premier plan, donc installé : la
/// compilation lit son dictionnaire) et gardé. Pas de fenêtre (-1728, -1719) : aucune page.
fn lire(bundle: &str, scripts: &mut HashMap<String, Id>) -> Lecture {
    let Some(&(id, _, safari)) = NAVIGATEURS.iter().find(|n| n.0.eq_ignore_ascii_case(bundle)) else { return Ok(None) };
    let script = match scripts.get(id) {
        Some(&s) => s,
        None => unsafe {
            match permission(id, true) {
                0 => {}
                -600 => return Ok(None),
                code => return Err(code as isize),
            }
            let s: Id = msg_send![class!(NSAppleScript), alloc];
            let s: Id = msg_send![s, initWithSource: &*texte(&source(id, safari))];
            let mut erreur: Id = std::ptr::null_mut();
            let compile: Bool = msg_send![s, compileAndReturnError: &mut erreur];
            if !compile.as_bool() {
                super::vues::lacher(s);
                return Err(numero(erreur));
            }
            scripts.insert(id.to_string(), s);
            s
        },
    };
    unsafe {
        let mut erreur: Id = std::ptr::null_mut();
        let resultat: Id = msg_send![script, executeAndReturnError: &mut erreur];
        if resultat.is_null() {
            return match numero(erreur) {
                -1728 | -1719 => Ok(None),
                code => Err(code),
            };
        }
        let url: Id = msg_send![resultat, stringValue];
        if url.is_null() {
            return Ok(None);
        }
        Ok(page(&(*(url as *const objc2_foundation::NSString)).to_string()))
    }
}

#[repr(C)]
struct AEDesc {
    type_descripteur: u32,
    donnees: *mut std::ffi::c_void,
}

#[link(name = "CoreServices", kind = "framework")]
unsafe extern "C" {
    fn AECreateDesc(type_code: u32, donnees: *const std::ffi::c_void, taille: isize, resultat: *mut AEDesc) -> i16;
    fn AEDisposeDesc(desc: *mut AEDesc) -> i16;
    fn AEDeterminePermissionToAutomateTarget(cible: *const AEDesc, classe: u32, id: u32, demander: u8) -> i32;
}

/// L'autorisation Automatisation de ce processus pour `bundle` : 0 accordée, -1743 refusée, -1744 jamais demandée
/// (avec `demander`, macOS pose la question et la réponse attend l'utilisateur), -600 navigateur fermé.
fn permission(bundle: &str, demander: bool) -> i32 {
    const BUNDLE_ID: u32 = u32::from_be_bytes(*b"bund");
    const TOUT: u32 = u32::from_be_bytes(*b"****");
    unsafe {
        let mut desc = AEDesc { type_descripteur: 0, donnees: std::ptr::null_mut() };
        let erreur = AECreateDesc(BUNDLE_ID, bundle.as_ptr().cast(), bundle.len() as isize, &mut desc);
        if erreur != 0 {
            return i32::from(erreur);
        }
        let statut = AEDeterminePermissionToAutomateTarget(&desc, TOUT, TOUT, u8::from(demander));
        AEDisposeDesc(&mut desc);
        statut
    }
}

/// Les navigateurs ouverts dont macOS refuse l'onglet actif à ce processus (sans rien demander à l'utilisateur) :
/// pour l'avis de `config::controler`. Un navigateur fermé répond -600 et n'est pas listé.
pub(crate) fn navigateurs_refuses() -> Vec<&'static str> {
    NAVIGATEURS.iter().filter(|n| permission(n.0, false) as isize == REFUSE).map(|n| n.1).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_sans_schema_identifiants_port_ni_requete() {
        assert_eq!(page("https://moi:secret@www.YouTube.com:443/watch?v=abc#t=3").as_deref(), Some("www.youtube.com/watch"));
        assert_eq!(page("http://localhost:8000/").as_deref(), Some("localhost/"));
        assert_eq!(page("https://mail.google.com").as_deref(), Some("mail.google.com/"));
        assert_eq!(page("HTTPS://exemple.org/A/b").as_deref(), Some("exemple.org/A/b"));
        for autre in ["file:///Users/moi/x.html", "about:blank", "chrome://newtab/", "favorites://", "", "https://"] {
            assert_eq!(page(autre), None, "{autre}");
        }
    }

    #[test]
    fn navigateurs_connus_sans_casse() {
        assert_eq!(navigateur("com.google.chrome"), Some("Google Chrome"));
        assert_eq!(navigateur("com.apple.Safari"), Some("Safari"));
        assert_eq!(navigateur("com.apple.TextEdit"), None);
        assert!(source("com.apple.Safari", true).contains("current tab") && source("com.brave.Browser", false).contains("active tab"));
    }

    /// `cargo test --release -- --ignored mesure_lecture --nocapture` : coût d'une lecture par `NSAppleScript` compilé
    /// (dans le processus) et par `osascript` (un processus par lecture), 40 lectures chacune. Le script ne parle à
    /// aucune app (`is running` ne demande pas d'autorisation) : c'est la part de Tactile, sans la réponse du navigateur.
    #[test]
    #[ignore = "mesure, à lancer à la main"]
    fn mesure_lecture() {
        // 0 : ce processus ; -1 : ses enfants attendus.
        let cpu = crate::essais::cpu_ms;
        let source = "if application id \"com.apple.Safari\" is running then\n\treturn \"https://exemple.org/a?b\"\nend if\nreturn \"https://exemple.org/\"";
        let mediane = |mut v: Vec<f64>| {
            v.sort_by(f64::total_cmp);
            v[v.len() / 2]
        };
        let (n, mut durees) = (40, Vec::new());
        let avant = cpu(0);
        objc2::rc::autoreleasepool(|_| unsafe {
            let s: Id = msg_send![class!(NSAppleScript), alloc];
            let s: Id = msg_send![s, initWithSource: &*texte(source)];
            let mut erreur: Id = std::ptr::null_mut();
            let _: Bool = msg_send![s, compileAndReturnError: &mut erreur];
            for _ in 0..n {
                let debut = std::time::Instant::now();
                let r: Id = msg_send![s, executeAndReturnError: &mut erreur];
                assert!(!r.is_null());
                durees.push(debut.elapsed().as_secs_f64() * 1e3);
            }
        });
        let ns = ((cpu(0) - avant) / n as f64, mediane(std::mem::take(&mut durees)));
        let avant = cpu(-1);
        for _ in 0..n {
            let debut = std::time::Instant::now();
            assert!(std::process::Command::new("/usr/bin/osascript").args(["-e", source]).output().unwrap().status.success());
            durees.push(debut.elapsed().as_secs_f64() * 1e3);
        }
        let osa = ((cpu(-1) - avant) / n as f64, mediane(durees));
        for (quoi, (cpu, latence)) in [("NSAppleScript", ns), ("osascript", osa)] {
            eprintln!("mesure : {quoi} : {cpu:.2} ms de CPU par lecture ({:.2} % à 2 lectures/s), latence médiane {latence:.1} ms", cpu * 2.0 / 10.0);
        }
    }

    /// L'appel ne demande rien à l'utilisateur et ne liste que des navigateurs connus (un navigateur fermé répond -600).
    #[test]
    fn verification_d_autorisation_sans_demande() {
        let refuses = navigateurs_refuses();
        assert!(refuses.iter().all(|n| NAVIGATEURS.iter().any(|m| m.1 == *n)), "{refuses:?}");
    }
}
