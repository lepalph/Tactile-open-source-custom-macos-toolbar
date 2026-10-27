//! Animation de démarrage (0,8 s) : un bref ✳ Tactile au centre, puis les éléments de la barre apparaissent de gauche à droite.
//! Rien ne bloque : le ✳ est une barre présentée 0,2 s (`performSelector:afterDelay:`), les éléments portent des animations
//! Core Animation qui démarrent chacune avec son retard (fondu + glissement, `fillMode` « backwards » pour rester cachées d'ici là).

use objc2::runtime::{AnyObject, Bool};
use objc2::{class, msg_send, sel};

use super::vues::{barre_touch, item_vue, texte};
use super::{cible, Id, PREFIXE};

#[link(name = "QuartzCore", kind = "framework")]
unsafe extern "C" {
    fn CACurrentMediaTime() -> f64;
}

/// Durée du ✳ Tactile, puis fondu de chaque élément et écart maximal entre deux éléments (total ≤ 0,8 s).
pub(super) const SPLASH: f64 = 0.2;
const FONDU: f64 = 0.2;
const ECART: f64 = 0.04;
const TOTAL: f64 = 0.8;

/// « Réduire les animations » coché dans Réglages Système.
pub(super) fn mouvement_reduit() -> bool {
    unsafe {
        let espace: Id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let reduit: Bool = msg_send![espace, accessibilityDisplayShouldReduceMotion];
        reduit.as_bool()
    }
}

/// La barre du ✳ Tactile (à +1).
pub(super) fn splash() -> Id {
    unsafe {
        let label: Id = msg_send![class!(NSTextField), labelWithString: &*texte("✳ Tactile")];
        let police: Id = msg_send![class!(NSFont), systemFontOfSize: 17.0f64, weight: 0.3f64];
        let _: () = msg_send![label, setFont: police];
        let _: () = msg_send![label, setAlignment: 2isize];
        let _: () = msg_send![label, setWantsLayer: Bool::YES];
        let calque: Id = msg_send![label, layer];
        ajouter(calque, "opacity", 0.0, 1.0, 0.0, SPLASH / 2.0);
        let item = item_vue(&format!("{PREFIXE}.demarrage"), label);
        barre_touch(&format!("{PREFIXE}.demarrage"), vec![null_flexible(), item, null_flexible()])
    }
}

fn null_flexible() -> Id {
    std::ptr::null_mut()
}

/// Anime les éléments de la barre montrée dans `modale`, de gauche à droite.
pub(super) fn apparition(modale: Id) {
    unsafe {
        let idents: Id = msg_send![modale, defaultItemIdentifiers];
        let n: usize = msg_send![idents, count];
        let vues: Vec<Id> = (0..n)
            .filter_map(|i| {
                let id: Id = msg_send![idents, objectAtIndex: i];
                let item: Id = msg_send![modale, itemForIdentifier: id];
                let vue: Id = if item.is_null() { item } else { msg_send![item, view] };
                (!vue.is_null()).then_some(vue)
            })
            .collect();
        let ecart = ECART.min((TOTAL - SPLASH - FONDU) / vues.len().max(1) as f64);
        for (k, vue) in vues.into_iter().enumerate() {
            let _: () = msg_send![vue, setWantsLayer: Bool::YES];
            let calque: Id = msg_send![vue, layer];
            if calque.is_null() {
                continue;
            }
            let retard = k as f64 * ecart;
            ajouter(calque, "opacity", 0.0, 1.0, retard, FONDU);
            ajouter(calque, "transform.translation.x", -12.0, 0.0, retard, FONDU);
        }
    }
}

/// Une animation de `de` à `a` sur `chemin`, qui commence dans `retard` secondes et tient `de` d'ici là.
unsafe fn ajouter(calque: Id, chemin: &str, de: f64, a: f64, retard: f64, duree: f64) {
    unsafe {
        let anim: Id = msg_send![class!(CABasicAnimation), animationWithKeyPath: &*texte(chemin)];
        let nombre = |v: f64| -> Id { msg_send![class!(NSNumber), numberWithDouble: v] };
        let _: () = msg_send![anim, setFromValue: nombre(de)];
        let _: () = msg_send![anim, setToValue: nombre(a)];
        let _: () = msg_send![anim, setDuration: duree];
        let _: () = msg_send![anim, setBeginTime: CACurrentMediaTime() + retard];
        let _: () = msg_send![anim, setFillMode: &*texte("backwards")];
        let fonction: Id = msg_send![class!(CAMediaTimingFunction), functionWithName: &*texte("easeOut")];
        let _: () = msg_send![anim, setTimingFunction: fonction];
        let _: () = msg_send![calque, addAnimation: anim, forKey: &*texte(&format!("tactile-{chemin}"))];
    }
}

/// Appelé `SPLASH` secondes après la présentation du ✳ : la vraie barre prend sa place.
pub(super) fn programmer_fin() {
    unsafe {
        let _: () = msg_send![cible(), performSelector: sel!(finDemarrage:), withObject: std::ptr::null_mut::<AnyObject>(), afterDelay: SPLASH];
    }
}
