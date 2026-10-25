//! Volume de la sortie audio par défaut (0.0 à 1.0) et coupure du son.

use super::Resultat;

pub fn lire() -> Resultat<f32> {
    Err("volume::lire : à faire".into())
}

pub fn regler(_valeur: f32) -> Resultat<()> {
    Err("volume::regler : à faire".into())
}

pub fn est_muet() -> Resultat<bool> {
    Err("volume::est_muet : à faire".into())
}

pub fn basculer_muet() -> Resultat<()> {
    Err("volume::basculer_muet : à faire".into())
}
