//! L'icône ✳ placée dans le Control Strip d'Apple, qui ramène la barre tactile.

use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2::{class, msg_send, sel};

use super::vues::{item_vue, texte};
use super::{cible, Id};

pub(super) const PLATEAU: &str = "fr.palph.tactile.plateau";

/// Déclare l'icône ✳ auprès du système, puis la rend visible.
pub(super) fn installer_plateau() {
    unsafe {
        let b: Id = msg_send![class!(NSButton), buttonWithTitle: &*texte("✳"), target: cible(), action: sel!(afficherBarre:)];
        let item = item_vue(PLATEAU, b);
        let _: () = msg_send![class!(NSTouchBarItem), addSystemTrayItem: item];
    }
    montrer_icone();
}

/// Rend l'icône ✳ visible dans le Control Strip (DFRFoundation, privé). Le système l'oublie quand une barre
/// système est présentée : comme MTMR, on la réaffiche à chaque retour vers la barre d'Apple.
pub(super) fn montrer_icone() {
    unsafe extern "C" {
        fn dlopen(chemin: *const std::ffi::c_char, mode: i32) -> *mut std::ffi::c_void;
        fn dlsym(module: *mut std::ffi::c_void, nom: *const std::ffi::c_char) -> *mut std::ffi::c_void;
    }
    type Presence = unsafe extern "C" fn(Id, Bool);
    unsafe {
        let module = dlopen(c"/System/Library/PrivateFrameworks/DFRFoundation.framework/DFRFoundation".as_ptr(), 2);
        let f = dlsym(module, c"DFRElementSetControlStripPresenceForIdentifier".as_ptr());
        if f.is_null() {
            eprintln!("tactile : icône du Control Strip indisponible");
            return;
        }
        let presence: Presence = std::mem::transmute(f);
        presence(Retained::as_ptr(&texte(PLATEAU)) as Id, Bool::YES);
    }
}
