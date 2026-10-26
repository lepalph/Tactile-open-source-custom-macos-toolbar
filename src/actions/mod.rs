//! Actions système, utilisables sans interface (testables en ligne de commande).

pub mod clavier;
pub mod luminosite;
pub mod media;
pub mod volume;

use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use serde::{Deserialize, Serialize, Serializer};
use std::process::Command;

pub type Resultat<T> = Result<T, String>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    #[serde(alias = "brightness_up")]
    LuminositePlus,
    #[serde(alias = "brightness_down")]
    LuminositeMoins,
    #[serde(alias = "brightness")]
    Luminosite(#[serde(serialize_with = "court")] f32),
    #[serde(alias = "volume_up")]
    VolumePlus,
    #[serde(alias = "volume_down")]
    VolumeMoins,
    Volume(#[serde(serialize_with = "court")] f32),
    #[serde(alias = "toggle_mute")]
    BasculerMuet,
    #[serde(alias = "play_pause")]
    LecturePause,
    #[serde(alias = "next")]
    Suivant,
    #[serde(alias = "previous")]
    Precedent,
    #[serde(alias = "sleep")]
    Veille,
    #[serde(alias = "esc")]
    Echap,
    #[serde(alias = "apple_bar")]
    BarreApple,
    Shell(String),
    /// `{ touche = "cmd+shift+4" }` : un raccourci clavier.
    #[serde(alias = "key")]
    Touche(clavier::Touche),
    /// `{ keycode = 53 }` : une touche par son code, pour celles qui n'ont pas de nom.
    Keycode(u16),
    /// `{ app = "Figma" }` : nom ou identifiant de bundle, lancé avec `open -a`.
    App(String),
    /// `{ url = "https://…" }`, ouvert avec `open`.
    Url(String),
    /// `{ applescript = "tell application \"Music\" to playpause" }`, lancé avec `osascript -e`.
    Applescript(String),
    MissionControl,
    /// App Exposé : les fenêtres de l'app active.
    #[serde(alias = "app_windows")]
    FenetresApp,
    Launchpad,
    #[serde(alias = "show_desktop")]
    Bureau,
    Spotlight,
    /// ⌘⇧3 : tout l'écran.
    #[serde(alias = "screenshot")]
    CaptureEcran,
    /// ⌘⇧4 : une zone.
    #[serde(alias = "screenshot_area")]
    CaptureZone,
    /// ⌘⇧5 : la palette de capture et d'enregistrement.
    #[serde(alias = "screenshot_options")]
    CaptureOptions,
    #[serde(alias = "lock_screen")]
    Verrouiller,
    /// La palette des émojis et symboles.
    #[serde(alias = "emoji")]
    Emojis,
    /// La fenêtre « Forcer à quitter ».
    #[serde(alias = "force_quit")]
    ForcerQuitter,
}

const PAS: f32 = 1.0 / 16.0;

/// Écrit `0.3` et non `0.30000001192092896` : un f32 passe par f64 dans le TOML. Le plus court décimal qui redonne
/// le même f32 (son `Display`), sans arrondi : 0,6100102 ne devient pas 0,61 à l'enregistrement.
fn court<S: Serializer>(v: &f32, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_f64(v.to_string().parse().unwrap_or(f64::from(*v)))
}

pub fn executer(action: &Action) -> Resultat<()> {
    match action {
        Action::LuminositePlus => luminosite::regler(luminosite::lire()? + PAS),
        Action::LuminositeMoins => luminosite::regler(luminosite::lire()? - PAS),
        Action::Luminosite(v) => luminosite::regler(*v),
        Action::VolumePlus => volume::regler(volume::lire()? + PAS),
        Action::VolumeMoins => volume::regler(volume::lire()? - PAS),
        Action::Volume(v) => volume::regler(*v),
        Action::BasculerMuet => volume::basculer_muet(),
        Action::LecturePause => media::lecture_pause(),
        Action::Suivant => media::suivant(),
        Action::Precedent => media::precedent(),
        Action::Veille => lancer("/usr/bin/pmset", &["sleepnow"]),
        Action::Echap => clavier::touche(53, 0),
        Action::Touche(t) => clavier::raccourci(t),
        Action::Keycode(code) => clavier::touche(*code, 0),
        Action::BarreApple => Err("barre_apple est géré par la barre, pas par les actions".into()),
        Action::Shell(commande) => lancer("/bin/zsh", &["-c", commande]),
        // `open -a` ne connaît que les noms ; un bundle id passe par `open -b`.
        Action::App(app) => journaliser("/usr/bin/open", &[if est_bundle(app) { "-b" } else { "-a" }, app]),
        Action::Url(url) => journaliser("/usr/bin/open", &[url]),
        Action::Applescript(script) => journaliser("/usr/bin/osascript", &["-e", script]),
        // L'app « Mission Control » prévient le Dock ; son argument choisit l'écran (1 = bureau, 2 = fenêtres de l'app, lu dans son code).
        Action::MissionControl => journaliser("/usr/bin/open", &["-b", MISSION_CONTROL]),
        Action::FenetresApp => journaliser("/usr/bin/open", &["-b", MISSION_CONTROL, "--args", "2"]),
        Action::Bureau => journaliser("/usr/bin/open", &["-b", MISSION_CONTROL, "--args", "1"]),
        Action::Launchpad => journaliser("/usr/bin/open", &["-b", "com.apple.launchpad.launcher"]),
        Action::CaptureOptions => journaliser("/usr/bin/open", &["-b", "com.apple.screenshot.launcher"]),
        Action::Spotlight | Action::CaptureEcran | Action::CaptureZone | Action::Verrouiller | Action::Emojis | Action::ForcerQuitter => {
            clavier::raccourci(&clavier::Touche::try_from(raccourci_systeme(action).unwrap_or_default().to_string())?)
        }
    }
}

const MISSION_CONTROL: &str = "com.apple.exposelauncher";

/// Raccourcis d'Apple sans app ni API publique pour les déclencher : simulés au clavier (Accessibilité), touches par défaut de macOS.
fn raccourci_systeme(action: &Action) -> Option<&'static str> {
    Some(match action {
        Action::Spotlight => "cmd+espace",
        Action::CaptureEcran => "cmd+shift+3",
        Action::CaptureZone => "cmd+shift+4",
        Action::Verrouiller => "ctrl+cmd+q",
        Action::Emojis => "ctrl+cmd+espace",
        Action::ForcerQuitter => "alt+cmd+esc",
        _ => return None,
    })
}

/// Le bundle id d'une app installée (`com.apple.Safari`), d'après LaunchServices.
fn est_bundle(nom: &str) -> bool {
    let nom = objc2_foundation::NSString::from_str(nom);
    unsafe {
        let espace: *mut AnyObject = msg_send![class!(NSWorkspace), sharedWorkspace];
        let url: *mut AnyObject = msg_send![espace, URLForApplicationWithBundleIdentifier: &*nom];
        !url.is_null()
    }
}

/// Une app installée, par bundle id, chemin ou nom (celui qu'`open -a` trouve), sans lancer de processus.
/// Partagée par `--verifier` (`config::controler`) et l'éditeur (qui y ajoute une suggestion).
pub fn app_existe(nom: &str) -> bool {
    chemin_app(nom).is_some()
}

/// Le chemin du `.app` (pour son icône), trouvé comme `app_existe`.
pub fn chemin_app(nom: &str) -> Option<String> {
    // `fullPathForApplication:` répond pour n'importe quel chemin, même absent.
    if nom.starts_with('/') {
        return std::path::Path::new(nom).exists().then(|| nom.to_string());
    }
    let texte = objc2_foundation::NSString::from_str(nom);
    unsafe {
        let espace: *mut AnyObject = msg_send![class!(NSWorkspace), sharedWorkspace];
        let url: *mut AnyObject = msg_send![espace, URLForApplicationWithBundleIdentifier: &*texte];
        let chemin: *const objc2_foundation::NSString = if url.is_null() { msg_send![espace, fullPathForApplication: &*texte] } else { msg_send![url, path] };
        chemin.as_ref().map(|c| c.to_string())
    }
}

/// Le bundle id d'une app installée (nom, bundle id ou chemin), par LaunchServices : `Finder` et `com.apple.finder` se rejoignent.
pub fn bundle_app(nom: &str) -> Option<String> {
    let chemin = chemin_app(nom)?;
    unsafe {
        let bundle: *mut AnyObject = msg_send![class!(NSBundle), bundleWithPath: &*objc2_foundation::NSString::from_str(&chemin)];
        let id: *const objc2_foundation::NSString = if bundle.is_null() { std::ptr::null() } else { msg_send![bundle, bundleIdentifier] };
        id.as_ref().map(|i| i.to_string())
    }
}

/// L'app de ce `.app` tourne-t-elle ? Par son bundle id, comme le point du Dock.
pub fn app_ouverte(chemin: &str) -> bool {
    unsafe {
        let bundle: *mut AnyObject = msg_send![class!(NSBundle), bundleWithPath: &*objc2_foundation::NSString::from_str(chemin)];
        let id: *mut AnyObject = if bundle.is_null() { bundle } else { msg_send![bundle, bundleIdentifier] };
        if id.is_null() {
            return false;
        }
        let apps: *mut AnyObject = msg_send![class!(NSRunningApplication), runningApplicationsWithBundleIdentifier: id];
        let n: usize = msg_send![apps, count];
        // À la notification de fin, l'app quittée peut encore être listée, marquée terminée.
        (0..n).any(|i| {
            let app: *mut AnyObject = msg_send![apps, objectAtIndex: i];
            let terminee: bool = msg_send![app, isTerminated];
            !terminee
        })
    }
}

/// Lance la commande dans un fil et écrit son erreur au journal si elle échoue (app introuvable, AppleScript faux).
fn journaliser(programme: &str, args: &[&str]) -> Resultat<()> {
    let (programme, args) = (programme.to_string(), args.iter().map(|a| a.to_string()).collect::<Vec<_>>());
    std::thread::spawn(move || {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        if let Err(e) = attendre(&programme, &args) {
            eprintln!("tactile : {e}");
        }
    });
    Ok(())
}

/// Lance une commande, attend sa fin ; en cas d'échec, l'erreur reprend sa sortie d'erreur.
pub fn attendre(programme: &str, args: &[&str]) -> Resultat<()> {
    let sortie = Command::new(programme).args(args).output().map_err(|e| format!("{programme} : {e}"))?;
    if sortie.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&sortie.stderr).trim().to_string();
    let nom = std::path::Path::new(programme).file_name().and_then(|n| n.to_str()).unwrap_or(programme);
    Err(format!("{nom} {} : {}", args.iter().map(|a| format!("« {a} »")).collect::<Vec<_>>().join(" "), if stderr.is_empty() { format!("code {}", sortie.status.code().unwrap_or(-1)) } else { stderr }))
}

/// Lance une commande sans attendre sa fin.
pub fn lancer(programme: &str, args: &[&str]) -> Resultat<()> {
    Command::new(programme)
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("{programme} : {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_introuvable_et_applescript_faux_remontent_leur_erreur() {
        let erreur = attendre("/usr/bin/open", &["-a", "AppQuiNexistePas"]).unwrap_err();
        assert!(erreur.starts_with("open « -a » « AppQuiNexistePas » : ") && erreur.contains("Unable to find application"), "{erreur}");
        let erreur = attendre("/usr/bin/osascript", &["-e", "tell blabla"]).unwrap_err();
        assert!(erreur.starts_with("osascript « -e » « tell blabla » : ") && erreur.contains("rror"), "{erreur}");
        assert_eq!(attendre("/usr/bin/osascript", &["-e", "return 1 + 1"]), Ok(()));
        assert!(attendre("/nexiste/pas", &[]).unwrap_err().starts_with("/nexiste/pas : "));
    }

    /// Un curseur réglé à 0,6100102 (une valeur d'f32 quelconque) revient à l'identique, et 0,3 s'écrit toujours `0.3`.
    #[test]
    fn valeur_de_curseur_ecrite_sans_perte() {
        #[derive(Serialize, Deserialize)]
        struct Fichier {
            action: Action,
        }
        for v in [0.6100102f32, 0.0625, 0.3, 1.0, 0.0, 1e-7, f32::MIN_POSITIVE] {
            for action in [Action::Volume(v), Action::Luminosite(v)] {
                let toml = toml::to_string(&Fichier { action: action.clone() }).unwrap();
                assert_eq!(toml::from_str::<Fichier>(&toml).unwrap().action, action, "{toml}");
                let json = serde_json::to_string(&action).unwrap();
                assert_eq!(serde_json::from_str::<Action>(&json).unwrap(), action, "{json}");
            }
        }
        assert_eq!(serde_json::to_string(&Action::Volume(0.3)).unwrap(), r#"{"volume":0.3}"#);
    }

    #[test]
    fn app_par_nom_ou_par_bundle_id() {
        assert!(app_existe("Safari") && app_existe("com.apple.Safari") && app_existe("TextEdit"));
        assert!(app_existe("/System/Applications/TextEdit.app"), "chemin");
        assert!(!app_existe("/Applications/Nexiste.app"), "chemin absent");
        assert!(!app_existe("AppQuiNexistePas") && !app_existe("com.exemple.nexiste.pas"));
        assert!(est_bundle("com.apple.Safari") && !est_bundle("Safari"));
        assert_eq!(chemin_app("com.apple.TextEdit").as_deref(), Some("/System/Applications/TextEdit.app"));
        assert_eq!(chemin_app("TextEdit"), chemin_app("com.apple.TextEdit"));
        assert_eq!(chemin_app("AppQuiNexistePas"), None);
    }

    #[test]
    #[cfg_attr(ci, ignore = "processus d'une session ouverte (Finder) : pas de session sur un runner de CI")]
    fn app_ouverte_selon_ses_processus() {
        assert!(app_ouverte(&chemin_app("com.apple.finder").unwrap()), "le Finder tourne toujours");
        let grapher = std::process::Command::new("/usr/bin/pgrep").args(["-x", "Grapher"]).status().unwrap().success();
        assert_eq!(app_ouverte("/System/Applications/Utilities/Grapher.app"), grapher);
        assert!(!app_ouverte("/nexiste/pas.app"));
    }
}
