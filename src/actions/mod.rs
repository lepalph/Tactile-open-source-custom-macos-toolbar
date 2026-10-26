//! Actions système, utilisables sans interface (testables en ligne de commande).

pub mod clavier;
pub mod luminosite;
pub mod media;
pub mod volume;

use serde::Deserialize;
use std::process::Command;

pub type Resultat<T> = Result<T, String>;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    LuminositePlus,
    LuminositeMoins,
    Luminosite(f32),
    VolumePlus,
    VolumeMoins,
    Volume(f32),
    BasculerMuet,
    LecturePause,
    Suivant,
    Precedent,
    Veille,
    Echap,
    BarreApple,
    Shell(String),
    /// `{ touche = "cmd+shift+4" }` : un raccourci clavier.
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
        Action::App(app) => journaliser("/usr/bin/open", &["-a", app]),
        Action::Url(url) => journaliser("/usr/bin/open", &[url]),
        Action::Applescript(script) => journaliser("/usr/bin/osascript", &["-e", script]),
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
}

/// Lance une commande sans attendre sa fin.
pub fn lancer(programme: &str, args: &[&str]) -> Resultat<()> {
    Command::new(programme)
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("{programme} : {e}"))
}
