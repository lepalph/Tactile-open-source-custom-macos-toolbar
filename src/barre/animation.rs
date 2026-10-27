//! GIF animés : un bouton dont l'image est un GIF de plusieurs images change d'image à la cadence du fichier.
//! Un `NSButton` n'anime pas ses images, et changer son image à chaque pas refait sa mise en page (2,5 % de CPU à 12 images/s, 1,2 % avec une `NSImageView`) :
//! le bouton garde une image vide de la bonne taille et une vue posée par-dessus change le contenu de son calque au minuteur
//! (0,6 %). Limite : un GIF en gabarit n'est pas teinté.
//! Le minuteur (durée d'image la plus courte) n'existe que tant qu'un bouton est animé ; à vide il s'arrête seul.

use objc2::runtime::{AnyObject, Bool};
use objc2::{class, msg_send, sel};
use objc2_foundation::{NSRect, NSSize};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use super::vues::Animation;
use super::{cible, Id};

struct Anime {
    bouton: Id,
    animation: Rc<Animation>,
    affichee: usize,
    /// La vue qui porte l'image, enfant du bouton, et les bornes du bouton pour lesquelles elle est placée.
    vue: Id,
    bornes: NSRect,
}

thread_local! {
    static ANIMES: RefCell<Vec<Anime>> = const { RefCell::new(Vec::new()) };
    static DEPART: RefCell<Option<Instant>> = const { RefCell::new(None) };
    static MINUTEUR: std::cell::Cell<Id> = const { std::cell::Cell::new(std::ptr::null_mut()) };
}

/// À appeler avant de reconstruire les boutons : les anciens vont disparaître.
pub(super) fn vider() {
    ANIMES.with_borrow_mut(Vec::clear);
}

/// Anime `bouton` avec `animation`, ou arrête de l'animer si `None` (un script qui change d'image).
pub(super) fn animer(bouton: Id, animation: Option<Rc<Animation>>) {
    ANIMES.with_borrow_mut(|liste| {
        liste.retain(|a| {
            let garder = a.bouton != bouton;
            if !garder {
                unsafe { let _: () = msg_send![a.vue, removeFromSuperview]; }
            }
            garder
        });
        if let Some(animation) = animation {
            let vue = unsafe {
                let taille: NSSize = msg_send![&*animation.cadres[0], size];
                let vide: Id = msg_send![class!(NSImage), alloc];
                let vide: Id = msg_send![vide, initWithSize: taille];
                let _: () = msg_send![bouton, setImage: vide];
                super::vues::lacher(vide);
                let vue: Id = msg_send![class!(NSView), alloc];
                let vue: Id = msg_send![vue, initWithFrame: NSRect::ZERO];
                let _: () = msg_send![vue, setWantsLayer: Bool::YES];
                let calque: Id = msg_send![vue, layer];
                let _: () = msg_send![calque, setContentsScale: 2.0f64];
                let _: () = msg_send![calque, setContents: cg(&animation.cadres[0])];
                let _: () = msg_send![bouton, addSubview: vue];
                super::vues::lacher(vue);
                vue
            };
            liste.push(Anime { bouton, animation, affichee: 0, vue, bornes: NSRect::ZERO });
        }
    });
    // Le minuteur bat à la plus courte durée d'image (20 ms au moins) : pas de réveil entre deux images.
    let pas = ANIMES.with_borrow(|l| {
        l.iter().flat_map(|a| a.animation.fins.iter().scan(0.0, |d, f| { let u = f - *d; *d = *f; Some(u) })).fold(f64::MAX, f64::min).max(0.02)
    });
    let ancien = MINUTEUR.replace(std::ptr::null_mut());
    if !ancien.is_null() {
        unsafe { let _: () = msg_send![ancien, invalidate]; }
    }
    if ANIMES.with_borrow(|l| l.is_empty()) {
        return;
    }
    if DEPART.with_borrow(|d| d.is_none()) {
        DEPART.with_borrow_mut(|d| *d = Some(Instant::now()));
    }
    let minuteur: Id = unsafe {
        msg_send![class!(NSTimer), scheduledTimerWithTimeInterval: pas, target: cible(), selector: sel!(anime:), userInfo: std::ptr::null_mut::<AnyObject>(), repeats: Bool::YES]
    };
    MINUTEUR.set(minuteur);
}

/// Un pas du minuteur : l'image de chaque bouton selon le temps écoulé.
pub(super) fn pas(minuteur: Id) {
    let t = DEPART.with_borrow(|d| d.map_or(0.0, |d| d.elapsed().as_secs_f64()));
    ANIMES.with_borrow_mut(|liste| {
        for a in liste.iter_mut() {
            unsafe {
                let bornes: NSRect = msg_send![a.bouton, bounds];
                if bornes != a.bornes {
                    a.bornes = bornes;
                    let cellule: Id = msg_send![a.bouton, cell];
                    let place: NSRect = msg_send![cellule, imageRectForBounds: bornes];
                    let _: () = msg_send![a.vue, setFrame: place];
                }
                let k = image_a(&a.animation.fins, t);
                if k != a.affichee {
                    a.affichee = k;
                    let calque: Id = msg_send![a.vue, layer];
                    let _: () = msg_send![calque, setContents: cg(&a.animation.cadres[k])];
                }
            }
        }
        if liste.is_empty() {
            unsafe { let _: () = msg_send![minuteur, invalidate]; }
            MINUTEUR.set(std::ptr::null_mut());
            DEPART.with_borrow_mut(|d| *d = None);
        }
    });
}

/// La `CGImage` d'une image réduite (un seul bitmap), posée telle quelle comme contenu du calque.
unsafe fn cg(image: &AnyObject) -> Id {
    unsafe { msg_send![image, CGImageForProposedRect: std::ptr::null_mut::<NSRect>(), context: std::ptr::null_mut::<AnyObject>(), hints: std::ptr::null_mut::<AnyObject>()] }
}

/// Indice de l'image affichée à l'instant `t` d'un GIF qui tourne en boucle (`fins` cumulées).
fn image_a(fins: &[f64], t: f64) -> usize {
    let total = fins.last().copied().unwrap_or(0.0);
    let t = if total > 0.0 { t % total } else { 0.0 };
    fins.iter().position(|&f| t < f).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_a_boucle() {
        let fins = [0.1, 0.3, 0.4];
        assert_eq!([0.0, 0.05, 0.1, 0.29, 0.35, 0.4, 0.55].map(|t| image_a(&fins, t)), [0, 0, 1, 1, 2, 0, 1]);
    }
}
