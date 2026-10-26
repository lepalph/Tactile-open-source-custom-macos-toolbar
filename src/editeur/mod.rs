//! La fenêtre « Personnaliser ma barre » : une WKWebView qui affiche `editeur.html` (embarqué),
//! reliée à Rust par un `WKScriptMessageHandler` (JS → Rust) et `evaluateJavaScript` (Rust → JS).
//!
//! JS → Rust, un objet JSON `{ "type": … }` : `pret`, `enregistrer { barre, fermer? }`, `revenir`,
//! `prereglage { nom }`, `symboles { noms }`, `choisirApp`, `verifierApp { nom }`, `modifie { valeur }`,
//! `fermer`, `ouvrirFichier`, `relire`, `titre { texte }`.
//! Rust → JS : `tactile.charger(barre, aPrecedent)`, `tactile.illisible(message, extrait, aPrecedent)`,
//! `tactile.glyphes({nom: dataURL|null})`, `tactile.enregistre(manuel, aPrecedent)`, `tactile.revenu()`,
//! `tactile.abimeGarde(nom)`, `tactile.erreur(texte)`, `tactile.appChoisie(nom)`,
//! `tactile.appVerifiee(nom, trouvee, suggestion)`, `tactile.demanderFermeture()`.

use crate::config::{self, Barre, Element};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool, NSObject};
use objc2::{class, define_class, msg_send, ClassType};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use serde_json::{json, Value};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::process::Command;

type Id = *mut AnyObject;

const PAGE: &str = include_str!("editeur.html");

thread_local! {
    static FENETRE: Cell<Id> = const { Cell::new(std::ptr::null_mut()) };
    static VUE: Cell<Id> = const { Cell::new(std::ptr::null_mut()) };
    /// Lancé avec `--personnaliser` : fermer la fenêtre quitte l'app.
    static SEUL: Cell<bool> = const { Cell::new(false) };
    /// Des changements non enregistrés dans la page (message `modifie`) : fermer demande confirmation.
    static MODIFIE: Cell<bool> = const { Cell::new(false) };
}

/// Versions gardées : `barre.toml.precedent` (la plus récente), puis `.precedent-2` … `.precedent-5`.
const VERSIONS: usize = 5;

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

        /// Avec des changements non enregistrés, la page pose la question ; elle répond `fermer` ou `enregistrer { fermer }`.
        #[unsafe(method(windowShouldClose:))]
        fn doit_fermer(&self, _fenetre: Id) -> Bool {
            if !MODIFIE.get() {
                return Bool::YES;
            }
            js("tactile.demanderFermeture()");
            Bool::NO
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
        "enregistrer" => {
            let chemin = config::chemin();
            match enregistrer(&chemin, message["barre"].clone()) {
                Ok(manuel) => {
                    js(&format!("tactile.enregistre({}, {})", json!(manuel), version(&chemin, 1).exists()));
                    if message["fermer"].as_bool() == Some(true) {
                        fermer();
                    }
                }
                Err(e) => js(&format!("tactile.erreur({})", json!(e))),
            }
        }
        "revenir" => match revenir(&config::chemin()) {
            Ok(()) => {
                envoyer_config(config::charger());
                js("tactile.revenu()");
            }
            Err(e) => js(&format!("tactile.erreur({})", json!(e))),
        },
        "relire" => envoyer_config(config::charger()),
        "ouvrirFichier" => {
            // -t : l'éditeur de texte par défaut, même si .toml est associé à une autre app.
            if let Err(e) = Command::new("/usr/bin/open").arg("-t").arg(config::chemin()).status() {
                js(&format!("tactile.erreur({})", json!(format!("impossible d'ouvrir le fichier : {e}"))));
            }
        }
        "modifie" => {
            let valeur = message["valeur"].as_bool().unwrap_or(false);
            MODIFIE.set(valeur);
            // Le point dans le bouton rouge, comme pour tout document Mac non enregistré.
            unsafe { let _: () = msg_send![FENETRE.get(), setDocumentEdited: Bool::new(valeur)]; }
        }
        "fermer" => fermer(),
        "verifierApp" => {
            let nom = message["nom"].as_str().unwrap_or("");
            let (trouvee, proche) = chercher_app(nom);
            js(&format!("tactile.appVerifiee({}, {trouvee}, {})", json!(nom), json!(proche)));
        }
        "prereglage" => {
            // Fichier illisible : il est mis de côté avant que la barre toute faite puisse le remplacer.
            let chemin = config::chemin();
            let mut garde = None;
            if chemin.exists() && config::charger().is_err() {
                match mettre_de_cote(&chemin) {
                    Ok(copie) => garde = copie.file_name().map(|n| n.to_string_lossy().into_owned()),
                    Err(e) => return js(&format!("tactile.erreur({})", json!(e))),
                }
            }
            let toml = match message["nom"].as_str().unwrap_or("") {
                "simple" => include_str!("../../exemples/simple.toml"),
                "developpeur" => include_str!("../../exemples/developpeur.toml"),
                _ => include_str!("../../exemples/barre.toml"),
            };
            // Une barre toute faite remplace la principale ; les barres par application de l'utilisateur restent.
            let applications = config::charger().map(|b| b.applications).unwrap_or_default();
            match config::lire(toml).map(|b| Barre { applications, ..b }) {
                Ok(barre) => {
                    js(&format!("tactile.charger({}, {})", json!(barre), version(&chemin, 1).exists()));
                    if let Some(nom) = garde {
                        js(&format!("tactile.abimeGarde({})", json!(nom)));
                    }
                }
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

/// La barre du fichier ; s'il est illisible, l'erreur seule : la page ne montre pas une barre vide à enregistrer.
fn envoyer_config(config: Result<Barre, String>) {
    let chemin = config::chemin();
    match config {
        Ok(barre) => js(&format!("tactile.charger({}, {})", json!(barre), version(&chemin, 1).exists())),
        Err(e) => {
            // Le chemin complet n'aide pas une débutante : « ligne 6 : … » suffit, avec la ligne citée.
            let message = e.strip_prefix(&format!("{}, ", chemin.display())).unwrap_or(&e).to_string();
            let extrait = numero_de_ligne(&message)
                .and_then(|n| std::fs::read_to_string(&chemin).ok()?.lines().nth(n.checked_sub(1)?).map(str::to_string));
            js(&format!("tactile.illisible({}, {}, {})", json!(message), json!(extrait), version(&chemin, 1).exists()));
        }
    }
}

/// « ligne 6 : … » → 6.
fn numero_de_ligne(message: &str) -> Option<usize> {
    let apres = &message[message.find("ligne ")? + "ligne ".len()..];
    apres.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
}

fn fermer() {
    MODIFIE.set(false);
    unsafe { let _: () = msg_send![FENETRE.get(), close]; }
}

/// Rust → JS. `serde_json` produit un littéral JavaScript valide.
fn js(code: &str) {
    let vue = VUE.get();
    if vue.is_null() {
        return;
    }
    unsafe { let _: () = msg_send![vue, evaluateJavaScript: &*texte(code), completionHandler: std::ptr::null_mut::<AnyObject>()]; }
}

/// `barre.toml` + `.precedent` → `barre.toml.precedent`.
fn voisin(chemin: &Path, suffixe: &str) -> PathBuf {
    let mut nom = chemin.as_os_str().to_owned();
    nom.push(suffixe);
    nom.into()
}

/// Version `n` de l'historique : 1 = `.precedent`, puis `.precedent-2` … `.precedent-5`.
fn version(chemin: &Path, n: usize) -> PathBuf {
    voisin(chemin, &if n == 1 { ".precedent".to_string() } else { format!(".precedent-{n}") })
}

/// JSON de l'éditeur → `Barre` → TOML, relu et vérifié, puis écrit. L'ancien fichier devient `.precedent`
/// (les 5 dernières versions sont gardées) et, s'il avait des commentaires écrits à la main, une copie
/// `.manuel` est faite une fois pour toutes. Renvoie le nom de cette copie quand elle vient d'être créée.
fn enregistrer(chemin: &Path, barre: Value) -> Result<Option<String>, String> {
    let barre: Barre = serde_json::from_value(barre).map_err(|e| format!("barre illisible : {e}"))?;
    // Défense en profondeur : une barre vide éteindrait la Touch Bar (cf. SYNTHESE E1).
    if barre.elements.is_empty() && barre.applications.is_empty() {
        return Err("la barre est vide : ajoute au moins un élément".into());
    }
    let toml = config::ecrire(&barre)?;
    let relue = config::lire(&toml)?;
    verifier(&relue.elements)?;
    for app in &relue.applications {
        verifier(&app.elements).map_err(|e| format!("application « {} », {e}", app.app))?;
    }
    if let Some(dossier) = chemin.parent() {
        std::fs::create_dir_all(dossier).map_err(|e| format!("impossible de créer {} : {e}", dossier.display()))?;
    }
    let mut manuel = None;
    if chemin.exists() {
        let copie = voisin(chemin, ".manuel");
        let actuel = std::fs::read_to_string(chemin).unwrap_or_default();
        if !copie.exists() && ecrit_a_la_main(&actuel) {
            std::fs::copy(chemin, &copie).map_err(|e| format!("copie de tes commentaires : {e}"))?;
            manuel = copie.file_name().map(|n| n.to_string_lossy().into_owned());
        }
        for n in (1..VERSIONS).rev() {
            if version(chemin, n).exists() {
                std::fs::rename(version(chemin, n), version(chemin, n + 1)).map_err(|e| format!("historique : {e}"))?;
            }
        }
        std::fs::copy(chemin, version(chemin, 1)).map_err(|e| format!("copie de secours : {e}"))?;
    }
    std::fs::write(chemin, toml).map_err(|e| format!("impossible d'écrire {} : {e}", chemin.display()))?;
    Ok(manuel)
}

/// Remonte d'un cran dans l'historique : `.precedent` devient le fichier, chaque version remonte,
/// et le fichier quitté passe en dernier. Rien n'est perdu : après la plus ancienne, on retombe sur la plus récente.
fn revenir(chemin: &Path) -> Result<(), String> {
    let precedente = std::fs::read_to_string(version(chemin, 1)).map_err(|_| "pas de version précédente".to_string())?;
    config::lire(&precedente).map_err(|e| format!("la version précédente est illisible, {e}"))?;
    if !chemin.exists() {
        return std::fs::write(chemin, precedente).map_err(|e| format!("impossible d'écrire {} : {e}", chemin.display()));
    }
    let mut chaine = vec![chemin.to_path_buf()];
    chaine.extend((1..=VERSIONS).map(|n| version(chemin, n)).take_while(|v| v.exists()));
    let textes = chaine.iter().map(std::fs::read).collect::<Result<Vec<_>, _>>().map_err(|e| format!("historique : {e}"))?;
    // Le fichier lui-même en dernier : c'est son changement qui recharge la barre.
    for (i, cible) in chaine.iter().enumerate().rev() {
        std::fs::write(cible, &textes[(i + 1) % textes.len()]).map_err(|e| format!("impossible d'écrire {} : {e}", cible.display()))?;
    }
    Ok(())
}

/// Copie le fichier illisible en `barre.toml.abime-AAAAMMJJ-HHMMSS`, sauf si une copie identique existe déjà.
fn mettre_de_cote(chemin: &Path) -> Result<PathBuf, String> {
    let contenu = std::fs::read(chemin).map_err(|e| format!("impossible de lire {} : {e}", chemin.display()))?;
    let prefixe = voisin(chemin, ".abime-");
    let prefixe = prefixe.file_name().unwrap_or_default().to_string_lossy();
    if let Some(dossier) = chemin.parent()
        && let Ok(entrees) = std::fs::read_dir(dossier)
        && let Some(deja) = entrees.flatten().map(|e| e.path()).find(|p| {
            p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(&*prefixe)) && std::fs::read(p).is_ok_and(|c| c == contenu)
        })
    {
        return Ok(deja);
    }
    let date = Command::new("/bin/date").arg("+%Y%m%d-%H%M%S").output().map_err(|e| format!("date : {e}"))?;
    let copie = voisin(chemin, &format!(".abime-{}", String::from_utf8_lossy(&date.stdout).trim()));
    std::fs::write(&copie, contenu).map_err(|e| format!("impossible de mettre ton fichier de côté : {e}"))?;
    Ok(copie)
}

/// Le fichier a-t-il des commentaires autres que l'en-tête que l'éditeur écrit lui-même ?
fn ecrit_a_la_main(texte: &str) -> bool {
    let barre = Barre { masquer_control_strip: true, elements: vec![Element::Souple], applications: vec![] };
    let entete = config::ecrire(&barre).unwrap_or_default();
    let entete: Vec<&str> = entete.lines().filter(|l| l.starts_with('#')).collect();
    texte.lines().filter(|l| !entete.contains(&l.trim_end())).any(a_un_commentaire)
}

/// Un `#` hors d'une chaîne entre guillemets (`couleur = "#FF0000"` n'en est pas un).
fn a_un_commentaire(ligne: &str) -> bool {
    let (mut guillemet, mut echappe) = (None, false);
    for c in ligne.chars() {
        match guillemet {
            Some(_) if echappe => echappe = false,
            Some('"') if c == '\\' => echappe = true,
            Some(q) if c == q => guillemet = None,
            Some(_) => {}
            None if c == '#' => return true,
            None if c == '"' || c == '\'' => guillemet = Some(c),
            None => {}
        }
    }
    false
}

/// Le nom sera-t-il trouvé par `open -a` ? Sinon, l'app installée au nom le plus proche.
fn chercher_app(nom: &str) -> (bool, Option<String>) {
    let nom = nom.trim();
    if nom.is_empty() {
        return (true, None);
    }
    if nom.starts_with('/') {
        return (Path::new(nom).exists(), None);
    }
    if nom.contains('.') && !nom.contains(char::is_whitespace) && bundle_installe(nom) {
        return (true, None);
    }
    let mdfind = |requete: &str| {
        Command::new("/usr/bin/mdfind").arg(requete).output().map(|s| String::from_utf8_lossy(&s.stdout).into_owned()).unwrap_or_default()
    };
    let echappe = nom.replace('\\', "\\\\").replace('\'', "\\'");
    let exacte = format!("kMDItemContentType == 'com.apple.application-bundle' && kMDItemDisplayName == '{echappe}'c");
    if !mdfind(&exacte).trim().is_empty() {
        return (true, None);
    }
    let noms: Vec<String> = mdfind("kMDItemContentType == 'com.apple.application-bundle'")
        .lines()
        .filter_map(|l| Path::new(l).file_stem().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    // shortcut: sans index Spotlight, aucune app n'est listée ; on ne signale rien plutôt que de signaler à tort.
    if noms.is_empty() || noms.iter().any(|n| n.eq_ignore_ascii_case(nom)) {
        return (true, None);
    }
    (false, suggestion(nom, &noms))
}

fn bundle_installe(bundle: &str) -> bool {
    unsafe {
        let espace: Id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let url: Id = msg_send![espace, URLForApplicationWithBundleIdentifier: &*texte(bundle)];
        !url.is_null()
    }
}

/// Le nom installé le plus proche : le plus court qui contient la saisie (« Photoshop » → « Adobe Photoshop 2025 »),
/// sinon celui qui en diffère de quelques fautes de frappe.
fn suggestion(nom: &str, noms: &[String]) -> Option<String> {
    let q = nom.trim().to_lowercase();
    if let Some(n) = noms.iter().filter(|n| n.to_lowercase().contains(&q)).min_by_key(|n| n.len()) {
        return Some(n.clone());
    }
    noms.iter()
        .map(|n| (distance(&q, &n.to_lowercase()), n))
        .filter(|(d, _)| *d <= q.chars().count() / 5 + 1)
        .min_by_key(|(d, _)| *d)
        .map(|(_, n)| n.clone())
}

/// Distance de Levenshtein (insertions, suppressions, substitutions).
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut ligne: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonale = ligne[0];
        ligne[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let haut = ligne[j + 1];
            ligne[j + 1] = (diagonale + usize::from(ca != *cb)).min(haut + 1).min(ligne[j] + 1);
            diagonale = haut;
        }
    }
    ligne[b.len()]
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

        enregistrer(&chemin, json!({ "element": [{ "type": "souple" }, { "type": "souple" }] })).unwrap();
        assert_eq!(noms(&dossier.join("sous")), ["barre.toml", "barre.toml.precedent"]);
        let elements = |p: &Path| config::lire(&std::fs::read_to_string(p).unwrap()).unwrap().elements.len();
        assert_eq!(elements(&version(&chemin, 1)), 1);

        revenir(&chemin).unwrap();
        assert_eq!(elements(&chemin), 1);
        assert_eq!(elements(&version(&chemin, 1)), 2);
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    #[test]
    fn une_barre_vide_n_est_jamais_ecrite() {
        let dossier = dossier_essai("vide");
        let chemin = dossier.join("barre.toml");
        let erreur = enregistrer(&chemin, json!({ "masquer_control_strip": true, "element": [] })).unwrap_err();
        assert!(erreur.contains("vide"), "{erreur}");
        assert!(!chemin.exists());
        // Une barre principale vide reste permise s'il y a des barres par application.
        enregistrer(&chemin, json!({ "application": [{ "app": "Safari", "element": [{ "type": "souple" }] }] })).unwrap();
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    /// Six enregistrements : cinq versions gardées ; « Revenir » remonte d'un cran à chaque fois et fait le tour sans rien perdre.
    #[test]
    fn historique_de_cinq_versions() {
        let dossier = dossier_essai("historique");
        let chemin = dossier.join("barre.toml");
        let barre = |n: usize| json!({ "element": [{ "type": "espace", "largeur": n }] });
        let largeur = |p: &Path| match &config::lire(&std::fs::read_to_string(p).unwrap()).unwrap().elements[0] {
            Element::Espace { largeur } => *largeur as usize,
            autre => panic!("{autre:?}"),
        };
        for n in 1..=7 {
            enregistrer(&chemin, barre(n)).unwrap();
        }
        assert_eq!(largeur(&chemin), 7);
        assert_eq!((1..=VERSIONS).map(|k| largeur(&version(&chemin, k))).collect::<Vec<_>>(), [6, 5, 4, 3, 2]);
        assert!(!version(&chemin, VERSIONS + 1).exists());

        revenir(&chemin).unwrap();
        assert_eq!(largeur(&chemin), 6);
        revenir(&chemin).unwrap();
        assert_eq!(largeur(&chemin), 5);
        for _ in 0..4 {
            revenir(&chemin).unwrap();
        }
        assert_eq!(largeur(&chemin), 7, "après la plus ancienne, on retombe sur la plus récente");
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    /// Un fichier commenté à la main : copié en `.manuel` au premier enregistrement, jamais écrasé ensuite.
    #[test]
    fn les_commentaires_sont_gardes_une_fois() {
        let dossier = dossier_essai("manuel");
        std::fs::create_dir_all(&dossier).unwrap();
        let chemin = dossier.join("barre.toml");
        let original = "# Ma barre\n[[element]]\ntype = \"bouton\"\ncouleur = \"#FF0000\"\naction = { keycode = 105 }   # F13\n";
        std::fs::write(&chemin, original).unwrap();
        let barre = json!({ "element": [{ "type": "souple" }] });
        assert_eq!(enregistrer(&chemin, barre.clone()).unwrap().as_deref(), Some("barre.toml.manuel"));
        assert_eq!(std::fs::read_to_string(voisin(&chemin, ".manuel")).unwrap(), original);
        std::fs::write(&chemin, "# encore à la main\n[[element]]\ntype = \"souple\"\n").unwrap();
        assert_eq!(enregistrer(&chemin, barre).unwrap(), None);
        assert_eq!(std::fs::read_to_string(voisin(&chemin, ".manuel")).unwrap(), original);
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    #[test]
    fn commentaires_detectes() {
        assert!(a_un_commentaire("# Ma barre"));
        assert!(a_un_commentaire("action = { keycode = 105 }   # F13"));
        assert!(!a_un_commentaire("couleur = \"#FF0000\""));
        assert!(!a_un_commentaire("texte = 'C#'"));
        assert!(!a_un_commentaire("texte = \"un \\\" puis #\""));
        // Ce que l'éditeur écrit lui-même n'est pas « écrit à la main ».
        let barre = config::lire("[[element]]\ntype = \"bouton\"\ncouleur = \"#222222\"\n").unwrap();
        assert!(!ecrit_a_la_main(&config::ecrire(&barre).unwrap()));
        assert!(ecrit_a_la_main(&format!("{}# ajouté à la main\n", config::ecrire(&barre).unwrap())));
    }

    #[test]
    fn un_fichier_illisible_est_mis_de_cote_une_fois() {
        let dossier = dossier_essai("abime");
        std::fs::create_dir_all(&dossier).unwrap();
        let chemin = dossier.join("barre.toml");
        std::fs::write(&chemin, "[[element]]\ntype = \"bouton\"\ncouleur = #A259FF\n").unwrap();
        let copie = mettre_de_cote(&chemin).unwrap();
        let nom = copie.file_name().unwrap().to_string_lossy().into_owned();
        assert!(nom.starts_with("barre.toml.abime-") && nom.len() == "barre.toml.abime-AAAAMMJJ-HHMMSS".len(), "{nom}");
        assert_eq!(std::fs::read(&copie).unwrap(), std::fs::read(&chemin).unwrap());
        assert_eq!(mettre_de_cote(&chemin).unwrap(), copie, "pas de seconde copie identique");
        assert_eq!(numero_de_ligne("ligne 3 : string values must be quoted"), Some(3));
        assert_eq!(numero_de_ligne("impossible de lire"), None);
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    #[test]
    fn suggestion_d_app() {
        let noms: Vec<String> = ["Adobe Photoshop 2025", "Adobe Photoshop 2025 Uninstaller", "Safari", "Figma", "Spotify", "Photos"].map(String::from).into();
        assert_eq!(suggestion("Photoshop", &noms).as_deref(), Some("Adobe Photoshop 2025"));
        assert_eq!(suggestion("Photoshop", &noms[2..]), None, "« Photos » est trop loin de « Photoshop »");
        assert_eq!(suggestion("spotfy", &noms).as_deref(), Some("Spotify"));
        assert_eq!(suggestion("Figam", &noms).as_deref(), Some("Figma"));
        assert_eq!(suggestion("Excel", &noms), None);
        assert_eq!(distance("kitten", "sitting"), 3);
    }

    #[test]
    fn app_installee_ou_non() {
        if std::env::var("CI").is_ok() {
            return;
        }
        assert_eq!(chercher_app("Safari"), (true, None));
        assert_eq!(chercher_app("safari"), (true, None));
        assert_eq!(chercher_app("com.apple.Safari"), (true, None));
        assert!(!chercher_app("AppQuiNexistePasDuTout").0);
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
