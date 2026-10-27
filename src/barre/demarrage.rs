//! Animation de démarrage (≈ 1 s) : le logo (les trois pastilles de l'icône) apparaît à gauche, recule pour prendre
//! son élan, puis file comme une fusée jusqu'au bord droit en laissant une traînée orange ; la barre apparaît ensuite
//! dans son sillage, de gauche à droite. Rien ne bloque : le logo est une barre présentée `SPLASH` secondes
//! (`performSelector:afterDelay:`), tout le reste est en animations Core Animation avec retard (`fillMode`
//! « backwards » pour rester cachées d'ici là, « both » pour que la fusée ne revienne pas en arrière).

use objc2::encode::{Encoding, RefEncode};
use objc2::runtime::{AnyObject, Bool, MessageReceiver};
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

/// Logo : entrée avec recul, puis vol ; la vraie barre arrive à `SPLASH`.
const ENTREE: f64 = 0.2;
const VOL: f64 = 0.45;
pub(super) const SPLASH: f64 = ENTREE + VOL;
/// Fondu de chaque élément et écart maximal entre deux éléments (de gauche à droite, total ≤ 0,4 s).
const FONDU: f64 = 0.2;
const ECART: f64 = 0.035;
const RESTE: f64 = 0.4;
/// Pastilles du logo (points) et leurs couleurs : celles de l'icône (scripts/icone.sh).
const PASTILLE: (f64, f64) = (18.0, 10.0);
const ESPACE: f64 = 4.0;
const COULEURS: [u32; 3] = [0xE5E5EA, 0xB4552F, 0xE5E5EA];
const ORANGE: u32 = 0xB4552F;
/// Courbes : « snappy » pour ce qui se pose, accélération franche pour le décollage.
const SNAPPY: [f32; 4] = [0.22, 1.0, 0.36, 1.0];
const DECOLLAGE: [f32; 4] = [0.6, 0.0, 0.9, 0.5];

/// « Réduire les animations » coché dans Réglages Système.
pub(super) fn mouvement_reduit() -> bool {
    unsafe {
        let espace: Id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let reduit: Bool = msg_send![espace, accessibilityDisplayShouldReduceMotion];
        reduit.as_bool()
    }
}

unsafe fn couleur(hex: u32, alpha: f64) -> *const CGColor {
    let rgb = |d: u32| ((hex >> d) & 0xFF) as f64 / 255.0;
    unsafe {
        let c: Id = msg_send![class!(NSColor), colorWithSRGBRed: rgb(16), green: rgb(8), blue: rgb(0), alpha: alpha];
        msg_send![c, CGColor]
    }
}

/// La barre du logo (à +1), large de `largeur` points (toute la zone de la barre).
pub(super) fn splash(largeur: f64) -> Id {
    unsafe {
        let largeur = largeur - 16.0;
        let vue: Id = msg_send![class!(NSView), alloc];
        let vue: Id = msg_send![vue, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(largeur, 30.0))];
        let _: () = msg_send![vue, setWantsLayer: Bool::YES];
        let fond: Id = msg_send![vue, layer];
        let _: () = msg_send![fond, setMasksToBounds: Bool::YES];

        // La fusée : un calque qui porte les pastilles et la traînée, et qui se déplace d'un bloc.
        let logo = 3.0 * PASTILLE.0 + 2.0 * ESPACE;
        let depart = 12.0;
        let fusee: Id = msg_send![class!(CALayer), layer];
        let _: () = msg_send![fusee, setFrame: NSRect::new(NSPoint::new(depart, 0.0), NSSize::new(logo, 30.0))];

        // Traînée : un dégradé orange → transparent derrière le logo, qui s'allume au décollage.
        let trainee: Id = msg_send![class!(CAGradientLayer), layer];
        let long = 220.0;
        let _: () = msg_send![trainee, setFrame: NSRect::new(NSPoint::new(-long, 12.0), NSSize::new(long + PASTILLE.0 / 2.0, 6.0))];
        let _: () = msg_send![trainee, setCornerRadius: 3.0f64];
        let couleurs: Id = msg_send![class!(NSMutableArray), array];
        let _: () = msg_send![couleurs, addObject: couleur(ORANGE, 0.0) as Id];
        let _: () = msg_send![couleurs, addObject: couleur(ORANGE, 0.9) as Id];
        let _: () = msg_send![trainee, setColors: couleurs];
        let _: () = msg_send![trainee, setStartPoint: NSPoint::new(0.0, 0.5)];
        let _: () = msg_send![trainee, setEndPoint: NSPoint::new(1.0, 0.5)];
        let _: () = msg_send![trainee, setOpacity: 0.0f32];
        ajouter(trainee, "opacity", 0.0, 1.0, ENTREE, VOL * 0.4, "both", SNAPPY);
        ajouter(trainee, "transform.scale.x", 0.2, 1.0, ENTREE, VOL * 0.6, "both", SNAPPY);
        let _: () = msg_send![fusee, addSublayer: trainee];

        for (k, hex) in COULEURS.into_iter().enumerate() {
            let pastille: Id = msg_send![class!(CALayer), layer];
            let x = k as f64 * (PASTILLE.0 + ESPACE);
            let _: () = msg_send![pastille, setFrame: NSRect::new(NSPoint::new(x, (30.0 - PASTILLE.1) / 2.0), NSSize::new(PASTILLE.0, PASTILLE.1))];
            let _: () = msg_send![pastille, setCornerRadius: 3.0f64];
            let _: () = msg_send![pastille, setBackgroundColor: couleur(hex, 1.0)];
            // Entrée : les pastilles arrivent l'une après l'autre en grossissant.
            let retard = k as f64 * 0.03;
            ajouter(pastille, "opacity", 0.0, 1.0, retard, ENTREE - 0.06, "backwards", SNAPPY);
            ajouter(pastille, "transform.scale", 0.5, 1.0, retard, ENTREE - 0.06, "backwards", SNAPPY);
            // En vol, elles se resserrent vers l'avant (l'orange au centre reste en place).
            ajouter(pastille, "transform.translation.x", 0.0, (k as f64 - 2.0) * -3.0, ENTREE, VOL * 0.5, "both", SNAPPY);
            let _: () = msg_send![fusee, addSublayer: pastille];
        }
        // Élan : un petit recul, puis le vol jusqu'au-delà du bord droit, en accélérant.
        ajouter(fusee, "transform.translation.x", 0.0, -6.0, 0.08, ENTREE - 0.08, "backwards", SNAPPY);
        // Assez loin pour que la traînée (allongée par l'étirement) sorte aussi de la barre.
        ajouter(fusee, "transform.translation.x", -6.0, largeur - depart + long * 1.6, ENTREE, VOL, "both", DECOLLAGE);
        ajouter(fusee, "transform.scale.x", 1.0, 1.25, ENTREE, VOL, "both", DECOLLAGE);
        let _: () = msg_send![fond, addSublayer: fusee];

        let item = item_vue(&format!("{PREFIXE}.demarrage"), vue);
        let _: () = msg_send![vue, release];
        barre_touch(&format!("{PREFIXE}.demarrage"), vec![item])
    }
}

/// Anime les éléments de la barre montrée dans `modale`, de gauche à droite : ils apparaissent dans le sillage de la fusée.
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
        let ecart = ECART.min((RESTE - FONDU) / vues.len().max(1) as f64);
        for (k, vue) in vues.into_iter().enumerate() {
            let _: () = msg_send![vue, setWantsLayer: Bool::YES];
            let calque: Id = msg_send![vue, layer];
            if calque.is_null() {
                continue;
            }
            let retard = k as f64 * ecart;
            ajouter(calque, "opacity", 0.0, 1.0, retard, FONDU, "backwards", SNAPPY);
            ajouter(calque, "transform.translation.x", -16.0, 0.0, retard, FONDU, "backwards", SNAPPY);
        }
    }
}

/// Une animation de `de` à `a` sur `chemin`, qui commence dans `retard` secondes, sur la courbe de Bézier `courbe`.
#[allow(clippy::too_many_arguments)]
unsafe fn ajouter(calque: Id, chemin: &str, de: f64, a: f64, retard: f64, duree: f64, remplissage: &str, courbe: [f32; 4]) {
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
        let [x1, y1, x2, y2] = courbe;
        let fonction: Id = MessageReceiver::send_message(class!(CAMediaTimingFunction), sel!(functionWithControlPoints::::), (x1, y1, x2, y2));
        let _: () = msg_send![anim, setTimingFunction: fonction];
        let cle = format!("tactile-{chemin}-{retard}-{de}");
        let _: () = msg_send![calque, addAnimation: anim, forKey: &*texte(&cle)];
    }
}

/// Appelé `SPLASH` secondes après la présentation du ✳ : la vraie barre prend sa place.
pub(super) fn programmer_fin() {
    unsafe {
        let _: () = msg_send![cible(), performSelector: sel!(finDemarrage:), withObject: std::ptr::null_mut::<AnyObject>(), afterDelay: SPLASH];
    }
}
