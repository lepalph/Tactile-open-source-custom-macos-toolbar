//! Actions système, utilisables sans interface (testables en ligne de commande).

pub mod clavier;
pub mod luminosite;
pub mod media;
pub mod volume;

use serde::{Deserialize, Serialize, Serializer};
use std::process::Command;

pub type Resultat<T> = Result<T, String>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    LuminositePlus,
    LuminositeMoins,
    Luminosite(#[serde(serialize_with = "court")] f32),
    VolumePlus,
    VolumeMoins,
    Volume(#[serde(serialize_with = "court")] f32),
    BasculerMuet,
    LecturePause,
    Suivant,
    Precedent,
    Veille,
    Echap,
    BarreApple,
    Shell(String),
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
        Action::Echap => clavier::touche(53),
        Action::BarreApple => Err("barre_apple est géré par la barre, pas par les actions".into()),
        Action::Shell(commande) => lancer("/bin/zsh", &["-c", commande]),
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
