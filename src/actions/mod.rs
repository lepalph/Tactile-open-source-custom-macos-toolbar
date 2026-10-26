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
}

const PAS: f32 = 1.0 / 16.0;

/// Écrit `0.3` et non `0.30000001192092896` : un f32 passe par f64 dans le TOML.
fn court<S: Serializer>(v: &f32, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_f64((f64::from(*v) * 1000.0).round() / 1000.0)
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
    }
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
    // `fullPathForApplication:` répond pour n'importe quel chemin, même absent.
    if nom.starts_with('/') {
        return std::path::Path::new(nom).exists();
    }
    let chemin = objc2_foundation::NSString::from_str(nom);
    est_bundle(nom)
        || unsafe {
            let espace: *mut AnyObject = msg_send![class!(NSWorkspace), sharedWorkspace];
            let chemin: *mut AnyObject = msg_send![espace, fullPathForApplication: &*chemin];
            !chemin.is_null()
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

    #[test]
    fn app_par_nom_ou_par_bundle_id() {
        assert!(app_existe("Safari") && app_existe("com.apple.Safari") && app_existe("TextEdit"));
        assert!(app_existe("/System/Applications/TextEdit.app"), "chemin");
        assert!(!app_existe("/Applications/Nexiste.app"), "chemin absent");
        assert!(!app_existe("AppQuiNexistePas") && !app_existe("com.exemple.nexiste.pas"));
        assert!(est_bundle("com.apple.Safari") && !est_bundle("Safari"));
    }
}
