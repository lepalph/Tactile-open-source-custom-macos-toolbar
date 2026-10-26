//! Contrôle et lecture de ce qui joue, quelle que soit l'app (Spotify, Chrome, Musique…).

use super::Resultat;
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct EnCours {
    pub titre: String,
    pub artiste: String,
    pub en_lecture: bool,
}

#[derive(Deserialize)]
struct Brut {
    title: Option<String>,
    artist: Option<String>,
    playing: Option<bool>,
}

// Depuis macOS 15.4, MediaRemote refuse aux apps non autorisées de lire ce qui joue, et MRMediaRemoteSendCommand
// répond « vrai » sans effet : media-control passe par un binaire autorisé. Sur Apple Silicon il échoue sous Rosetta,
// d'où arch -arm64 ; sur Intel, Homebrew l'installe dans /usr/local.
fn programme(existe: impl Fn(&str) -> bool) -> Option<Vec<&'static str>> {
    if existe("/opt/homebrew/bin/media-control") {
        Some(vec!["/usr/bin/arch", "-arm64", "/opt/homebrew/bin/media-control"])
    } else if existe("/usr/local/bin/media-control") {
        Some(vec!["/usr/local/bin/media-control"])
    } else {
        None
    }
}

fn media_control(args: &[&str]) -> Resultat<Vec<u8>> {
    let commande = programme(|chemin| std::path::Path::new(chemin).exists())
        .ok_or("média : media-control introuvable, installe-le avec « brew install media-control »")?;
    let sortie = Command::new(commande[0])
        .args(&commande[1..])
        .args(args)
        .output()
        .map_err(|e| format!("média : impossible de lancer media-control ({e})"))?;
    if !sortie.status.success() {
        let erreur = String::from_utf8_lossy(&sortie.stderr);
        return Err(format!("média : media-control {} a échoué : {}", args.join(" "), erreur.trim()));
    }
    Ok(sortie.stdout)
}

fn analyser(json: &str) -> Resultat<Option<EnCours>> {
    let brut: Option<Brut> =
        serde_json::from_str(json).map_err(|e| format!("média : réponse de media-control illisible ({e})"))?;
    Ok(brut.map(|b| EnCours {
        titre: b.title.unwrap_or_default(),
        artiste: b.artist.unwrap_or_default(),
        en_lecture: b.playing.unwrap_or(false),
    }))
}

pub fn en_cours() -> Resultat<Option<EnCours>> {
    analyser(&String::from_utf8_lossy(&media_control(&["get", "-h"])?))
}

pub fn lecture_pause() -> Resultat<()> {
    media_control(&["toggle-play-pause"]).map(|_| ())
}

pub fn suivant() -> Resultat<()> {
    media_control(&["next-track"]).map(|_| ())
}

pub fn precedent() -> Resultat<()> {
    media_control(&["previous-track"]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn programme_selon_le_mac() {
        let apple_silicon = programme(|c| c.starts_with("/opt/homebrew")).unwrap();
        assert_eq!(apple_silicon, ["/usr/bin/arch", "-arm64", "/opt/homebrew/bin/media-control"]);
        let intel = programme(|c| c.starts_with("/usr/local")).unwrap();
        assert_eq!(intel, ["/usr/local/bin/media-control"]);
        assert!(programme(|_| false).is_none());
    }

    #[test]
    fn analyse_lecture() {
        let json = r#"{"title":"Lawson thème","artist":"NeS","playing":true,"playbackRate":1,"duration":170.1}"#;
        let attendu = EnCours { titre: "Lawson thème".into(), artiste: "NeS".into(), en_lecture: true };
        assert_eq!(analyser(json).unwrap(), Some(attendu));
    }

    #[test]
    fn analyse_pause_sans_artiste() {
        let json = r#"{"title":"Vidéo","playing":false,"bundleIdentifier":"com.google.Chrome"}"#;
        let attendu = EnCours { titre: "Vidéo".into(), artiste: String::new(), en_lecture: false };
        assert_eq!(analyser(json).unwrap(), Some(attendu));
    }

    #[test]
    fn analyse_rien() {
        assert_eq!(analyser("null\n").unwrap(), None);
        assert!(analyser("pas du json").is_err());
    }

    #[test]
    #[ignore = "drives real playback"]
    fn basculer_deux_fois() {
        let avant = en_cours().unwrap().expect("rien en cours de lecture").en_lecture;
        lecture_pause().unwrap();
        sleep(Duration::from_millis(800));
        let pendant = en_cours().unwrap().map(|e| e.en_lecture);
        lecture_pause().unwrap();
        sleep(Duration::from_millis(800));
        assert_eq!(pendant, Some(!avant));
        assert_eq!(en_cours().unwrap().map(|e| e.en_lecture), Some(avant));
    }

    #[test]
    #[ignore = "skips the current track"]
    fn suivant_change_de_titre() {
        let avant = en_cours().unwrap().expect("rien en cours de lecture").titre;
        suivant().unwrap();
        sleep(Duration::from_millis(1500));
        let pendant = en_cours().unwrap().map(|e| e.titre);
        precedent().unwrap();
        assert_ne!(pendant, Some(avant));
    }
}
