mod actions;
mod config;

use objc2::runtime::AnyObject;
use objc2::{class, msg_send, sel};
use objc2_foundation::NSString;

fn main() {
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let _: objc2::runtime::Bool = msg_send![app, setActivationPolicy: 1isize];

        let ident = NSString::from_str("fr.palph.tactile.essai");
        let titre = NSString::from_str("✳ tactile");
        let bouton: *mut AnyObject = msg_send![class!(NSButton), buttonWithTitle: &*titre, target: std::ptr::null_mut::<AnyObject>(), action: sel!(description)];
        let item: *mut AnyObject = msg_send![class!(NSCustomTouchBarItem), alloc];
        let item: *mut AnyObject = msg_send![item, initWithIdentifier: &*ident];
        let _: () = msg_send![item, setView: bouton];

        let barre: *mut AnyObject = msg_send![class!(NSTouchBar), new];
        let idents: *mut AnyObject = msg_send![class!(NSArray), arrayWithObject: &*ident];
        let _: () = msg_send![barre, setDefaultItemIdentifiers: idents];
        let items: *mut AnyObject = msg_send![class!(NSSet), setWithObject: item];
        let _: () = msg_send![barre, setTemplateItems: items];

        let plateau = NSString::from_str("fr.palph.tactile.plateau");
        let _: () = msg_send![class!(NSTouchBar), presentSystemModalTouchBar: barre, placement: 1i64, systemTrayItemIdentifier: &*plateau];

        let _: () = msg_send![app, run];
    }
}

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}
