//! Luminosité de l'écran intégré (0.0 à 1.0).

use super::Resultat;

pub fn lire() -> Resultat<f32> {
    Err("luminosite::lire : à faire".into())
}

pub fn regler(_valeur: f32) -> Resultat<()> {
    Err("luminosite::regler : à faire".into())
}
