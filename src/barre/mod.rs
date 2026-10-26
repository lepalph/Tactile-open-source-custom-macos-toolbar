//! La Touch Bar : construction depuis la config, présentation en barre système, rafraîchissements.
//!
//! Les popovers d'AppKit ne s'ouvrent pas dans une barre système : un slider ou un groupe
//! est donc une sous-barre (✕ + contenu) dont le contenu remplace celui de la principale
//! dans l'unique barre système.

mod autotest;
mod batterie;
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
use rafraichi::{rafraichir, Processus, Rafraichi, Sortie};
use std::cell::RefCell;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Instant;
use vues::{barre_touch, texte};
/// Pour le validateur de `config::controler` : il juge comme la barre.
pub(crate) use elements::{debordement, largeur_barre};
pub(crate) use vues::{image, symbole};

type Id = *mut AnyObject;

/// Préfixe des identifiants d'items ; la suite (`3`, `3.1`) numérote les éléments de la config.
const PREFIXE: &str = "fr.palph.tactile";

#[derive(Clone)]
enum Commande {
    Systeme(Action),
    Ouvrir(usize),
    Fermer,
}


struct Etat {
    /// La config affichée, pour la reconstruire (autotest).
    config: Barre,
    /// Bouton rouge « ⚠ config ligne N » ajouté à la barre principale après une config refusée.
    erreur: Option<Id>,
    commandes: Vec<(Option<Commande>, Option<Commande>)>,
    barres: Vec<Id>,
    /// Une barre par `[[application]]` de la config, dans le même ordre.
    applis: Vec<Id>,
    rafraichis: Vec<Rafraichi>,
    /// Scripts en cours, tués quand la barre est remplacée.
    processus: Processus,
    prochain_media: Instant,
    envoi: Sender<(usize, Sortie)>,
    reception: Receiver<(usize, Sortie)>,
    /// Contrôles nommés (« ouvrir sun.max.fill », « fermer », « slider luminosite »…), pour l'autotest.
    controles: Vec<(String, Id)>,
}

thread_local! {
    static ETAT: RefCell<Option<Etat>> = const { RefCell::new(None) };
    static CIBLE: RefCell<Option<Retained<Cible>>> = const { RefCell::new(None) };
    static PLEIN_ECRAN: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
    static COURANTE: std::cell::Cell<Id> = const { std::cell::Cell::new(std::ptr::null_mut()) };
    static MODALE: std::cell::Cell<Id> = const { std::cell::Cell::new(std::ptr::null_mut()) };
    /// Autotest écran verrouillé : `loginwindow` garde le premier plan, l'app active est alors simulée.
    static APP_SIMULEE: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
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

        #[unsafe(method(etapeAutotest:))]
        fn etape_autotest(&self, _envoyeur: Id) {
            autotest::etape();
        }

        #[unsafe(method(tic:))]
        fn tic(&self, _minuteur: Id) {
            let recharge = RECHARGES.with_borrow(|r| r.as_ref().and_then(|r| r.try_recv().ok()));
            match recharge {
                Some(Ok(config)) => installer(&config),
                Some(Err(e)) => {
                    eprintln!("tactile : config ignorée, {e}");
                    afficher_erreur(&e);
                }
                None => {}
            }
            suivre_app();
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
        Some((Commande::Ouvrir(k), barres)) => montrer(barres[k]),
        Some((Commande::Fermer, barres)) => montrer(base().unwrap_or(barres[0])),
        None => {}
    }
}


fn cible() -> Id {
    CIBLE.with_borrow(|c| Retained::as_ptr(c.as_ref().unwrap()) as Id)
}


/// L'unique barre système de Tactile. Présentée une seule fois ; ouvrir une sous-barre change son
/// contenu au lieu de présenter une autre barre : les présentations d'Apple sont animées et
/// asynchrones, et deux barres présentées à la suite s'empilent ou se croisent (vu en capture).
fn modale() -> Id {
    if MODALE.get().is_null() {
        let barre: Id = unsafe { msg_send![class!(NSTouchBar), new] };
        MODALE.set(barre);
    }
    MODALE.get()
}

/// Affiche le contenu de `barre` dans la barre système, sans nouvelle présentation.
fn montrer(barre: Id) {
    COURANTE.set(barre);
    unsafe {
        let items: Id = msg_send![barre, templateItems];
        let idents: Id = msg_send![barre, defaultItemIdentifiers];
        let _: () = msg_send![modale(), setTemplateItems: items];
        let _: () = msg_send![modale(), setDefaultItemIdentifiers: idents];
    }
}

/// Affiche `barre` et (re)présente la barre système : au lancement, après la barre d'Apple, au rechargement.
fn presenter(barre: Id) {
    montrer(barre);
    unsafe {
        let plateau = texte(PLATEAU);
        let _: () = msg_send![class!(NSTouchBar), presentSystemModalTouchBar: modale(), placement: if PLEIN_ECRAN.get() { 1i64 } else { 0i64 }, systemTrayItemIdentifier: &*plateau];
    }
}

/// Remet la barre tactile à la place de celle d'Apple.
pub fn afficher() {
    if let Some(barre) = base() {
        presenter(barre);
    }
}

/// Bundle id et nom affiché de l'app au premier plan. Sondé au `tic` plutôt qu'observé
/// (`NSWorkspaceDidActivateApplicationNotification`) : le sondage attend aussi sans effort qu'une sous-barre se ferme.
pub(super) fn app_active() -> (String, String) {
    if let Some(app) = APP_SIMULEE.with_borrow(Clone::clone) {
        return app;
    }
    unsafe {
        let espace: Id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let app: Id = msg_send![espace, frontmostApplication];
        if app.is_null() {
            return Default::default();
        }
        let chaine = |s: Id| if s.is_null() { String::new() } else { (*(s as *const objc2_foundation::NSString)).to_string() };
        (chaine(msg_send![app, bundleIdentifier]), chaine(msg_send![app, localizedName]))
    }
}

/// La barre de base : celle de l'`[[application]]` qui désigne l'app au premier plan, sinon la principale.
fn base() -> Option<Id> {
    ETAT.with_borrow(|e| {
        let e = e.as_ref()?;
        if e.applis.is_empty() {
            return Some(e.barres[0]);
        }
        let (bundle, nom) = app_active();
        Some(e.config.application_pour(&bundle, &nom).map_or(e.barres[0], |k| e.applis[k]))
    })
}

/// Montre la barre de l'app au premier plan. Une sous-barre ouverte reste jusqu'à son ✕, qui ramène alors la bonne base ;
/// sous la barre d'Apple, seul le contenu change, sans nouvelle présentation.
fn suivre_app() {
    let Some(voulue) = base() else { return };
    let courante = COURANTE.get();
    let sur_une_base = ETAT.with_borrow(|e| e.as_ref().is_some_and(|e| courante == e.barres[0] || e.applis.contains(&courante)));
    if courante != voulue && sur_une_base {
        montrer(voulue);
    }
}

/// Rend la barre d'Apple ; l'icône ✳ du Control Strip ramène la barre tactile.
pub fn masquer() {
    if !MODALE.get().is_null() {
        unsafe { let _: () = msg_send![class!(NSTouchBar), minimizeSystemModalTouchBar: MODALE.get()]; }
        plateau::montrer_icone();
    }
}


/// Libellé court de l'élément d'erreur : « ⚠ config ligne 6 ».
fn libelle_erreur(message: &str) -> String {
    match message.find("ligne ") {
        Some(debut) => format!("⚠ config {}", message[debut..].split(" :").next().unwrap_or("").trim()),
        None => "⚠ config invalide".into(),
    }
}

/// Montre l'erreur dans la barre principale elle-même : depuis le Finder, le journal est invisible
/// et les notifications macOS demandent une app notarisée. L'élément disparaît à la config valide
/// suivante, qui reconstruit la barre.
fn afficher_erreur(message: &str) {
    let libelle = libelle_erreur(message);
    ETAT.with_borrow_mut(|etat| {
        let Some(etat) = etat.as_mut() else { return };
        if let Some(b) = etat.erreur {
            return vues::titre(b, &libelle);
        }
        let index = elements::enregistrer(etat, None, None);
        let b = elements::bouton(Some(&libelle), None, index, false, "");
        etat.controles.push(("erreur".into(), b));
        etat.erreur = Some(b);
        let principale = etat.barres[0];
        let ident = format!("{PREFIXE}.erreur");
        unsafe {
            if let Some(rouge) = vues::couleur("#B00020") {
                let _: () = msg_send![b, setBezelColor: rouge];
            }
            let items: Id = msg_send![principale, templateItems];
            let items: Id = msg_send![items, setByAddingObject: vues::item_vue(&ident, b)];
            let _: () = msg_send![principale, setTemplateItems: items];
            let idents: Id = msg_send![principale, defaultItemIdentifiers];
            let idents: Id = msg_send![idents, mutableCopy];
            let _: () = msg_send![idents, insertObject: &*texte(&ident), atIndex: 0usize];
            let _: () = msg_send![principale, setDefaultItemIdentifiers: idents];
        }
        if COURANTE.get() == principale {
            montrer(principale);
        }
    });
}

fn installer(config: &Barre) {
    // Le placement n'est lu qu'à une nouvelle présentation : s'il change, on retire la barre d'abord.
    if PLEIN_ECRAN.get() != config.masquer_control_strip && !MODALE.get().is_null() {
        unsafe { let _: () = msg_send![class!(NSTouchBar), minimizeSystemModalTouchBar: MODALE.get()]; }
    }
    PLEIN_ECRAN.set(config.masquer_control_strip);
    let (envoi, reception) = channel();
    let mut etat = Etat {
        config: config.clone(),
        erreur: None,
        commandes: Vec::new(),
        controles: Vec::new(),
        barres: vec![std::ptr::null_mut()],
        applis: Vec::new(),
        rafraichis: Vec::new(),
        processus: Processus::new(std::sync::Mutex::new(Some(Vec::new()))),
        prochain_media: Instant::now(),
        envoi,
        reception,
    };
    if let Some((total, caches)) = elements::debordement(&config.elements, elements::largeur_barre(config.masquer_control_strip)) {
        eprintln!("tactile : la barre déborde (environ {total:.0} pt pour {:.0}) : {caches} élément(s) sur {} seront cachés", elements::largeur_barre(config.masquer_control_strip), config.elements.len());
    }
    let items = construire(&config.elements, PREFIXE, &mut etat);
    etat.barres[0] = barre_touch(PREFIXE, items);
    let applis = config.applications.iter().enumerate().map(|(k, app)| {
        let prefixe = format!("{PREFIXE}.app{}", k + 1);
        let items = construire(&app.elements_avec(&config.elements), &prefixe, &mut etat);
        barre_touch(&prefixe, items)
    }).collect();
    etat.applis = applis;
    let a_presenter = std::env::var("TACTILE_OUVRIR").ok().and_then(|v| v.parse::<usize>().ok()).and_then(|k| etat.barres.get(k).copied());
    let principale = etat.barres[0];
    ETAT.with_borrow_mut(|place| {
        if let Some(ancien) = place.as_ref() {
            rafraichi::tuer_tous(&ancien.processus);
        }
        *place = Some(etat);
    });
    presenter(a_presenter.or(base()).unwrap_or(principale));
}

/// Crée la cible des contrôles (une fois) ; les tests construisent des boutons sans lancer l'app.
fn preparer_cible() {
    CIBLE.with_borrow_mut(|place| {
        if place.is_none() {
            *place = Some(unsafe { msg_send![Cible::class(), new] });
        }
    });
}

pub fn lancer(config: Barre, recharges: Receiver<Result<Barre, String>>) {
    unsafe {
        let app: Id = msg_send![class!(NSApplication), sharedApplication];
        let _: Bool = msg_send![app, setActivationPolicy: 1isize];

        preparer_cible();
        RECHARGES.with_borrow_mut(|place| *place = Some(recharges));
        installer_plateau();
        crate::menu::installer();
        installer(&config);
        if std::env::var("TACTILE_MASQUER").is_ok() {
            let _: () = msg_send![cible(), performSelector: sel!(masquerBarre:), withObject: std::ptr::null_mut::<AnyObject>(), afterDelay: 2.0f64];
        }
        if std::env::var("TACTILE_AUTOTEST").is_ok() {
            autotest::programmer();
        }
        if std::env::var("TACTILE_RETOUR").is_ok() {
            let _: () = msg_send![cible(), performSelector: sel!(afficherBarre:), withObject: std::ptr::null_mut::<AnyObject>(), afterDelay: 4.0f64];
        }

        let _: Id = msg_send![class!(NSTimer), scheduledTimerWithTimeInterval: 0.1f64, target: cible(), selector: sel!(tic:), userInfo: std::ptr::null_mut::<AnyObject>(), repeats: Bool::YES];
        rafraichir();
        let _: () = msg_send![app, run];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libelle_d_erreur_garde_la_ligne() {
        assert_eq!(libelle_erreur("/x/barre.toml, ligne 6 : valeur inconnue «truc»"), "⚠ config ligne 6");
        assert_eq!(libelle_erreur("impossible de lire /x"), "⚠ config invalide");
    }
}
