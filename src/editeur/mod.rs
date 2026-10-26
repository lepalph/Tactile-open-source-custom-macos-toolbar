//! La fenêtre « Personnaliser ma barre » : une WKWebView qui affiche `editeur.html` (embarqué),
//! reliée à Rust par un `WKScriptMessageHandler` (JS → Rust) et `evaluateJavaScript` (Rust → JS).
//!
//! JS → Rust, un objet JSON `{ "type": … }` : `pret`, `enregistrer { barre, fermer? }`, `revenir`,
//! `prereglage { nom }`, `symboles { noms }`, `choisirApp`, `verifierApp { nom }`, `modifie { valeur }`,
//! `fermer`, `ouvrirFichier`, `relire`, `titre { texte }`, `journal { texte }`, `capture { chemin }`,
//! `iconesApps { noms }`, `choisirImage`, `deposerImage { nom, donnees }` (fichier glissé, en base64), `importerImage { chemin }`
//! (comme `choisirImage` sans le sélecteur : pour le pilote `TACTILE_EDITEUR_JS`), `images { chemins }`,
//! `exporter { barre, zip? }` (NSSavePanel), `exporterVers { barre, chemin }`, `importer` (NSOpenPanel),
//! `importerBarre { chemin }`, `deposerBarre { nom, donnees }`, `confirmerImport { ajouter?, barre? }`, `annulerImport`,
//! `mesurer { barre }`, `chercherVille { nom, langue }`, `meteo { nom, latitude, longitude }`, `ouvrirReglages`. Un chemin
//! venu de la page n'est suivi que s'il passe `chemin_permis` (pilote, ou fichier rendu par NSSavePanel).
//! Rust → JS : `tactile.charger(barre, aPrecedent)`, `tactile.illisible(message, extrait, aPrecedent)`,
//! `tactile.glyphes({nom: dataURL|null})`, `tactile.iconesApps({nom: dataURL|null})`, `tactile.enregistre(manuel, aPrecedent, avis)`, `tactile.revenu()`,
//! `tactile.abimeGarde(nom)`, `tactile.erreur(texte)`, `tactile.appChoisie(nom)`,
//! `tactile.appVerifiee(nom, trouvee, suggestion, localise)`, `tactile.demanderFermeture()`, `tactile.deborde(caches)`,
//! `tactile.villes(liste|null, erreur)`, `tactile.meteoPrete(source, element)`,
//! `tactile.imageChoisie(chemin, dataURL)`, `tactile.imageRefusee(texte)`, `tactile.images({chemin: dataURL|null})`,
//! `tactile.exportAvertir(logos)`, `tactile.exporte(nom, logos, avis)`, `tactile.importPret(apercu)`,
//! `tactile.importe(manuel, aPrecedent)`, `tactile.partageRefuse({fr, en})`.

mod barres;
mod icones;
mod partage;

pub use partage::{exporter_cli, importer_cli};

use crate::config::{self, Barre, Element};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool, NSObject};
use objc2::runtime::Sel;
use objc2::{class, define_class, msg_send, sel, ClassType};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::process::Command;

type Id = *mut AnyObject;

const PAGE: &str = include_str!("editeur.html");

thread_local! {
    static FENETRE: Cell<Id> = const { Cell::new(std::ptr::null_mut()) };
    static VUE: Cell<Id> = const { Cell::new(std::ptr::null_mut()) };
    /// Le délégué de la fenêtre : cible du menu Présentation et des réponses calculées sur un autre fil.
    static PONT: Cell<Id> = const { Cell::new(std::ptr::null_mut()) };
    /// Lancé avec `--personnaliser` : fermer la fenêtre quitte l'app.
    static SEUL: Cell<bool> = const { Cell::new(false) };
    /// Des changements non enregistrés dans la page (message `modifie`) : fermer demande confirmation.
    static MODIFIE: Cell<bool> = const { Cell::new(false) };
    /// La barre importée que la page montre en aperçu, en attente de « Remplacer ma barre ».
    static IMPORT: RefCell<Option<partage::Paquet>> = const { RefCell::new(None) };
    /// Le fichier que NSSavePanel vient de rendre à la page (`exportAvertir`) : le seul que `exporterVers` écrit hors pilote.
    static ACCORDE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
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
            // La page envoie du JSON en texte ; un objet (dictionnaire) ferait planter la conversion en NSString.
            let est_texte: Bool = unsafe { msg_send![corps, isKindOfClass: class!(NSString)] };
            if corps.is_null() || !est_texte.as_bool() {
                return eprintln!("tactile : message de l'éditeur ignoré, il doit être une chaîne JSON");
            }
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

        /// Une réponse calculée sur un autre fil (géocodage) : le JS est évalué ici, sur le fil principal.
        #[unsafe(method(executer:))]
        fn executer(&self, code: Id) {
            if !code.is_null() {
                js(&unsafe { (*(code as *const NSString)).to_string() });
            }
        }

        #[unsafe(method(agrandir:))]
        fn agrandir(&self, _envoyeur: Id) {
            zoomer(1);
        }

        #[unsafe(method(reduire:))]
        fn reduire(&self, _envoyeur: Id) {
            zoomer(-1);
        }

        #[unsafe(method(tailleReelle:))]
        fn taille_reelle(&self, _envoyeur: Id) {
            zoomer(0);
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
        "capture" => match message["chemin"].as_str() {
            Some(chemin) if chemin_permis(chemin, pilote(), None) => capturer(Path::new(chemin)),
            _ => refuser_chemin("capture", &message),
        },
        "enregistrer" => {
            let chemin = config::chemin();
            match enregistrer(&chemin, message["barre"].clone()) {
                Ok(manuel) => {
                    // Ce que `--verifier` signalerait (app absente, débordement…) : enregistré, mais dit tout de suite.
                    let avis = config::lire(&std::fs::read_to_string(&chemin).unwrap_or_default()).map(|b| barres::avertissements(&b)).unwrap_or_default();
                    js(&format!("tactile.enregistre({}, {}, {})", json!(manuel), version(&chemin, 1).exists(), json!(avis)));
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
            let (trouvee, proche, localise) = chercher_app(nom);
            js(&format!("tactile.appVerifiee({}, {trouvee}, {}, {localise})", json!(nom), json!(proche)));
        }
        "mesurer" => {
            // La même estimation que `--verifier` : combien d'éléments de la principale la Touch Bar cachera.
            let caches = serde_json::from_value::<Barre>(message["barre"].clone())
                .ok()
                .and_then(|b| crate::barre::debordement(&b.elements, crate::barre::largeur_barre(b.masquer_control_strip)))
                .map_or(0, |(_, n)| n);
            js(&format!("tactile.deborde({caches})"));
        }
        "chercherVille" => chercher_ville(message["nom"].as_str().unwrap_or(""), message["langue"].as_str().unwrap_or("fr")),
        "meteo" => {
            let (lat, lon) = (message["latitude"].as_f64().unwrap_or(f64::NAN), message["longitude"].as_f64().unwrap_or(f64::NAN));
            match barres::meteo(message["nom"].as_str().unwrap_or("meteo"), lat, lon) {
                Ok((source, element)) => js(&format!("tactile.meteoPrete({}, {})", json!(source), json!(element))),
                Err(e) => js(&format!("tactile.villes(null, {})", json!(e))),
            }
        }
        "ouvrirReglages" => {
            // Adresse fixe : la page ne choisit pas ce qui est ouvert.
            let _ = Command::new("/usr/bin/open").arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility").status();
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
            // Une barre toute faite remplace la principale ; les barres par application de l'utilisateur restent,
            // avec les sources qu'elles citent.
            match config::lire(toml).map(|b| barres::prereglage(config::charger().ok(), b)) {
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
        "choisirImage" => {
            if let Some(source) = icones::choisir(&icones::FORMATS) {
                importer_image(icones::importer(&source, &icones::dossier(&config::chemin())));
            }
        }
        "importerImage" => match message["chemin"].as_str() {
            Some(source) if chemin_permis(source, pilote(), None) => importer_image(icones::importer(Path::new(source), &icones::dossier(&config::chemin()))),
            _ => refuser_chemin("importerImage", &message),
        },
        "deposerImage" => {
            let (nom, donnees) = (message["nom"].as_str().unwrap_or(""), message["donnees"].as_str().unwrap_or(""));
            importer_image(icones::importer_depot(nom, donnees, &icones::dossier(&config::chemin())));
        }
        "images" => {
            let chemins = message["chemins"].as_array().cloned().unwrap_or_default();
            let images: serde_json::Map<String, Value> = chemins
                .iter()
                .filter_map(Value::as_str)
                .map(|c| (c.to_string(), icones::apercu(c).map_or(Value::Null, Value::String)))
                .collect();
            js(&format!("tactile.images({})", Value::Object(images)));
        }
        "iconesApps" => {
            let noms = message["noms"].as_array().cloned().unwrap_or_default();
            let icones: serde_json::Map<String, Value> =
                noms.iter().filter_map(Value::as_str).map(|n| (n.to_string(), icone_app(n).map_or(Value::Null, Value::String))).collect();
            js(&format!("tactile.iconesApps({})", Value::Object(icones)));
        }
        "exporter" | "exporterVers" => exporter(&message),
        "importer" => {
            if let Some(fichier) = icones::choisir(&["toml", "zip"]) {
                preparer_import(partage::ouvrir(&fichier), &fichier.to_string_lossy());
            }
        }
        "importerBarre" => match message["chemin"].as_str() {
            Some(chemin) if chemin_permis(chemin, pilote(), None) => preparer_import(partage::ouvrir(Path::new(chemin)), chemin),
            _ => refuser_chemin("importerBarre", &message),
        },
        "deposerBarre" => {
            let nom = message["nom"].as_str().unwrap_or("");
            let paquet = icones::decoder_depot(message["donnees"].as_str().unwrap_or(""))
                .map_err(|e| partage::Texte { fr: e, en: "file too large: 10 MB at most".into() })
                .and_then(|octets| partage::ouvrir_octets(octets, None));
            preparer_import(paquet, nom);
        }
        "confirmerImport" => {
            let Some(mut paquet) = IMPORT.take() else { return };
            // « Ajouter à ma barre » : la barre de la page (enregistrée ou non) reçoit celle importée ; les logos
            // importés gardent leurs chemins temporaires, que `installer` réécrit comme pour un remplacement.
            if message["ajouter"].as_bool() == Some(true) {
                match serde_json::from_value::<Barre>(message["barre"].clone()) {
                    Ok(mienne) => paquet.barre = barres::fusionner(mienne, paquet.barre.clone()),
                    Err(e) => return js(&format!("tactile.partageRefuse({})", json!(partage::Texte { fr: format!("ta barre est illisible : {e}"), en: format!("your bar is unreadable: {e}") }))),
                }
            }
            match partage::installer(&paquet, &config::chemin()) {
                Ok(manuel) => {
                    envoyer_config(config::charger());
                    js(&format!("tactile.importe({}, {})", json!(manuel), version(&config::chemin(), 1).exists()));
                }
                Err(e) => js(&format!("tactile.partageRefuse({})", json!(e))),
            }
        }
        "annulerImport" => drop(IMPORT.take()),
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

/// Un message à chemin refusé : rien n'est lu ni écrit, le journal le dit.
fn refuser_chemin(type_: &str, message: &Value) {
    eprintln!("tactile : éditeur, « {type_} » refusé : chemin {} hors des dossiers permis (sélecteur, ou pilote TACTILE_EDITEUR_JS dans le dossier temporaire ou le dépôt)", message["chemin"]);
}

/// Le pilote `TACTILE_EDITEUR_JS` est-il actif ?
fn pilote() -> bool {
    std::env::var_os("TACTILE_EDITEUR_JS").is_some_and(|c| !c.is_empty())
}

/// Un chemin venu de la page (`capture`, `importerImage`, `importerBarre`, `exporterVers`) : la page n'a pas à
/// désigner un fichier de son choix, même si une injection y entrait. Il est suivi s'il vient d'être rendu par le
/// sélecteur de fichier (`accorde`) ; sinon seulement en mode pilote, dans le dossier temporaire ou le dépôt.
fn chemin_permis(chemin: &str, pilote: bool, accorde: Option<&Path>) -> bool {
    let chemin = Path::new(chemin);
    if accorde == Some(chemin) {
        return true;
    }
    if !pilote || !chemin.is_absolute() || chemin.components().any(|c| c == std::path::Component::ParentDir) {
        return false;
    }
    // Le dossier réel (liens résolus), et pas de lien au bout : `/tmp/x -> /etc` ne mène nulle part.
    let Some(dossier) = chemin.parent().and_then(|d| d.canonicalize().ok()) else { return false };
    if std::fs::symlink_metadata(chemin).is_ok_and(|m| m.file_type().is_symlink()) {
        return false;
    }
    // Le dépôt : le dossier courant s'il a un Cargo.toml (lancé depuis le Finder, c'est `/`).
    let depot = std::env::current_dir().ok().filter(|d| d.join("Cargo.toml").is_file());
    [Some(std::env::temp_dir()), Some(PathBuf::from("/tmp")), depot].into_iter().flatten().filter_map(|z| z.canonicalize().ok()).any(|z| dossier.starts_with(z))
}

/// La copie faite (chemin avec `~` et aperçu), ou la raison du refus.
fn importer_image(copie: Result<PathBuf, String>) {
    match copie {
        Ok(copie) => {
            let chemin = icones::avec_tilde(&copie);
            js(&format!("tactile.imageChoisie({}, {})", json!(chemin), json!(icones::apercu(&chemin))));
        }
        Err(e) => js(&format!("tactile.imageRefusee({})", json!(e))),
    }
}

/// La barre importée, lue et vérifiée : gardée en attente, montrée en aperçu avec ce qu'elle lance.
fn preparer_import(paquet: Result<partage::Paquet, partage::Texte>, nom: &str) {
    match paquet {
        Ok(p) => {
            // Avis sans numéro de ligne ni « bundle id » : le fichier reçu, personne ne l'a lu.
            let avis: Vec<partage::Texte> = p.avis.iter().map(|a| if a.fr == a.en { barres::avis_humain(&a.fr) } else { a.clone() }).collect();
            let apercu = json!({
                "nom": Path::new(nom).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                "barre": p.barre, "commandes": p.commandes, "avis": avis, "logos": p.logos(), "alertes": p.alertes(),
                "code": barres::execute_du_code(&p.barre),
            });
            IMPORT.set(Some(p));
            js(&format!("tactile.importPret({apercu})"));
        }
        Err(e) => js(&format!("tactile.partageRefuse({})", json!(e))),
    }
}

/// `exporter { barre, zip? }` : NSSavePanel puis écriture ; `exporterVers { barre, chemin }` écrit sans sélecteur.
/// Un `.toml` choisi pour une barre qui a des logos : la page propose le `.zip` (`exportAvertir`) avant d'écrire.
fn exporter(message: &Value) {
    let barre = match serde_json::from_value::<Barre>(message["barre"].clone()).map_err(|e| e.to_string()).and_then(|b| config::lire(&config::ecrire(&b)?)) {
        Ok(b) => b,
        Err(e) => return js(&format!("tactile.partageRefuse({})", json!(partage::Texte { fr: format!("barre illisible : {e}"), en: format!("unreadable bar: {e}") }))),
    };
    let logos = partage::logos(&barre);
    let cible = match message["chemin"].as_str() {
        // Le .toml que NSSavePanel vient de rendre (après `exportAvertir`), ou un pilote.
        Some(c) if chemin_permis(c, pilote(), ACCORDE.take().as_deref()) => PathBuf::from(c),
        Some(_) => return refuser_chemin("exporterVers", message),
        None => {
            let zip = logos > 0 || message["zip"].as_bool() == Some(true);
            let Some(choisi) = choisir_destination(if zip { "barre.zip" } else { "barre.toml" }, zip) else { return };
            if logos > 0 && message["zip"].as_bool() != Some(true) && choisi.extension().is_some_and(|e| e.eq_ignore_ascii_case("toml")) {
                ACCORDE.set(Some(choisi.clone()));
                return js(&format!("tactile.exportAvertir({logos}, {})", json!(choisi)));
            }
            choisi
        }
    };
    match partage::exporter(&barre, None, &cible) {
        Ok(e) => {
            let nom = e.fichier.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            js(&format!("tactile.exporte({}, {}, {})", json!(nom), e.logos, json!(e.avis)));
        }
        Err(e) => js(&format!("tactile.partageRefuse({})", json!(e))),
    }
}

/// NSSavePanel : où écrire l'export. `zip` : seule l'extension .zip est proposée.
fn choisir_destination(nom: &str, zip: bool) -> Option<PathBuf> {
    unsafe {
        let panneau: Id = msg_send![class!(NSSavePanel), savePanel];
        let _: () = msg_send![panneau, setNameFieldStringValue: &*texte(nom)];
        let types: Id = msg_send![class!(NSMutableArray), array];
        for t in if zip { &["zip"][..] } else { &["toml", "zip"][..] } {
            let _: () = msg_send![types, addObject: &*texte(t)];
        }
        let _: () = msg_send![panneau, setAllowedFileTypes: types];
        let reponse: isize = msg_send![panneau, runModal];
        if reponse != 1 {
            return None;
        }
        let url: Id = msg_send![panneau, URL];
        let chemin: Id = msg_send![url, path];
        Some(PathBuf::from((*(chemin as *const NSString)).to_string()))
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
    let toml = config::ecrire(&barre)?;
    let relue = config::lire(&toml)?;
    verifier(&relue.elements)?;
    for app in &relue.applications {
        verifier(&app.elements).map_err(|e| format!("application « {} », {e}", app.app))?;
    }
    // Le validateur partagé avec `--verifier` et la barre refuse ce qui la rendrait inutilisable : une principale vide
    // (cf. SYNTHESE E1), même avec des barres d'application, puisque la barre montrerait alors « ⚠ config vide ».
    if let Some(avis) = config::controler(&relue, None).into_iter().find(|a| a.grave) {
        return Err(avis.to_string());
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
    let barre = Barre { masquer_control_strip: true, elements: vec![Element::Souple], applications: vec![], sources: vec![] };
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

/// Le nom sera-t-il trouvé par `open -a` (`actions::app_existe`, comme `--verifier`) ? Sinon, l'app dont c'est le nom
/// traduit (« Musique » → « Music », `localise`), ou l'app installée au nom le plus proche.
fn chercher_app(nom: &str) -> (bool, Option<String>, bool) {
    let nom = nom.trim();
    if nom.is_empty() || crate::actions::app_existe(nom) {
        return (true, None, false);
    }
    if let Some(app) = app_au_nom_traduit(nom) {
        return (false, Some(app), true);
    }
    let sortie = Command::new("/usr/bin/mdfind").arg("kMDItemContentType == 'com.apple.application-bundle'").output();
    let noms: Vec<String> = sortie
        .map(|s| String::from_utf8_lossy(&s.stdout).into_owned())
        .unwrap_or_default()
        .lines()
        .filter_map(|l| Path::new(l).file_stem().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    (false, suggestion(nom, &noms), false)
}

/// Le Finder et le Dock montrent le nom traduit (« Musique », « Calendrier », « Réglages Système ») ; `open -a`
/// ne connaît que le nom du dossier (« Music »). Cherche, dans les dossiers d'apps, celle dont un nom traduit
/// (`CFBundleDisplayName` ou `CFBundleName`, dans n'importe quelle langue) est `nom`, sans casse.
fn app_au_nom_traduit(nom: &str) -> Option<String> {
    let cherche = nom.to_lowercase();
    let home = std::env::var("HOME").unwrap_or_default();
    let dossiers = ["/Applications", "/Applications/Utilities", "/System/Applications", "/System/Applications/Utilities", &format!("{home}/Applications")];
    dossiers
        .iter()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "app"))
        .find(|app| noms_traduits(app).iter().any(|n| n.to_lowercase() == cherche))
        .and_then(|app| app.file_stem().map(|n| n.to_string_lossy().into_owned()))
}

/// Les noms d'une app dans toutes ses langues : `InfoPlist.loctable` (apps d'Apple) ou `<langue>.lproj/InfoPlist.strings`.
fn noms_traduits(app: &Path) -> Vec<String> {
    let ressources = app.join("Contents/Resources");
    let mut tables = Vec::new();
    // La loctable est un dictionnaire { langue: { clé: texte } } ; un .strings, directement { clé: texte }.
    if let Some(loc) = dictionnaire(&ressources.join("InfoPlist.loctable")) {
        let valeurs: Id = unsafe { msg_send![loc, allValues] };
        let n: usize = unsafe { msg_send![valeurs, count] };
        tables.extend((0..n).map(|i| -> Id { unsafe { msg_send![valeurs, objectAtIndex: i] } }));
    }
    if let Ok(entrees) = std::fs::read_dir(&ressources) {
        for e in entrees.flatten().filter(|e| e.path().extension().is_some_and(|x| x == "lproj")) {
            tables.extend(dictionnaire(&e.path().join("InfoPlist.strings")));
        }
    }
    let mut noms = Vec::new();
    for table in tables {
        let est_dico: Bool = unsafe { msg_send![table, isKindOfClass: class!(NSDictionary)] };
        if !est_dico.as_bool() {
            continue;
        }
        for cle in ["CFBundleDisplayName", "CFBundleName"] {
            let v: Id = unsafe { msg_send![table, objectForKey: &*texte(cle)] };
            let est_texte: Bool = if v.is_null() { Bool::NO } else { unsafe { msg_send![v, isKindOfClass: class!(NSString)] } };
            if est_texte.as_bool() {
                noms.push(unsafe { (*(v as *const NSString)).to_string() });
            }
        }
    }
    noms
}

/// Un fichier plist (ou .strings) lu en `NSDictionary`, s'il existe et se lit.
fn dictionnaire(chemin: &Path) -> Option<Id> {
    if !chemin.exists() {
        return None;
    }
    let d: Id = unsafe { msg_send![class!(NSDictionary), dictionaryWithContentsOfFile: &*texte(&chemin.to_string_lossy())] };
    (!d.is_null()).then_some(d)
}

/// Géocodage Open-Meteo, seulement quand l'utilisateur cherche une ville : par `curl` comme les sources http,
/// sur un autre fil (la fenêtre reste vivante), réponse `tactile.villes(liste, erreur)` sur le fil principal.
fn chercher_ville(nom: &str, langue: &str) {
    if nom.trim().is_empty() {
        return;
    }
    let adresse = barres::adresse_geocodage(nom, langue);
    let pont = PONT.get() as usize;
    std::thread::spawn(move || {
        let processus = crate::barre::Processus::new(std::sync::Mutex::new(Some(Vec::new())));
        let code = match crate::sources::http::requete(&adresse, &[], None, std::time::Duration::from_secs(8), &processus).and_then(|r| r.valeur()) {
            Ok(v) => format!("tactile.villes({}, null)", Value::Array(barres::villes(&v))),
            Err(e) => format!("tactile.villes(null, {})", json!(e)),
        };
        let code = texte(&code);
        unsafe { let _: () = msg_send![pont as Id, performSelectorOnMainThread: sel!(executer:), withObject: &*code, waitUntilDone: Bool::NO]; }
    });
}

/// Paliers de zoom de la page (⌘+ / ⌘− / ⌘0), comme Safari ; le choix est gardé d'une ouverture à l'autre.
const ZOOMS: [f64; 9] = [0.75, 0.85, 1.0, 1.15, 1.3, 1.5, 1.75, 2.0, 2.5];
const CLE_ZOOM: &str = "TactileZoomEditeur";

/// `sens` : 1 agrandit, -1 réduit, 0 revient à 100 %.
fn zoomer(sens: i32) {
    let vue = VUE.get();
    if vue.is_null() {
        return;
    }
    unsafe {
        let actuel: f64 = msg_send![vue, pageZoom];
        let i = ZOOMS.iter().position(|z| (z - actuel).abs() < 0.01).unwrap_or(2) as i32;
        let z = if sens == 0 { 1.0 } else { ZOOMS[(i + sens).clamp(0, ZOOMS.len() as i32 - 1) as usize] };
        let _: () = msg_send![vue, setPageZoom: z];
        let reglages: Id = msg_send![class!(NSUserDefaults), standardUserDefaults];
        let _: () = msg_send![reglages, setDouble: z, forKey: &*texte(CLE_ZOOM)];
    }
}

/// Menu principal de l'app quand la fenêtre est ouverte : sans lui, ⌘C, ⌘V, ⌘X, ⌘A et ⌘Z ne font rien dans les champs
/// de la page (AppKit cherche ces raccourcis dans le menu). Édition vise le premier répondeur (la WKWebView), comme
/// toute app ; Fichier › Fermer passe par `windowShouldClose:` ; Présentation règle le zoom de la page.
fn menu_principal(pont: Id, seul: bool, fr: bool) -> Id {
    let l = |f: &'static str, e: &'static str| if fr { f } else { e };
    unsafe {
        let barre = sous_menu_vide("");
        let app = sous_menu(barre, "Tactile");
        if seul {
            entree(app, l("Quitter Tactile", "Quit Tactile"), Some(sel!(terminate:)), "q", std::ptr::null_mut());
        }
        let fichier = sous_menu(barre, l("Fichier", "File"));
        entree(fichier, l("Fermer", "Close"), Some(sel!(performClose:)), "w", std::ptr::null_mut());
        let edition = sous_menu(barre, l("Édition", "Edit"));
        entree(edition, l("Annuler", "Undo"), Some(sel!(undo:)), "z", std::ptr::null_mut());
        entree(edition, l("Rétablir", "Redo"), Some(sel!(redo:)), "Z", std::ptr::null_mut());
        let separateur: Id = msg_send![class!(NSMenuItem), separatorItem];
        let _: () = msg_send![edition, addItem: separateur];
        entree(edition, l("Couper", "Cut"), Some(sel!(cut:)), "x", std::ptr::null_mut());
        entree(edition, l("Copier", "Copy"), Some(sel!(copy:)), "c", std::ptr::null_mut());
        entree(edition, l("Coller", "Paste"), Some(sel!(paste:)), "v", std::ptr::null_mut());
        entree(edition, l("Tout sélectionner", "Select All"), Some(sel!(selectAll:)), "a", std::ptr::null_mut());
        let presentation = sous_menu(barre, l("Présentation", "View"));
        entree(presentation, l("Agrandir", "Zoom In"), Some(sel!(agrandir:)), "+", pont);
        // ⌘= aussi (la touche + sans Maj sur un clavier américain), sans ligne de plus dans le menu.
        let egal = entree(presentation, l("Agrandir", "Zoom In"), Some(sel!(agrandir:)), "=", pont);
        let _: () = msg_send![egal, setHidden: Bool::YES];
        let _: () = msg_send![egal, setAllowsKeyEquivalentWhenHidden: Bool::YES];
        entree(presentation, l("Réduire", "Zoom Out"), Some(sel!(reduire:)), "-", pont);
        entree(presentation, l("Taille réelle", "Actual Size"), Some(sel!(tailleReelle:)), "0", pont);
        barre
    }
}

unsafe fn sous_menu_vide(titre: &str) -> Id {
    unsafe {
        let menu: Id = msg_send![class!(NSMenu), alloc];
        msg_send![menu, initWithTitle: &*texte(titre)]
    }
}

/// Ajoute à `barre` un menu titré `titre` et le renvoie.
unsafe fn sous_menu(barre: Id, titre: &str) -> Id {
    unsafe {
        let menu = sous_menu_vide(titre);
        let item = entree(barre, titre, None, "", std::ptr::null_mut());
        let _: () = msg_send![item, setSubmenu: menu];
        menu
    }
}

/// Une entrée ⌘`cle` (une majuscule ajoute ⇧) ; `cible` nulle : le premier répondeur qui sait faire l'action.
unsafe fn entree(menu: Id, titre: &str, action: Option<Sel>, cle: &str, cible: Id) -> Id {
    unsafe {
        let item: Id = msg_send![class!(NSMenuItem), alloc];
        let item: Id = msg_send![item, initWithTitle: &*texte(titre), action: action, keyEquivalent: &*texte(cle)];
        let _: () = msg_send![item, setTarget: cible];
        let _: () = msg_send![menu, addItem: item];
        item
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
    config::rgb(c).is_some()
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
        icones::png(image).map(|png| format!("data:image/png;base64,{}", icones::base64(&png)))
    }
}

/// L'icône de l'app (nom, bundle id ou chemin) telle que le lanceur la dessine dans la barre, sans le point « ouverte »,
/// en data URL PNG. L'image vient du cache de la barre (`barre::icone_app`). `None` si l'app est introuvable.
fn icone_app(nom: &str) -> Option<String> {
    let chemin = crate::actions::chemin_app(nom.trim()).filter(|_| !nom.trim().is_empty())?;
    let image = crate::barre::icone_app(&chemin, false)?;
    unsafe { icones::png(image) }.map(|png| format!("data:image/png;base64,{}", icones::base64(&png)))
}

/// Image PNG de la page, rendue par WebKit lui-même (`takeSnapshotWithConfiguration:`) : elle marche écran
/// verrouillé, là où `CGWindowListCreateImage` rend du blanc, et sans l'autorisation « Enregistrement de l'écran ».
/// Asynchrone : le PNG est écrit quand WebKit rend la main, une fraction de seconde plus tard.
fn capturer(chemin: &Path) {
    let chemin = chemin.to_path_buf();
    let fin = block2::RcBlock::new(move |image: Id, _erreur: Id| {
        if let Err(e) = ecrire_png(image, &chemin) {
            eprintln!("tactile : capture, {e}");
        }
    });
    unsafe {
        let _: () = msg_send![VUE.get(), takeSnapshotWithConfiguration: std::ptr::null_mut::<AnyObject>(), completionHandler: &*fin];
    }
}

fn ecrire_png(image: Id, chemin: &Path) -> Result<(), String> {
    if image.is_null() {
        return Err("WebKit n'a rendu aucune image".into());
    }
    unsafe {
        let image = avec_cadre(image);
        let tiff: Id = msg_send![image, TIFFRepresentation];
        let bitmap: Id = msg_send![class!(NSBitmapImageRep), imageRepWithData: tiff];
        let proprietes: Id = msg_send![class!(NSDictionary), dictionary];
        let png: Id = msg_send![bitmap, representationUsingType: 4usize, properties: proprietes];
        let ok: Bool = msg_send![png, writeToFile: &*texte(&chemin.to_string_lossy()), atomically: Bool::YES];
        if ok.as_bool() { Ok(()) } else { Err(format!("impossible d'écrire {}", chemin.display())) }
    }
}

/// La page posée dans le cadre de la fenêtre (barre de titre, boutons rouge-orange-vert), dessiné par AppKit hors écran.
unsafe fn avec_cadre(page: Id) -> Id {
    unsafe {
        let fenetre = FENETRE.get();
        let vue: Id = msg_send![fenetre, contentView];
        let cadre: Id = msg_send![vue, superview];
        let tout: NSRect = msg_send![cadre, bounds];
        let contenu: NSRect = msg_send![vue, frame];
        let rep: Id = msg_send![cadre, bitmapImageRepForCachingDisplayInRect: tout];
        let _: () = msg_send![cadre, cacheDisplayInRect: tout, toBitmapImageRep: rep];
        let image: Id = msg_send![class!(NSImage), alloc];
        let image: Id = msg_send![image, initWithSize: tout.size];
        let _: () = msg_send![image, lockFocus];
        // Les coins arrondis d'une fenêtre macOS (10 pt).
        let coins: Id = msg_send![class!(NSBezierPath), bezierPathWithRoundedRect: tout, xRadius: 10.0f64, yRadius: 10.0f64];
        let _: () = msg_send![coins, addClip];
        let _: Bool = msg_send![rep, drawInRect: tout];
        let _: () = msg_send![page, drawInRect: contenu];
        let _: () = msg_send![image, unlockFocus];
        image
    }
}

/// `TACTILE_APPARENCE=clair|sombre` force le thème de l'app (et donc `prefers-color-scheme` dans la page), pour les captures.
fn forcer_apparence(app: Id) {
    let nom = match std::env::var("TACTILE_APPARENCE").as_deref() {
        Ok("clair") => "NSAppearanceNameAqua",
        Ok("sombre") => "NSAppearanceNameDarkAqua",
        _ => return,
    };
    unsafe {
        let apparence: Id = msg_send![class!(NSAppearance), appearanceNamed: &*texte(nom)];
        let _: () = msg_send![app, setAppearance: apparence];
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
        forcer_apparence(app);
        if FENETRE.get().is_null() {
            let pont = Retained::into_raw(msg_send![Pont::class(), new]) as Id;
            let configuration: Id = msg_send![class!(WKWebViewConfiguration), new];
            let controleur: Id = msg_send![configuration, userContentController];
            let _: () = msg_send![controleur, addScriptMessageHandler: pont, name: &*texte("tactile")];
            let cadre = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1180.0, 760.0));
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
            // Comme Réglages Système : pas de titre répété au-dessus de celui de la page, une seule bande du haut.
            let _: () = msg_send![fenetre, setTitleVisibility: 1isize];
            let _: () = msg_send![fenetre, setTitlebarAppearsTransparent: Bool::YES];
            let _: () = msg_send![fenetre, setContentMinSize: NSSize::new(1060.0, 600.0)];
            let _: () = msg_send![fenetre, setContentView: vue];
            let _: () = msg_send![fenetre, setDelegate: pont];
            let _: () = msg_send![fenetre, center];
            FENETRE.set(fenetre);
            VUE.set(vue);
            PONT.set(pont);
            let reglages: Id = msg_send![class!(NSUserDefaults), standardUserDefaults];
            let zoom: f64 = msg_send![reglages, doubleForKey: &*texte(CLE_ZOOM)];
            if ZOOMS.contains(&zoom) {
                let _: () = msg_send![vue, setPageZoom: zoom];
            }
            let langues: Id = msg_send![class!(NSLocale), preferredLanguages];
            let premiere: Id = msg_send![langues, firstObject];
            let fr = !premiere.is_null() && (*(premiere as *const NSString)).to_string().starts_with("fr");
            let _: () = msg_send![app, setMainMenu: menu_principal(pont, SEUL.get(), fr)];
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

    /// S5 (revue 0.3, trouvaille 4) : un chemin venu de la page n'est suivi qu'en mode pilote, dans le dossier
    /// temporaire ou le dépôt, ou s'il vient d'être rendu par un sélecteur de fichier.
    #[test]
    fn chemins_venus_de_la_page() {
        let d = dossier_essai("chemins");
        std::fs::create_dir_all(&d).unwrap();
        let dans_tmp = d.join("capture.png").to_string_lossy().into_owned();
        let depot = std::env::current_dir().unwrap().join("docs/capture.png").to_string_lossy().into_owned();
        // Sans pilote : rien, sauf le fichier que le sélecteur vient de rendre.
        assert!(!chemin_permis(&dans_tmp, false, None));
        assert!(!chemin_permis(&depot, false, None));
        assert!(chemin_permis("/Users/x/Desktop/barre.toml", false, Some(Path::new("/Users/x/Desktop/barre.toml"))));
        assert!(!chemin_permis("/Users/x/Desktop/autre.toml", false, Some(Path::new("/Users/x/Desktop/barre.toml"))));
        // Pilote : dossier temporaire et dépôt seulement.
        assert!(chemin_permis(&dans_tmp, true, None));
        assert!(chemin_permis(&depot, true, None));
        assert!(chemin_permis("/tmp/x.png", true, None));
        let home = std::env::var("HOME").unwrap();
        let refuses = [
            "/etc/passwd".to_string(),
            format!("{home}/.ssh/authorized_keys"),
            format!("{home}/Library/LaunchAgents/x.plist"),
            "~/.zshrc".into(),
            "capture.png".into(),
            format!("{}/../../../etc/passwd", d.display()),
            d.join("absent/x.png").to_string_lossy().into_owned(),
            String::new(),
        ];
        for c in &refuses {
            assert!(!chemin_permis(c, true, None), "{c}");
        }
        // Un lien posé dans le dossier temporaire ne mène pas ailleurs.
        std::os::unix::fs::symlink("/etc", d.join("lien")).unwrap();
        std::os::unix::fs::symlink("/etc/hosts", d.join("hosts")).unwrap();
        assert!(!chemin_permis(&d.join("lien/passwd").to_string_lossy(), true, None));
        assert!(!chemin_permis(&d.join("hosts").to_string_lossy(), true, None));
        std::fs::remove_dir_all(&d).unwrap();
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
        // Même avec des barres par application : la barre montrerait la barre par défaut et « ⚠ config vide ». Un seul message.
        let avec_app = enregistrer(&chemin, json!({ "application": [{ "app": "Safari", "element": [{ "type": "souple" }] }] })).unwrap_err();
        assert_eq!(avec_app, erreur);
        assert!(!chemin.exists());
        let _ = std::fs::remove_dir_all(&dossier); // rien n'a été écrit : le dossier peut ne pas exister
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
    #[cfg_attr(ci, ignore = "installed apps and the Spotlight index are specific to this Mac")]
    fn app_installee_ou_non() {
        assert_eq!(chercher_app("Safari"), (true, None, false));
        assert_eq!(chercher_app("safari"), (true, None, false));
        assert_eq!(chercher_app("com.apple.Safari"), (true, None, false));
        assert!(!chercher_app("AppQuiNexistePasDuTout").0);
        // Nadia (0.3) : le Finder français dit « Musique », la barre a besoin de « Music ».
        assert_eq!(chercher_app("Musique"), (false, Some("Music".into()), true));
        assert_eq!(chercher_app("calendrier"), (false, Some("Calendar".into()), true));
        assert_eq!(chercher_app("Réglages Système"), (false, Some("System Settings".into()), true));
    }

    /// Sans menu Édition, ⌘C ⌘V ⌘X ⌘A ⌘Z ne font rien dans les champs : chaque raccourci vise l'action standard
    /// du premier répondeur (cible nulle), le zoom vise le pont.
    #[test]
    fn menu_principal_complet() {
        let pont: Id = Retained::into_raw(unsafe { msg_send![Pont::class(), new] }) as Id;
        let menu = menu_principal(pont, true, true);
        let mut vus = Vec::new();
        unsafe {
            let n: isize = msg_send![menu, numberOfItems];
            for i in 0..n {
                let haut: Id = msg_send![menu, itemAtIndex: i];
                let sous: Id = msg_send![haut, submenu];
                let m: isize = msg_send![sous, numberOfItems];
                for j in 0..m {
                    let item: Id = msg_send![sous, itemAtIndex: j];
                    let action: Option<Sel> = msg_send![item, action];
                    let cle: Id = msg_send![item, keyEquivalent];
                    let cible: Id = msg_send![item, target];
                    let titre: Id = msg_send![item, title];
                    if let Some(a) = action {
                        vus.push((a.name().to_str().unwrap().to_string(), (*(cle as *const NSString)).to_string(), cible == pont, (*(titre as *const NSString)).to_string()));
                    }
                }
            }
        }
        let attendu = [
            ("terminate:", "q", false, "Quitter Tactile"),
            ("performClose:", "w", false, "Fermer"),
            ("undo:", "z", false, "Annuler"),
            ("redo:", "Z", false, "Rétablir"),
            ("cut:", "x", false, "Couper"),
            ("copy:", "c", false, "Copier"),
            ("paste:", "v", false, "Coller"),
            ("selectAll:", "a", false, "Tout sélectionner"),
            ("agrandir:", "+", true, "Agrandir"),
            ("agrandir:", "=", true, "Agrandir"),
            ("reduire:", "-", true, "Réduire"),
            ("tailleReelle:", "0", true, "Taille réelle"),
        ];
        assert_eq!(vus, attendu.map(|(a, c, p, t)| (a.to_string(), c.to_string(), p, t.to_string())));
        // Le pont sait répondre au zoom (sinon le menu le griserait).
        let repond: Bool = unsafe { msg_send![pont, respondsToSelector: sel!(agrandir:)] };
        assert!(repond.as_bool());
        // Hors `--personnaliser`, ⌘Q ne quitte pas la barre depuis la fenêtre.
        let barre_seule = menu_principal(pont, false, false);
        let app: Id = unsafe { msg_send![barre_seule, itemAtIndex: 0isize] };
        let sous: Id = unsafe { msg_send![app, submenu] };
        let n: isize = unsafe { msg_send![sous, numberOfItems] };
        assert_eq!(n, 0);
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
    #[cfg_attr(ci, ignore = "SF Symbol rendering: no graphics session on a CI runner")]
    fn un_symbole_inconnu_est_refuse() {
        assert!(glyphe("play.fill").is_some_and(|g| g.starts_with("data:image/png;base64,")));
        assert!(glyphe("truc.machin").is_none());
        let barre = config::lire("[[element]]\ntype = \"bouton\"\nsymbole = \"truc.machin\"\n").unwrap();
        let erreur = verifier(&barre.elements).unwrap_err();
        assert!(erreur.contains("truc.machin"), "{erreur}");
    }

    #[test]
    #[cfg_attr(ci, ignore = "AppKit rendering into a bitmap context: no graphics session on a CI runner")]
    fn icone_d_app_pour_l_apercu() {
        let safari = icone_app("Safari").unwrap();
        assert!(safari.starts_with("data:image/png;base64,iVBOR"), "{}", &safari[..40]);
        assert_eq!(icone_app("com.apple.Safari"), Some(safari));
        assert_eq!(icone_app("AppQuiNexistePas"), None);
        assert_eq!(icone_app("  "), None);
    }

    #[test]
    fn revenir_sans_historique_ou_sur_une_version_abimee() {
        let dossier = dossier_essai("revenir");
        std::fs::create_dir_all(&dossier).unwrap();
        let chemin = dossier.join("barre.toml");
        assert_eq!(revenir(&chemin).unwrap_err(), "pas de version précédente");
        // Une version précédente abîmée : refusée, le fichier reste tel quel.
        std::fs::write(&chemin, "[[element]]\ntype = \"souple\"\n").unwrap();
        std::fs::write(version(&chemin, 1), "[[element]]\ntype = \"truc\"\n").unwrap();
        assert!(revenir(&chemin).unwrap_err().starts_with("la version précédente est illisible, ligne 2"));
        assert_eq!(std::fs::read_to_string(&chemin).unwrap(), "[[element]]\ntype = \"souple\"\n");
        // Le fichier a disparu : la version précédente le remplace.
        std::fs::remove_file(&chemin).unwrap();
        std::fs::write(version(&chemin, 1), "[[element]]\ntype = \"heure\"\n").unwrap();
        revenir(&chemin).unwrap();
        assert_eq!(std::fs::read_to_string(&chemin).unwrap(), "[[element]]\ntype = \"heure\"\n");
        assert!(enregistrer(&chemin, json!({ "element": "pas une liste" })).unwrap_err().starts_with("barre illisible"));
        assert!(enregistrer(&chemin, json!([1, 2])).unwrap_err().starts_with("barre illisible"));
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    /// Par propriétés : pour 150 barres générées, `enregistrer` écrit exactement la barre (relue à l'identique) et
    /// garde l'ancien fichier dans `.precedent`, ou refuse et ne touche à rien. Sans symbole SF : pas de rendu AppKit en CI.
    #[test]
    fn enregistrer_des_barres_generees() {
        let dossier = dossier_essai("generees");
        let chemin = dossier.join("barre.toml");
        enregistrer(&chemin, json!({ "element": [{ "type": "souple" }] })).unwrap();
        let (mut ecrites, mut refusees) = (0, 0);
        for graine in 0..150 {
            let mut barre = crate::essais::Hasard::new(graine).barre();
            fn sans_symbole(elements: &mut [Element]) {
                for e in elements {
                    match e {
                        Element::Bouton { symbole, .. } | Element::Slider { symbole, .. } => *symbole = None,
                        Element::Groupe { symbole, elements, .. } => {
                            *symbole = None;
                            sans_symbole(elements);
                        }
                        _ => {}
                    }
                }
            }
            sans_symbole(&mut barre.elements);
            barre.applications.iter_mut().for_each(|a| sans_symbole(&mut a.elements));
            let avant = std::fs::read(&chemin).unwrap();
            match enregistrer(&chemin, serde_json::to_value(&barre).unwrap()) {
                Ok(_) => {
                    ecrites += 1;
                    assert_eq!(config::lire(&std::fs::read_to_string(&chemin).unwrap()).unwrap(), barre, "graine {graine}");
                    assert_eq!(std::fs::read(version(&chemin, 1)).unwrap(), avant, "graine {graine}");
                }
                Err(e) => {
                    refusees += 1;
                    assert_eq!(std::fs::read(&chemin).unwrap(), avant, "graine {graine} refusée ({e}) mais le fichier a changé");
                }
            }
        }
        assert!(ecrites > 20 && refusees > 20, "{ecrites} écrites, {refusees} refusées");
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    /// Un nombre du fichier passe par la page (JSON) et revient au bit près : serde_json, par défaut, se trompe parfois d'un cran.
    #[test]
    fn un_nombre_traverse_le_json_au_bit_pres() {
        for largeur in [508.278_594_479_536_37_f64, 0.1, 59.761, 1e6 / 3.0] {
            let barre = Barre { masquer_control_strip: true, elements: vec![Element::Espace { largeur }], applications: vec![], sources: vec![] };
            let json = serde_json::to_string(&barre).unwrap();
            assert_eq!(serde_json::from_str::<Barre>(&json).unwrap(), barre, "{json}");
        }
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
