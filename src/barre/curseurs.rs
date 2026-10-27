//! Curseurs (luminosité, volume, ou libres : `regler` / `lire`) et sélecteurs à choix.
//!
//! Le style `classique` garde le curseur d'Apple (`NSSliderTouchBarItem`, message `glisse:`). Les autres styles sont
//! une vue dessinée par Tactile (classe `TactileCurseur`, sous-classe de `NSView` créée à l'exécution) qui suit le doigt
//! par `touchesBegan/Moved/Ended` : un mouvement ne redessine que cette vue, jamais la barre.

use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, Sel};
use objc2::{class, msg_send, sel};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::elements::depliable;
use super::rafraichi::{executer_script, Processus, Rafraichi, Sortie};
use super::vues::{item_vue, lacher, largeur_fixe, symbole, texte};
use super::{cible, mesure, Etat, Id, COURANTE, ETAT};
use crate::actions::{self, Action};
use crate::config::{rgb, Element, Reglage, StyleCurseur};

/// Secondes entre deux lectures de `lire` sans `toutes_les`.
pub(crate) const LECTURE_DEFAUT: f64 = 5.0;
/// `toutes_les` plus court est ramené à ce minimum.
pub(crate) const LECTURE_MIN: f64 = 0.5;
/// `regler` part au plus une fois par intervalle pendant le glissé.
const ECRITURE_MIN: Duration = Duration::from_millis(100);
/// Après un toucher, une lecture qui arrive est ignorée pendant ce temps : la valeur ne saute pas sous le doigt.
const SANS_LECTURE: Duration = Duration::from_secs(1);
/// Couleur des styles dessinés sans `couleurs` : le blanc des symboles de la barre.
const BLANC: u32 = 0xF5F5F7;
/// Place de la valeur écrite à droite des styles en ligne.
const ETIQUETTE: f64 = 58.0;
/// Pour le cadran : points de glissé pour parcourir tout l'intervalle.
pub(super) const COURSE_CADRAN: f64 = 160.0;

/// Le dernier `regler` voulu et le fil qui l'exécute (un seul à la fois, dans l'ordre : la valeur du lâcher passe en dernier).
#[derive(Default)]
struct Ecriture {
    voulue: Option<String>,
    en_cours: bool,
}

pub(super) struct Curseur {
    /// La vue dessinée, ou l'item `NSSliderTouchBarItem` du style classique.
    pub vue: Id,
    native: bool,
    /// Index de sa sous-barre dans `Etat::barres`.
    pub barre: usize,
    reglage: Option<Reglage>,
    pub lire: Option<String>,
    regler: Option<String>,
    min: f64,
    max: f64,
    pas: f64,
    valeur: f64,
    style: StyleCurseur,
    crans: usize,
    couleurs: Vec<u32>,
    seuils: Vec<f64>,
    haptique: bool,
    pub toutes_les: Duration,
    pub prochain: Instant,
    pub en_cours: bool,
    pub derniere_erreur: String,
    touche: Option<Instant>,
    ecriture: Arc<Mutex<Ecriture>>,
    processus: Processus,
    /// Cadran : position du doigt et valeur au début du glissé.
    depart: (f64, f64),
    /// Pas à pas : −1 ou +1 pendant l'appui sur − ou +, 0 sinon.
    appui: i8,
}

pub(super) struct Choix {
    pub controle: Id,
    options: Vec<String>,
    action: Option<Action>,
    pub lire: Option<String>,
    pub toutes_les: Duration,
    pub prochain: Instant,
    pub en_cours: bool,
    pub derniere_erreur: String,
}

// ---------- `{valeur}` ----------

/// Le nombre tel qu'il part dans `regler` : autant de décimales que `pas`, jamais de texte (pas d'injection possible).
pub(crate) fn formater(valeur: f64, pas: f64) -> String {
    let decimales = format!("{pas}").split_once('.').map_or(0, |(_, d)| d.len().min(6));
    let s = format!("{:.*}", decimales, valeur);
    if s.starts_with('-') && s[1..].chars().all(|c| c == '0' || c == '.') { s[1..].to_string() } else { s }
}

/// `{valeur}` (ou `{value}`) remplacé par `valeur`.
pub(crate) fn avec_valeur(commande: &str, valeur: &str) -> String {
    commande.replace("{valeur}", valeur).replace("{value}", valeur)
}

pub(crate) fn contient_valeur(commande: &str) -> bool {
    commande.contains("{valeur}") || commande.contains("{value}")
}

/// Un texte entre apostrophes pour zsh : rien n'y est interprété (`$`, `;`, `` ` ``…), une apostrophe s'écrit `'\''`.
pub(crate) fn pour_shell(texte: &str) -> String {
    format!("'{}'", texte.replace('\'', r"'\''"))
}

/// Un texte AppleScript entre guillemets : `\` et `"` échappés.
pub(crate) fn pour_applescript(texte: &str) -> String {
    format!("\"{}\"", texte.replace('\\', r"\\").replace('"', "\\\""))
}

/// L'action d'un choix pour l'option touchée : `{valeur}` devient l'option protégée pour le shell ou pour AppleScript.
/// Les autres actions ne contiennent pas de texte exécuté : elles partent telles quelles.
pub(crate) fn action_choisie(action: &Action, option: &str) -> Action {
    match action {
        Action::Shell(c) => Action::Shell(avec_valeur(c, &pour_shell(option))),
        Action::Applescript(c) => Action::Applescript(avec_valeur(c, &pour_applescript(option))),
        autre => autre.clone(),
    }
}

/// Bornes et pas d'un curseur : 0 à 100 (en %) pour luminosité et volume, sinon `min` (0), `max` (100), `pas` (1).
/// Des bornes inversées ou un pas impossible retombent sur ces valeurs (`controler` le signale).
pub(crate) fn bornes(reglage: Option<Reglage>, min: Option<f64>, max: Option<f64>, pas: Option<f64>) -> (f64, f64, f64) {
    if reglage.is_some() {
        return (0.0, 100.0, 1.0);
    }
    let (min, max) = match (min.unwrap_or(0.0), max.unwrap_or(100.0)) {
        (a, b) if a < b => (a, b),
        _ => (0.0, 100.0),
    };
    let pas = pas.filter(|p| *p > 0.0 && *p <= max - min).unwrap_or(if max - min >= 10.0 { 1.0 } else { (max - min) / 100.0 });
    (min, max, pas)
}

/// La valeur ramenée dans les bornes, sur un multiple du pas.
fn arrondir(v: f64, min: f64, max: f64, pas: f64) -> f64 {
    let v = if v.is_finite() { v.clamp(min, max) } else { min };
    (min + ((v - min) / pas).round() * pas).clamp(min, max)
}

/// La couleur du niveau `v` : `couleurs[k]` où `k` est le nombre de seuils atteints ; une seule couleur sans seuils.
pub(crate) fn couleur_du_niveau(couleurs: &[u32], seuils: &[f64], v: f64) -> u32 {
    match couleurs {
        [] => BLANC,
        [seule] => *seule,
        _ if couleurs.len() == seuils.len() + 1 => couleurs[seuils.iter().filter(|s| v >= **s).count()],
        _ => couleurs[0],
    }
}

// ---------- Construction ----------

fn duree(toutes_les: Option<f64>) -> Duration {
    Duration::from_secs_f64(toutes_les.unwrap_or(LECTURE_DEFAUT).clamp(LECTURE_MIN, 24.0 * 3600.0))
}

fn lire_reglage(reglage: Reglage) -> Option<f64> {
    let lu = match reglage {
        Reglage::Luminosite => actions::luminosite::lire(),
        Reglage::Volume => actions::volume::lire(),
    };
    lu.ok().map(|v| (v as f64 * 100.0).clamp(0.0, 100.0))
}

/// Largeur de la vue dessinée : toute la sous-barre après le ✕ (64 pt et les espacements de 8 pt), comme le curseur
/// d'Apple ; le pas à pas reste compact.
fn largeur(style: StyleCurseur, masquer_control_strip: bool) -> f64 {
    match style {
        StyleCurseur::PasAPas => 260.0,
        _ => super::largeur_barre(masquer_control_strip) - 64.0 - 3.0 * 8.0,
    }
}

/// Le bouton qui ouvre la sous-barre du curseur (✕ + le curseur).
pub(super) fn curseur(identifiant: &str, element: &Element, etat: &mut Etat) -> Id {
    let Element::Slider { reglage, symbole: s, lire, regler, min, max, pas, toutes_les, style, crans, couleurs, seuils, haptique } = element else {
        unreachable!("curseur() reçoit un slider")
    };
    let (min, max, pas) = bornes(*reglage, *min, *max, *pas);
    let valeur = arrondir(reglage.and_then(lire_reglage).unwrap_or(min), min, max, pas);
    let style = style.unwrap_or_default();
    let native = style == StyleCurseur::Classique;
    let couleurs: Vec<u32> = couleurs.iter().filter_map(|c| rgb(c)).collect();
    let large = largeur(style, etat.config.masquer_control_strip);
    let item = if native {
        native_item(identifiant, *reglage, min, max, valeur)
    } else {
        let vue: Id = unsafe {
            let v: Id = msg_send![classe(), alloc];
            msg_send![v, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(large, 30.0))]
        };
        unsafe { let _: () = msg_send![vue, setAllowedTouchTypes: 1usize]; }
        largeur_fixe(vue, large);
        let item = item_vue(&format!("{identifiant}.slider"), vue);
        lacher(vue);
        item
    };
    let vue = if native { item } else { unsafe { msg_send![item, view] } };
    let nom = match reglage {
        Some(Reglage::Luminosite) => "slider luminosite".to_string(),
        Some(Reglage::Volume) => "slider volume".to_string(),
        None => format!("curseur {}", s.as_deref().unwrap_or("slider.horizontal.3")),
    };
    etat.controles.push((nom, vue));
    let defaut = match reglage {
        Some(Reglage::Luminosite) => "sun.max.fill",
        Some(Reglage::Volume) => "speaker.wave.2.fill",
        None => "slider.horizontal.3",
    };
    let bouton = depliable(identifiant, Some(s.as_deref().unwrap_or(defaut)), None, None, vec![item], etat);
    let processus = etat.processus.clone();
    etat.rafraichis.push(Rafraichi::Curseur(Box::new(Curseur {
        vue,
        native,
        barre: etat.barres.len() - 1,
        reglage: *reglage,
        lire: lire.clone().filter(|_| reglage.is_none()),
        regler: regler.clone().filter(|_| reglage.is_none()),
        min,
        max,
        pas,
        valeur,
        style,
        crans: crans.map_or(10, |c| c.clamp(2, 40)) as usize,
        couleurs,
        seuils: seuils.clone(),
        haptique: *haptique,
        toutes_les: duree(*toutes_les),
        prochain: Instant::now(),
        en_cours: false,
        derniere_erreur: String::new(),
        touche: None,
        ecriture: Arc::default(),
        processus,
        depart: (0.0, valeur),
        appui: 0,
    })));
    bouton
}

/// Le curseur d'Apple, avec les symboles de luminosité ou de volume aux deux bouts.
fn native_item(identifiant: &str, reglage: Option<Reglage>, min: f64, max: f64, valeur: f64) -> Id {
    let bouts = match reglage {
        Some(Reglage::Luminosite) => Some(("sun.min.fill", "sun.max.fill")),
        Some(Reglage::Volume) => Some(("speaker.fill", "speaker.wave.3.fill")),
        None => None,
    };
    unsafe {
        let item: Id = msg_send![class!(NSSliderTouchBarItem), alloc];
        let item: Id = msg_send![item, initWithIdentifier: &*texte(&format!("{identifiant}.slider"))];
        let s: Id = msg_send![item, slider];
        let _: () = msg_send![s, setMinValue: min];
        let _: () = msg_send![s, setMaxValue: max];
        let _: () = msg_send![s, setDoubleValue: valeur];
        let _: () = msg_send![s, setContinuous: Bool::YES];
        if let Some((Some(petit), Some(grand))) = bouts.map(|(p, g)| (symbole(p), symbole(g))) {
            let acc_min: Id = msg_send![class!(NSSliderAccessory), accessoryWithImage: petit];
            let acc_max: Id = msg_send![class!(NSSliderAccessory), accessoryWithImage: grand];
            let _: () = msg_send![item, setMinimumValueAccessory: acc_min];
            let _: () = msg_send![item, setMaximumValueAccessory: acc_max];
        }
        let _: () = msg_send![item, setMinimumSliderWidth: 600.0f64];
        let _: () = msg_send![item, setTarget: cible()];
        let _: () = msg_send![item, setAction: sel!(glisse:)];
        item
    }
}

/// Le contrôle segmenté d'un `choix`.
pub(super) fn choix(identifiant: &str, element: &Element, etat: &mut Etat) -> Id {
    let Element::Choix { options, action, lire, toutes_les, largeur } = element else { unreachable!("choix() reçoit un choix") };
    let controle: Id = unsafe {
        let libelles: Id = msg_send![class!(NSMutableArray), array];
        for o in options {
            let _: () = msg_send![libelles, addObject: &*texte(o)];
        }
        msg_send![class!(NSSegmentedControl), segmentedControlWithLabels: libelles, trackingMode: 0usize, target: cible(), action: sel!(choix:)]
    };
    if let Some(l) = largeur {
        largeur_fixe(controle, *l);
    }
    etat.controles.push(("choix".into(), controle));
    etat.rafraichis.push(Rafraichi::Choix(Box::new(Choix {
        controle,
        options: options.clone(),
        action: action.clone(),
        lire: lire.clone(),
        toutes_les: duree(*toutes_les),
        prochain: Instant::now(),
        en_cours: false,
        derniere_erreur: String::new(),
    })));
    item_vue(identifiant, controle)
}

// ---------- Réglage ----------

fn trouver<R>(vue: Id, f: impl FnOnce(&mut Curseur) -> R) -> Option<R> {
    ETAT.with_borrow_mut(|e| {
        e.as_mut()?.rafraichis.iter_mut().find_map(|r| match r {
            Rafraichi::Curseur(c) if c.vue == vue => Some(c),
            _ => None,
        }).map(|c| f(c))
    })
}

impl Curseur {
    fn cran(&self, v: f64) -> i64 {
        ((v - self.min) / (self.max - self.min) * self.crans as f64).round() as i64
    }

    /// Fixe la valeur voulue par le doigt et l'applique : réglage système tout de suite, commande au plus toutes les 0,1 s.
    fn fixer(&mut self, v: f64, fini: bool) {
        let v = arrondir(v, self.min, self.max, self.pas);
        let change = v != self.valeur;
        if self.haptique && self.cran(v) != self.cran(self.valeur) {
            unsafe {
                let p: Id = msg_send![class!(NSHapticFeedbackManager), defaultPerformer];
                // 2 = NSHapticFeedbackPatternLevelChange, 1 = NSHapticFeedbackPerformanceTimeNow.
                let _: () = msg_send![p, performFeedbackPattern: 2isize, performanceTime: 1usize];
            }
        }
        self.valeur = v;
        self.touche = Some(Instant::now());
        if !change && !fini {
            return;
        }
        match (self.reglage, &self.regler) {
            (Some(Reglage::Luminosite), _) if change => super::signaler(actions::executer(&Action::Luminosite((v / 100.0) as f32))),
            (Some(Reglage::Volume), _) if change => super::signaler(actions::executer(&Action::Volume((v / 100.0) as f32))),
            (None, Some(regler)) => ecrire(&self.ecriture, avec_valeur(regler, &formater(v, self.pas)), &self.processus),
            _ => {}
        }
        if change {
            self.montrer();
        }
    }

    fn montrer(&self) {
        unsafe {
            if self.native {
                let s: Id = msg_send![self.vue, slider];
                let _: () = msg_send![s, setDoubleValue: self.valeur];
                if !self.couleurs.is_empty() {
                    let _: () = msg_send![s, setTrackFillColor: couleur(couleur_du_niveau(&self.couleurs, &self.seuils, self.valeur), 1.0)];
                }
            } else {
                let _: () = msg_send![self.vue, setNeedsDisplay: Bool::YES];
            }
        }
    }

    fn texte_valeur(&self) -> String {
        let n = formater(self.valeur, self.pas);
        if self.reglage.is_some() { format!("{n} %") } else { n }
    }
}

/// Lance `commande` sur le fil d'écriture du curseur ; s'il tourne, elle remplace la précédente en attente.
fn ecrire(ecriture: &Arc<Mutex<Ecriture>>, commande: String, processus: &Processus) {
    let mut e = ecriture.lock().unwrap();
    e.voulue = Some(commande);
    if e.en_cours {
        return;
    }
    e.en_cours = true;
    drop(e);
    let (ecriture, processus) = (ecriture.clone(), processus.clone());
    std::thread::spawn(move || loop {
        let commande = {
            let mut e = ecriture.lock().unwrap();
            let c = e.voulue.take();
            e.en_cours = c.is_some();
            c
        };
        let Some(commande) = commande else { break };
        let debut = Instant::now();
        let sortie = executer_script(&commande, Duration::from_secs(5), &processus);
        if !sortie.erreur.is_empty() {
            eprintln!("tactile : curseur « {commande} » : {}", sortie.erreur);
        }
        if let Some(reste) = ECRITURE_MIN.checked_sub(debut.elapsed()) {
            std::thread::sleep(reste);
        }
    });
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Phase {
    Debut,
    Glisse,
    Fin,
}

/// Un doigt à l'abscisse `x` (points, dans la vue) : le chemin d'un vrai toucher, et celui de l'autotest.
pub(super) fn toucher(vue: Id, x: f64, phase: Phase) {
    let debut = Instant::now();
    let largeur = unsafe { let b: NSRect = msg_send![vue, bounds]; b.size.width };
    let repeter = trouver(vue, |c| {
        let etendue = c.max - c.min;
        match (c.style, phase) {
            (StyleCurseur::PasAPas, Phase::Debut) => {
                c.appui = if x < largeur / 3.0 { -1 } else if x > largeur * 2.0 / 3.0 { 1 } else { 0 };
                if c.appui != 0 {
                    c.fixer(c.valeur + c.appui as f64 * c.pas, true);
                    c.montrer();
                }
                c.appui != 0
            }
            (StyleCurseur::PasAPas, Phase::Fin) => {
                c.appui = 0;
                c.montrer();
                false
            }
            (StyleCurseur::PasAPas, Phase::Glisse) => false,
            (StyleCurseur::Cadran, Phase::Debut) => {
                c.depart = (x, c.valeur);
                false
            }
            (StyleCurseur::Cadran, _) => {
                c.fixer(c.depart.1 + (x - c.depart.0) / COURSE_CADRAN * etendue, phase == Phase::Fin);
                false
            }
            _ => {
                let (x0, x1) = (10.0, largeur - ETIQUETTE);
                c.fixer(c.min + ((x - x0) / (x1 - x0)).clamp(0.0, 1.0) * etendue, phase == Phase::Fin);
                false
            }
        }
    });
    unsafe {
        let _: () = msg_send![class!(NSObject), cancelPreviousPerformRequestsWithTarget: vue];
        if repeter == Some(true) {
            let _: () = msg_send![vue, performSelector: sel!(repeter:), withObject: std::ptr::null_mut::<AnyObject>(), afterDelay: 0.4f64];
        }
    }
    if mesure() {
        eprintln!("mesure : curseur mouvement {:.3} ms", debut.elapsed().as_secs_f64() * 1000.0);
    }
}

/// Pour l'autotest : la valeur d'un curseur, et l'abscisse où un doigt la placerait à `fraction` de la piste.
pub(super) fn valeur(vue: Id) -> Option<f64> {
    trouver(vue, |c| c.valeur)
}

pub(super) fn abscisse_pour(vue: Id, fraction: f64) -> f64 {
    let largeur = unsafe { let b: NSRect = msg_send![vue, bounds]; b.size.width };
    10.0 + fraction * (largeur - ETIQUETTE - 10.0)
}

/// Pas à pas, doigt posé sur − ou + : un pas de plus toutes les 0,08 s.
fn repeter(vue: Id) {
    if trouver(vue, |c| {
        if c.appui != 0 {
            c.fixer(c.valeur + c.appui as f64 * c.pas, true);
        }
        c.appui != 0
    }) == Some(true)
    {
        unsafe { let _: () = msg_send![vue, performSelector: sel!(repeter:), withObject: std::ptr::null_mut::<AnyObject>(), afterDelay: 0.08f64]; }
    }
}

/// `glisse:` du curseur d'Apple (style classique).
pub(super) fn glisse(item: Id) {
    let valeur: f64 = unsafe { let s: Id = msg_send![item, slider]; msg_send![s, doubleValue] };
    trouver(item, |c| c.fixer(valeur, false));
}

/// `choix:` : l'option touchée part dans l'action.
pub(super) fn choisi(controle: Id) {
    let k: isize = unsafe { msg_send![controle, selectedSegment] };
    let action = ETAT.with_borrow(|e| {
        e.as_ref()?.rafraichis.iter().find_map(|r| match r {
            Rafraichi::Choix(c) if c.controle == controle => Some((c.action.clone()?, c.options.get(usize::try_from(k).ok()?)?.clone())),
            _ => None,
        })
    });
    if let Some((action, option)) = action {
        super::signaler(actions::executer(&action_choisie(&action, &option)));
    }
}

/// La sous-barre `k` vient d'être montrée : luminosité et volume relus tout de suite, `lire` au `tic` suivant.
pub(super) fn ouvert(k: usize) {
    ETAT.with_borrow_mut(|e| {
        let Some(e) = e.as_mut() else { return };
        for r in &mut e.rafraichis {
            if let Rafraichi::Curseur(c) = r
                && c.barre == k
            {
                c.prochain = Instant::now();
                if let Some(v) = c.reglage.and_then(lire_reglage) {
                    c.valeur = arrondir(v, c.min, c.max, c.pas);
                    c.montrer();
                }
            }
        }
    });
}

/// Faut-il lancer `lire` maintenant ? Un curseur n'est relu que si sa sous-barre est montrée.
pub(super) fn a_lire(r: &Rafraichi, barres: &[Id], maintenant: Instant) -> Option<String> {
    match r {
        Rafraichi::Curseur(c) if !c.en_cours && maintenant >= c.prochain && barres.get(c.barre) == Some(&COURANTE.get()) => c.lire.clone(),
        Rafraichi::Choix(c) if !c.en_cours && maintenant >= c.prochain => c.lire.clone(),
        _ => None,
    }
}

/// Note que `lire` part (au retour, `valeur_lue`).
pub(super) fn lecture_lancee(r: &mut Rafraichi, maintenant: Instant) {
    match r {
        Rafraichi::Curseur(c) => (c.en_cours, c.prochain) = (true, maintenant + c.toutes_les),
        Rafraichi::Choix(c) => (c.en_cours, c.prochain) = (true, maintenant + c.toutes_les),
        _ => {}
    }
}

/// La sortie de `lire` : un nombre pour un curseur, une des options pour un choix (aucune n'est sélectionnée sinon).
pub(super) fn valeur_lue(r: &mut Rafraichi, sortie: Sortie) {
    let texte = sortie.texte.trim();
    let (erreur, derniere) = match r {
        Rafraichi::Curseur(c) => {
            c.en_cours = false;
            let lu = texte.replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite());
            if let Some(v) = lu
                && c.touche.is_none_or(|t| t.elapsed() > SANS_LECTURE)
            {
                c.valeur = arrondir(v, c.min, c.max, c.pas);
                c.montrer();
            }
            let erreur = match (lu, sortie.erreur.is_empty()) {
                (_, false) => sortie.erreur,
                (None, true) => format!("« {} » n'est pas un nombre", texte.chars().take(40).collect::<String>()),
                _ => String::new(),
            };
            (erreur, &mut c.derniere_erreur)
        }
        Rafraichi::Choix(c) => {
            c.en_cours = false;
            let k = c.options.iter().position(|o| o == texte).map_or(-1, |k| k as isize);
            unsafe { let _: () = msg_send![c.controle, setSelectedSegment: k]; }
            (sortie.erreur, &mut c.derniere_erreur)
        }
        _ => return,
    };
    if erreur != *derniere {
        if !erreur.is_empty() {
            eprintln!("tactile : lecture d'un curseur ou d'un choix : {erreur}");
        }
        *derniere = erreur;
    }
}

// ---------- La vue dessinée ----------

/// La classe `TactileCurseur` (sous-classe de `NSView`), créée une fois.
fn classe() -> &'static AnyClass {
    static CLASSE: OnceLock<&'static AnyClass> = OnceLock::new();
    CLASSE.get_or_init(|| {
        let mut c = ClassBuilder::new(c"TactileCurseur", class!(NSView)).expect("classe TactileCurseur déjà définie");
        unsafe {
            c.add_method(sel!(drawRect:), dessiner as extern "C-unwind" fn(*mut AnyObject, Sel, NSRect));
            c.add_method(sel!(touchesBeganWithEvent:), commence as extern "C-unwind" fn(*mut AnyObject, Sel, Id));
            c.add_method(sel!(touchesMovedWithEvent:), bouge as extern "C-unwind" fn(*mut AnyObject, Sel, Id));
            c.add_method(sel!(touchesEndedWithEvent:), finit as extern "C-unwind" fn(*mut AnyObject, Sel, Id));
            c.add_method(sel!(touchesCancelledWithEvent:), finit as extern "C-unwind" fn(*mut AnyObject, Sel, Id));
            c.add_method(sel!(repeter:), repete as extern "C-unwind" fn(*mut AnyObject, Sel, Id));
        }
        c.register()
    })
}

/// Abscisse du doigt dans la vue (toucher direct de la Touch Bar).
fn abscisse(vue: Id, evenement: Id) -> Option<f64> {
    unsafe {
        let touches: Id = msg_send![evenement, allTouches];
        let touche: Id = msg_send![touches, anyObject];
        if touche.is_null() {
            return None;
        }
        let p: NSPoint = msg_send![touche, locationInView: vue];
        Some(p.x)
    }
}

extern "C-unwind" fn commence(vue: *mut AnyObject, _: Sel, evenement: Id) {
    if let Some(x) = abscisse(vue, evenement) {
        toucher(vue, x, Phase::Debut);
        toucher(vue, x, Phase::Glisse);
    }
}

extern "C-unwind" fn bouge(vue: *mut AnyObject, _: Sel, evenement: Id) {
    if let Some(x) = abscisse(vue, evenement) {
        toucher(vue, x, Phase::Glisse);
    }
}

extern "C-unwind" fn finit(vue: *mut AnyObject, _: Sel, evenement: Id) {
    let x = abscisse(vue, evenement).unwrap_or(0.0);
    toucher(vue, x, Phase::Fin);
}

extern "C-unwind" fn repete(vue: *mut AnyObject, _: Sel, _: Id) {
    repeter(vue);
}

fn couleur(rgb: u32, alpha: f64) -> Id {
    let c = |d: u32| ((rgb >> d) & 0xff) as f64 / 255.0;
    unsafe { msg_send![class!(NSColor), colorWithSRGBRed: c(16), green: c(8), blue: c(0), alpha: alpha] }
}

fn peindre(rgb: u32, alpha: f64) {
    unsafe { let _: () = msg_send![couleur(rgb, alpha), set]; }
}

fn arrondi(x: f64, y: f64, l: f64, h: f64, r: f64) {
    unsafe {
        let p: Id = msg_send![class!(NSBezierPath), bezierPathWithRoundedRect: NSRect::new(NSPoint::new(x, y), NSSize::new(l, h)), xRadius: r, yRadius: r];
        let _: () = msg_send![p, fill];
    }
}

/// Écrit `s` centré verticalement, en `(taille, graisse)` et `(couleur, opacité)` ; `ancre` : 0 = à gauche de `x`, 0,5 = centré, 1 = à droite.
fn ecrire_texte(s: &str, x: f64, hauteur: f64, (taille, poids): (f64, f64), (rgb, alpha): (u32, f64), ancre: f64) {
    unsafe {
        let police: Id = msg_send![class!(NSFont), monospacedDigitSystemFontOfSize: taille, weight: poids];
        let attributs: Id = msg_send![class!(NSMutableDictionary), dictionary];
        let _: () = msg_send![attributs, setObject: police, forKey: &*texte("NSFont")];
        let _: () = msg_send![attributs, setObject: couleur(rgb, alpha), forKey: &*texte("NSColor")];
        let s = texte(s);
        let t: NSSize = msg_send![&*s, sizeWithAttributes: attributs];
        let _: () = msg_send![&*s, drawAtPoint: NSPoint::new(x - t.width * ancre, (hauteur - t.height) / 2.0), withAttributes: attributs];
    }
}

/// Ce qu'il faut pour dessiner, copié hors de l'état (le dessin ne garde pas `ETAT` emprunté).
struct Dessin {
    style: StyleCurseur,
    fraction: f64,
    crans: usize,
    min: f64,
    etendue: f64,
    couleurs: Vec<u32>,
    seuils: Vec<f64>,
    texte: String,
    appui: i8,
    valeur: f64,
}

extern "C-unwind" fn dessiner(vue: *mut AnyObject, _: Sel, _rect: NSRect) {
    let debut = Instant::now();
    let dessin = ETAT.with(|e| {
        let e = e.try_borrow().ok()?;
        e.as_ref()?.rafraichis.iter().find_map(|r| match r {
            Rafraichi::Curseur(c) if c.vue == vue => Some(Dessin {
                style: c.style,
                fraction: (c.valeur - c.min) / (c.max - c.min),
                crans: c.crans,
                min: c.min,
                etendue: c.max - c.min,
                couleurs: c.couleurs.clone(),
                seuils: c.seuils.clone(),
                texte: c.texte_valeur(),
                appui: c.appui,
                valeur: c.valeur,
            }),
            _ => None,
        })
    });
    let Some(d) = dessin else { return };
    let b: NSRect = unsafe { msg_send![vue, bounds] };
    peindre_curseur(&d, b.size.width, b.size.height);
    if mesure() {
        eprintln!("mesure : curseur dessin {:.3} ms", debut.elapsed().as_secs_f64() * 1000.0);
    }
}

const ETEINT: f64 = 0.16;

fn peindre_curseur(d: &Dessin, l: f64, h: f64) {
    let niveau = |v: f64| couleur_du_niveau(&d.couleurs, &d.seuils, v);
    let actuelle = niveau(d.valeur);
    let (x0, x1) = (10.0, l - ETIQUETTE);
    let n = d.crans;
    let case = (x1 - x0) / n as f64;
    let allumes = (d.fraction * n as f64).round() as usize;
    // La couleur du cran k : celle du niveau qu'il représente (le dernier cran allumé porte la couleur de la valeur).
    let couleur_cran = |k: usize| niveau(d.min + (k + 1) as f64 / n as f64 * d.etendue);
    match d.style {
        StyleCurseur::Grille | StyleCurseur::Points | StyleCurseur::Egaliseur => {
            for k in 0..n {
                let allume = k < allumes;
                if allume { peindre(couleur_cran(k), 1.0) } else { peindre(BLANC, ETEINT) }
                let centre = x0 + case * (k as f64 + 0.5);
                match d.style {
                    StyleCurseur::Grille => {
                        let lb = (case * 0.42).clamp(3.0, 7.0);
                        arrondi(centre - lb / 2.0, (h - 20.0) / 2.0, lb, 20.0, lb / 2.0);
                    }
                    StyleCurseur::Points => {
                        let diam = (case * 0.55).clamp(5.0, 12.0);
                        arrondi(centre - diam / 2.0, (h - diam) / 2.0, diam, diam, diam / 2.0);
                    }
                    _ => {
                        let lb = (case * 0.6).clamp(4.0, 18.0);
                        let hb = 5.0 + (k + 1) as f64 / n as f64 * (h - 11.0);
                        arrondi(centre - lb / 2.0, 3.0, lb, hb, 2.0);
                    }
                }
            }
            ecrire_texte(&d.texte, l - 8.0, h, (14.0, 0.3), (BLANC, 0.85), 1.0);
        }
        StyleCurseur::Jauge => {
            peindre(BLANC, ETEINT);
            arrondi(x0, (h - 10.0) / 2.0, x1 - x0, 10.0, 5.0);
            peindre(actuelle, 1.0);
            let plein = (x1 - x0) * d.fraction;
            if plein > 0.5 {
                arrondi(x0, (h - 10.0) / 2.0, plein.max(10.0), 10.0, 5.0);
            }
            peindre(BLANC, 0.45);
            for s in d.seuils.iter().filter(|s| **s > d.min && **s < d.min + d.etendue) {
                let x = x0 + (s - d.min) / d.etendue * (x1 - x0);
                arrondi(x - 0.75, (h - 18.0) / 2.0, 1.5, 18.0, 0.75);
            }
            ecrire_texte(&d.texte, l - 8.0, h, (14.0, 0.3), (BLANC, 0.85), 1.0);
        }
        StyleCurseur::Cadran => unsafe {
            let centre = NSPoint::new(22.0, h / 2.0);
            let rayon = (h / 2.0 - 4.0).min(11.0);
            let anneau = |rgb: u32, alpha: f64, fin: f64| {
                let p: Id = msg_send![class!(NSBezierPath), bezierPath];
                let _: () = msg_send![p, appendBezierPathWithArcWithCenter: centre, radius: rayon, startAngle: 90.0f64, endAngle: fin, clockwise: Bool::YES];
                let _: () = msg_send![p, setLineWidth: 4.0f64];
                let _: () = msg_send![p, setLineCapStyle: 1usize];
                peindre(rgb, alpha);
                let _: () = msg_send![p, stroke];
            };
            anneau(BLANC, ETEINT, 90.0 - 359.99);
            if d.fraction > 0.0 {
                anneau(actuelle, 1.0, 90.0 - 360.0 * d.fraction.min(0.9999));
            }
            ecrire_texte(&d.texte, 44.0, h, (16.0, 0.4), (actuelle, 1.0), 0.0);
            // La piste de glisse : ↔ puis un rail fin où un point suit la valeur (gris clair, contraste > 3:1 sur le noir).
            ecrire_texte("↔", 104.0, h, (15.0, 0.3), (BLANC, 0.6), 0.0);
            let (r0, r1) = (132.0, l - 12.0);
            peindre(BLANC, 0.3);
            arrondi(r0, h / 2.0 - 1.0, r1 - r0, 2.0, 1.0);
            peindre(actuelle, 1.0);
            arrondi(r0 + (r1 - r0) * d.fraction - 4.0, h / 2.0 - 4.0, 8.0, 8.0, 4.0);
        },
        StyleCurseur::PasAPas => {
            for (signe, x, appuye) in [("−", 4.0, d.appui < 0), ("+", l - 68.0, d.appui > 0)] {
                peindre(if appuye { 0x5A5A5A } else { 0x363636 }, 1.0);
                arrondi(x, 0.0, 64.0, h, 6.0);
                ecrire_texte(signe, x + 32.0, h, (20.0, 0.3), (BLANC, 1.0), 0.5);
            }
            ecrire_texte(&d.texte, l / 2.0, h, (20.0, 0.4), (actuelle, 1.0), 0.5);
        }
        StyleCurseur::Classique => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valeur_formatee_selon_le_pas() {
        assert_eq!(formater(62.0, 1.0), "62");
        assert_eq!(formater(0.5, 0.1), "0.5");
        assert_eq!(formater(1.25, 0.05), "1.25");
        assert_eq!(formater(-0.0001, 1.0), "0");
        assert_eq!(formater(-3.0, 1.0), "-3");
        assert!(formater(1e300, 1.0).chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn bornes_et_arrondi() {
        assert_eq!(bornes(Some(Reglage::Volume), Some(5.0), Some(1.0), None), (0.0, 100.0, 1.0));
        assert_eq!(bornes(None, None, None, None), (0.0, 100.0, 1.0));
        assert_eq!(bornes(None, Some(50.0), Some(10.0), None), (0.0, 100.0, 1.0), "bornes inversées");
        assert_eq!(bornes(None, Some(0.0), Some(1.0), None), (0.0, 1.0, 0.01));
        assert_eq!(bornes(None, Some(0.0), Some(10.0), Some(-1.0)), (0.0, 10.0, 1.0));
        assert_eq!(arrondir(47.6, 0.0, 100.0, 5.0), 50.0);
        assert_eq!(arrondir(150.0, 0.0, 100.0, 1.0), 100.0);
        assert_eq!(arrondir(f64::NAN, 10.0, 100.0, 1.0), 10.0);
    }

    #[test]
    fn couleur_par_niveau() {
        let c = [0x30D158, 0xFFD60A, 0xFF453A];
        assert_eq!(couleur_du_niveau(&c, &[60.0, 85.0], 10.0), 0x30D158);
        assert_eq!(couleur_du_niveau(&c, &[60.0, 85.0], 60.0), 0xFFD60A);
        assert_eq!(couleur_du_niveau(&c, &[60.0, 85.0], 99.0), 0xFF453A);
        assert_eq!(couleur_du_niveau(&[], &[], 99.0), BLANC);
        assert_eq!(couleur_du_niveau(&[0x0A84FF], &[], 3.0), 0x0A84FF);
        assert_eq!(couleur_du_niveau(&c, &[60.0], 99.0), 0x30D158, "nombre faux : la première");
    }

    /// Une option hostile reste un seul argument : zsh ne l'interprète pas.
    #[test]
    fn option_echappee_pour_le_shell() {
        let hostile = "x'; touch /tmp/tactile-pirate; echo '$(id)`id`";
        let Action::Shell(c) = action_choisie(&Action::Shell("printf %s {valeur}".into()), hostile) else { panic!() };
        let sortie = std::process::Command::new("/bin/zsh").args(["-c", &c]).output().unwrap();
        assert_eq!(String::from_utf8_lossy(&sortie.stdout), hostile);
        assert!(!std::path::Path::new("/tmp/tactile-pirate").exists());
        let Action::Applescript(a) = action_choisie(&Action::Applescript("return {value}".into()), "a\"b\\c") else { panic!() };
        assert_eq!(a, r#"return "a\"b\\c""#);
        assert_eq!(action_choisie(&Action::Echap, "x"), Action::Echap);
        assert_eq!(avec_valeur("m1ddc set luminance {valeur}", "40"), "m1ddc set luminance 40");
        assert!(contient_valeur("x {value}") && !contient_valeur("x {val}"));
    }

    /// Pendant un glissé, `regler` ne part pas plus d'une fois par 0,1 s, et la dernière valeur voulue part en dernier.
    #[test]
    fn ecriture_limitee_et_derniere_valeur_appliquee() {
        let fichier = std::env::temp_dir().join(format!("tactile-curseur-{}", std::process::id()));
        let journal = fichier.with_extension("log");
        let processus: Processus = Arc::new(Mutex::new(Some(Vec::new())));
        let ecriture = Arc::new(Mutex::new(Ecriture::default()));
        let glisse = Instant::now();
        for v in 0..30 {
            ecrire(&ecriture, format!("echo {v} > {0}; echo {v} >> {1}", fichier.display(), journal.display()), &processus);
            std::thread::sleep(Duration::from_millis(10));
        }
        // Une machine lente allonge le glissé : la limite suit la durée réelle (une écriture par 0,1 s, plus la dernière).
        let permises = (glisse.elapsed().as_secs_f64() / 0.1).ceil() as usize + 2;
        let debut = Instant::now();
        while ecriture.lock().unwrap().en_cours && debut.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(std::fs::read_to_string(&fichier).unwrap().trim(), "29");
        let lancees = std::fs::read_to_string(&journal).unwrap().lines().count();
        assert!(lancees <= permises, "{lancees} lancements, {permises} permis pour ce glissé");
        let _ = (std::fs::remove_file(&fichier), std::fs::remove_file(&journal));
    }
}
