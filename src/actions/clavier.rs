//! Frappes clavier simulées : `echap`, `{ touche = "cmd+shift+4" }`, `{ touche = "cmd+shift+keycode:21" }`, `{ keycode = 53 }`.
//! Demande l'autorisation Accessibilité.

use super::Resultat;
use serde::{Deserialize, Serialize};
use std::ffi::c_void;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreateKeyboardEvent(source: *const c_void, touche: u16, appuyee: bool) -> *mut c_void;
    fn CGEventSetFlags(evenement: *mut c_void, drapeaux: u64);
    fn CGEventPost(cible: u32, evenement: *mut c_void);
    fn CFRelease(objet: *const c_void);
    fn CFDataGetBytePtr(donnees: *const c_void) -> *const u8;
    #[cfg(test)]
    fn CFRetain(objet: *const c_void) -> *const c_void;
}

// La disposition courante (TIS / UCKeyTranslate) : sur un clavier français, le keycode 0 est « q », pas « a ».
#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn TISCopyCurrentKeyboardLayoutInputSource() -> *mut c_void;
    fn TISGetInputSourceProperty(source: *mut c_void, cle: *const c_void) -> *const c_void;
    static kTISPropertyUnicodeKeyLayoutData: *const c_void;
    #[cfg(test)]
    fn TISCreateInputSourceList(proprietes: *const c_void, toutes: bool) -> *const c_void;
    #[cfg(test)]
    static kTISPropertyInputSourceID: *const c_void;
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

const CMD: u64 = 1 << 20;
const SHIFT: u64 = 1 << 17;
const ALT: u64 = 1 << 19;
const CTRL: u64 = 1 << 18;

/// Les chiffres visent la rangée du haut par sa position (keycodes ANSI de 0 à 9), quelle que soit la
/// disposition : les raccourcis d'Apple (⌘⇧3, ⌘⇧4, ⌘⇧5) sont liés au keycode (`com.apple.symbolichotkeys` :
/// ⌘⇧4 = keycode 21 + cmd + shift), et en AZERTY le premier code qui donne « 4 » sans Maj est celui du pavé (86).
/// Avec cmd ou ctrl, les modificateurs envoyés sont ceux écrits (`cmd+4` ne devient pas ⌘⇧4) ; sans eux,
/// Maj est ajouté si la disposition l'exige pour taper le chiffre (« 4 » seul en AZERTY).
const CHIFFRES: [u16; 10] = [29, 18, 19, 20, 21, 23, 22, 26, 28, 25];

/// Le pavé numérique : un caractère ne s'y résout que si aucune touche principale ne le donne.
const PAVE: std::ops::RangeInclusive<u16> = 65..=92;

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

/// Un raccourci lu dans la config : `"cmd+shift+4"`, `"f5"`, `"esc"`, `"cmd+shift+keycode:21"`. Vérifié à la lecture, résolu à l'appui.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Touche {
    /// Dernier mot du raccourci : touche nommée, caractère seul ou `keycode:n`.
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
                let connue = mot.chars().count() == 1 || NOMMEES.iter().any(|(n, _)| *n == mot) || keycode(&mot).is_some();
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

/// Réécrit le raccourci pour le TOML : `"cmd+shift+4"` (un seul nom par modificateur, dans l'ordre de `MODIFICATEURS`).
impl From<Touche> for String {
    fn from(t: Touche) -> String {
        let mut vus = 0;
        let mut mots = Vec::new();
        for (nom, d) in MODIFICATEURS {
            if t.drapeaux & d != 0 && vus & d == 0 {
                vus |= d;
                mots.push(*nom);
            }
        }
        mots.push(&t.nom);
        mots.join("+")
    }
}

/// `keycode:21` : une touche par son code, pour lui donner des modificateurs.
fn keycode(mot: &str) -> Option<u16> {
    mot.strip_prefix("keycode:")?.parse().ok()
}

impl Touche {
    /// Keycode et modificateurs complets : ceux du raccourci, plus ceux qu'exige la disposition (`\` = ⌥⇧: en AZERTY).
    fn code(&self) -> Resultat<(u16, u64)> {
        if let Some(code) = NOMMEES.iter().find(|(n, _)| *n == self.nom).map(|(_, c)| *c).or_else(|| keycode(&self.nom)) {
            return Ok((code, self.drapeaux));
        }
        let c = self.nom.chars().next().ok_or("touche vide")?;
        let source = unsafe { TISCopyCurrentKeyboardLayoutInputSource() };
        unsafe { sur_disposition(source, |d| chercher(d, c, self.drapeaux)) }
            .flatten()
            .map(|(code, ajout)| (code, self.drapeaux | ajout))
            .ok_or_else(|| format!("clavier : la touche « {c} » n'existe pas sur la disposition actuelle"))
    }
}

/// Appelle `f` avec la table `uchr` de `source`, puis relâche `source`. `None` si la source n'en a pas (méthode de saisie).
unsafe fn sur_disposition<R>(source: *mut c_void, f: impl FnOnce(*const c_void) -> R) -> Option<R> {
    if source.is_null() {
        return None;
    }
    let donnees = unsafe { TISGetInputSourceProperty(source, kTISPropertyUnicodeKeyLayoutData) };
    let resultat = (!donnees.is_null()).then(|| f(unsafe { CFDataGetBytePtr(donnees) } as *const c_void));
    unsafe { CFRelease(source) };
    resultat
}

/// Ce que la touche `code` tape avec les modificateurs CGEvent `drapeaux`.
fn tape(disposition: *const c_void, code: u16, drapeaux: u64) -> Vec<u16> {
    // État Carbon (>> 8) : cmd = 1, shift = 2, option = 8. Ctrl est laissé de côté : il donne des caractères de contrôle.
    let etat = [(CMD, 1), (SHIFT, 2), (ALT, 8)].iter().filter(|(d, _)| drapeaux & d != 0).map(|(_, e)| e).sum::<u32>();
    let (mut etat_mort, mut longueur, mut tampon) = (0u32, 0usize, [0u16; 4]);
    // action 0 = kUCKeyActionDown, option 1 = kUCKeyTranslateNoDeadKeysBit.
    let erreur = unsafe { UCKeyTranslate(disposition, code, 0, etat, LMGetKbdType() as u32, 1, &mut etat_mort, 4, &mut longueur, tampon.as_mut_ptr()) };
    if erreur == 0 { tampon[..longueur].to_vec() } else { Vec::new() }
}

/// Keycode de `c` sur `disposition`, et les modificateurs à ajouter à `drapeaux` pour le taper. Ordre : rangée du haut
/// pour un chiffre (voir `CHIFFRES`) ; sinon la zone principale sans modificateur, avec Maj, Option, Option+Maj ; le pavé numérique en dernier.
/// Seul cmd des `drapeaux` compte pour la recherche : certaines dispositions (AZERTY) ont une table à part sous cmd.
fn chercher(disposition: *const c_void, c: char, drapeaux: u64) -> Option<(u16, u64)> {
    let voulu: Vec<u16> = c.to_lowercase().collect::<String>().encode_utf16().collect();
    let produit = |code: u16, ajout: u64| tape(disposition, code, (drapeaux & CMD) | ajout) == voulu;
    if let Some(n) = c.to_digit(10) {
        let code = CHIFFRES[n as usize];
        let raccourci = drapeaux & (CMD | CTRL) != 0;
        return Some((code, if raccourci { 0 } else { [0, SHIFT].into_iter().find(|&a| produit(code, a)).unwrap_or(0) }));
    }
    [0, SHIFT, ALT, ALT | SHIFT]
        .into_iter()
        .find_map(|ajout| (0u16..128).filter(|k| !PAVE.contains(k)).find(|&k| produit(k, ajout)).map(|k| (k, ajout)))
        .or_else(|| PAVE.into_iter().find(|&k| produit(k, 0)).map(|k| (k, 0)))
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
    let (code, drapeaux) = t.code()?;
    touche(code, drapeaux)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les fonctions TIS ne supportent pas d'être appelées de deux fils à la fois (abandon vu avec les tests en parallèle).
    static TIS: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn lire(s: &str) -> Result<Touche, String> {
        Touche::try_from(s.to_string())
    }

    #[test]
    fn raccourci_lu_et_resolu() {
        let t = lire("cmd+shift+4").unwrap();
        assert_eq!(t.drapeaux, CMD | SHIFT);
        assert_eq!(t.nom, "4");
        assert_eq!(lire("Ctrl + Alt + Esc").unwrap(), Touche { nom: "esc".into(), drapeaux: (1 << 18) | ALT });
        assert_eq!(lire("fn+F5").unwrap(), Touche { nom: "f5".into(), drapeaux: 1 << 23 });
        assert_eq!(lire("f5").unwrap().code(), Ok((96, 0)));
        assert_eq!(lire("esc").unwrap().code(), Ok((53, 0)));
        assert_eq!(lire("echap").unwrap().code(), Ok((53, 0)));
        assert_eq!(lire("cmd+gauche").unwrap().code(), Ok((123, CMD)));
    }

    #[test]
    fn keycode_avec_modificateurs() {
        let t = lire("cmd+shift+keycode:21").unwrap();
        assert_eq!(t.code(), Ok((21, CMD | SHIFT)));
        assert_eq!(String::from(t), "cmd+shift+keycode:21");
        assert_eq!(lire("keycode:53").unwrap().code(), Ok((53, 0)));
        assert!(lire("cmd+keycode:abc").is_err());
        assert!(lire("cmd+keycode:").is_err());
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
    fn un_chiffre_vise_la_rangee_du_haut() {
        let _tis = TIS.lock().unwrap();
        // Indépendant de la disposition : la capture d'écran d'Apple est liée au keycode 21, Maj et cmd compris.
        assert_eq!(lire("cmd+shift+4").unwrap().code(), Ok((21, CMD | SHIFT)));
        assert_eq!(lire("cmd+shift+3").unwrap().code(), Ok((20, CMD | SHIFT)));
        for (n, attendu) in CHIFFRES.iter().enumerate() {
            let (code, drapeaux) = lire(&format!("cmd+shift+{n}")).unwrap().code().unwrap();
            assert_eq!((code, drapeaux), (*attendu, CMD | SHIFT), "chiffre {n}");
            assert!(!PAVE.contains(&code));
        }
    }

    #[test]
    fn caractere_resolu_sur_la_disposition_courante() {
        let _tis = TIS.lock().unwrap();
        // Le code dépend du clavier (a = 0 en QWERTY, 12 en AZERTY) mais existe toujours, hors du pavé ; un caractère absent n'a pas de code.
        for t in ["cmd+a", "A", "cmd+/", "cmd+,"] {
            let (code, _) = lire(t).unwrap().code().unwrap();
            assert!(!PAVE.contains(&code), "{t} → {code}");
        }
        assert!(lire("cmd+\u{1F600}").unwrap().code().is_err());
    }

    /// La disposition « French » chargée explicitement : le test d'AZERTY tourne sur n'importe quel Mac.
    fn francais() -> *mut c_void {
        use objc2::runtime::AnyObject;
        use objc2::{class, msg_send};
        unsafe {
            let valeur = objc2_foundation::NSString::from_str("com.apple.keylayout.French");
            let dico: *mut AnyObject = msg_send![class!(NSDictionary), dictionaryWithObject: &*valeur, forKey: kTISPropertyInputSourceID as *const AnyObject];
            let liste = TISCreateInputSourceList(dico as *const c_void, true);
            if liste.is_null() {
                return std::ptr::null_mut();
            }
            let premiere: *mut AnyObject = msg_send![liste as *const AnyObject, firstObject];
            let source = if premiere.is_null() { std::ptr::null() } else { CFRetain(premiere as *const c_void) };
            CFRelease(liste);
            source as *mut c_void
        }
    }

    #[test]
    #[cfg_attr(ci, ignore = "dispositions clavier (TIS) de la session : pas de session ouverte sur un runner de CI")]
    fn azerty_chiffres_antislash_et_caracteres_a_option() {
        let _tis = TIS.lock().unwrap();
        let source = francais();
        if source.is_null() {
            return eprintln!("disposition French absente : test sauté");
        }
        let resultats = unsafe {
            sur_disposition(source, |d| {
                [('4', CMD | SHIFT), ('4', 0), ('6', 0), ('a', CMD), ('\\', CMD), ('[', CMD), ('/', 0), ('&', 0), ('4', CMD)].map(|(c, m)| ((c, m), chercher(d, c, m)))
            })
        }
        .unwrap();
        for ((c, m), r) in resultats {
            eprintln!("{c:?} {m:#x} → {r:?}");
        }
        let r = |i: usize| resultats[i].1;
        // ⌘⇧4 : keycode 21, Maj déjà écrit ; « 4 » seul demande Maj en AZERTY (la touche donne « ' »).
        assert_eq!(r(0), Some((21, 0)));
        assert_eq!(r(1), Some((21, SHIFT)));
        assert_eq!(r(2), Some((22, SHIFT)));
        assert_eq!(r(3), Some((12, 0)));
        // « \ » et « [ » n'existent qu'avec Option (et Maj) : on les ajoute.
        for i in [4, 5] {
            let (k, ajout) = r(i).unwrap();
            assert!(!PAVE.contains(&k) && ajout & ALT != 0, "{:?}", resultats[i]);
        }
        // « / » est sur la zone principale avec Maj, pas sur le pavé (75).
        let (k, ajout) = r(6).unwrap();
        assert!(!PAVE.contains(&k) && ajout == SHIFT, "{:?}", resultats[6]);
        assert_eq!(r(7), Some((18, 0)));
        // ⌘4 reste ⌘4 (keycode 21, sans Maj ajouté) : sinon ce serait la capture d'écran.
        assert_eq!(r(8), Some((21, 0)));
    }
}
