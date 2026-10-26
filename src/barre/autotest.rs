//! Autotest (`TACTILE_AUTOTEST=1`) : appuie sur chaque contrôle par le même chemin qu'un doigt
//! et vérifie l'effet réel. Une étape toutes les 2,5 s, pour laisser `scripts/verifier.sh` capturer
//! la barre entre deux. Écrit « AUTOTEST n ok|ÉCHEC … » sur la sortie d'erreur, puis « AUTOTEST fin ».

use objc2::runtime::AnyObject;
use objc2::{msg_send, sel};
use std::cell::Cell;

use super::{cible, Id, COURANTE, ETAT};
use crate::actions::{luminosite, media, volume};

thread_local! {
    static ETAPE: Cell<usize> = const { Cell::new(0) };
    static ECHECS: Cell<usize> = const { Cell::new(0) };
    static LECTURE: Cell<Option<bool>> = const { Cell::new(None) };
    static CONVERSATION: Cell<usize> = const { Cell::new(0) };
}

const ETAT_VOCAL: &str = "/tmp/claude-voix/etat";

pub(super) fn programmer() {
    unsafe {
        let _: () = msg_send![cible(), performSelector: sel!(etapeAutotest:), withObject: std::ptr::null_mut::<AnyObject>(), afterDelay: 2.5f64];
    }
}

fn controle(nom: &str) -> Option<Id> {
    ETAT.with_borrow(|e| e.as_ref()?.controles.iter().find(|(n, _)| n == nom).map(|(_, id)| *id))
}

fn principale() -> Id {
    ETAT.with_borrow(|e| e.as_ref().map(|e| e.barres[0]).unwrap_or(std::ptr::null_mut()))
}

fn noter(n: usize, ok: bool, quoi: &str) {
    if !ok {
        ECHECS.set(ECHECS.get() + 1);
    }
    eprintln!("AUTOTEST {n} {} {quoi}", if ok { "ok" } else { "ÉCHEC" });
}

fn cliquer(n: usize, nom: &str) -> bool {
    match controle(nom) {
        Some(b) => {
            unsafe { let _: () = msg_send![b, performClick: std::ptr::null_mut::<AnyObject>()]; }
            true
        }
        None => {
            noter(n, false, &format!("contrôle « {nom} » introuvable"));
            false
        }
    }
}

/// Règle un slider comme un doigt : nouvelle valeur, puis le message que la Touch Bar envoie.
fn glisser(nom: &str, valeur: f32) -> bool {
    let Some(item) = controle(nom) else { return false };
    unsafe {
        let slider: Id = msg_send![item, slider];
        let _: () = msg_send![slider, setDoubleValue: valeur as f64];
        let _: () = msg_send![cible(), glisse: item];
    }
    true
}

fn etat_vocal() -> String {
    std::fs::read_to_string(ETAT_VOCAL).unwrap_or_default()
}

fn lignes_conversation() -> usize {
    std::fs::read_to_string("/tmp/claude-voix/conversation.md").map(|t| t.lines().count()).unwrap_or(0)
}

fn verifier_ecoute(n: usize) {
    let e = etat_vocal();
    noter(n, e.starts_with('🎙'), &format!("Claude écoute (état « {e} »)"));
}

fn verifier_annule(n: usize, quoi: &str) {
    let (e, lignes) = (etat_vocal(), lignes_conversation());
    noter(n, e.is_empty() && lignes == CONVERSATION.get(), &format!("{quoi} (état « {e} », conversation inchangée : {})", lignes == CONVERSATION.get()));
}

fn cliquer_indicateur(n: usize) -> bool {
    let nom = ETAT.with_borrow(|e| e.as_ref()?.controles.iter().find(|(c, _)| c.contains("claude-voix/etat")).map(|(c, _)| c.clone()));
    match nom {
        Some(nom) => cliquer(n, &nom),
        None => { noter(n, false, "indicateur de Claude vocal introuvable"); false }
    }
}

/// Même chemin qu'un appui long reconnu : la commande longue du bouton.
fn appui_long(n: usize, nom: &str) -> bool {
    match controle(nom) {
        Some(b) => { super::declencher(super::etiquette(b), true); true }
        None => { noter(n, false, &format!("contrôle « {nom} » introuvable")); false }
    }
}

fn verifier_slider(n: usize, nom: &str, lire: fn() -> Result<f32, String>, vers_le_bas: bool) {
    let Ok(origine) = lire() else { return noter(n, false, &format!("{nom} illisible")) };
    let cible_ = if vers_le_bas || origine > 0.5 { (origine - 0.1).max(0.0) } else { origine + 0.1 };
    glisser(&format!("slider {nom}"), cible_);
    let relue = lire().unwrap_or(-1.0);
    glisser(&format!("slider {nom}"), origine);
    let restauree = lire().unwrap_or(-1.0);
    noter(n, (relue - cible_).abs() < 0.03 && (restauree - origine).abs() < 0.03,
        &format!("slider {nom} : {origine:.2} → {cible_:.2}, relu {relue:.2}, restauré {restauree:.2}"));
}

/// Écran qui doit être visible après l'action de l'étape n (comparé aux captures par scripts/verifier.sh).
fn attendu(n: usize) -> &'static str {
    match n {
        1 | 2 | 4 | 5 => "sous",
        7 | 8 => "apple",
        20 => "erreur",
        _ => "principale",
    }
}

pub(super) fn etape() {
    let n = ETAPE.get();
    ETAPE.set(n + 1);
    eprintln!("AUTOTEST {n} pas attendu={}", attendu(n));
    let suivante = match n {
        0 => { noter(n, COURANTE.get() == principale(), "barre principale présentée"); true }
        1 => cliquer(n, "ouvrir sun.max.fill"),
        2 => { noter(n, COURANTE.get() != principale(), "sous-barre luminosité ouverte"); verifier_slider(n, "luminosite", luminosite::lire, false); true }
        3 => cliquer(n, "fermer"),
        4 => { noter(n, COURANTE.get() == principale(), "✕ ramène la barre principale"); cliquer(n, "ouvrir speaker.wave.2.fill") }
        5 => { noter(n, COURANTE.get() != principale(), "sous-barre volume ouverte"); verifier_slider(n, "volume", volume::lire, true); true }
        6 => cliquer(n, "fermer"),
        7 => { noter(n, COURANTE.get() == principale(), "✕ ramène la barre principale"); cliquer(n, "apple.logo") }
        8 => { eprintln!("AUTOTEST {n} info barre d'Apple demandée (voir la capture : ✳ doit être dans le Control Strip)"); true }
        9 => { unsafe { let _: () = msg_send![cible(), afficherBarre: std::ptr::null_mut::<AnyObject>()]; } noter(n, COURANTE.get() == principale(), "✳ ramène la barre Tactile"); true }
        10 => match controle("✳ Claude") {
            Some(_) => { CONVERSATION.set(lignes_conversation()); cliquer(n, "✳ Claude") }
            None => { eprintln!("AUTOTEST {n} info pas de bouton ✳ Claude, étapes vocales sautées"); ETAPE.set(17); true }
        },
        11 => { verifier_ecoute(n); cliquer_indicateur(n) }
        12 => { verifier_annule(n, "toucher l'indicateur annule"); cliquer(n, "✳ Claude") }
        13 => { verifier_ecoute(n); appui_long(n, "✳ Claude") }
        14 => { verifier_annule(n, "appui long sur ✳ Claude annule"); cliquer(n, "✳ Claude") }
        15 => { verifier_ecoute(n); cliquer(n, "✳ Claude") }
        16 => { let e = etat_vocal(); noter(n, !e.starts_with('🎙'), &format!("second appui envoie la question (état « {e} »)")); true }
        17 => match media::en_cours() {
            Ok(Some(m)) => { LECTURE.set(Some(m.en_lecture)); cliquer(n, "en_cours") }
            _ => { eprintln!("AUTOTEST {n} info rien ne joue, étapes musique sautées"); ETAPE.set(19); true }
        },
        18 => {
            let avant = LECTURE.get().unwrap_or(false);
            let apres = media::en_cours().ok().flatten().map(|m| m.en_lecture);
            noter(n, apres == Some(!avant), &format!("appui sur le titre : lecture {avant} → {apres:?}"));
            cliquer(n, "en_cours")
        }
        19 => {
            if let Some(avant) = LECTURE.get() {
                let apres = media::en_cours().ok().flatten().map(|m| m.en_lecture);
                noter(n, apres == Some(avant), &format!("second appui : lecture revenue à {avant}"));
            }
            true
        }
        20 => { super::afficher_erreur("essai, ligne 6 : valeur inconnue «truc»"); noter(n, controle("erreur").is_some(), "config refusée : « ⚠ config ligne 6 » dans la barre"); true }
        21 => {
            let config = ETAT.with_borrow(|e| e.as_ref().map(|e| e.config.clone()));
            super::installer(&config.unwrap());
            noter(n, controle("erreur").is_none() && COURANTE.get() == principale(), "config valide : l'erreur disparaît");
            eprintln!("AUTOTEST fin {} échec(s)", ECHECS.get());
            false
        }
        _ => false,
    };
    if suivante {
        programmer();
    }
}
