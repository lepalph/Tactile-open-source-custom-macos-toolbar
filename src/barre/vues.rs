//! Petits outils AppKit sans état : textes, symboles SF, couleurs, tailles, items et barres.

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool};
use objc2::{class, msg_send};
use objc2_foundation::NSString;

use super::Id;

pub(super) fn texte(s: &str) -> Retained<NSString> {
    NSString::from_str(s)
}

/// `None` si le symbole SF n'existe pas : un `buttonWithImage: nil` fait paniquer AppKit.
pub(super) fn symbole(nom: &str) -> Option<Id> {
    let image: Id = unsafe { msg_send![class!(NSImage), imageWithSystemSymbolName: &*texte(nom), accessibilityDescription: std::ptr::null_mut::<AnyObject>()] };
    (!image.is_null()).then_some(image)
}

pub(super) fn couleur(hexa: &str) -> Option<Id> {
    let h = hexa.trim_start_matches('#');
    let v = u32::from_str_radix(h, 16).ok().filter(|_| h.len() == 6)?;
    let c = |d: u32| ((v >> d) & 0xff) as f64 / 255.0;
    Some(unsafe { msg_send![class!(NSColor), colorWithSRGBRed: c(16), green: c(8), blue: c(0), alpha: 1.0f64] })
}

pub(super) fn largeur_fixe(vue: Id, largeur: f64) {
    unsafe {
        let ancre: Id = msg_send![vue, widthAnchor];
        let contrainte: Id = msg_send![ancre, constraintEqualToConstant: largeur];
        let _: () = msg_send![contrainte, setActive: Bool::YES];
    }
}

pub(super) fn item_vue(identifiant: &str, vue: Id) -> Id {
    unsafe {
        let item: Id = msg_send![class!(NSCustomTouchBarItem), alloc];
        let item: Id = msg_send![item, initWithIdentifier: &*texte(identifiant)];
        let _: () = msg_send![item, setView: vue];
        item
    }
}

pub(super) fn barre_touch(identifiant: &str, items: Vec<Id>) -> Id {
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

pub(super) fn titre(bouton: Id, valeur: &str) {
    unsafe {
        let actuel: Id = msg_send![bouton, title];
        if (*(actuel as *const NSString)).to_string() != valeur {
            let _: () = msg_send![bouton, setTitle: &*texte(valeur)];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbole_inconnu_ne_donne_pas_d_image() {
        assert!(symbole("xmark").is_some());
        assert!(symbole("symbole.qui.nexiste.pas").is_none());
    }
}
