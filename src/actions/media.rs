//! Contrôle et lecture de ce qui joue, quelle que soit l'app (Spotify, Chrome, Musique…).

use super::Resultat;

#[derive(Debug, Clone, PartialEq)]
pub struct EnCours {
    pub titre: String,
    pub artiste: String,
    pub en_lecture: bool,
}

pub fn en_cours() -> Resultat<Option<EnCours>> {
    Err("media::en_cours : à faire".into())
}

pub fn lecture_pause() -> Resultat<()> {
    Err("media::lecture_pause : à faire".into())
}

pub fn suivant() -> Resultat<()> {
    Err("media::suivant : à faire".into())
}

pub fn precedent() -> Resultat<()> {
    Err("media::precedent : à faire".into())
}
