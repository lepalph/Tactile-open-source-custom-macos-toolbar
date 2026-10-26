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
pub(crate) fn symbole(nom: &str) -> Option<Id> {
    let image: Id = unsafe { msg_send![class!(NSImage), imageWithSystemSymbolName: &*texte(nom), accessibilityDescription: std::ptr::null_mut::<AnyObject>()] };
    (!image.is_null()).then_some(image)
}

/// Hauteur des images personnalisées, celle des symboles SF dans la barre.
const HAUTEUR_IMAGE: f64 = 20.0;

/// `None` si le fichier n'existe pas ou n'est pas une image. `~` est étendu ; `gabarit` teinte l'image comme un symbole.
pub(crate) fn image(chemin: &str, gabarit: bool) -> Option<Id> {
    let chemin = match chemin.strip_prefix("~/") {
        Some(suite) => format!("{}/{suite}", std::env::var("HOME").unwrap_or_default()),
        None => chemin.to_string(),
    };
    unsafe {
        let image: Id = msg_send![class!(NSImage), alloc];
        let image: Id = msg_send![image, initWithContentsOfFile: &*texte(&chemin)];
        if image.is_null() {
            return None;
        }
        let taille: objc2_foundation::NSSize = msg_send![image, size];
        if taille.height > 0.0 {
            let _: () = msg_send![image, setSize: objc2_foundation::NSSize::new(taille.width * HAUTEUR_IMAGE / taille.height, HAUTEUR_IMAGE)];
        }
        let _: () = msg_send![image, setTemplate: Bool::new(gabarit)];
        Some(image)
    }
}

/// Couleur du texte et taille de police d'un bouton ; sans couleur, le titre redevient ordinaire.
/// À rappeler après chaque `titre` : `setTitle:` efface les attributs.
pub(super) fn styler(bouton: Id, couleur_texte: Option<&str>, taille: Option<f64>) {
    unsafe {
        if let Some(t) = taille {
            let police: Id = msg_send![class!(NSFont), systemFontOfSize: t];
            let _: () = msg_send![bouton, setFont: police];
        }
        let Some(col) = couleur_texte.and_then(couleur) else { return };
        let actuel: Id = msg_send![bouton, title];
        let police: Id = msg_send![bouton, font];
        let attributs: Id = msg_send![class!(NSMutableDictionary), dictionary];
        let _: () = msg_send![attributs, setObject: col, forKey: &*texte("NSColor")];
        let _: () = msg_send![attributs, setObject: police, forKey: &*texte("NSFont")];
        let paragraphe: Id = msg_send![class!(NSMutableParagraphStyle), new];
        let _: () = msg_send![paragraphe, setAlignment: 2isize];
        let _: () = msg_send![attributs, setObject: paragraphe, forKey: &*texte("NSParagraphStyle")];
        let titre: Id = msg_send![class!(NSAttributedString), alloc];
        let titre: Id = msg_send![titre, initWithString: actuel, attributes: attributs];
        let _: () = msg_send![bouton, setAttributedTitle: titre];
    }
}

/// `#RRGGBB` seulement, comme l'éditeur et `--verifier` (`config::rgb`).
pub(super) fn couleur(hexa: &str) -> Option<Id> {
    let v = crate::config::rgb(hexa)?;
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

/// Plafonne la largeur : le titre est alors coupé avec « … » au lieu de pousser les voisins hors de la barre.
pub(super) fn largeur_max(vue: Id, largeur: f64) {
    unsafe {
        let ancre: Id = msg_send![vue, widthAnchor];
        let contrainte: Id = msg_send![ancre, constraintLessThanOrEqualToConstant: largeur];
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

    #[test]
    fn image_redimensionnee_ou_absente() {
        let i = image("/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/GenericApplicationIcon.icns", true).unwrap();
        let taille: objc2_foundation::NSSize = unsafe { msg_send![i, size] };
        assert_eq!((taille.width, taille.height), (HAUTEUR_IMAGE, HAUTEUR_IMAGE));
        assert!(unsafe { msg_send![i, isTemplate] });
        assert!(image("~/image-qui-nexiste-pas.png", false).is_none());
        assert!(image("/etc/hosts", false).is_none());
    }
}
