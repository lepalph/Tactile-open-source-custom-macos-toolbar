//! Autotest (`TACTILE_AUTOTEST=1`) : appuie sur chaque contrôle par le même chemin qu'un doigt
//! et vérifie l'effet réel. Une étape toutes les 2,5 s, pour laisser `scripts/verifier.sh` capturer
//! la barre entre deux. Écrit « AUTOTEST n ok|ÉCHEC … » sur la sortie d'erreur, puis « AUTOTEST fin ».

use objc2::runtime::AnyObject;
use objc2::{msg_send, sel};
use std::cell::Cell;

use super::{cible, Id, COURANTE, ETAT};
use crate::actions::{self, luminosite, media, volume, Action};

thread_local! {
    static ETAPE: Cell<usize> = const { Cell::new(0) };
    static ECHECS: Cell<usize> = const { Cell::new(0) };
    static LECTURE: Cell<Option<bool>> = const { Cell::new(None) };
    static CONVERSATION: Cell<usize> = const { Cell::new(0) };
}

const ETAT_VOCAL: &str = "/tmp/claude-voix/etat";

/// Barre des étapes 22-26 : script JSON (vert, en tête : c'est ce que verifier.sh reconnaît), image, groupe large,
/// batterie, raccourci sans effet (f15), app inoffensive et AppleScript qui la ferme.
const BARRE_RICHE: &str = r##"
[[element]]
type = "script"
commande = '''echo '{"texte":"ok","couleur":"#00AA00"}' '''
intervalle = 5
largeur = 60

[[element]]
type = "bouton"
texte = "Pr"
image = "/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/GenericApplicationIcon.icns"
couleur_texte = "#FFFFFF"
taille = 15

[[element]]
type = "groupe"
texte = "Export"
largeur = 140

[[element]]
type = "batterie"

[[element]]
type = "bouton"
symbole = "keyboard"
action = { touche = "f15" }

[[element]]
type = "bouton"
texte = "TextEdit"
action = { app = "TextEdit" }

[[element]]
type = "bouton"
texte = "Quit"
action = { applescript = 'tell application "TextEdit" to quit' }
"##;

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

fn titre(nom: &str) -> String {
    controle(nom).map(|b| unsafe { let t: Id = msg_send![b, title]; (*(t as *const objc2_foundation::NSString)).to_string() }).unwrap_or_default()
}

fn a_une_image(nom: &str) -> bool {
    controle(nom).is_some_and(|b| { let i: Id = unsafe { msg_send![b, image] }; !i.is_null() })
}

fn textedit_ouvert() -> bool {
    std::process::Command::new("/usr/bin/pgrep").args(["-x", "TextEdit"]).output().is_ok_and(|s| s.status.success())
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
        22..=25 => "riche",
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
            true
        }
        22 => {
            super::installer(&crate::config::lire(BARRE_RICHE).expect("barre riche invalide"));
            let manquants: Vec<&str> = ["Pr", "ouvrir Export", "batterie", "keyboard", "TextEdit", "Quit"].into_iter().filter(|c| controle(c).is_none()).collect();
            noter(n, manquants.is_empty(), &format!("barre riche installée (manquants : {manquants:?})"));
            true
        }
        23 => {
            let script = ETAT.with_borrow(|e| e.as_ref().and_then(|e| e.controles.iter().find(|(c, _)| c.starts_with("script echo")).map(|(c, _)| c.clone()))).unwrap_or_default();
            let fond: Id = controle(&script).map_or(std::ptr::null_mut(), |b| unsafe { msg_send![b, bezelColor] });
            noter(n, titre(&script) == "ok" && !fond.is_null(), &format!("script JSON : texte « {} », fond coloré {}", titre(&script), !fond.is_null()));
            noter(n, a_une_image("Pr") && titre("Pr") == "Pr", "bouton avec image de fichier et texte");
            let pile = titre("batterie");
            noter(n, pile.trim_end_matches(" %").parse::<u8>().is_ok() && a_une_image("batterie"), &format!("batterie « {pile} » avec symbole"));
            noter(n, actions::executer(&Action::Touche(crate::actions::clavier::Touche::try_from("f15".to_string()).unwrap())).is_ok(), "raccourci f15 envoyé");
            cliquer(n, "keyboard") && cliquer(n, "TextEdit")
        }
        24 => { noter(n, textedit_ouvert(), "action app : TextEdit ouvert"); cliquer(n, "Quit") }
        25 => { noter(n, !textedit_ouvert(), "action applescript : TextEdit fermé"); true }
        26 => {
            let config = ETAT.with_borrow(|e| e.as_ref().map(|e| e.config.clone()));
            super::installer(&crate::config::charger().unwrap_or_else(|_| config.unwrap()));
            noter(n, controle("Quit").is_none() && COURANTE.get() == principale(), "barre de l'utilisateur rétablie");
            eprintln!("AUTOTEST fin {} échec(s)", ECHECS.get());
            false
        }
        _ => false,
    };
    if suivante {
        programmer();
    }
}
