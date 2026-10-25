//! Frappes clavier simulées (esc, etc.). Demande l'autorisation Accessibilité.

use super::Resultat;
use std::ffi::c_void;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreateKeyboardEvent(source: *const c_void, touche: u16, appuyee: bool) -> *mut c_void;
    fn CGEventPost(cible: u32, evenement: *mut c_void);
    fn CFRelease(objet: *const c_void);
}

pub fn touche(code: u16) -> Resultat<()> {
    for appuyee in [true, false] {
        unsafe {
            let e = CGEventCreateKeyboardEvent(std::ptr::null(), code, appuyee);
            if e.is_null() {
                return Err(format!("clavier : impossible de créer la touche {code}"));
            }
            CGEventPost(0, e);
            CFRelease(e);
        }
    }
    Ok(())
}
