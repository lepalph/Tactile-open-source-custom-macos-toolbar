//! Construction de chaque type d'élément de la config en item de Touch Bar.

use objc2::runtime::Bool;
use objc2::{class, msg_send, sel};
use std::time::{Duration, Instant};

use super::rafraichi::Rafraichi;
use super::vues::{barre_touch, couleur, item_vue, largeur_fixe, largeur_max, symbole, texte};
use super::{cible, Commande, Etat, Id, PREFIXE};
use crate::actions::{self, Action};
use crate::config::{Element, Reglage};

pub(super) fn enregistrer(etat: &mut Etat, court: Option<Commande>, long: Option<Commande>) -> usize {
    etat.commandes.push((court, long));
    etat.commandes.len() - 1
}

/// `element` est l'identifiant de l'item (`fr.palph.tactile.3.1` = premier enfant du troisième élément), pour le journal.
pub(super) fn bouton(titre: Option<&str>, image: Option<&str>, index: usize, avec_appui_long: bool, element: &str) -> Id {
    // Un symbole inconnu n'est pas une raison de perdre la barre : son nom s'affiche en texte.
    let (titre, image) = match image.map(|nom| (nom, symbole(nom))) {
        Some((_, Some(i))) => (titre, Some(i)),
        Some((nom, None)) => {
            eprintln!("tactile : symbole SF inconnu « {nom} » (élément {})", element.strip_prefix(PREFIXE).map_or(element, |e| e.trim_start_matches('.')));
            (Some(nom), None)
        }
        None => (titre, None),
    };
    unsafe {
        let b: Id = match image {
            Some(i) => msg_send![class!(NSButton), buttonWithImage: i, target: cible(), action: sel!(appui:)],
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

pub(super) fn bouton_texte(etat: &mut Etat, action: Option<Action>) -> Id {
    let index = enregistrer(etat, action.map(Commande::Systeme), None);
    let b = bouton(Some(""), None, index, false, "");
    unsafe { let _: () = msg_send![b, setBordered: Bool::NO]; }
    b
}

/// Bouton replié (symbole + chevron) qui ouvre une sous-barre faite de ✕ suivi de `items`.
pub(super) fn depliable(identifiant: &str, nom: Option<&str>, libelle: Option<&str>, items: Vec<Id>, etat: &mut Etat) -> Id {
    let fermer = enregistrer(etat, Some(Commande::Fermer), None);
    let croix = bouton(None, Some("xmark"), fermer, false, identifiant);
    etat.controles.push(("fermer".into(), croix));
    largeur_fixe(croix, 64.0);
    let mut contenu = vec![item_vue(&format!("{identifiant}.fermer"), croix)];
    contenu.extend(items);
    etat.barres.push(barre_touch(&format!("{identifiant}.barre"), contenu));
    let ouvrir = enregistrer(etat, Some(Commande::Ouvrir(etat.barres.len() - 1)), None);
    let titre = format!("{}  ›", libelle.unwrap_or(""));
    let b = bouton(Some(titre.trim_start()), nom, ouvrir, false, identifiant);
    etat.controles.push((format!("ouvrir {}", nom.or(libelle).unwrap_or("")), b));
    largeur_fixe(b, 72.0);
    item_vue(identifiant, b)
}

pub(super) fn slider(identifiant: &str, reglage: Reglage, nom: &str, etat: &mut Etat) -> Id {
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
        if let (Some(petit), Some(grand)) = (symbole(petit), symbole(grand)) {
            let acc_min: Id = msg_send![class!(NSSliderAccessory), accessoryWithImage: petit];
            let acc_max: Id = msg_send![class!(NSSliderAccessory), accessoryWithImage: grand];
            let _: () = msg_send![item, setMinimumValueAccessory: acc_min];
            let _: () = msg_send![item, setMaximumValueAccessory: acc_max];
        }
        let _: () = msg_send![item, setMinimumSliderWidth: 600.0f64];
        let _: () = msg_send![item, setTarget: cible()];
        let _: () = msg_send![item, setAction: sel!(glisse:)];
        item
    };
    let nom_reglage = if tag == 0 { "luminosite" } else { "volume" };
    etat.controles.push((format!("slider {nom_reglage}"), item));
    depliable(identifiant, Some(nom), None, vec![item], etat)
}

pub(super) fn construire(elements: &[Element], prefixe: &str, etat: &mut Etat) -> Vec<Id> {
    let mut items = Vec::new();
    for (i, element) in elements.iter().enumerate() {
        let ident = format!("{prefixe}.{}", i + 1);
        let item = match element {
            Element::Bouton { texte: t, symbole: s, couleur: c, largeur, action, appui_long } => {
                let index = enregistrer(etat, action.clone().map(Commande::Systeme), appui_long.clone().map(Commande::Systeme));
                let b = bouton(t.as_deref(), s.as_deref(), index, appui_long.is_some(), &ident);
                etat.controles.push((t.clone().or(s.clone()).unwrap_or_default(), b));
                if let Some(col) = c.as_deref().and_then(couleur) {
                    unsafe { let _: () = msg_send![b, setBezelColor: col]; }
                }
                match largeur {
                    Some(l) => largeur_fixe(b, *l),
                    None if t.is_some() => largeur_max(b, LARGEUR_MAX_TEXTE),
                    None => {}
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
                largeur_max(b, LARGEUR_MAX_TEXTE);
                etat.rafraichis.push(Rafraichi::Heure { bouton: b, formateur });
                item_vue(&ident, b)
            }
            Element::EnCours { largeur } => {
                let b = bouton_texte(etat, Some(Action::LecturePause));
                etat.controles.push(("en_cours".into(), b));
                match largeur {
                    Some(l) => largeur_fixe(b, *l),
                    None => largeur_max(b, LARGEUR_MAX_TEXTE),
                }
                etat.rafraichis.push(Rafraichi::EnCours { bouton: b });
                item_vue(&ident, b)
            }
            Element::Script { commande, intervalle, largeur, action } => {
                let b = bouton_texte(etat, action.clone());
                largeur_fixe(b, largeur_script(*largeur));
                etat.controles.push((format!("script {commande}"), b));
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

/// Largeur d'un widget script sans `largeur` : sa sortie change, et une sortie qui grandit
/// pousserait ses voisins hors de la barre (la Touch Bar retire ce qui dépasse).
pub(super) const LARGEUR_SCRIPT: f64 = 120.0;

pub(super) fn largeur_script(largeur: Option<f64>) -> f64 {
    largeur.unwrap_or(LARGEUR_SCRIPT)
}

/// Largeur au-delà de laquelle un texte sans `largeur` (bouton, heure, titre en cours) est coupé avec « … ».
pub(super) const LARGEUR_MAX_TEXTE: f64 = 200.0;

/// Largeur de la barre en points : toute la Touch Bar, ou ce qu'il reste à côté du Control Strip.
pub(super) fn largeur_barre(masquer_control_strip: bool) -> f64 {
    if masquer_control_strip { 1004.0 } else { 685.0 }
}

/// Largeur approximative d'un élément (texte : ~9 pt par caractère plus les marges ; symbole seul : 72, mesuré).
fn largeur_estimee(element: &Element) -> f64 {
    let texte = |t: &str| (t.chars().count() as f64 * 9.0 + 24.0).min(LARGEUR_MAX_TEXTE);
    match element {
        Element::Bouton { largeur: Some(l), .. } | Element::EnCours { largeur: Some(l) } | Element::Espace { largeur: l } => *l,
        Element::Bouton { texte: t, symbole: s, .. } => t.as_deref().map_or(72.0, texte) + if t.is_some() && s.is_some() { 24.0 } else { 0.0 },
        Element::Heure { format } => texte(format.as_deref().unwrap_or("HH:mm")),
        Element::EnCours { largeur: None } => LARGEUR_MAX_TEXTE,
        Element::Script { largeur, .. } => largeur_script(*largeur),
        Element::Slider { .. } | Element::Groupe { .. } => 72.0,
        Element::Souple => 0.0,
    }
}

/// La Touch Bar retire en silence ce qui dépasse : largeur totale estimée et nombre d'éléments
/// qui seront cachés (du premier qui dépasse `disponible` jusqu'au dernier), ou `None` si tout tient.
pub(super) fn debordement(elements: &[Element], disponible: f64) -> Option<(f64, usize)> {
    const ESPACEMENT: f64 = 8.0;
    let largeurs: Vec<f64> = elements.iter().map(|e| largeur_estimee(e) + ESPACEMENT).collect();
    let mut cumul = 0.0;
    let premier_cache = largeurs.iter().position(|l| {
        cumul += l;
        cumul > disponible
    })?;
    Some((largeurs.iter().sum(), elements.len() - premier_cache))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbole_inconnu_donne_un_bouton_texte() {
        super::super::preparer_cible();
        let b = bouton(None, Some("symbole.qui.nexiste.pas"), 0, false, "fr.palph.tactile.3");
        let titre: Id = unsafe { msg_send![b, title] };
        let image: Id = unsafe { msg_send![b, image] };
        assert_eq!(unsafe { (*(titre as *const objc2_foundation::NSString)).to_string() }, "symbole.qui.nexiste.pas");
        assert!(image.is_null());
    }

    #[test]
    fn debordement_compte_les_elements_caches() {
        let bouton = |texte: Option<&str>, largeur: Option<f64>| Element::Bouton {
            texte: texte.map(String::from),
            symbole: Some("bolt.fill".into()),
            couleur: None,
            largeur,
            action: None,
            appui_long: None,
        };
        let defaut = crate::config::lire(include_str!("../../exemples/barre.toml")).unwrap();
        assert_eq!(debordement(&defaut.elements, largeur_barre(true)), None);
        assert_eq!(debordement(&[bouton(None, Some(100_000.0)), bouton(None, None)], 1004.0).map(|d| d.1), Some(2));
        let vingt_cinq: Vec<Element> = (0..25).map(|_| bouton(None, None)).collect();
        let (total, caches) = debordement(&vingt_cinq, 1004.0).unwrap();
        assert_eq!(total, 25.0 * 80.0);
        assert_eq!(caches, 25 - 12);
        // Un texte long sans largeur est plafonné, il ne cache pas tout seul ses voisins.
        let long = "x".repeat(200);
        assert_eq!(debordement(&[bouton(Some(&long), None), bouton(None, None)], 1004.0), None);
    }

    #[test]
    fn un_script_a_toujours_une_largeur_fixe() {
        assert_eq!(largeur_script(None), LARGEUR_SCRIPT);
        assert_eq!(largeur_script(Some(80.0)), 80.0);
    }
}
