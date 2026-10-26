//! L'icône de Tactile dans la barre des menus et son menu.

use crate::{barre, config, editeur};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool, NSObject, Sel};
use objc2::{class, define_class, msg_send, sel, ClassType};
use objc2_foundation::NSString;

type Id = *mut AnyObject;

const DEMARRAGE: isize = 1;

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "TactileMenu"]
    struct Menu;

    impl Menu {
        #[unsafe(method(personnaliser:))]
        fn personnaliser(&self, _envoyeur: Id) {
            editeur::ouvrir();
        }

        #[unsafe(method(afficher:))]
        fn afficher(&self, _envoyeur: Id) {
            barre::afficher();
        }

        #[unsafe(method(masquer:))]
        fn masquer(&self, _envoyeur: Id) {
            barre::masquer();
        }

        #[unsafe(method(ouvrirConfiguration:))]
        fn ouvrir_configuration(&self, _envoyeur: Id) {
            if let Err(e) = ouvrir_config() {
                eprintln!("tactile : {e}");
            }
        }

        #[unsafe(method(basculerDemarrage:))]
        fn basculer_demarrage(&self, _envoyeur: Id) {
            if let Err(e) = basculer_service() {
                eprintln!("tactile : lancement au démarrage, {e}");
            }
        }

        #[unsafe(method(menuWillOpen:))]
        fn menu_will_open(&self, menu: Id) {
            let item: Id = unsafe { msg_send![menu, itemWithTag: DEMARRAGE] };
            if dans_bundle() && !item.is_null() {
                let etat = (statut() == 1) as isize;
                unsafe { let _: () = msg_send![item, setState: etat]; }
            }
        }
    }
);

fn texte(s: &str) -> Retained<NSString> {
    NSString::from_str(s)
}

fn dans_bundle() -> bool {
    unsafe {
        let bundle: Id = msg_send![class!(NSBundle), mainBundle];
        let chemin: Id = msg_send![bundle, bundlePath];
        (*(chemin as *const NSString)).to_string().ends_with(".app")
    }
}

fn service() -> Id {
    unsafe { msg_send![class!(SMAppService), mainAppService] }
}

/// 0 non inscrit, 1 activé, 2 en attente d'accord dans Réglages, 3 introuvable.
fn statut() -> isize {
    unsafe { msg_send![service(), status] }
}

fn basculer_service() -> Result<(), String> {
    let mut erreur: Id = std::ptr::null_mut();
    let ok: Bool = unsafe {
        if statut() == 1 {
            msg_send![service(), unregisterAndReturnError: &mut erreur as *mut Id]
        } else {
            msg_send![service(), registerAndReturnError: &mut erreur as *mut Id]
        }
    };
    if statut() == 2 {
        unsafe { let _: () = msg_send![class!(SMAppService), openSystemSettingsLoginItems]; }
    }
    if ok.as_bool() {
        return Ok(());
    }
    let description: Id = unsafe { msg_send![erreur, localizedDescription] };
    Err(if description.is_null() { "échec".into() } else { unsafe { (*(description as *const NSString)).to_string() } })
}

fn ouvrir_config() -> Result<(), String> {
    let chemin = config::chemin();
    if !chemin.exists() {
        let dossier = chemin.parent().unwrap();
        std::fs::create_dir_all(dossier).map_err(|e| format!("impossible de créer {} : {e}", dossier.display()))?;
        std::fs::write(&chemin, include_str!("../exemples/barre.toml")).map_err(|e| format!("impossible d'écrire {} : {e}", chemin.display()))?;
    }
    std::thread::spawn(move || {
        let ouvert = std::process::Command::new("/usr/bin/open").arg(&chemin).status().is_ok_and(|s| s.success());
        // Sans application associée aux .toml, on se rabat sur l'éditeur de texte par défaut.
        if !ouvert {
            let _ = std::process::Command::new("/usr/bin/open").arg("-t").arg(&chemin).status();
        }
    });
    Ok(())
}

fn ajouter(menu: Id, titre: &str, cible: Id, action: Option<Sel>) -> Id {
    unsafe {
        let item: Id = msg_send![class!(NSMenuItem), alloc];
        let item: Id = msg_send![item, initWithTitle: &*texte(titre), action: action, keyEquivalent: &*texte("")];
        let _: () = msg_send![item, setTarget: cible];
        let _: () = msg_send![menu, addItem: item];
        item
    }
}

fn separateur(menu: Id) {
    unsafe {
        let s: Id = msg_send![class!(NSMenuItem), separatorItem];
        let _: () = msg_send![menu, addItem: s];
    }
}

pub fn installer() {
    // La cible vit aussi longtemps que l'app : NSMenuItem ne la retient pas.
    let cible = Retained::into_raw(unsafe { msg_send![Menu::class(), new] }) as Id;
    unsafe {
        let menu: Id = msg_send![class!(NSMenu), new];
        let _: () = msg_send![menu, setDelegate: cible];
        ajouter(menu, "Personnaliser ma barre…", cible, Some(sel!(personnaliser:)));
        separateur(menu);
        ajouter(menu, "Afficher ma barre", cible, Some(sel!(afficher:)));
        ajouter(menu, "Barre d'Apple", cible, Some(sel!(masquer:)));
        separateur(menu);
        ajouter(menu, "Ouvrir la configuration", cible, Some(sel!(ouvrirConfiguration:)));
        let demarrage = if dans_bundle() {
            ajouter(menu, "Lancer au démarrage", cible, Some(sel!(basculerDemarrage:)))
        } else {
            ajouter(menu, "Lancer au démarrage (depuis Tactile.app seulement)", cible, None)
        };
        let _: () = msg_send![demarrage, setTag: DEMARRAGE];        separateur(menu);
        let app: Id = msg_send![class!(NSApplication), sharedApplication];
        ajouter(menu, "Quitter Tactile", app, Some(sel!(terminate:)));

        let barre_systeme: Id = msg_send![class!(NSStatusBar), systemStatusBar];
        let item: Id = msg_send![barre_systeme, statusItemWithLength: -1.0f64];
        let _: Id = msg_send![item, retain];
        let bouton: Id = msg_send![item, button];
        let image: Id = msg_send![class!(NSImage), imageWithSystemSymbolName: &*texte("rectangle.inset.filled"), accessibilityDescription: &*texte("Tactile")];
        if image.is_null() {
            let _: () = msg_send![bouton, setTitle: &*texte("✳")];
        } else {
            let _: () = msg_send![bouton, setImage: image];
        }
        let _: () = msg_send![item, setMenu: menu];
    }
}

#[link(name = "ServiceManagement", kind = "framework")]
unsafe extern "C" {}
