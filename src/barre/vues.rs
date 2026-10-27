//! Petits outils AppKit sans état : textes, symboles SF, couleurs, tailles, items et barres.

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool};
use objc2::{class, msg_send};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
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
/// Gardé par nom : un NSImage neuf à chaque rechargement restait en mémoire (la Touch Bar le garde), +60 Ko par symbole.
pub(crate) fn symbole(nom: &str) -> Option<Id> {
    SYMBOLES.with_borrow_mut(|cache| {
        if let Some(i) = cache.get(nom) {
            return Some(Retained::as_ptr(i) as Id);
        }
        let image: Id = unsafe { msg_send![class!(NSImage), imageWithSystemSymbolName: &*texte(nom), accessibilityDescription: std::ptr::null_mut::<AnyObject>()] };
        let image = unsafe { Retained::retain(image) }?;
        let id = Retained::as_ptr(&image) as Id;
        cache.insert(nom.to_string(), image);
        Some(id)
    })
}

/// Hauteur des images personnalisées, celle des symboles SF dans la barre.
const HAUTEUR_IMAGE: f64 = 20.0;

/// Densité de l'écran de la Touch Bar : une image est gardée à 2 pixels par point.
const ECHELLE: f64 = 2.0;

/// Un GIF de plusieurs images, chacune réduite comme une image fixe : `fins[k]` = instant (s) où l'image `k` laisse la place.
pub(crate) struct Animation {
    pub(super) cadres: Vec<Retained<AnyObject>>,
    pub(super) fins: Vec<f64>,
}

/// Chemin et gabarit (ou `app:chemin` et ouverte) → date du fichier, image réduite (la première, pour un GIF) et son animation.
type Cache = HashMap<(String, bool), (Option<SystemTime>, Retained<AnyObject>, Option<Rc<Animation>>)>;

thread_local! {
    /// Images de fichier déjà lues et réduites, par chemin et gabarit, avec la date du fichier : un script qui renvoie
    /// la même image à chaque sortie ne la relit pas (20 → 141 Mo en 60 s avant) ; un fichier modifié est relu.
    static IMAGES: RefCell<Cache> = RefCell::new(HashMap::new());
    /// Symboles SF déjà demandés, par nom (bornés par les noms de la config et de la batterie).
    static SYMBOLES: RefCell<HashMap<String, Retained<AnyObject>>> = RefCell::new(HashMap::new());
}

/// `None` si le fichier n'existe pas ou n'est pas une image. `~` est étendu ; `gabarit` teinte l'image comme un symbole.
/// L'image est réduite à 20 pt de haut (40 px) dès la lecture : un PNG de 10 000 px gardait 400 Mo par bouton.
pub(crate) fn image(chemin: &str, gabarit: bool) -> Option<Id> {
    let chemin = developper(chemin);
    en_cache(&chemin, (chemin.clone(), gabarit), || {
        let fixe = reduire(&chemin, gabarit).map_err(|e| eprintln!("tactile : {e}")).ok()?;
        let anim = cadres(&chemin, gabarit).map(Rc::new);
        Some((anim.as_ref().map_or(fixe, |a| a.cadres[0].clone()), anim))
    })
}

fn developper(chemin: &str) -> String {
    match chemin.strip_prefix("~/") {
        Some(suite) => format!("{}/{suite}", std::env::var("HOME").unwrap_or_default()),
        None => chemin.to_string(),
    }
}

/// L'animation de l'image de fichier `chemin` si c'est un GIF de plusieurs images (déjà passée par `image`).
pub(crate) fn animation_image(chemin: &str, gabarit: bool) -> Option<Rc<Animation>> {
    IMAGES.with_borrow(|c| c.get(&(developper(chemin), gabarit)).and_then(|(_, _, a)| a.clone()))
}

/// L'icône du `.app` à `chemin` (celle du Finder et du Dock), réduite comme une image de fichier et gardée dans le même
/// cache (clé `app:chemin`), relue si l'app change. `ouverte` : un point dessous, comme dans le Dock.
pub(crate) fn icone_app(chemin: &str, ouverte: bool) -> Option<Id> {
    en_cache(chemin, (format!("app:{chemin}"), ouverte), || unsafe {
        let espace: Id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let icone: Id = msg_send![espace, iconForFile: &*texte(chemin)];
        let cote = HAUTEUR_IMAGE - POINT;
        let cadre = NSRect::new(NSPoint::new(POINT / 2.0, POINT), NSSize::new(cote, cote));
        redessiner(&*icone, NSSize::new(HAUTEUR_IMAGE, HAUTEUR_IMAGE), cadre, ouverte, false).map(|i| (i, None))
    })
}

/// Hauteur réservée sous une icône d'app au point « ouverte » (le bas d'une icône d'app est déjà transparent).
const POINT: f64 = 2.5;

/// L'image de `cle` tant que `fichier` n'a pas changé de date, sinon `creer` la refait. `None` si `fichier` n'existe pas.
fn en_cache(fichier: &str, cle: (String, bool), creer: impl FnOnce() -> Option<(Retained<AnyObject>, Option<Rc<Animation>>)>) -> Option<Id> {
    let date = std::fs::metadata(fichier).ok()?.modified().ok();
    IMAGES.with_borrow_mut(|cache| {
        if let Some((_, i, _)) = cache.get(&cle).filter(|(d, _, _)| *d == date) {
            return Some(Retained::as_ptr(i) as Id);
        }
        let (image, anim) = creer()?;
        let id = Retained::as_ptr(&image) as Id;
        cache.insert(cle, (date, image, anim));
        Some(id)
    })
}

/// Lit le fichier et le redessine à `HAUTEUR_IMAGE` × `ECHELLE` ; l'original est relâché aussitôt.
/// L'éditeur s'en sert aussi pour convertir un SVG, un PDF ou un icns en PNG au même rendu.
/// Les dimensions passent d'abord par `controler_image` : rien n'est décodé si l'image est trop grande.
pub(crate) fn reduire(chemin: &str, gabarit: bool) -> Result<Retained<AnyObject>, String> {
    controler_image(chemin)?;
    let illisible = || format!("{chemin} n'est pas une image lisible");
    unsafe {
        let source: Id = msg_send![class!(NSImage), alloc];
        let source: Retained<AnyObject> = Retained::from_raw(msg_send![source, initWithContentsOfFile: &*texte(chemin)]).ok_or_else(illisible)?;
        let taille: NSSize = msg_send![&*source, size];
        if taille.height <= 0.0 || taille.width <= 0.0 {
            return Err(illisible());
        }
        let points = NSSize::new(taille.width * HAUTEUR_IMAGE / taille.height, HAUTEUR_IMAGE);
        // Le rendu est borné aussi pour un SVG ou un PDF (sans taille en pixels) : 40 px de haut, et pas plus de COTE_MAX de large.
        let largeur = (points.width * ECHELLE).ceil();
        if largeur > COTE_MAX as f64 {
            return Err(format!("image trop large : {largeur} × {} px une fois réduite, maximum {COTE_MAX}", HAUTEUR_IMAGE * ECHELLE));
        }
        redessiner(&source, points, NSRect::new(NSPoint::new(0.0, 0.0), points), false, gabarit).ok_or_else(illisible)
    }
}

use std::ffi::c_void;
#[link(name = "ImageIO", kind = "framework")]
unsafe extern "C" {
    fn CGImageSourceCreateWithURL(url: *const c_void, options: *const c_void) -> *const c_void;
    fn CGImageSourceGetType(source: *const c_void) -> *const c_void;
    fn CGImageSourceGetCount(source: *const c_void) -> usize;
    fn CGImageSourceCopyPropertiesAtIndex(source: *const c_void, index: usize, options: *const c_void) -> *const c_void;
    fn CGImageSourceCreateImageAtIndex(source: *const c_void, index: usize, options: *const c_void) -> *const c_void;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(objet: *const c_void);
}

/// Un GIF refusé au-delà de ces bornes : chaque image est gardée en mémoire et redessinée toutes les 40 ms.
pub(crate) const GIF_IMAGES_MAX: usize = 60;
pub(crate) const GIF_OCTETS_MAX: u64 = 2 << 20;

/// Les images d'un GIF de plusieurs images, réduites comme `reduire` (ImageIO les compose déjà), avec leur durée.
/// `None` pour tout autre fichier, un GIF d'une image ou un GIF refusé par `controler_image`.
fn cadres(chemin: &str, gabarit: bool) -> Option<Animation> {
    controler_image(chemin).ok()?;
    unsafe {
        let url: Id = msg_send![class!(NSURL), fileURLWithPath: &*texte(chemin)];
        let source = CGImageSourceCreateWithURL(url as *const c_void, std::ptr::null());
        if source.is_null() {
            return None;
        }
        let type_ = CGImageSourceGetType(source) as *const NSString;
        let n = CGImageSourceGetCount(source);
        let mut anim = Animation { cadres: Vec::new(), fins: Vec::new() };
        if type_.as_ref().is_some_and(|t| t.to_string() == GIF) && n > 1 {
            let mut total = 0.0;
            for i in 0..n {
                let Some((cadre, duree)) = cadre(source, i, gabarit) else { break };
                total += duree;
                anim.cadres.push(cadre);
                anim.fins.push(total);
            }
        }
        CFRelease(source);
        (anim.cadres.len() == n && n > 1).then_some(anim)
    }
}

/// Pour que `msg_send!` reconnaisse le type `^{CGImage=}`.
#[repr(C)]
struct CGImage([u8; 0]);
unsafe impl objc2::RefEncode for CGImage {
    const ENCODING_REF: objc2::Encoding = objc2::Encoding::Pointer(&objc2::Encoding::Struct("CGImage", &[]));
}

const GIF: &str = "com.compuserve.gif";

unsafe fn cadre(source: *const c_void, i: usize, gabarit: bool) -> Option<(Retained<AnyObject>, f64)> {
    unsafe {
        let proprietes = CGImageSourceCopyPropertiesAtIndex(source, i, std::ptr::null()) as Id;
        let mut duree = 0.1;
        if !proprietes.is_null() {
            let gif: Id = msg_send![proprietes, objectForKey: &*texte("{GIF}")];
            let nombre = |cle: &str| -> f64 {
                let n: Id = if gif.is_null() { gif } else { msg_send![gif, objectForKey: &*texte(cle)] };
                if n.is_null() { 0.0 } else { msg_send![n, doubleValue] }
            };
            let d = nombre("UnclampedDelayTime");
            // Comme les navigateurs : un délai nul ou de 10 ms ou moins vaut 100 ms ; jamais sous 20 ms.
            duree = if d > 0.01 { d.max(0.02) } else if nombre("DelayTime") > 0.01 { nombre("DelayTime") } else { 0.1 };
            CFRelease(proprietes as *const c_void);
        }
        let cg = CGImageSourceCreateImageAtIndex(source, i, std::ptr::null());
        if cg.is_null() {
            return None;
        }
        let ns: Id = msg_send![class!(NSImage), alloc];
        let ns: Option<Retained<AnyObject>> = Retained::from_raw(msg_send![ns, initWithCGImage: cg as *const CGImage, size: NSSize::ZERO]);
        CFRelease(cg);
        let ns = ns?;
        let taille: NSSize = msg_send![&*ns, size];
        if taille.height <= 0.0 || taille.width <= 0.0 {
            return None;
        }
        let points = NSSize::new(taille.width * HAUTEUR_IMAGE / taille.height, HAUTEUR_IMAGE);
        if (points.width * ECHELLE).ceil() > COTE_MAX as f64 {
            return None;
        }
        redessiner(&ns, points, NSRect::new(NSPoint::new(0.0, 0.0), points), false, gabarit).map(|c| (c, duree))
    }
}

/// Côté maximal, en pixels, d'une image qu'on accepte de décoder (4096 × 4096 × 4 octets = 64 Mo au pire).
pub(crate) const COTE_MAX: u64 = 4096;

/// Les dimensions de chaque image du fichier (pages d'un TIFF, images d'un GIF), lues dans son en-tête par ImageIO sans
/// décoder un pixel, refusées au-delà de `COTE_MAX` : un PNG de quelques octets peut annoncer 30000 × 30000 et faire
/// allouer 3,6 Go au décodage. À appeler avant tout décodage d'un fichier venu de l'utilisateur ou d'un import ; `reduire`
/// le fait. Un format vectoriel (SVG, PDF) n'a pas de taille en pixels : `reduire` borne son rendu.
pub(crate) fn controler_image(chemin: &str) -> Result<(), String> {
    let trop = |l: u64, h: u64| if l > COTE_MAX || h > COTE_MAX { Err(format!("image trop grande : {l} × {h} px, maximum {COTE_MAX}")) } else { Ok(()) };
    // ImageIO ne dit rien d'un PNG trop grand pour lui ou sans données : son en-tête IHDR (octets 16 à 24) donne la taille annoncée.
    let mut entete = [0u8; 24];
    let png = std::fs::File::open(chemin).and_then(|mut f| std::io::Read::read_exact(&mut f, &mut entete)).is_ok()
        && entete.starts_with(b"\x89PNG\r\n\x1a\n")
        && &entete[12..16] == b"IHDR";
    if png {
        let mot = |i: usize| u32::from_be_bytes([entete[i], entete[i + 1], entete[i + 2], entete[i + 3]]) as u64;
        trop(mot(16), mot(20))?;
    }
    unsafe {
        let url: Id = msg_send![class!(NSURL), fileURLWithPath: &*texte(chemin)];
        let source = CGImageSourceCreateWithURL(url as *const c_void, std::ptr::null());
        if source.is_null() {
            return Ok(()); // fichier absent : NSImage ne lira rien non plus
        }
        let type_ = CGImageSourceGetType(source) as *const NSString;
        // Pas de type : pas une image pour ImageIO (SVG, texte), NSImage dira s'il la lit. PDF : vectoriel.
        let matriciel = type_.as_ref().is_some_and(|t| t.to_string() != "com.adobe.pdf");
        let mut resultat = Ok(());
        if type_.as_ref().is_some_and(|t| t.to_string() == GIF) {
            let images = CGImageSourceGetCount(source);
            if images > GIF_IMAGES_MAX {
                CFRelease(source);
                return Err(format!("GIF trop long : {images} images, maximum {GIF_IMAGES_MAX}"));
            }
            if std::fs::metadata(chemin).is_ok_and(|m| m.len() > GIF_OCTETS_MAX) {
                CFRelease(source);
                return Err("GIF trop lourd : 2 Mo au plus".into());
            }
        }
        for i in 0..if matriciel { CGImageSourceGetCount(source) } else { 0 } {
            let proprietes = CGImageSourceCopyPropertiesAtIndex(source, i, std::ptr::null()) as Id;
            let cote = |cle: &str| -> Option<u64> {
                let n: Id = if proprietes.is_null() { proprietes } else { msg_send![proprietes, objectForKey: &*texte(cle)] };
                if n.is_null() { None } else { Some(msg_send![n, unsignedLongLongValue]) }
            };
            let (l, h) = (cote("PixelWidth"), cote("PixelHeight"));
            if !proprietes.is_null() {
                CFRelease(proprietes as *const c_void);
            }
            resultat = match (l, h) {
                (Some(l), Some(h)) => trop(l, h),
                // Une image dont ImageIO ne sait pas lire la taille ne sera pas décodée non plus (sauf le PNG déjà vu).
                _ if png => Ok(()),
                _ => Err("image refusée : ses dimensions sont illisibles".to_string()),
            };
            if resultat.is_err() {
                break;
            }
        }
        CFRelease(source);
        resultat
    }
}

/// Un PNG de 45 octets qui annonce `l` × `h` pixels (en-tête IHDR écrit à la main, sans données d'image) :
/// ce qu'un fichier hostile enverrait pour faire allouer des gigaoctets au décodage.
#[cfg(test)]
pub(crate) fn png_annonce(l: u32, h: u32) -> Vec<u8> {
    fn crc(octets: &[u8]) -> u32 {
        !octets.iter().fold(!0u32, |c, &o| (0..8).fold(c ^ o as u32, |c, _| if c & 1 == 1 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 }))
    }
    let morceau = |nom: &[u8], donnees: &[u8]| {
        let corps = [nom, donnees].concat();
        [&(donnees.len() as u32).to_be_bytes()[..], &corps, &crc(&corps).to_be_bytes()].concat()
    };
    let ihdr = [&l.to_be_bytes()[..], &h.to_be_bytes(), &[8, 6, 0, 0, 0]].concat();
    [&b"\x89PNG\r\n\x1a\n"[..], &morceau(b"IHDR", &ihdr), &morceau(b"IEND", &[])].concat()
}

/// Pic de mémoire du processus (octets), pour prouver qu'une image refusée n'a rien fait allouer.
#[cfg(test)]
pub(crate) fn memoire_max() -> i64 {
    #[repr(C)]
    struct Usage {
        temps: [i64; 4],
        max_rss: i64,
        reste: [i64; 13],
    }
    unsafe extern "C" {
        fn getrusage(qui: i32, usage: *mut Usage) -> i32;
    }
    let mut u = Usage { temps: [0; 4], max_rss: 0, reste: [0; 13] };
    unsafe { getrusage(0, &mut u) };
    let _ = (u.temps, u.reste);
    u.max_rss
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

    /// Un rechargement reconstruit tous les boutons : un nouvel NSImage par symbole et par rechargement restait en mémoire
    /// (+1 NSImage et ~60 Ko par symbole, 16 → 26 Mo en 30 min d'endurance). Le même symbole donne le même objet.
    #[test]
    fn symbole_reutilise_d_un_rechargement_a_l_autre() {
        assert_eq!(symbole("xmark"), symbole("xmark"));
        assert_ne!(symbole("xmark"), symbole("sun.max.fill"));
    }

    #[test]
    #[cfg_attr(ci, ignore = "AppKit rendering into a bitmap context: no graphical session on a CI runner")]
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

    /// Un PNG qui annonce 30000 × 30000 : refusé sur sa seule en-tête, sans allocation géante (3,6 Go s'il était décodé).
    #[test]
    fn image_geante_refusee_avant_decodage() {
        let d = std::env::temp_dir().join(format!("tactile-geante-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let (geante, normale, svg) = (d.join("geante.png"), d.join("normale.png"), d.join("fine.svg"));
        std::fs::write(&geante, png_annonce(30000, 30000)).unwrap();
        std::fs::write(&normale, png_annonce(4096, 64)).unwrap();
        std::fs::write(&svg, r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1000"/>"#).unwrap();
        let avant = memoire_max();
        let message = "image trop grande : 30000 × 30000 px, maximum 4096";
        assert_eq!(controler_image(&geante.to_string_lossy()), Err(message.into()));
        assert_eq!(reduire(&geante.to_string_lossy(), false).err().as_deref(), Some(message));
        assert!(image(&geante.to_string_lossy(), false).is_none());
        assert!(memoire_max() - avant < 64 << 20, "pic mémoire : +{} Mo", (memoire_max() - avant) >> 20);
        assert_eq!(controler_image(&normale.to_string_lossy()), Ok(()), "4096 de côté : accepté");
        assert_eq!(controler_image(&png_dans(&d, "large.png", 4097, 1)), Err("image trop grande : 4097 × 1 px, maximum 4096".into()));
        assert_eq!(controler_image("/etc/hosts"), Ok(()), "pas une image : NSImage le refusera");
        assert_eq!(controler_image(&svg.to_string_lossy()), Ok(()), "vectoriel : pas de taille en pixels");
        std::fs::remove_dir_all(&d).unwrap();
    }

    fn png_dans(d: &std::path::Path, nom: &str, l: u32, h: u32) -> String {
        std::fs::write(d.join(nom), png_annonce(l, h)).unwrap();
        d.join(nom).to_string_lossy().into_owned()
    }

    /// Un SVG très large (1000:1) : le rendu à 40 px de haut ferait 40000 px de large, refusé.
    #[test]
    #[cfg_attr(ci, ignore = "AppKit rendering into a bitmap context: no graphical session on a CI runner")]
    fn rendu_vectoriel_borne() {
        let d = std::env::temp_dir().join(format!("tactile-vectoriel-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let svg = d.join("bande.svg");
        std::fs::write(&svg, r#"<svg xmlns="http://www.w3.org/2000/svg" width="20000" height="20" viewBox="0 0 20000 20"><rect width="20000" height="20"/></svg>"#).unwrap();
        assert_eq!(reduire(&svg.to_string_lossy(), false).err().as_deref(), Some("image trop large : 40000 × 40 px une fois réduite, maximum 4096"));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    #[cfg_attr(ci, ignore = "AppKit rendering into a bitmap context: no graphical session on a CI runner")]
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

    /// Un GIF de `n` images de 1 × 1 px, 100 ms chacune (octets écrits à la main).
    fn gif(n: usize) -> Vec<u8> {
        let mut o = b"GIF89a\x01\x00\x01\x00\x80\x00\x00\x00\x00\x00\xff\xff\xff".to_vec();
        for _ in 0..n {
            o.extend(b"\x21\xf9\x04\x00\x0a\x00\x00\x00\x2c\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02\x44\x01\x00");
        }
        o.push(0x3b);
        o
    }

    #[test]
    #[cfg_attr(ci, ignore = "AppKit rendering into a bitmap context: no graphical session on a CI runner")]
    fn gif_anime_garde_ses_images() {
        let d = std::env::temp_dir().join(format!("tactile-gif-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let (trois, une, longue) = (d.join("trois.gif"), d.join("une.gif"), d.join("longue.gif"));
        std::fs::write(&trois, gif(3)).unwrap();
        std::fs::write(&une, gif(1)).unwrap();
        std::fs::write(&longue, gif(GIF_IMAGES_MAX + 1)).unwrap();
        let (trois, une, longue) = (trois.to_string_lossy(), une.to_string_lossy(), longue.to_string_lossy());
        assert!(image(&trois, false).is_some());
        let a = animation_image(&trois, false).expect("3 images gardées");
        assert_eq!(a.cadres.len(), 3);
        assert!((a.fins[2] - 0.3).abs() < 1e-9, "{:?}", a.fins);
        let taille: NSSize = unsafe { msg_send![&*a.cadres[1], size] };
        assert_eq!(taille.height, HAUTEUR_IMAGE);
        assert!(image(&une, false).is_some() && animation_image(&une, false).is_none(), "une image : fixe");
        assert_eq!(controler_image(&longue), Err("GIF trop long : 61 images, maximum 60".into()));
        assert!(image(&longue, false).is_none());
        // Plus de 2 Mo : refusé (le fichier est complété par des zéros après le marqueur de fin).
        let lourd = d.join("lourd.gif");
        let mut o = gif(2);
        o.resize(GIF_OCTETS_MAX as usize + 1, 0);
        std::fs::write(&lourd, o).unwrap();
        assert_eq!(controler_image(&lourd.to_string_lossy()), Err("GIF trop lourd : 2 Mo au plus".into()));
        std::fs::remove_dir_all(&d).unwrap();
    }
}
