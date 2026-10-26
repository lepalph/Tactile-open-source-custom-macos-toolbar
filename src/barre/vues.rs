//! Petits outils AppKit sans état : textes, symboles SF, couleurs, tailles, items et barres.

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool};
use objc2::{class, msg_send};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::SystemTime;

use super::Id;

/// Relâche un objet reçu à +1 (`alloc`/`init`, `new`, `mutableCopy`) une fois confié à son propriétaire.
/// Sans ça, chaque rechargement gardait l'ancienne barre entière (14 → 32 Mo en 20 rechargements).
pub(super) fn lacher(objet: Id) {
    if !objet.is_null() {
        drop(unsafe { Retained::from_raw(objet) });
    }
}

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

/// Densité de l'écran de la Touch Bar : une image est gardée à 2 pixels par point.
const ECHELLE: f64 = 2.0;

/// Chemin et gabarit (ou `app:chemin` et ouverte) → date du fichier et image réduite.
type Cache = HashMap<(String, bool), (Option<SystemTime>, Retained<AnyObject>)>;

thread_local! {
    /// Images de fichier déjà lues et réduites, par chemin et gabarit, avec la date du fichier : un script qui renvoie
    /// la même image à chaque sortie ne la relit pas (20 → 141 Mo en 60 s avant) ; un fichier modifié est relu.
    static IMAGES: RefCell<Cache> = RefCell::new(HashMap::new());
}

/// `None` si le fichier n'existe pas ou n'est pas une image. `~` est étendu ; `gabarit` teinte l'image comme un symbole.
/// L'image est réduite à 20 pt de haut (40 px) dès la lecture : un PNG de 10 000 px gardait 400 Mo par bouton.
pub(crate) fn image(chemin: &str, gabarit: bool) -> Option<Id> {
    let chemin = match chemin.strip_prefix("~/") {
        Some(suite) => format!("{}/{suite}", std::env::var("HOME").unwrap_or_default()),
        None => chemin.to_string(),
    };
    en_cache(&chemin, (chemin.clone(), gabarit), || reduire(&chemin, gabarit))
}

/// L'icône du `.app` à `chemin` (celle du Finder et du Dock), réduite comme une image de fichier et gardée dans le même
/// cache (clé `app:chemin`), relue si l'app change. `ouverte` : un point dessous, comme dans le Dock.
pub(crate) fn icone_app(chemin: &str, ouverte: bool) -> Option<Id> {
    en_cache(chemin, (format!("app:{chemin}"), ouverte), || unsafe {
        let espace: Id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let icone: Id = msg_send![espace, iconForFile: &*texte(chemin)];
        let cote = HAUTEUR_IMAGE - POINT;
        let cadre = NSRect::new(NSPoint::new(POINT / 2.0, POINT), NSSize::new(cote, cote));
        redessiner(&*icone, NSSize::new(HAUTEUR_IMAGE, HAUTEUR_IMAGE), cadre, ouverte, false)
    })
}

/// Hauteur réservée sous une icône d'app au point « ouverte » (le bas d'une icône d'app est déjà transparent).
const POINT: f64 = 2.5;

/// L'image de `cle` tant que `fichier` n'a pas changé de date, sinon `creer` la refait. `None` si `fichier` n'existe pas.
fn en_cache(fichier: &str, cle: (String, bool), creer: impl FnOnce() -> Option<Retained<AnyObject>>) -> Option<Id> {
    let date = std::fs::metadata(fichier).ok()?.modified().ok();
    IMAGES.with_borrow_mut(|cache| {
        if let Some((_, i)) = cache.get(&cle).filter(|(d, _)| *d == date) {
            return Some(Retained::as_ptr(i) as Id);
        }
        let image = creer()?;
        let id = Retained::as_ptr(&image) as Id;
        cache.insert(cle, (date, image));
        Some(id)
    })
}

/// Lit le fichier et le redessine à `HAUTEUR_IMAGE` × `ECHELLE` ; l'original est relâché aussitôt.
fn reduire(chemin: &str, gabarit: bool) -> Option<Retained<AnyObject>> {
    unsafe {
        let source: Id = msg_send![class!(NSImage), alloc];
        let source: Retained<AnyObject> = Retained::from_raw(msg_send![source, initWithContentsOfFile: &*texte(chemin)])?;
        let taille: NSSize = msg_send![&*source, size];
        if taille.height <= 0.0 || taille.width <= 0.0 {
            return None;
        }
        let points = NSSize::new(taille.width * HAUTEUR_IMAGE / taille.height, HAUTEUR_IMAGE);
        redessiner(&source, points, NSRect::new(NSPoint::new(0.0, 0.0), points), false, gabarit)
    }
}

/// Une image de `points` (à `ECHELLE` pixels par point) où `source` occupe `cadre`, avec le point « ouverte » en bas si `point`.
fn redessiner(source: &AnyObject, points: NSSize, cadre: NSRect, point: bool, gabarit: bool) -> Option<Retained<AnyObject>> {
    unsafe {
        let rep: Id = msg_send![class!(NSBitmapImageRep), alloc];
        let rep: Retained<AnyObject> = Retained::from_raw(msg_send![rep,
            initWithBitmapDataPlanes: std::ptr::null_mut::<*mut u8>(),
            pixelsWide: (points.width * ECHELLE).ceil().max(1.0) as isize,
            pixelsHigh: (points.height * ECHELLE) as isize,
            bitsPerSample: 8isize,
            samplesPerPixel: 4isize,
            hasAlpha: Bool::YES,
            isPlanar: Bool::NO,
            colorSpaceName: &*texte("NSCalibratedRGBColorSpace"),
            bytesPerRow: 0isize,
            bitsPerPixel: 0isize])?;
        let _: () = msg_send![&*rep, setSize: points];
        let contexte: Id = msg_send![class!(NSGraphicsContext), graphicsContextWithBitmapImageRep: &*rep];
        let _: () = msg_send![class!(NSGraphicsContext), saveGraphicsState];
        let _: () = msg_send![class!(NSGraphicsContext), setCurrentContext: contexte];
        // 2 = NSCompositingOperationSourceOver.
        let _: () = msg_send![source, drawInRect: cadre, fromRect: NSRect::ZERO, operation: 2usize, fraction: 1.0f64];
        if point {
            let gris: Id = msg_send![class!(NSColor), colorWithWhite: 0.85f64, alpha: 1.0f64];
            let _: () = msg_send![gris, set];
            let rond = NSRect::new(NSPoint::new((points.width - POINT) / 2.0, 0.0), NSSize::new(POINT, POINT));
            let chemin: Id = msg_send![class!(NSBezierPath), bezierPathWithOvalInRect: rond];
            let _: () = msg_send![chemin, fill];
        }
        let _: () = msg_send![class!(NSGraphicsContext), restoreGraphicsState];
        let image: Id = msg_send![class!(NSImage), alloc];
        let image: Retained<AnyObject> = Retained::from_raw(msg_send![image, initWithSize: points])?;
        let _: () = msg_send![&*image, addRepresentation: &*rep];
        let _: () = msg_send![&*image, setTemplate: Bool::new(gabarit)];
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
        lacher(titre);
        lacher(paragraphe);
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

/// Item à +1, à confier à `barre_touch` (ou à relâcher avec `lacher`).
pub(super) fn item_vue(identifiant: &str, vue: Id) -> Id {
    unsafe {
        let item: Id = msg_send![class!(NSCustomTouchBarItem), alloc];
        let item: Id = msg_send![item, initWithIdentifier: &*texte(identifiant)];
        let _: () = msg_send![item, setView: vue];
        item
    }
}

/// Une `NSTouchBar` (à +1, relâchée avec `Etat`) qui prend possession des `items` (reçus à +1 : `item_vue`, slider).
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
            lacher(item);
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
    #[cfg_attr(ci, ignore = "rendu AppKit dans un contexte bitmap : pas de session graphique sur un runner de CI")]
    fn image_redimensionnee_ou_absente() {
        let i = image("/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/GenericApplicationIcon.icns", true).unwrap();
        let taille: NSSize = unsafe { msg_send![i, size] };
        assert_eq!((taille.width, taille.height), (HAUTEUR_IMAGE, HAUTEUR_IMAGE));
        assert!(unsafe { msg_send![i, isTemplate] });
        // Gardée réduite (40 px de haut, une seule représentation), et lue une seule fois par chemin et gabarit.
        let reps: Id = unsafe { msg_send![i, representations] };
        let n: usize = unsafe { msg_send![reps, count] };
        let rep: Id = unsafe { msg_send![reps, firstObject] };
        let pixels: isize = unsafe { msg_send![rep, pixelsHigh] };
        assert_eq!((n, pixels), (1, 40));
        assert_eq!(image("/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/GenericApplicationIcon.icns", true), Some(i));
        assert_ne!(image("/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/GenericApplicationIcon.icns", false), Some(i));
        assert!(image("~/image-qui-nexiste-pas.png", false).is_none());
        assert!(image("/etc/hosts", false).is_none());
    }

    #[test]
    #[cfg_attr(ci, ignore = "rendu AppKit dans un contexte bitmap : pas de session graphique sur un runner de CI")]
    fn icone_d_app_reduite_avec_ou_sans_point() {
        let chemin = "/System/Applications/TextEdit.app";
        let (fermee, ouverte) = (icone_app(chemin, false).unwrap(), icone_app(chemin, true).unwrap());
        assert_ne!(fermee, ouverte);
        assert_eq!(icone_app(chemin, false), Some(fermee), "gardée en cache");
        let taille: NSSize = unsafe { msg_send![ouverte, size] };
        assert_eq!((taille.width, taille.height), (HAUTEUR_IMAGE, HAUTEUR_IMAGE));
        // Le point : un pixel clair au milieu du bas de l'image ouverte, transparent sur la fermée.
        let bas = |i: Id| unsafe {
            let reps: Id = msg_send![i, representations];
            let rep: Id = msg_send![reps, firstObject];
            // y = 0 en haut : les 5 dernières lignes (2,5 pt) sont celles du point.
            let couleur: Id = msg_send![rep, colorAtX: 20isize, y: 37isize];
            let alpha: f64 = msg_send![couleur, alphaComponent];
            alpha
        };
        assert!(bas(ouverte) > 0.9 && bas(fermee) < 0.1, "{} {}", bas(ouverte), bas(fermee));
        assert!(icone_app("/Applications/Nexiste.app", false).is_none());
    }
}
