//! Construction de chaque type d'élément de la config en item de Touch Bar.

use objc2::runtime::Bool;
use objc2::{class, msg_send, sel};
use std::time::{Duration, Instant};

use super::rafraichi::Rafraichi;
use super::vues::{barre_touch, couleur, item_vue, largeur_fixe, symbole, texte};
use super::{cible, Commande, Etat, Id};
use crate::actions::{self, Action};
use crate::config::{Element, Reglage};

pub(super) fn enregistrer(etat: &mut Etat, court: Option<Commande>, long: Option<Commande>) -> usize {
    etat.commandes.push((court, long));
    etat.commandes.len() - 1
}

pub(super) fn bouton(titre: Option<&str>, image: Option<&str>, index: usize, avec_appui_long: bool) -> Id {
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

pub(super) fn bouton_texte(etat: &mut Etat, action: Option<Action>) -> Id {
    let index = enregistrer(etat, action.map(Commande::Systeme), None);
    let b = bouton(Some(""), None, index, false);
    unsafe { let _: () = msg_send![b, setBordered: Bool::NO]; }
    b
}

/// Bouton replié (symbole + chevron) qui ouvre une sous-barre faite de ✕ suivi de `items`.
pub(super) fn depliable(identifiant: &str, nom: Option<&str>, libelle: Option<&str>, items: Vec<Id>, etat: &mut Etat) -> Id {
    let fermer = enregistrer(etat, Some(Commande::Fermer), None);
    let croix = bouton(None, Some("xmark"), fermer, false);
    etat.controles.push(("fermer".into(), croix));
    largeur_fixe(croix, 64.0);
    let mut contenu = vec![item_vue(&format!("{identifiant}.fermer"), croix)];
    contenu.extend(items);
    etat.barres.push(barre_touch(&format!("{identifiant}.barre"), contenu));
    let ouvrir = enregistrer(etat, Some(Commande::Ouvrir(etat.barres.len() - 1)), None);
    let titre = format!("{}  ›", libelle.unwrap_or(""));
    let b = bouton(Some(titre.trim_start()), nom, ouvrir, false);
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
        let acc_min: Id = msg_send![class!(NSSliderAccessory), accessoryWithImage: symbole(petit)];
        let acc_max: Id = msg_send![class!(NSSliderAccessory), accessoryWithImage: symbole(grand)];
        let _: () = msg_send![item, setMinimumValueAccessory: acc_min];
        let _: () = msg_send![item, setMaximumValueAccessory: acc_max];
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
        let ident = format!("{prefixe}.{i}");
        let item = match element {
            Element::Bouton { texte: t, symbole: s, couleur: c, largeur, action, appui_long } => {
                let index = enregistrer(etat, action.clone().map(Commande::Systeme), appui_long.clone().map(Commande::Systeme));
                let b = bouton(t.as_deref(), s.as_deref(), index, appui_long.is_some());
                etat.controles.push((t.clone().or(s.clone()).unwrap_or_default(), b));
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
                etat.controles.push(("en_cours".into(), b));
                if let Some(l) = largeur {
                    largeur_fixe(b, *l);
                }
                etat.rafraichis.push(Rafraichi::EnCours { bouton: b });
                item_vue(&ident, b)
            }
            Element::Script { commande, intervalle, largeur, action } => {
                let b = bouton_texte(etat, action.clone());
                largeur_fixe(b, largeur_script(*largeur));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_script_a_toujours_une_largeur_fixe() {
        assert_eq!(largeur_script(None), LARGEUR_SCRIPT);
        assert_eq!(largeur_script(Some(80.0)), 80.0);
    }
}
