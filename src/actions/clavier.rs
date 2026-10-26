//! Frappes clavier simulées : `echap`, `{ touche = "cmd+shift+4" }`, `{ keycode = 53 }`. Demande l'autorisation Accessibilité.

use super::Resultat;
use serde::Deserialize;
use std::ffi::c_void;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreateKeyboardEvent(source: *const c_void, touche: u16, appuyee: bool) -> *mut c_void;
    fn CGEventSetFlags(evenement: *mut c_void, drapeaux: u64);
    fn CGEventPost(cible: u32, evenement: *mut c_void);
    fn CFRelease(objet: *const c_void);
    fn CFDataGetBytePtr(donnees: *const c_void) -> *const u8;
}

// La disposition courante (TIS / UCKeyTranslate) : sur un clavier français, le keycode 0 est « q », pas « a ».
#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn TISCopyCurrentKeyboardLayoutInputSource() -> *mut c_void;
    fn TISGetInputSourceProperty(source: *mut c_void, cle: *const c_void) -> *const c_void;
    static kTISPropertyUnicodeKeyLayoutData: *const c_void;
    fn LMGetKbdType() -> u8;
    fn UCKeyTranslate(disposition: *const c_void, touche: u16, action: u16, modificateurs: u32, type_clavier: u32, options: u32, etat_mort: *mut u32, longueur_max: usize, longueur: *mut usize, caracteres: *mut u16) -> i32;
}

/// Drapeaux CGEvent des modificateurs.
const MODIFICATEURS: &[(&str, u64)] = &[
    ("cmd", 1 << 20),
    ("command", 1 << 20),
    ("shift", 1 << 17),
    ("maj", 1 << 17),
    ("ctrl", 1 << 18),
    ("control", 1 << 18),
    ("alt", 1 << 19),
    ("option", 1 << 19),
    ("fn", 1 << 23),
];

/// Touches dont le code ne dépend pas de la disposition (et ses alias).
const NOMMEES: &[(&str, u16)] = &[
    ("enter", 36), ("return", 36), ("entree", 36), ("retour", 36),
    ("tab", 48),
    ("space", 49), ("espace", 49),
    ("delete", 51), ("backspace", 51), ("suppr", 51), ("effacer", 51),
    ("esc", 53), ("escape", 53), ("echap", 53),
    ("forwarddelete", 117),
    ("home", 115), ("debut", 115),
    ("end", 119), ("fin", 119),
    ("pageup", 116), ("pagedown", 121),
    ("left", 123), ("gauche", 123),
    ("right", 124), ("droite", 124),
    ("down", 125), ("bas", 125),
    ("up", 126), ("haut", 126),
    ("f1", 122), ("f2", 120), ("f3", 99), ("f4", 118), ("f5", 96), ("f6", 97), ("f7", 98), ("f8", 100), ("f9", 101),
    ("f10", 109), ("f11", 103), ("f12", 111), ("f13", 105), ("f14", 107), ("f15", 113), ("f16", 106), ("f17", 64),
    ("f18", 79), ("f19", 80), ("f20", 90),
];

/// Un raccourci lu dans la config : `"cmd+shift+4"`, `"f5"`, `"esc"`. Vérifié à la lecture, résolu à l'appui.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "String")]
pub struct Touche {
    /// Dernier mot du raccourci : touche nommée ou caractère seul.
    nom: String,
    drapeaux: u64,
}

impl TryFrom<String> for Touche {
    type Error = String;

    fn try_from(texte: String) -> Result<Self, String> {
        let mut drapeaux = 0;
        let mut mots = texte.split('+').map(|m| m.trim().to_lowercase()).peekable();
        while let Some(mot) = mots.next() {
            if mots.peek().is_none() {
                let connue = mot.chars().count() == 1 || NOMMEES.iter().any(|(n, _)| *n == mot);
                return if connue && !mot.is_empty() {
                    Ok(Touche { nom: mot, drapeaux })
                } else {
                    Err(format!("touche inconnue « {mot} » ; attendu : cmd+shift+4, f5, keycode = 53"))
                };
            }
            match MODIFICATEURS.iter().find(|(n, _)| *n == mot) {
                Some((_, d)) => drapeaux |= d,
                None => return Err(format!("modificateur inconnu « {mot} » ; attendu : cmd, shift, ctrl, alt, fn")),
            }
        }
        Err("touche vide".into())
    }
}

impl Touche {
    fn code(&self) -> Resultat<u16> {
        if let Some((_, code)) = NOMMEES.iter().find(|(n, _)| *n == self.nom) {
            return Ok(*code);
        }
        let c = self.nom.chars().next().ok_or("touche vide")?;
        code_du_caractere(c).ok_or_else(|| format!("clavier : la touche « {c} » n'existe pas sur la disposition actuelle"))
    }
}

/// Premier keycode qui produit `c` sans modificateur sur la disposition courante.
fn code_du_caractere(c: char) -> Option<u16> {
    unsafe {
        let source = TISCopyCurrentKeyboardLayoutInputSource();
        if source.is_null() {
            return None;
        }
        let donnees = TISGetInputSourceProperty(source, kTISPropertyUnicodeKeyLayoutData);
        let resultat = (!donnees.is_null()).then(|| {
            let disposition = CFDataGetBytePtr(donnees) as *const c_void;
            let voulu: Vec<u16> = c.to_lowercase().collect::<String>().encode_utf16().collect();
            (0u16..128).find(|&code| {
                let (mut etat_mort, mut longueur, mut tampon) = (0u32, 0usize, [0u16; 4]);
                // action 0 = kUCKeyActionDown, option 1 = kUCKeyTranslateNoDeadKeysBit.
                let erreur = UCKeyTranslate(disposition, code, 0, 0, LMGetKbdType() as u32, 1, &mut etat_mort, 4, &mut longueur, tampon.as_mut_ptr());
                erreur == 0 && tampon[..longueur] == voulu[..]
            })
        }).flatten();
        CFRelease(source);
        resultat
    }
}

/// Appuie puis relâche `code` avec les modificateurs `drapeaux`.
pub fn touche(code: u16, drapeaux: u64) -> Resultat<()> {
    for appuyee in [true, false] {
        unsafe {
            let e = CGEventCreateKeyboardEvent(std::ptr::null(), code, appuyee);
            if e.is_null() {
                return Err(format!("clavier : impossible de créer la touche {code}"));
            }
            CGEventSetFlags(e, drapeaux);
            CGEventPost(0, e);
            CFRelease(e);
        }
    }
    Ok(())
}

pub fn raccourci(t: &Touche) -> Resultat<()> {
    touche(t.code()?, t.drapeaux)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lire(s: &str) -> Result<Touche, String> {
        Touche::try_from(s.to_string())
    }

    #[test]
    fn raccourci_lu_et_resolu() {
        let t = lire("cmd+shift+4").unwrap();
        assert_eq!(t.drapeaux, (1 << 20) | (1 << 17));
        assert_eq!(t.nom, "4");
        assert_eq!(lire("Ctrl + Alt + Esc").unwrap(), Touche { nom: "esc".into(), drapeaux: (1 << 18) | (1 << 19) });
        assert_eq!(lire("fn+F5").unwrap(), Touche { nom: "f5".into(), drapeaux: 1 << 23 });
        assert_eq!(lire("f5").unwrap().code(), Ok(96));
        assert_eq!(lire("esc").unwrap().code(), Ok(53));
        assert_eq!(lire("echap").unwrap().code(), Ok(53));
        assert_eq!(lire("cmd+gauche").unwrap().code(), Ok(123));
    }

    #[test]
    fn touche_inconnue_dit_la_forme_attendue() {
        assert_eq!(lire("cmd+truc").unwrap_err(), "touche inconnue « truc » ; attendu : cmd+shift+4, f5, keycode = 53");
        assert_eq!(lire("cmd+").unwrap_err(), "touche inconnue «  » ; attendu : cmd+shift+4, f5, keycode = 53");
        assert_eq!(lire("super+a").unwrap_err(), "modificateur inconnu « super » ; attendu : cmd, shift, ctrl, alt, fn");
    }

    #[test]
    fn table_des_touches_nommees_sans_doublon() {
        for (i, (nom, _)) in NOMMEES.iter().enumerate() {
            assert!(!NOMMEES[..i].iter().any(|(n, _)| n == nom), "{nom} en double");
            assert_eq!(*nom, nom.to_lowercase());
        }
        assert_eq!(NOMMEES.iter().filter(|(n, _)| n.starts_with('f') && n[1..].parse::<u8>().is_ok()).count(), 20);
    }

    #[test]
    fn caractere_resolu_sur_la_disposition_courante() {
        // Le code dépend du clavier (a = 0 en QWERTY, 12 en AZERTY) mais existe toujours ; un caractère absent n'a pas de code.
        assert!(lire("cmd+a").unwrap().code().is_ok());
        assert!(lire("A").unwrap().code().is_ok());
        assert_eq!(code_du_caractere('\u{1F600}'), None);
    }
}
