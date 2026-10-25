//! Luminosité de l'écran intégré (0.0 à 1.0).

use super::Resultat;
use std::ffi::{CStr, c_char, c_int, c_void};

type Lire = unsafe extern "C" fn(u32, *mut f32) -> c_int;
type Regler = unsafe extern "C" fn(u32, f32) -> c_int;

const DISPLAY_SERVICES: &CStr = c"/System/Library/PrivateFrameworks/DisplayServices.framework/DisplayServices";
const RTLD_NOW: c_int = 2;

unsafe extern "C" {
    fn dlopen(chemin: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(module: *mut c_void, nom: *const c_char) -> *mut c_void;
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGMainDisplayID() -> u32;
}

fn symbole(nom: &CStr) -> Resultat<*mut c_void> {
    unsafe {
        let module = dlopen(DISPLAY_SERVICES.as_ptr(), RTLD_NOW);
        if module.is_null() {
            return Err("luminosité : impossible de charger DisplayServices".into());
        }
        let f = dlsym(module, nom.as_ptr());
        if f.is_null() {
            return Err(format!("luminosité : {} introuvable dans DisplayServices", nom.to_string_lossy()));
        }
        Ok(f)
    }
}

pub fn lire() -> Resultat<f32> {
    let get: Lire = unsafe { std::mem::transmute(symbole(c"DisplayServicesGetBrightness")?) };
    let mut valeur = 0.0;
    match unsafe { get(CGMainDisplayID(), &mut valeur) } {
        0 => Ok(valeur),
        code => Err(format!("luminosité : lecture refusée (code {code})")),
    }
}

pub fn regler(valeur: f32) -> Resultat<()> {
    let set: Regler = unsafe { std::mem::transmute(symbole(c"DisplayServicesSetBrightness")?) };
    match unsafe { set(CGMainDisplayID(), valeur.clamp(0.0, 1.0)) } {
        0 => Ok(()),
        code => Err(format!("luminosité : réglage refusé (code {code})")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regler_puis_relire_et_restaurer() {
        let origine = lire().unwrap();
        let cible = if origine > 0.5 { origine - 0.2 } else { origine + 0.2 };
        regler(cible).unwrap();
        let relue = lire();
        regler(origine).unwrap();
        assert!((relue.unwrap() - cible).abs() < 0.02);
        assert!((lire().unwrap() - origine).abs() < 0.02);
    }
}
