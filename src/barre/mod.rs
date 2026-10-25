//! La Touch Bar : construction depuis la config, présentation en barre système, rafraîchissements.
//!
//! Les popovers d'AppKit ne s'ouvrent pas dans une barre système : un slider ou un groupe
//! est donc une sous-barre système à part entière, présentée à la place de la principale,
//! avec un bouton ✕ qui ramène la principale.

mod elements;
mod plateau;
mod rafraichi;
mod vues;

use crate::actions::{self, Action};
use crate::config::Barre;
use elements::construire;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool, NSObject};
use objc2::{class, define_class, msg_send, sel, ClassType};
use plateau::{installer_plateau, PLATEAU};
use rafraichi::{rafraichir, Rafraichi};
use std::cell::RefCell;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Instant;
use vues::{barre_touch, texte};

type Id = *mut AnyObject;

#[derive(Clone)]
enum Commande {
    Systeme(Action),
    Ouvrir(usize),
    Fermer,
}


struct Etat {
    commandes: Vec<(Option<Commande>, Option<Commande>)>,
    barres: Vec<Id>,
    rafraichis: Vec<Rafraichi>,
    prochain_media: Instant,
    envoi: Sender<(usize, String)>,
    reception: Receiver<(usize, String)>,
}

thread_local! {
    static ETAT: RefCell<Option<Etat>> = const { RefCell::new(None) };
    static CIBLE: RefCell<Option<Retained<Cible>>> = const { RefCell::new(None) };
    static PLEIN_ECRAN: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
    static COURANTE: std::cell::Cell<Id> = const { std::cell::Cell::new(std::ptr::null_mut()) };
    static RECHARGES: RefCell<Option<Receiver<Result<Barre, String>>>> = const { RefCell::new(None) };
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "TactileCible"]
    struct Cible;

    impl Cible {
        #[unsafe(method(appui:))]
        fn appui(&self, envoyeur: Id) {
            declencher(etiquette(envoyeur), false);
        }

        #[unsafe(method(appuiLong:))]
        fn appui_long(&self, reconnaisseur: Id) {
            let etat: isize = unsafe { msg_send![reconnaisseur, state] };
            if etat == 1 {
                let vue: Id = unsafe { msg_send![reconnaisseur, view] };
                declencher(etiquette(vue), true);
            }
        }

        #[unsafe(method(glisse:))]
        fn glisse(&self, item: Id) {
            let slider: Id = unsafe { msg_send![item, slider] };
            let valeur: f64 = unsafe { msg_send![slider, doubleValue] };
            let action = if etiquette(slider) == 0 { Action::Luminosite(valeur as f32) } else { Action::Volume(valeur as f32) };
            signaler(actions::executer(&action));
        }

        #[unsafe(method(afficherBarre:))]
        fn afficher_barre(&self, _envoyeur: Id) {
            afficher();
        }

        #[unsafe(method(masquerBarre:))]
        fn masquer_barre(&self, _envoyeur: Id) {
            masquer();
        }

        #[unsafe(method(tic:))]
        fn tic(&self, _minuteur: Id) {
            let recharge = RECHARGES.with_borrow(|r| r.as_ref().and_then(|r| r.try_recv().ok()));
            match recharge {
                Some(Ok(config)) => installer(&config),
                Some(Err(e)) => eprintln!("tactile : config ignorée, {e}"),
                None => {}
            }
            rafraichir();
        }
    }
);

fn etiquette(vue: Id) -> isize {
    unsafe { msg_send![vue, tag] }
}

fn signaler<T>(resultat: Result<T, String>) {
    if let Err(e) = resultat {
        eprintln!("tactile : {e}");
    }
}

fn declencher(index: isize, long: bool) {
    let trouve = ETAT.with_borrow(|etat| {
        let etat = etat.as_ref()?;
        let (court, long_) = etat.commandes.get(index as usize)?.clone();
        let commande = if long { long_ } else { court }?;
        Some((commande, etat.barres.clone()))
    });
    match trouve {
        Some((Commande::Systeme(Action::BarreApple), _)) => masquer(),
        Some((Commande::Systeme(action), _)) => signaler(actions::executer(&action)),
        Some((Commande::Ouvrir(k), barres)) => presenter(barres[k]),
        Some((Commande::Fermer, barres)) => presenter(barres[0]),
        None => {}
    }
}


fn cible() -> Id {
    CIBLE.with_borrow(|c| Retained::as_ptr(c.as_ref().unwrap()) as Id)
}


fn presenter(barre: Id) {
    COURANTE.set(barre);
    unsafe {
        let plateau = texte(PLATEAU);
        let _: () = msg_send![class!(NSTouchBar), presentSystemModalTouchBar: barre, placement: if PLEIN_ECRAN.get() { 1i64 } else { 0i64 }, systemTrayItemIdentifier: &*plateau];
    }
}


/// Remet la barre tactile à la place de celle d'Apple.
pub fn afficher() {
    if let Some(principale) = ETAT.with_borrow(|e| e.as_ref().map(|e| e.barres[0])) {
        presenter(principale);
    }
}

/// Rend la barre d'Apple ; l'icône ✳ du Control Strip ramène la barre tactile.
pub fn masquer() {
    let courante = COURANTE.get();
    if !courante.is_null() {
        unsafe { let _: () = msg_send![class!(NSTouchBar), minimizeSystemModalTouchBar: courante]; }
    }
}


fn installer(config: &Barre) {
    PLEIN_ECRAN.set(config.masquer_control_strip);
    let (envoi, reception) = channel();
    let mut etat = Etat {
        commandes: Vec::new(),
        barres: vec![std::ptr::null_mut()],
        rafraichis: Vec::new(),
        prochain_media: Instant::now(),
        envoi,
        reception,
    };
    let items = construire(&config.elements, "fr.palph.tactile", &mut etat);
    etat.barres[0] = barre_touch("fr.palph.tactile", items);
    let a_presenter = std::env::var("TACTILE_OUVRIR").ok().and_then(|v| v.parse::<usize>().ok()).and_then(|k| etat.barres.get(k).copied());
    let principale = etat.barres[0];
    ETAT.with_borrow_mut(|place| *place = Some(etat));
    presenter(a_presenter.unwrap_or(principale));
}

pub fn lancer(config: Barre, recharges: Receiver<Result<Barre, String>>) {
    unsafe {
        let app: Id = msg_send![class!(NSApplication), sharedApplication];
        let _: Bool = msg_send![app, setActivationPolicy: 1isize];

        let c: Retained<Cible> = msg_send![Cible::class(), new];
        CIBLE.with_borrow_mut(|place| *place = Some(c));
        RECHARGES.with_borrow_mut(|place| *place = Some(recharges));
        installer_plateau();
        crate::menu::installer();
        installer(&config);
        if std::env::var("TACTILE_MASQUER").is_ok() {
            let _: () = msg_send![cible(), performSelector: sel!(masquerBarre:), withObject: std::ptr::null_mut::<AnyObject>(), afterDelay: 2.0f64];
        }

        let _: Id = msg_send![class!(NSTimer), scheduledTimerWithTimeInterval: 0.1f64, target: cible(), selector: sel!(tic:), userInfo: std::ptr::null_mut::<AnyObject>(), repeats: Bool::YES];
        rafraichir();
        let _: () = msg_send![app, run];
    }
}
