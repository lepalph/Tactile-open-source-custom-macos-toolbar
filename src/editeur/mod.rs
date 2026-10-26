//! La fenêtre « Personnaliser ma barre » : une WKWebView qui affiche `editeur.html` (embarqué),
//! reliée à Rust par un `WKScriptMessageHandler` (JS → Rust) et `evaluateJavaScript` (Rust → JS).
//!
//! JS → Rust, un objet JSON `{ "type": … }` : `pret`, `enregistrer { barre }`, `revenir`,
//! `prereglage { nom }`, `symboles { noms }`, `choisirApp`, `titre { texte }`.
//! Rust → JS : `tactile.charger(barre, aPrecedent)`, `tactile.glyphes({nom: dataURL|null})`,
//! `tactile.enregistre()`, `tactile.erreur(texte)`, `tactile.appChoisie(nom)`.

use crate::config::{self, Barre, Element};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool, NSObject};
use objc2::{class, define_class, msg_send, ClassType};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use serde_json::{json, Value};
use std::cell::Cell;
use std::path::{Path, PathBuf};

type Id = *mut AnyObject;

const PAGE: &str = include_str!("editeur.html");

thread_local! {
    static FENETRE: Cell<Id> = const { Cell::new(std::ptr::null_mut()) };
    static VUE: Cell<Id> = const { Cell::new(std::ptr::null_mut()) };
    /// Lancé avec `--personnaliser` : fermer la fenêtre quitte l'app.
    static SEUL: Cell<bool> = const { Cell::new(false) };
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "TactileEditeur"]
    struct Pont;

    impl Pont {
        #[unsafe(method(userContentController:didReceiveScriptMessage:))]
        fn message(&self, _controleur: Id, message: Id) {
            let corps: Id = unsafe { msg_send![message, body] };
            let texte = unsafe { (*(corps as *const NSString)).to_string() };
            match serde_json::from_str::<Value>(&texte) {
                Ok(v) => recevoir(v),
                Err(e) => eprintln!("tactile : message de l'éditeur illisible, {e}"),
            }
        }

        #[unsafe(method(windowWillClose:))]
        fn ferme(&self, _notification: Id) {
            if SEUL.get() {
                unsafe {
                    let app: Id = msg_send![class!(NSApplication), sharedApplication];
                    let _: () = msg_send![app, terminate: std::ptr::null_mut::<AnyObject>()];
                }
            }
        }
    }
);

fn texte(s: &str) -> Retained<NSString> {
    NSString::from_str(s)
}

fn recevoir(message: Value) {
    match message["type"].as_str().unwrap_or("") {
        "pret" => {
            envoyer_config(config::charger());
            // Pilotage sans les mains (captures, essais) : un script JS évalué une fois la page prête.
            if let Some(script) = std::env::var_os("TACTILE_EDITEUR_JS").and_then(|c| std::fs::read_to_string(c).ok()) {
                js(&script);
            }
        }
        "capture" => {
            if let Some(chemin) = message["chemin"].as_str()
                && let Err(e) = capturer(Path::new(chemin))
            {
                eprintln!("tactile : capture, {e}");
            }
        }
        "enregistrer" => match enregistrer(&config::chemin(), message["barre"].clone()) {
            Ok(()) => js("tactile.enregistre()"),
            Err(e) => js(&format!("tactile.erreur({})", json!(e))),
        },
        "revenir" => match revenir(&config::chemin()) {
            Ok(()) => envoyer_config(config::charger()),
            Err(e) => js(&format!("tactile.erreur({})", json!(e))),
        },
        "prereglage" => {
            let toml = match message["nom"].as_str().unwrap_or("") {
                "simple" => include_str!("../../exemples/simple.toml"),
                "developpeur" => include_str!("../../exemples/developpeur.toml"),
                _ => include_str!("../../exemples/barre.toml"),
            };
            // Une barre toute faite remplace la principale ; les barres par application de l'utilisateur restent.
            let applications = config::charger().map(|b| b.applications).unwrap_or_default();
            match config::lire(toml).map(|b| Barre { applications, ..b }) {
                Ok(barre) => js(&format!("tactile.charger({}, {})", json!(barre), precedent(&config::chemin()).exists())),
                Err(e) => js(&format!("tactile.erreur({})", json!(e))),
            }
        }
        "symboles" => {
            let noms = message["noms"].as_array().cloned().unwrap_or_default();
            let glyphes: serde_json::Map<String, Value> = noms
                .iter()
                .filter_map(Value::as_str)
                .map(|n| (n.to_string(), glyphe(n).map_or(Value::Null, Value::String)))
                .collect();
            js(&format!("tactile.glyphes({})", Value::Object(glyphes)));
        }
        "choisirApp" => {
            if let Some(nom) = choisir_app() {
                js(&format!("tactile.appChoisie({})", json!(nom)));
            }
        }
        "journal" => eprintln!("tactile : éditeur, {}", message["texte"].as_str().unwrap_or("")),
        "titre" => {
            if let Some(t) = message["texte"].as_str() {
                unsafe { let _: () = msg_send![FENETRE.get(), setTitle: &*texte(t)]; }
            }
        }
        autre => eprintln!("tactile : message inconnu de l'éditeur « {autre} »"),
    }
}

fn envoyer_config(config: Result<Barre, String>) {
    match config {
        Ok(barre) => js(&format!("tactile.charger({}, {})", json!(barre), precedent(&config::chemin()).exists())),
        Err(e) => js(&format!("tactile.erreur({})", json!(e))),
    }
}

/// Rust → JS. `serde_json` produit un littéral JavaScript valide.
fn js(code: &str) {
    let vue = VUE.get();
    if vue.is_null() {
        return;
    }
    unsafe { let _: () = msg_send![vue, evaluateJavaScript: &*texte(code), completionHandler: std::ptr::null_mut::<AnyObject>()]; }
}

fn precedent(chemin: &Path) -> PathBuf {
    chemin.with_extension("toml.precedent")
}

/// JSON de l'éditeur → `Barre` → TOML, relu et vérifié, puis écrit ; l'ancien fichier devient `.precedent`.
fn enregistrer(chemin: &Path, barre: Value) -> Result<(), String> {
    let barre: Barre = serde_json::from_value(barre).map_err(|e| format!("barre illisible : {e}"))?;
    let toml = config::ecrire(&barre)?;
    let relue = config::lire(&toml)?;
    verifier(&relue.elements)?;
    for app in &relue.applications {
        verifier(&app.elements).map_err(|e| format!("application « {} », {e}", app.app))?;
    }
    if let Some(dossier) = chemin.parent() {
        std::fs::create_dir_all(dossier).map_err(|e| format!("impossible de créer {} : {e}", dossier.display()))?;
    }
    if chemin.exists() {
        std::fs::copy(chemin, precedent(chemin)).map_err(|e| format!("copie de secours : {e}"))?;
    }
    std::fs::write(chemin, toml).map_err(|e| format!("impossible d'écrire {} : {e}", chemin.display()))
}

/// Échange le fichier et sa copie `.precedent` : revenir deux fois revient au point de départ.
fn revenir(chemin: &Path) -> Result<(), String> {
    let secours = precedent(chemin);
    let ancien = std::fs::read_to_string(&secours).map_err(|_| "pas de version précédente".to_string())?;
    config::lire(&ancien)?;
    if chemin.exists() {
        std::fs::copy(chemin, &secours).map_err(|e| format!("copie de secours : {e}"))?;
    }
    std::fs::write(chemin, ancien).map_err(|e| format!("impossible d'écrire {} : {e}", chemin.display()))
}

/// Ce que `config::lire` accepte mais que la barre ne saurait pas afficher.
fn verifier(elements: &[Element]) -> Result<(), String> {
    for (i, element) in elements.iter().enumerate() {
        let n = i + 1;
        let symbole = match element {
            Element::Bouton { symbole, .. } | Element::Slider { symbole, .. } | Element::Groupe { symbole, .. } => symbole.as_deref(),
            _ => None,
        };
        if let Some(s) = symbole.filter(|s| !s.is_empty() && glyphe(s).is_none()) {
            return Err(format!("élément {n} : le symbole « {s} » n'existe pas"));
        }
        match element {
            Element::Bouton { couleur: Some(c), .. } | Element::Bouton { couleur_texte: Some(c), .. } if !couleur_valide(c) => {
                return Err(format!("élément {n} : la couleur « {c} » doit s'écrire #RRGGBB"));
            }
            Element::Script { intervalle, .. } if *intervalle < 0.1 => {
                return Err(format!("élément {n} : l'intervalle doit être d'au moins 0,1 seconde"));
            }
            Element::Groupe { elements, .. } => verifier(elements).map_err(|e| format!("groupe {n}, {e}"))?,
            _ => {}
        }
    }
    Ok(())
}

fn couleur_valide(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

/// Le symbole SF rendu par AppKit en PNG (data URL), à 2× pour un affichage à 17 pt. `None` s'il n'existe pas.
fn glyphe(nom: &str) -> Option<String> {
    unsafe {
        let image: Id = msg_send![class!(NSImage), imageWithSystemSymbolName: &*texte(nom), accessibilityDescription: std::ptr::null_mut::<AnyObject>()];
        if image.is_null() {
            return None;
        }
        let configuration: Id = msg_send![class!(NSImageSymbolConfiguration), configurationWithPointSize: 34.0f64, weight: 0.23f64, scale: 2isize];
        let image: Id = msg_send![image, imageWithSymbolConfiguration: configuration];
        let tiff: Id = msg_send![image, TIFFRepresentation];
        let bitmap: Id = msg_send![class!(NSBitmapImageRep), imageRepWithData: tiff];
        let proprietes: Id = msg_send![class!(NSDictionary), dictionary];
        let png: Id = msg_send![bitmap, representationUsingType: 4usize, properties: proprietes];
        if png.is_null() {
            return None;
        }
        let base64: Id = msg_send![png, base64EncodedStringWithOptions: 0usize];
        Some(format!("data:image/png;base64,{}", *(base64 as *const NSString)))
    }
}

/// Image PNG de la fenêtre, prise par l'app elle-même : `screencapture` demande l'autorisation
/// « Enregistrement de l'écran », pas la capture de ses propres fenêtres.
fn capturer(chemin: &Path) -> Result<(), String> {
    #[repr(C)]
    struct CGRect([f64; 4]);
    unsafe extern "C" {
        fn CGWindowListCreateImage(rect: CGRect, option: u32, fenetre: u32, image_option: u32) -> Id;
        fn CGImageRelease(image: Id);
    }
    unsafe {
        let numero: isize = msg_send![FENETRE.get(), windowNumber];
        // Rect nul = la fenêtre entière ; 8 = seulement cette fenêtre ; 1 = sans l'ombre.
        let image = CGWindowListCreateImage(CGRect([f64::INFINITY, f64::INFINITY, 0.0, 0.0]), 8, numero as u32, 1);
        if image.is_null() {
            return Err("CGWindowListCreateImage a renvoyé nul".into());
        }
        let bitmap: Id = msg_send![class!(NSBitmapImageRep), alloc];
        let bitmap: Id = msg_send![bitmap, initWithCGImage: image];
        CGImageRelease(image);
        let proprietes: Id = msg_send![class!(NSDictionary), dictionary];
        let png: Id = msg_send![bitmap, representationUsingType: 4usize, properties: proprietes];
        let ok: Bool = msg_send![png, writeToFile: &*texte(&chemin.to_string_lossy()), atomically: Bool::YES];
        if ok.as_bool() { Ok(()) } else { Err(format!("impossible d'écrire {}", chemin.display())) }
    }
}

/// NSOpenPanel sur /Applications ; renvoie le nom de l'app choisie, sans « .app ».
fn choisir_app() -> Option<String> {
    unsafe {
        let panneau: Id = msg_send![class!(NSOpenPanel), openPanel];
        let _: () = msg_send![panneau, setCanChooseDirectories: Bool::NO];
        let _: () = msg_send![panneau, setAllowsMultipleSelection: Bool::NO];
        let _: () = msg_send![panneau, setTreatsFilePackagesAsDirectories: Bool::NO];
        let dossier: Id = msg_send![class!(NSURL), fileURLWithPath: &*texte("/Applications")];
        let _: () = msg_send![panneau, setDirectoryURL: dossier];
        let types: Id = msg_send![class!(NSArray), arrayWithObject: &*texte("app")];
        let _: () = msg_send![panneau, setAllowedFileTypes: types];
        let reponse: isize = msg_send![panneau, runModal];
        if reponse != 1 {
            return None;
        }
        let url: Id = msg_send![panneau, URL];
        let chemin: Id = msg_send![url, path];
        let chemin = (*(chemin as *const NSString)).to_string();
        Path::new(&chemin).file_stem().map(|n| n.to_string_lossy().into_owned())
    }
}

/// Ouvre la fenêtre (une seule ; la ramène devant si elle existe déjà).
pub fn ouvrir() {
    unsafe {
        let app: Id = msg_send![class!(NSApplication), sharedApplication];
        if FENETRE.get().is_null() {
            let pont = Retained::into_raw(msg_send![Pont::class(), new]) as Id;
            let configuration: Id = msg_send![class!(WKWebViewConfiguration), new];
            let controleur: Id = msg_send![configuration, userContentController];
            let _: () = msg_send![controleur, addScriptMessageHandler: pont, name: &*texte("tactile")];
            let cadre = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1160.0, 720.0));
            let vue: Id = msg_send![class!(WKWebView), alloc];
            let vue: Id = msg_send![vue, initWithFrame: cadre, configuration: configuration];
            // Une origine `file:` plutôt qu'opaque : localStorage (langue choisie) y est permis.
            let base: Id = msg_send![class!(NSURL), fileURLWithPath: &*texte("/")];
            let _: Id = msg_send![vue, loadHTMLString: &*texte(PAGE), baseURL: base];

            // 15 = titre + fermer + réduire + redimensionner.
            let fenetre: Id = msg_send![class!(NSWindow), alloc];
            let fenetre: Id = msg_send![fenetre, initWithContentRect: cadre, styleMask: 15usize, backing: 2usize, defer: Bool::NO];
            let _: () = msg_send![fenetre, setReleasedWhenClosed: Bool::NO];
            let _: () = msg_send![fenetre, setTitle: &*texte("Personnaliser ma barre")];
            let _: () = msg_send![fenetre, setContentMinSize: NSSize::new(1060.0, 600.0)];
            let _: () = msg_send![fenetre, setContentView: vue];
            let _: () = msg_send![fenetre, setDelegate: pont];
            let _: () = msg_send![fenetre, center];
            FENETRE.set(fenetre);
            VUE.set(vue);
        }
        let _: () = msg_send![FENETRE.get(), makeKeyAndOrderFront: std::ptr::null_mut::<AnyObject>()];
        let _: Bool = msg_send![app, activateIgnoringOtherApps: Bool::YES];
    }
}

/// `tactile --personnaliser` : la fenêtre seule, sans Touch Bar ni icône de menu.
pub fn lancer_seul() {
    SEUL.set(true);
    unsafe {
        let app: Id = msg_send![class!(NSApplication), sharedApplication];
        let _: Bool = msg_send![app, setActivationPolicy: 0isize];
        ouvrir();
        let _: () = msg_send![app, run];
    }
}

#[link(name = "WebKit", kind = "framework")]
unsafe extern "C" {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un dossier par test : ils tournent en parallèle dans le même processus.
    fn dossier_essai(nom: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tactile-editeur-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn enregistrer_n_ecrit_que_le_fichier_indique_et_sa_copie() {
        let dossier = dossier_essai("ecrit");
        let chemin = dossier.join("sous/barre.toml");
        let barre = json!({ "masquer_control_strip": true, "element": [{ "type": "heure", "format": "HH:mm" }] });
        enregistrer(&chemin, barre.clone()).unwrap();
        let noms = |d: &Path| {
            let mut n: Vec<String> = std::fs::read_dir(d).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
            n.sort();
            n
        };
        assert_eq!(noms(&dossier.join("sous")), ["barre.toml"]);

        let ecrit = std::fs::read_to_string(&chemin).unwrap();
        assert!(ecrit.starts_with("# Ta barre Tactile"), "{ecrit}");
        assert_eq!(config::lire(&ecrit).unwrap().elements, [Element::Heure { format: Some("HH:mm".into()) }]);

        enregistrer(&chemin, json!({ "element": [] })).unwrap();
        assert_eq!(noms(&dossier.join("sous")), ["barre.toml", "barre.toml.precedent"]);
        assert_eq!(config::lire(&std::fs::read_to_string(precedent(&chemin)).unwrap()).unwrap().elements.len(), 1);

        revenir(&chemin).unwrap();
        assert_eq!(config::lire(&std::fs::read_to_string(&chemin).unwrap()).unwrap().elements.len(), 1);
        assert!(config::lire(&std::fs::read_to_string(precedent(&chemin)).unwrap()).unwrap().elements.is_empty());
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    #[test]
    fn une_barre_invalide_n_est_pas_ecrite() {
        let dossier = dossier_essai("invalide");
        let chemin = dossier.join("barre.toml");
        let erreur = enregistrer(&chemin, json!({ "element": [{ "type": "bouton", "couleur": "violet" }] })).unwrap_err();
        assert!(erreur.contains("#RRGGBB"), "{erreur}");
        let erreur = enregistrer(&chemin, json!({ "element": [{ "type": "script", "commande": "date", "intervalle": 0 }] })).unwrap_err();
        assert!(erreur.contains("0,1"), "{erreur}");
        let erreur = enregistrer(&chemin, json!({ "element": [{ "type": "bouton", "truc": 1 }] })).unwrap_err();
        assert!(erreur.contains("truc"), "{erreur}");
        assert!(!chemin.exists());
        assert!(!dossier.exists() || std::fs::read_dir(&dossier).unwrap().next().is_none());
        let _ = std::fs::remove_dir_all(&dossier);
    }

    #[test]
    fn un_symbole_inconnu_est_refuse() {
        if std::env::var("CI").is_ok() {
            return;
        }
        assert!(glyphe("play.fill").is_some_and(|g| g.starts_with("data:image/png;base64,")));
        assert!(glyphe("truc.machin").is_none());
        let barre = config::lire("[[element]]\ntype = \"bouton\"\nsymbole = \"truc.machin\"\n").unwrap();
        let erreur = verifier(&barre.elements).unwrap_err();
        assert!(erreur.contains("truc.machin"), "{erreur}");
    }

    /// Ce que l'éditeur ne sait pas régler (image, keycode, [[application]]…) doit traverser JSON → Barre → TOML sans se perdre.
    #[test]
    fn le_json_ne_perd_aucun_champ() {
        let barre = config::lire(config::BARRE_COMPLETE).unwrap();
        let json = serde_json::to_value(&barre).unwrap();
        assert_eq!(serde_json::from_value::<Barre>(json.clone()).unwrap(), barre);
        let dossier = dossier_essai("complet");
        let chemin = dossier.join("barre.toml");
        assert!(json["application"][0]["herite"].as_bool().unwrap(), "{json}");
        enregistrer(&chemin, json).unwrap();
        assert_eq!(config::lire(&std::fs::read_to_string(&chemin).unwrap()).unwrap(), barre);
        // Une barre d'application aussi est vérifiée avant l'écriture.
        let erreur = enregistrer(&chemin, json!({ "application": [{ "app": "Safari", "element": [{ "type": "bouton", "couleur": "bleu" }] }] })).unwrap_err();
        assert!(erreur.starts_with("application « Safari », élément 1"), "{erreur}");
        std::fs::remove_dir_all(&dossier).unwrap();
    }
}
