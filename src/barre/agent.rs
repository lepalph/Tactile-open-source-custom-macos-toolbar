//! Barre temporaire de l'agent (MCP, voir `crate::agent`) : un message ou une question avec des boutons,
//! présentés par-dessus la barre de l'utilisateur, puis la barre normale revient (`base()`, comme après le ✳ de `demarrage`).
//!
//! La socket est écoutée dans un fil ; ici seul le fil principal touche à AppKit : `soumettre` (fil de la
//! connexion) dépose la requête et réveille le `tic`, `pas` (fil principal, à chaque `tic`) la présente et la termine.
//! Une seule demande à la fois : la suivante reçoit « occupé » tout de suite (l'agent réessaie quand il veut,
//! plutôt qu'une file qui empile des questions périmées devant l'utilisateur).

use super::vues::{barre_touch, item_vue, lacher, largeur_fixe, largeur_max, texte};
use super::{base, cible, montrer, presenter, Id, PREFIXE};
use crate::agent::Requete;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send, sel};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant};

type Reponse = Sender<Result<String, String>>;

/// La requête déposée par un fil de connexion, en attente du prochain `tic`.
static ATTENTE: Mutex<Option<(Requete, Reponse)>> = Mutex::new(None);
/// Vrai de `soumettre` jusqu'au retour de la barre normale.
static OCCUPE: AtomicBool = AtomicBool::new(false);

struct Actif {
    barre: Id,
    /// `demander` : attend le libellé touché ; `afficher` a déjà répondu.
    reponse: Option<Reponse>,
    fin: Instant,
    choix: Vec<String>,
    /// Posé par l'appui d'un bouton ; la barre n'est défaite qu'au `tic` suivant, pas dans l'action du bouton.
    resultat: Option<String>,
}

thread_local! {
    static ACTIF: RefCell<Option<Actif>> = const { RefCell::new(None) };
}

/// Appelé depuis le fil d'une connexion : bloque jusqu'à la réponse (libellé touché, « aucun », ou fin de l'affichage lancé).
pub(crate) fn soumettre(requete: Requete) -> Result<String, String> {
    if OCCUPE.swap(true, Ordering::AcqRel) {
        return Err("occupé : une autre demande est déjà sur la Touch Bar, réessaie dans un instant".into());
    }
    let (envoi, recu) = channel();
    let delai = requete.duree_max();
    *ATTENTE.lock().unwrap() = Some((requete, envoi));
    super::reveiller();
    recu.recv_timeout(Duration::from_secs(delai + 10)).unwrap_or_else(|_| {
        // Le fil principal n'a rien pris : on libère. S'il a pris la requête, c'est lui qui libérera.
        if ATTENTE.lock().unwrap().take().is_some() {
            OCCUPE.store(false, Ordering::Release);
        }
        Err("la Touch Bar n'a pas répondu".into())
    })
}

/// Fil principal, à chaque `tic` : termine la barre échue ou touchée, puis présente la requête en attente.
pub(super) fn pas() {
    let a_terminer = ACTIF.with_borrow(|a| a.as_ref().is_some_and(|a| a.resultat.is_some() || Instant::now() >= a.fin));
    if a_terminer {
        terminer();
    }
    if ACTIF.with_borrow(Option::is_none)
        && let Some((requete, reponse)) = ATTENTE.lock().unwrap().take()
    {
        presenter_requete(requete, reponse);
    }
}

/// Un bouton a été touché : `etiquette` 0 = ✕, k = k-ième choix.
pub(super) fn choisi(etiquette: isize) {
    ACTIF.with_borrow_mut(|a| {
        if let Some(a) = a.as_mut().filter(|a| a.resultat.is_none()) {
            a.resultat = Some(if etiquette <= 0 { "aucun".into() } else { a.choix.get(etiquette as usize - 1).cloned().unwrap_or_else(|| "aucun".into()) });
        }
    });
}

fn terminer() {
    let Some(a) = ACTIF.take() else { return };
    if let Some(r) = a.reponse {
        let _ = r.send(Ok(a.resultat.unwrap_or_else(|| "aucun".into())));
    }
    if let Some(b) = base() {
        montrer(b);
    }
    lacher(a.barre);
    OCCUPE.store(false, Ordering::Release);
}

fn bouton(titre: &str, etiquette: isize, largeur: f64) -> Id {
    unsafe {
        let b: Id = msg_send![class!(NSButton), buttonWithTitle: &*texte(titre), target: cible(), action: sel!(agentChoix:)];
        let _: () = msg_send![b, setTag: etiquette];
        largeur_max(b, largeur);
        b
    }
}

fn presenter_requete(requete: Requete, reponse: Reponse) {
    let (libelle, choix, duree, repond_plus_tard) = match &requete {
        Requete::Afficher { texte, duree_s } => (texte.clone(), Vec::new(), *duree_s, false),
        Requete::Demander { question, choix, delai_s } => (question.clone(), choix.clone(), *delai_s, true),
    };
    let barre = unsafe {
        let label: Id = msg_send![class!(NSTextField), labelWithString: &*texte(&libelle)];
        let police: Id = msg_send![class!(NSFont), systemFontOfSize: 15.0f64, weight: 0.3f64];
        let _: () = msg_send![label, setFont: police];
        let _: () = msg_send![label, setLineBreakMode: 4isize];
        largeur_max(label, if choix.is_empty() { 760.0 } else { 360.0 });
        let flexible = std::ptr::null_mut::<AnyObject>();
        let mut items = Vec::new();
        if repond_plus_tard {
            let croix = bouton("✕", 0, 64.0);
            largeur_fixe(croix, 64.0);
            items.push(item_vue(&format!("{PREFIXE}.agent.fermer"), croix));
        } else {
            items.push(flexible);
        }
        items.push(item_vue(&format!("{PREFIXE}.agent.texte"), label));
        items.push(flexible);
        for (i, c) in choix.iter().enumerate() {
            let b = bouton(c, i as isize + 1, 150.0);
            items.push(item_vue(&format!("{PREFIXE}.agent.{}", i + 1), b));
        }
        barre_touch(&format!("{PREFIXE}.agent"), items)
    };
    presenter(barre);
    // Banc d'essai : TACTILE_AGENT_APPUI=n touche le n-ième choix 2 s après (jamais de vrai doigt dans un script).
    if let Some(n) = std::env::var("TACTILE_AGENT_APPUI").ok().and_then(|n| n.parse::<usize>().ok()).filter(|_| repond_plus_tard) {
        unsafe {
            let bouton = bouton_numero(barre, n.max(1));
            if !bouton.is_null() {
                let _: () = msg_send![bouton, performSelector: sel!(performClick:), withObject: std::ptr::null_mut::<AnyObject>(), afterDelay: 2.0f64];
            }
        }
    }
    if !repond_plus_tard {
        let _ = reponse.send(Ok("affiché".into()));
    }
    ACTIF.set(Some(Actif {
        barre,
        reponse: repond_plus_tard.then_some(reponse),
        fin: Instant::now() + Duration::from_secs(duree),
        choix,
        resultat: None,
    }));
}

/// La vue du n-ième bouton de choix (1 = premier) dans `barre`, ou nul.
unsafe fn bouton_numero(barre: Id, n: usize) -> Id {
    unsafe {
        let item: Id = msg_send![barre, itemForIdentifier: &*texte(&format!("{PREFIXE}.agent.{n}"))];
        if item.is_null() { item } else { msg_send![item, view] }
    }
}
