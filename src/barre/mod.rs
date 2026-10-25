//! La Touch Bar : construction depuis la config, présentation en barre système, rafraîchissements.
//!
//! Les popovers d'AppKit ne s'ouvrent pas dans une barre système : un slider ou un groupe
//! est donc une sous-barre système à part entière, présentée à la place de la principale,
//! avec un bouton ✕ qui ramène la principale.

use crate::actions::{self, media, Action};
use crate::config::{Barre, Element, Reglage};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool, NSObject};
use objc2::{class, define_class, msg_send, sel, ClassType};
use objc2_foundation::NSString;
use std::cell::RefCell;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

type Id = *mut AnyObject;

#[derive(Clone)]
enum Commande {
    Systeme(Action),
    Ouvrir(usize),
    Fermer,
}

enum Rafraichi {
    Heure { bouton: Id, formateur: Id },
    EnCours { bouton: Id },
    Script { bouton: Id, commande: String, intervalle: Duration, prochain: Instant, en_cours: bool },
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

fn texte(s: &str) -> Retained<NSString> {
    NSString::from_str(s)
}

fn cible() -> Id {
    CIBLE.with_borrow(|c| Retained::as_ptr(c.as_ref().unwrap()) as Id)
}

fn symbole(nom: &str) -> Id {
    unsafe { msg_send![class!(NSImage), imageWithSystemSymbolName: &*texte(nom), accessibilityDescription: std::ptr::null_mut::<AnyObject>()] }
}

fn couleur(hexa: &str) -> Option<Id> {
    let h = hexa.trim_start_matches('#');
    let v = u32::from_str_radix(h, 16).ok().filter(|_| h.len() == 6)?;
    let c = |d: u32| ((v >> d) & 0xff) as f64 / 255.0;
    Some(unsafe { msg_send![class!(NSColor), colorWithSRGBRed: c(16), green: c(8), blue: c(0), alpha: 1.0f64] })
}

fn largeur_fixe(vue: Id, largeur: f64) {
    unsafe {
        let ancre: Id = msg_send![vue, widthAnchor];
        let contrainte: Id = msg_send![ancre, constraintEqualToConstant: largeur];
        let _: () = msg_send![contrainte, setActive: Bool::YES];
    }
}

fn item_vue(identifiant: &str, vue: Id) -> Id {
    unsafe {
        let item: Id = msg_send![class!(NSCustomTouchBarItem), alloc];
        let item: Id = msg_send![item, initWithIdentifier: &*texte(identifiant)];
        let _: () = msg_send![item, setView: vue];
        item
    }
}

fn enregistrer(etat: &mut Etat, court: Option<Commande>, long: Option<Commande>) -> usize {
    etat.commandes.push((court, long));
    etat.commandes.len() - 1
}

fn bouton(titre: Option<&str>, image: Option<&str>, index: usize, avec_appui_long: bool) -> Id {
    unsafe {
        let b: Id = match image {
            Some(nom) => msg_send![class!(NSButton), buttonWithImage: symbole(nom), target: cible(), action: sel!(appui:)],
            None => msg_send![class!(NSButton), buttonWithTitle: &*texte(titre.unwrap_or("")), target: cible(), action: sel!(appui:)],
        };
        if let (Some(t), Some(_)) = (titre, image) {
            let _: () = msg_send![b, setTitle: &*texte(t)];
            let _: () = msg_send![b, setImagePosition: 2isize];
        }
        let _: () = msg_send![b, setTag: index as isize];
        if avec_appui_long {
            let r: Id = msg_send![class!(NSPressGestureRecognizer), alloc];
            let r: Id = msg_send![r, initWithTarget: cible(), action: sel!(appuiLong:)];
            let _: () = msg_send![r, setMinimumPressDuration: 0.5f64];
            let _: () = msg_send![r, setAllowedTouchTypes: 1usize];
            let _: () = msg_send![b, addGestureRecognizer: r];
        }
        b
    }
}

fn bouton_texte(etat: &mut Etat, action: Option<Action>) -> Id {
    let index = enregistrer(etat, action.map(Commande::Systeme), None);
    let b = bouton(Some(""), None, index, false);
    unsafe { let _: () = msg_send![b, setBordered: Bool::NO]; }
    b
}

/// Bouton replié (symbole + chevron) qui ouvre une sous-barre faite de ✕ suivi de `items`.
fn depliable(identifiant: &str, nom: Option<&str>, libelle: Option<&str>, items: Vec<Id>, etat: &mut Etat) -> Id {
    let fermer = enregistrer(etat, Some(Commande::Fermer), None);
    let croix = bouton(None, Some("xmark"), fermer, false);
    largeur_fixe(croix, 64.0);
    let mut contenu = vec![item_vue(&format!("{identifiant}.fermer"), croix)];
    contenu.extend(items);
    etat.barres.push(barre_touch(&format!("{identifiant}.barre"), contenu));
    let ouvrir = enregistrer(etat, Some(Commande::Ouvrir(etat.barres.len() - 1)), None);
    let titre = format!("{}  ›", libelle.unwrap_or(""));
    let b = bouton(Some(titre.trim_start()), nom, ouvrir, false);
    largeur_fixe(b, 72.0);
    item_vue(identifiant, b)
}

fn slider(identifiant: &str, reglage: Reglage, nom: &str, etat: &mut Etat) -> Id {
    let (valeur, tag, petit, grand) = match reglage {
        Reglage::Luminosite => (actions::luminosite::lire(), 0isize, "sun.min.fill", "sun.max.fill"),
        Reglage::Volume => (actions::volume::lire(), 1, "speaker.fill", "speaker.wave.3.fill"),
    };
    let item = unsafe {
        let item: Id = msg_send![class!(NSSliderTouchBarItem), alloc];
        let item: Id = msg_send![item, initWithIdentifier: &*texte(&format!("{identifiant}.slider"))];
        let s: Id = msg_send![item, slider];
        let _: () = msg_send![s, setMinValue: 0.0f64];
        let _: () = msg_send![s, setMaxValue: 1.0f64];
        let _: () = msg_send![s, setDoubleValue: valeur.unwrap_or(0.5) as f64];
        let _: () = msg_send![s, setTag: tag];
        let _: () = msg_send![s, setContinuous: Bool::YES];
        let acc_min: Id = msg_send![class!(NSSliderAccessory), accessoryWithImage: symbole(petit)];
        let acc_max: Id = msg_send![class!(NSSliderAccessory), accessoryWithImage: symbole(grand)];
        let _: () = msg_send![item, setMinimumValueAccessory: acc_min];
        let _: () = msg_send![item, setMaximumValueAccessory: acc_max];
        let _: () = msg_send![item, setMinimumSliderWidth: 600.0f64];
        let _: () = msg_send![item, setTarget: cible()];
        let _: () = msg_send![item, setAction: sel!(glisse:)];
        item
    };
    depliable(identifiant, Some(nom), None, vec![item], etat)
}

fn barre_touch(identifiant: &str, items: Vec<Id>) -> Id {
    unsafe {
        let barre: Id = msg_send![class!(NSTouchBar), new];
        let _: () = msg_send![barre, setCustomizationIdentifier: &*texte(identifiant)];
        let idents: Id = msg_send![class!(NSMutableArray), array];
        let ensemble: Id = msg_send![class!(NSMutableSet), set];
        for item in items {
            if item.is_null() {
                let _: () = msg_send![idents, addObject: &*texte("NSTouchBarItemIdentifierFlexibleSpace")];
                continue;
            }
            let id: Id = msg_send![item, identifier];
            let _: () = msg_send![idents, addObject: id];
            let _: () = msg_send![ensemble, addObject: item];
        }
        let _: () = msg_send![barre, setDefaultItemIdentifiers: idents];
        let _: () = msg_send![barre, setTemplateItems: ensemble];
        barre
    }
}

fn construire(elements: &[Element], prefixe: &str, etat: &mut Etat) -> Vec<Id> {
    let mut items = Vec::new();
    for (i, element) in elements.iter().enumerate() {
        let ident = format!("{prefixe}.{i}");
        let item = match element {
            Element::Bouton { texte: t, symbole: s, couleur: c, largeur, action, appui_long } => {
                let index = enregistrer(etat, action.clone().map(Commande::Systeme), appui_long.clone().map(Commande::Systeme));
                let b = bouton(t.as_deref(), s.as_deref(), index, appui_long.is_some());
                if let Some(col) = c.as_deref().and_then(couleur) {
                    unsafe { let _: () = msg_send![b, setBezelColor: col]; }
                }
                if let Some(l) = largeur {
                    largeur_fixe(b, *l);
                }
                item_vue(&ident, b)
            }
            Element::Slider { reglage, symbole: s } => {
                let defaut = if *reglage == Reglage::Luminosite { "sun.max.fill" } else { "speaker.wave.2.fill" };
                slider(&ident, *reglage, s.as_deref().unwrap_or(defaut), etat)
            }
            Element::Heure { format } => {
                let b = bouton_texte(etat, None);
                let formateur: Id = unsafe { msg_send![class!(NSDateFormatter), new] };
                unsafe { let _: () = msg_send![formateur, setDateFormat: &*texte(format.as_deref().unwrap_or("HH:mm"))]; }
                etat.rafraichis.push(Rafraichi::Heure { bouton: b, formateur });
                item_vue(&ident, b)
            }
            Element::EnCours { largeur } => {
                let b = bouton_texte(etat, Some(Action::LecturePause));
                if let Some(l) = largeur {
                    largeur_fixe(b, *l);
                }
                etat.rafraichis.push(Rafraichi::EnCours { bouton: b });
                item_vue(&ident, b)
            }
            Element::Script { commande, intervalle, largeur, action } => {
                let b = bouton_texte(etat, action.clone());
                if let Some(l) = largeur {
                    largeur_fixe(b, *l);
                }
                etat.rafraichis.push(Rafraichi::Script {
                    bouton: b,
                    commande: commande.clone(),
                    intervalle: Duration::from_secs_f64(intervalle.max(0.1)),
                    prochain: Instant::now(),
                    en_cours: false,
                });
                item_vue(&ident, b)
            }
            Element::Espace { largeur } => {
                let vue: Id = unsafe { msg_send![class!(NSView), new] };
                largeur_fixe(vue, *largeur);
                item_vue(&ident, vue)
            }
            Element::Souple => std::ptr::null_mut(),
            Element::Groupe { texte: t, symbole: s, elements: enfants } => {
                let sous_items = construire(enfants, &ident, etat);
                depliable(&ident, s.as_deref(), t.as_deref(), sous_items, etat)
            }
        };
        items.push(item);
    }
    items
}

fn titre(bouton: Id, valeur: &str) {
    unsafe {
        let actuel: Id = msg_send![bouton, title];
        if (*(actuel as *const NSString)).to_string() != valeur {
            let _: () = msg_send![bouton, setTitle: &*texte(valeur)];
        }
    }
}

fn rafraichir() {
    ETAT.with_borrow_mut(|etat| {
        let Some(etat) = etat.as_mut() else { return };
        while let Ok((i, sortie)) = etat.reception.try_recv() {
            match etat.rafraichis.get_mut(i) {
                Some(Rafraichi::Script { bouton, en_cours, .. }) => {
                    *en_cours = false;
                    titre(*bouton, sortie.trim_end());
                }
                Some(Rafraichi::EnCours { bouton }) => titre(*bouton, &sortie),
                _ => {}
            }
        }
        let maintenant = Instant::now();
        let media_du = maintenant >= etat.prochain_media;
        if media_du {
            etat.prochain_media = maintenant + Duration::from_secs(2);
        }
        for (i, r) in etat.rafraichis.iter_mut().enumerate() {
            match r {
                Rafraichi::Heure { bouton, formateur } => unsafe {
                    let date: Id = msg_send![class!(NSDate), date];
                    let s: Id = msg_send![*formateur, stringFromDate: date];
                    titre(*bouton, &(*(s as *const NSString)).to_string());
                },
                Rafraichi::EnCours { .. } if media_du => {
                    let envoi = etat.envoi.clone();
                    std::thread::spawn(move || {
                        let s = match media::en_cours() {
                            Ok(Some(m)) => {
                                let signe = if m.en_lecture { "♫" } else { "❚❚" };
                                let ligne = if m.artiste.is_empty() { format!("{signe}  {}", m.titre) } else { format!("{signe}  {}  ·  {}", m.titre, m.artiste) };
                                ligne.chars().take(44).collect()
                            }
                            _ => String::new(),
                        };
                        let _ = envoi.send((i, s));
                    });
                }
                Rafraichi::Script { commande, intervalle, prochain, en_cours, .. } if !*en_cours && maintenant >= *prochain => {
                    *en_cours = true;
                    *prochain = maintenant + *intervalle;
                    let (envoi, commande) = (etat.envoi.clone(), commande.clone());
                    std::thread::spawn(move || {
                        let sortie = std::process::Command::new("/bin/zsh").args(["-c", &commande]).output();
                        let s = sortie.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
                        let _ = envoi.send((i, s));
                    });
                }
                _ => {}
            }
        }
    });
}

fn presenter(barre: Id) {
    COURANTE.set(barre);
    unsafe {
        let plateau = texte(PLATEAU);
        let _: () = msg_send![class!(NSTouchBar), presentSystemModalTouchBar: barre, placement: 1i64, systemTrayItemIdentifier: &*plateau];
    }
}

const PLATEAU: &str = "fr.palph.tactile.plateau";

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

/// Place l'icône ✳ dans le Control Strip d'Apple (API privées de DFRFoundation, comme MTMR).
fn installer_plateau() {
    unsafe extern "C" {
        fn dlopen(chemin: *const std::ffi::c_char, mode: i32) -> *mut std::ffi::c_void;
        fn dlsym(module: *mut std::ffi::c_void, nom: *const std::ffi::c_char) -> *mut std::ffi::c_void;
    }
    type Presence = unsafe extern "C" fn(Id, Bool);
    unsafe {
        let b: Id = msg_send![class!(NSButton), buttonWithTitle: &*texte("✳"), target: cible(), action: sel!(afficherBarre:)];
        let item = item_vue(PLATEAU, b);
        let _: () = msg_send![class!(NSTouchBarItem), addSystemTrayItem: item];
        let module = dlopen(c"/System/Library/PrivateFrameworks/DFRFoundation.framework/DFRFoundation".as_ptr(), 2);
        let f = dlsym(module, c"DFRElementSetControlStripPresenceForIdentifier".as_ptr());
        if f.is_null() {
            eprintln!("tactile : icône du Control Strip indisponible");
            return;
        }
        let presence: Presence = std::mem::transmute(f);
        presence(Retained::as_ptr(&texte(PLATEAU)) as Id, Bool::YES);
    }
}

fn installer(config: &Barre) {
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
        installer(&config);
        if std::env::var("TACTILE_MASQUER").is_ok() {
            let _: () = msg_send![cible(), performSelector: sel!(masquerBarre:), withObject: std::ptr::null_mut::<AnyObject>(), afterDelay: 2.0f64];
        }

        let _: Id = msg_send![class!(NSTimer), scheduledTimerWithTimeInterval: 0.1f64, target: cible(), selector: sel!(tic:), userInfo: std::ptr::null_mut::<AnyObject>(), repeats: Bool::YES];
        rafraichir();
        let _: () = msg_send![app, run];
    }
}
