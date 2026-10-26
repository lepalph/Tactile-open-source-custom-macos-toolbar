//! Les logos de l'utilisateur : copiés dans `icones/` à côté du fichier de config, sous un nom stable
//! (nom d'origine nettoyé + 8 caractères du hachage du contenu), et rendus en data URL pour l'aperçu.

use super::{texte, Id};
use objc2::runtime::Bool;
use objc2::{class, msg_send};
use objc2_foundation::NSString;
use std::path::{Path, PathBuf};

/// Ce que le sélecteur propose et ce que la copie accepte.
pub(super) const FORMATS: [&str; 10] = ["png", "jpg", "jpeg", "heic", "gif", "tif", "tiff", "svg", "pdf", "icns"];

/// Lus par `NSImage` mais pas affichés tels quels par la barre : convertis en PNG 2× (40 px de haut) à la copie.
const A_CONVERTIR: [&str; 3] = ["svg", "pdf", "icns"];

/// `icones/` à côté du fichier de config : `~/.config/tactile/icones`, ou le voisin de `TACTILE_CONFIG`.
pub(super) fn dossier(config: &Path) -> PathBuf {
    config.parent().unwrap_or(Path::new(".")).join("icones")
}

/// FNV-1a 64 bits : stable d'une version de Rust à l'autre, contrairement à `DefaultHasher`.
fn hachage(octets: &[u8]) -> String {
    let h = octets.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &o| (h ^ o as u64).wrapping_mul(0x0100_0000_01b3));
    format!("{h:016x}")[..8].to_string()
}

/// « Mon Logo (v2) » → « mon-logo-v2 » : ASCII, minuscules, 40 caractères au plus.
fn nettoyer(nom: &str) -> String {
    let mut propre = String::new();
    for c in nom.chars().flat_map(char::to_lowercase) {
        // Les accents courants perdent leur accent (« Déposé » → « depose ») ; un accent combinant (NFD du Finder) disparaît.
        let c = match c {
            'à' | 'â' | 'ä' | 'á' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            '\u{300}'..='\u{36f}' => continue,
            c => c,
        };
        if c.is_ascii_alphanumeric() || c == '_' {
            propre.push(c);
        } else if !propre.ends_with('-') {
            propre.push('-');
        }
    }
    let propre: String = propre.trim_matches('-').chars().take(40).collect();
    if propre.is_empty() { "image".into() } else { propre.trim_end_matches('-').into() }
}

/// `/Users/moi/x.png` → `~/x.png` ; ailleurs, le chemin tel quel.
pub(super) fn avec_tilde(chemin: &Path) -> String {
    let maison = std::env::var("HOME").unwrap_or_default();
    match chemin.strip_prefix(&maison) {
        Ok(suite) if !maison.is_empty() => format!("~/{}", suite.display()),
        _ => chemin.display().to_string(),
    }
}

/// Copie `source` dans `dossier` et renvoie le chemin de la copie. Même contenu = même nom : rien n'est recopié.
pub(super) fn importer(source: &Path, dossier: &Path) -> Result<PathBuf, String> {
    let extension = source.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if !FORMATS.contains(&extension.as_str()) {
        return Err(format!("format non pris en charge : {}. Choisis un PNG, JPEG, HEIC, GIF, TIFF, SVG, PDF ou ICNS", source.display()));
    }
    let octets = std::fs::read(source).map_err(|e| format!("impossible de lire {} : {e}", source.display()))?;
    // `reduire` est ce que la barre fait de l'image : s'il échoue, la barre ne l'afficherait pas non plus.
    let reduite = crate::barre::reduire(&source.to_string_lossy(), false).ok_or_else(|| format!("{} n'est pas une image lisible", source.display()))?;
    let convertir = A_CONVERTIR.contains(&extension.as_str());
    let stem = source.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let cible = dossier.join(format!("{}-{}.{}", nettoyer(&stem), hachage(&octets), if convertir { "png" } else { &extension }));
    if cible.exists() {
        return Ok(cible);
    }
    std::fs::create_dir_all(dossier).map_err(|e| format!("impossible de créer {} : {e}", dossier.display()))?;
    if convertir {
        let png = unsafe { png(objc2::rc::Retained::as_ptr(&reduite) as Id) }.ok_or("conversion en PNG impossible")?;
        std::fs::write(&cible, png).map_err(|e| format!("impossible d'écrire {} : {e}", cible.display()))?;
    } else {
        std::fs::write(&cible, &octets).map_err(|e| format!("impossible d'écrire {} : {e}", cible.display()))?;
    }
    Ok(cible)
}

/// Fichier déposé dans la page (lu en base64 par le JS) : écrit à part sous son nom, puis importé comme un fichier choisi.
pub(super) fn importer_depot(nom: &str, base64: &str, dossier: &Path) -> Result<PathBuf, String> {
    // Une icône pèse quelques centaines de Ko ; 30 Mo en base64 suffisent largement.
    if base64.len() > 30_000_000 {
        return Err("image trop lourde (plus de 20 Mo)".into());
    }
    let octets = unsafe {
        let donnees: Id = msg_send![class!(NSData), alloc];
        let donnees: Id = msg_send![donnees, initWithBase64EncodedString: &*texte(base64), options: 0usize];
        if donnees.is_null() {
            return Err("fichier déposé illisible".into());
        }
        let donnees = objc2::rc::Retained::from_raw(donnees).ok_or("fichier déposé illisible")?;
        let longueur: usize = msg_send![&*donnees, length];
        let debut: *const std::ffi::c_void = msg_send![&*donnees, bytes];
        if debut.is_null() { Vec::new() } else { std::slice::from_raw_parts(debut.cast::<u8>(), longueur).to_vec() }
    };
    let nom = Path::new(nom).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "image".into());
    let temporaire = std::env::temp_dir().join(format!("tactile-depot-{}-{}", std::process::id(), hachage(&octets)));
    std::fs::create_dir_all(&temporaire).map_err(|e| format!("dossier temporaire : {e}"))?;
    let fichier = temporaire.join(&nom);
    let resultat = std::fs::write(&fichier, &octets).map_err(|e| format!("dossier temporaire : {e}")).and_then(|_| importer(&fichier, dossier));
    let _ = std::fs::remove_dir_all(&temporaire);
    resultat
}

/// L'image telle que la barre l'affiche (réduite à 40 px de haut), en data URL PNG. `None` si introuvable ou illisible.
pub(super) fn apercu(chemin: &str) -> Option<String> {
    let image = crate::barre::image(chemin, false)?;
    let png = unsafe { png(image) }?;
    Some(format!("data:image/png;base64,{}", base64(&png)))
}

/// PNG d'une `NSImage` (sa première représentation bitmap).
pub(super) unsafe fn png(image: Id) -> Option<Vec<u8>> {
    unsafe {
        let tiff: Id = msg_send![image, TIFFRepresentation];
        let bitmap: Id = msg_send![class!(NSBitmapImageRep), imageRepWithData: tiff];
        if bitmap.is_null() {
            return None;
        }
        let proprietes: Id = msg_send![class!(NSDictionary), dictionary];
        // 4 = NSBitmapImageFileTypePNG.
        let donnees: Id = msg_send![bitmap, representationUsingType: 4usize, properties: proprietes];
        if donnees.is_null() {
            return None;
        }
        let longueur: usize = msg_send![donnees, length];
        let debut: *const std::ffi::c_void = msg_send![donnees, bytes];
        Some(std::slice::from_raw_parts(debut.cast::<u8>(), longueur).to_vec())
    }
}

pub(super) fn base64(octets: &[u8]) -> String {
    unsafe {
        let donnees: Id = msg_send![class!(NSData), dataWithBytes: octets.as_ptr().cast::<std::ffi::c_void>(), length: octets.len()];
        let texte: Id = msg_send![donnees, base64EncodedStringWithOptions: 0usize];
        (*(texte as *const NSString)).to_string()
    }
}

/// NSOpenPanel sur les images ; renvoie le fichier choisi.
pub(super) fn choisir() -> Option<PathBuf> {
    unsafe {
        let panneau: Id = msg_send![class!(NSOpenPanel), openPanel];
        let _: () = msg_send![panneau, setCanChooseDirectories: Bool::NO];
        let _: () = msg_send![panneau, setAllowsMultipleSelection: Bool::NO];
        let types: Id = msg_send![class!(NSMutableArray), array];
        for f in FORMATS {
            let _: () = msg_send![types, addObject: &*texte(f)];
        }
        let _: () = msg_send![panneau, setAllowedFileTypes: types];
        let reponse: isize = msg_send![panneau, runModal];
        if reponse != 1 {
            return None;
        }
        let url: Id = msg_send![panneau, URL];
        let chemin: Id = msg_send![url, path];
        Some(PathBuf::from((*(chemin as *const NSString)).to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ICNS: &str = "/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/GenericApplicationIcon.icns";

    fn dossier_essai(nom: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tactile-icones-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Hauteur et largeur en pixels d'un PNG (en-tête IHDR).
    fn taille_png(chemin: &Path) -> (u32, u32) {
        let o = std::fs::read(chemin).unwrap();
        assert_eq!(&o[1..4], b"PNG");
        let mot = |i: usize| u32::from_be_bytes(o[i..i + 4].try_into().unwrap());
        (mot(16), mot(20))
    }

    #[test]
    fn dossier_voisin_de_la_config() {
        assert_eq!(dossier(Path::new("/tmp/essai/barre.toml")), Path::new("/tmp/essai/icones"));
        let maison = std::env::var("HOME").unwrap();
        assert_eq!(dossier(&Path::new(&maison).join(".config/tactile/barre.toml")), Path::new(&maison).join(".config/tactile/icones"));
    }

    #[test]
    fn noms_propres_et_chemin_avec_tilde() {
        assert_eq!(nettoyer("Mon Logo (v2)"), "mon-logo-v2");
        assert_eq!(nettoyer("Logo Déposé"), "logo-depose");
        assert_eq!(nettoyer("Logo De\u{301}pose\u{301}"), "logo-depose", "nom NFD venu du Finder");
        assert_eq!(nettoyer("日本"), "image");
        assert_eq!(nettoyer("***"), "image");
        assert_eq!(nettoyer(&"a".repeat(80)).len(), 40);
        assert_eq!(hachage(b"abc"), hachage(b"abc"));
        assert_ne!(hachage(b"abc"), hachage(b"abd"));
        let maison = std::env::var("HOME").unwrap();
        assert_eq!(avec_tilde(&Path::new(&maison).join(".config/tactile/icones/x.png")), "~/.config/tactile/icones/x.png");
        assert_eq!(avec_tilde(Path::new("/tmp/x.png")), "/tmp/x.png");
    }

    #[test]
    #[cfg_attr(ci, ignore = "rendu AppKit dans un contexte bitmap : pas de session graphique sur un runner de CI")]
    fn copie_stable_sans_collision() {
        let d = dossier_essai("copie");
        let icones = d.join("icones");
        // Un PNG : copié tel quel, au même nom à chaque fois.
        let source = d.join("Mon Logo.png");
        std::fs::write(&source, std::fs::read(importer(Path::new(ICNS), &d.join("a")).unwrap()).unwrap()).unwrap();
        let copie = importer(&source, &icones).unwrap();
        assert_eq!(copie.parent().unwrap(), icones);
        let nom = copie.file_name().unwrap().to_string_lossy().into_owned();
        assert!(nom.starts_with("mon-logo-") && nom.ends_with(".png") && nom.len() == "mon-logo-".len() + 8 + 4, "{nom}");
        assert_eq!(std::fs::read(&copie).unwrap(), std::fs::read(&source).unwrap());
        assert_eq!(importer(&source, &icones).unwrap(), copie, "même contenu, même nom");
        // Un autre fichier du même nom : une autre copie, la première intacte.
        let autre = d.join("autre");
        std::fs::create_dir_all(&autre).unwrap();
        let source2 = autre.join("Mon Logo.png");
        std::fs::write(&source2, std::fs::read(importer(Path::new("/System/Applications/Calculator.app/Contents/Resources/AppIcon.icns"), &d.join("b")).unwrap()).unwrap()).unwrap();
        let copie2 = importer(&source2, &icones).unwrap();
        assert_ne!(copie2, copie);
        assert_eq!(std::fs::read_dir(&icones).unwrap().count(), 2);
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    #[cfg_attr(ci, ignore = "rendu AppKit dans un contexte bitmap : pas de session graphique sur un runner de CI")]
    fn svg_pdf_icns_convertis_en_png_de_40_px() {
        let d = dossier_essai("conversion");
        let svg = d.join("rond.svg");
        std::fs::write(&svg, r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100" viewBox="0 0 200 100"><rect width="200" height="100" fill="#A259FF"/></svg>"##).unwrap();
        let copie = importer(&svg, &d.join("icones")).unwrap();
        assert!(copie.extension().unwrap() == "png");
        assert_eq!(taille_png(&copie), (80, 40), "largeur proportionnelle, 40 px de haut");
        let icns = importer(Path::new(ICNS), &d.join("icones")).unwrap();
        assert!(icns.file_name().unwrap().to_string_lossy().starts_with("genericapplicationicon-"));
        assert_eq!(taille_png(&icns), (40, 40));
        // Un PDF d'une page, fait par AppKit lui-même.
        let pdf = d.join("carre.pdf");
        unsafe {
            let vue: Id = msg_send![class!(NSView), alloc];
            let vue: Id = msg_send![vue, initWithFrame: objc2_foundation::NSRect::new(objc2_foundation::NSPoint::new(0.0, 0.0), objc2_foundation::NSSize::new(50.0, 100.0))];
            let donnees: Id = msg_send![vue, dataWithPDFInsideRect: objc2_foundation::NSRect::new(objc2_foundation::NSPoint::new(0.0, 0.0), objc2_foundation::NSSize::new(50.0, 100.0))];
            let _: Bool = msg_send![donnees, writeToFile: &*texte(&pdf.to_string_lossy()), atomically: Bool::YES];
        }
        assert_eq!(taille_png(&importer(&pdf, &d.join("icones")).unwrap()), (20, 40));
        // Refusés : autre extension, ou fichier qui n'est pas une image.
        assert!(importer(Path::new("/etc/hosts"), &d.join("icones")).unwrap_err().contains("format"));
        let faux = d.join("faux.png");
        std::fs::write(&faux, "pas une image").unwrap();
        assert!(importer(&faux, &d.join("icones")).unwrap_err().contains("lisible"));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    #[cfg_attr(ci, ignore = "rendu AppKit dans un contexte bitmap : pas de session graphique sur un runner de CI")]
    fn depot_et_apercu() {
        let d = dossier_essai("depot");
        let png = std::fs::read(importer(Path::new(ICNS), &d.join("a")).unwrap()).unwrap();
        let copie = importer_depot("Glissé.png", &base64(&png), &d.join("icones")).unwrap();
        assert!(copie.file_name().unwrap().to_string_lossy().starts_with("glisse-"), "{}", copie.display());
        assert_eq!(std::fs::read(&copie).unwrap(), png);
        assert!(importer_depot("x.png", "!!!", &d.join("icones")).is_err());
        assert!(apercu(&copie.to_string_lossy()).is_some_and(|u| u.starts_with("data:image/png;base64,iVBOR")));
        // Image introuvable : signalée par `None`, la page affiche l'avertissement.
        std::fs::remove_file(&copie).unwrap();
        assert!(apercu(&copie.to_string_lossy()).is_none());
        std::fs::remove_dir_all(&d).unwrap();
    }
}
