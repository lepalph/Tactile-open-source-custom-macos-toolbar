//! Animation de démarrage (≈ 0,9 s) : le logo (les trois pastilles de l'icône) apparaît au centre, se scinde en deux,
//! puis les éléments de la barre sortent du centre vers les bords. Rien ne bloque : le logo est une barre présentée
//! `SPLASH` secondes (`performSelector:afterDelay:`), tout le reste est en animations Core Animation avec retard
//! (`fillMode` « backwards » pour rester cachées d'ici là, « both » pour que la scission ne revienne pas en arrière).

use objc2::runtime::{AnyObject, Bool, MessageReceiver};
use objc2::encode::{Encoding, RefEncode};
use objc2::{class, msg_send, sel};
use objc2_foundation::{NSPoint, NSRect, NSSize};

use super::vues::{barre_touch, item_vue, texte};
use super::{cible, Id, PREFIXE};

#[link(name = "QuartzCore", kind = "framework")]
unsafe extern "C" {
    fn CACurrentMediaTime() -> f64;
}

/// `CGColorRef` opaque, pour que `msg_send!` vérifie le bon type (un `Id` serait refusé en debug).
#[repr(C)]
struct CGColor {
    _opaque: [u8; 0],
}

unsafe impl RefEncode for CGColor {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Encoding::Struct("CGColor", &[]));
}

/// Logo : entrée, puis scission ; la vraie barre arrive à `SPLASH`.
const ENTREE: f64 = 0.2;
const SCISSION: f64 = 0.22;
pub(super) const SPLASH: f64 = ENTREE + SCISSION;
/// Fondu de chaque élément et écart maximal entre deux rangs (du centre vers les bords, total ≤ 0,5 s).
const FONDU: f64 = 0.24;
const ECART: f64 = 0.045;
const RESTE: f64 = 0.5;
/// Pastilles du logo (points) et leurs couleurs : celles de l'icône (scripts/icone.sh).
const PASTILLE: (f64, f64) = (26.0, 14.0);
const ESPACE: f64 = 6.0;
const COULEURS: [u32; 3] = [0xE5E5EA, 0xB4552F, 0xE5E5EA];

/// « Réduire les animations » coché dans Réglages Système.
pub(super) fn mouvement_reduit() -> bool {
    unsafe {
        let espace: Id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let reduit: Bool = msg_send![espace, accessibilityDisplayShouldReduceMotion];
        reduit.as_bool()
    }
}

/// La barre du logo (à +1).
pub(super) fn splash() -> Id {
    unsafe {
        let largeur = 3.0 * PASTILLE.0 + 2.0 * ESPACE;
        let vue: Id = msg_send![class!(NSView), alloc];
        let vue: Id = msg_send![vue, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(largeur, 30.0))];
        let _: () = msg_send![vue, setWantsLayer: Bool::YES];
        let fond: Id = msg_send![vue, layer];
        for (k, hex) in COULEURS.into_iter().enumerate() {
            let calque: Id = msg_send![class!(CALayer), layer];
            let x = k as f64 * (PASTILLE.0 + ESPACE);
            let _: () = msg_send![calque, setFrame: NSRect::new(NSPoint::new(x, (30.0 - PASTILLE.1) / 2.0), NSSize::new(PASTILLE.0, PASTILLE.1))];
            let _: () = msg_send![calque, setCornerRadius: 4.0f64];
            let rgb = |d: u32| ((hex >> d) & 0xFF) as f64 / 255.0;
            let couleur: Id = msg_send![class!(NSColor), colorWithSRGBRed: rgb(16), green: rgb(8), blue: rgb(0), alpha: 1.0f64];
            let cg: *const CGColor = msg_send![couleur, CGColor];
            let _: () = msg_send![calque, setBackgroundColor: cg];
            // Entrée : le logo grossit en fondu, la pastille orange un rien après les deux autres.
            let retard = if k == 1 { 0.04 } else { 0.0 };
            ajouter(calque, "opacity", 0.0, 1.0, retard, ENTREE - 0.04, "backwards");
            ajouter(calque, "transform.scale", 0.6, 1.0, retard, ENTREE - 0.04, "backwards");
            // Scission : les pastilles claires partent vers les bords, l'orange s'étire puis s'efface.
            let ecart = (k as f64 - 1.0) * 60.0;
            if k == 1 {
                ajouter(calque, "transform.scale.x", 1.0, 2.2, ENTREE, SCISSION, "both");
            } else {
                ajouter(calque, "transform.translation.x", 0.0, ecart, ENTREE, SCISSION, "both");
            }
            ajouter(calque, "opacity", 1.0, 0.0, ENTREE + SCISSION * 0.3, SCISSION * 0.7, "both");
            let _: () = msg_send![fond, addSublayer: calque];
        }
        let item = item_vue(&format!("{PREFIXE}.demarrage"), vue);
        let _: () = msg_send![vue, release];
        barre_touch(&format!("{PREFIXE}.demarrage"), vec![null_flexible(), item, null_flexible()])
    }
}

fn null_flexible() -> Id {
    std::ptr::null_mut()
}

/// Anime les éléments de la barre montrée dans `modale`, du centre vers les bords : ils sortent du logo scindé.
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
        let centre = (vues.len() as f64 - 1.0) / 2.0;
        let ecart = ECART.min((RESTE - FONDU) / centre.max(1.0));
        for (k, vue) in vues.into_iter().enumerate() {
            let _: () = msg_send![vue, setWantsLayer: Bool::YES];
            let calque: Id = msg_send![vue, layer];
            if calque.is_null() {
                continue;
            }
            let rang = k as f64 - centre;
            let retard = rang.abs() * ecart;
            ajouter(calque, "opacity", 0.0, 1.0, retard, FONDU, "backwards");
            // Chaque élément part un peu vers le centre (au plus 24 points) et glisse à sa place.
            ajouter(calque, "transform.translation.x", (-rang * 8.0).clamp(-24.0, 24.0), 0.0, retard, FONDU, "backwards");
            ajouter(calque, "transform.scale", 0.92, 1.0, retard, FONDU, "backwards");
        }
    }
}

/// Une animation de `de` à `a` sur `chemin`, qui commence dans `retard` secondes (courbe « snappy » de l'éditeur).
unsafe fn ajouter(calque: Id, chemin: &str, de: f64, a: f64, retard: f64, duree: f64, remplissage: &str) {
    unsafe {
        let anim: Id = msg_send![class!(CABasicAnimation), animationWithKeyPath: &*texte(chemin)];
        let nombre = |v: f64| -> Id { msg_send![class!(NSNumber), numberWithDouble: v] };
        let _: () = msg_send![anim, setFromValue: nombre(de)];
        let _: () = msg_send![anim, setToValue: nombre(a)];
        let _: () = msg_send![anim, setDuration: duree];
        let _: () = msg_send![anim, setBeginTime: CACurrentMediaTime() + retard];
        let _: () = msg_send![anim, setFillMode: &*texte(remplissage)];
        let _: () = msg_send![anim, setRemovedOnCompletion: Bool::from(remplissage != "both")];
        // `msg_send!` ne sait pas écrire un sélecteur à parties vides : envoi direct.
        let fonction: Id = MessageReceiver::send_message(class!(CAMediaTimingFunction), sel!(functionWithControlPoints::::), (0.22f32, 1.0f32, 0.36f32, 1.0f32));
        let _: () = msg_send![anim, setTimingFunction: fonction];
        let cle = format!("tactile-{chemin}-{retard}");
        let _: () = msg_send![calque, addAnimation: anim, forKey: &*texte(&cle)];
    }
}

/// Appelé `SPLASH` secondes après la présentation du ✳ : la vraie barre prend sa place.
pub(super) fn programmer_fin() {
    unsafe {
        let _: () = msg_send![cible(), performSelector: sel!(finDemarrage:), withObject: std::ptr::null_mut::<AnyObject>(), afterDelay: SPLASH];
    }
}
