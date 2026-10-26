//! Autotest (`TACTILE_AUTOTEST=1`) : appuie sur chaque contrôle par le même chemin qu'un doigt
//! et vérifie l'effet réel. Une étape toutes les 2,5 s, pour laisser `scripts/verifier.sh` capturer
//! la barre entre deux. Écrit « AUTOTEST n ok|ÉCHEC … » sur la sortie d'erreur, puis « AUTOTEST fin ».

use objc2::runtime::AnyObject;
use objc2::{msg_send, sel};
use std::cell::Cell;

use super::{cible, modale, vues, Id, APP_SIMULEE, COURANTE, ETAT, PREFIXE};
use crate::actions::{self, luminosite, media, volume, Action};

thread_local! {
    static ETAPE: Cell<usize> = const { Cell::new(0) };
    static ECHECS: Cell<usize> = const { Cell::new(0) };
    static LECTURE: Cell<Option<bool>> = const { Cell::new(None) };
    static CONVERSATION: Cell<usize> = const { Cell::new(0) };
    /// Relances d'une étape qui attend une condition (`patienter`).
    static REPRISES: Cell<usize> = const { Cell::new(0) };
    static SERVEUR_HTTP: std::cell::RefCell<Option<std::process::Child>> = const { std::cell::RefCell::new(None) };
    static PID_MCP: Cell<i32> = const { Cell::new(0) };
    static CONFIG_ORIGINALE: std::cell::RefCell<Option<Vec<u8>>> = const { std::cell::RefCell::new(None) };
}

/// Port du petit serveur HTTP local des étapes 31-35.
const PORT_HTTP: u16 = 47811;

const ETAT_VOCAL: &str = "/tmp/claude-voix/etat";

/// Barre des étapes 22-26 : script JSON (vert, en tête : c'est ce que verifier.sh reconnaît), image, groupe large,
/// batterie, raccourci sans effet (f15), lanceur d'une app inoffensive et AppleScript qui la ferme.
const BARRE_RICHE: &str = r##"
[[element]]
type = "script"
commande = '''echo '{"texte":"ok","couleur":"#00AA00"}' '''
intervalle = 5
largeur = 60

[[element]]
type = "bouton"
texte = "Pr"
image = "/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/GenericApplicationIcon.icns"
couleur_texte = "#FFFFFF"
taille = 15

[[element]]
type = "groupe"
texte = "Export"
largeur = 140

[[element]]
type = "batterie"

[[element]]
type = "bouton"
symbole = "keyboard"
action = { touche = "f15" }

[[element]]
type = "lanceur"
app = "TextEdit"
texte = "TextEdit"

[[element]]
type = "bouton"
texte = "Quit"
action = { applescript = 'tell application "TextEdit" to quit' }
"##;

/// Barre d'application des étapes 26-30 : un bouton bleu en tête (ce que verifier.sh reconnaît). Le nom
/// affiché en minuscules : la casse ne compte pas.
const BARRE_APPLI: &str = r##"
[[application]]
app = "textedit"

[[application.element]]
type = "bouton"
texte = "Appli TextEdit"
couleur = "#0050C8"
largeur = 300
"##;

/// Barre des étapes 31-38 : le script vert en tête (reconnu par verifier.sh comme « riche »), trois `texte`
/// liés à une commande, à un serveur HTTP local et au serveur MCP d'exemple, un logo monochrome, deux actions
/// système non destructives et deux captures (construites, jamais appuyées).
fn barre_sources(dossier: &std::path::Path) -> String {
    let d = dossier.display();
    let mcp = concat!(env!("CARGO_MANIFEST_DIR"), "/exemples/mcp/horloge-mcp.py");
    let logo = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icone-1024.png");
    format!(r##"
[[element]]
type = "script"
commande = '''echo '{{"texte":"ok","couleur":"#00AA00"}}' '''
intervalle = 5
largeur = 60

[[element]]
type = "texte"
source = "commande"
gabarit = "val {{texte}}"
largeur = 90

[[element]]
type = "texte"
source = "http"
gabarit = "{{t}} °C"
largeur = 90

[[element]]
type = "texte"
source = "mcp"
gabarit = "MCP {{heure}}"
largeur = 130

[[element]]
type = "bouton"
texte = "Logo"
image = "{logo}"
gabarit = true
affichage = "icone"

[[element]]
type = "bouton"
symbole = "rectangle.3.group"
action = "mission_control"

[[element]]
type = "bouton"
symbole = "menubar.dock.rectangle"
action = "bureau"

[[element]]
type = "bouton"
symbole = "camera.viewfinder"
action = "capture_ecran"

[[element]]
type = "bouton"
symbole = "camera.metering.matrix"
action = "capture_zone"

[[source]]
nom = "commande"
commande = "cat {d}/valeur.txt"
toutes_les = 5

[[source]]
nom = "http"
http = "http://127.0.0.1:{PORT_HTTP}/valeur.json"
toutes_les = 5

[[source]]
nom = "mcp"
mcp = {{ commande = "/usr/bin/python3 {mcp}", outil = "heure" }}
toutes_les = 5
"##)
}

/// Barre importée aux étapes 39-40 : un bouton orange avec un logo (exportée en `.zip`).
fn barre_a_importer(logo: &std::path::Path) -> String {
    format!("[[element]]\ntype = \"bouton\"\ntexte = \"Importée\"\nimage = \"{}\"\ncouleur = \"#FF8C00\"\nlargeur = 300\n", logo.display())
}

fn dossier_essai() -> std::path::PathBuf {
    std::env::temp_dir().join("tactile-autotest")
}

pub(super) fn programmer() {
    programmer_dans(2.5);
}

fn programmer_dans(secondes: f64) {
    unsafe {
        let _: () = msg_send![cible(), performSelector: sel!(etapeAutotest:), withObject: std::ptr::null_mut::<AnyObject>(), afterDelay: secondes];
    }
}

/// Étape qui attend une condition : rejouée toutes les 0,5 s (sans nouvelle capture) jusqu'à `secondes`.
/// `false` : l'étape est reprogrammée, ne rien faire de plus.
fn patienter(n: usize, pret: bool, secondes: f64) -> bool {
    if pret || REPRISES.get() as f64 * 0.5 >= secondes {
        REPRISES.set(0);
        return true;
    }
    REPRISES.set(REPRISES.get() + 1);
    ETAPE.set(n);
    programmer_dans(0.5);
    false
}

/// L'action système d'un bouton, telle qu'un appui la déclencherait.
fn action_de(nom: &str) -> Option<Action> {
    let index = super::etiquette(controle(nom)?) as usize;
    ETAT.with_borrow(|e| match e.as_ref()?.commandes.get(index)?.0.clone()? {
        super::Commande::Systeme(a) => Some(a),
        _ => None,
    })
}

/// Pid du serveur MCP d'exemple lancé par cette barre (enfant direct : `zsh -c`, remplacé par python).
fn pid_mcp() -> i32 {
    std::process::Command::new("/usr/bin/pgrep").args(["-P", &std::process::id().to_string(), "-f", "horloge-mcp"]).output()
        .ok().and_then(|s| String::from_utf8_lossy(&s.stdout).lines().next().and_then(|l| l.trim().parse().ok())).unwrap_or(0)
}

/// `tactile <args>` (ce binaire), avec `TACTILE_CONFIG` donné ou hérité ; vrai si le code de sortie est 0.
fn cli(args: &[&str], config: Option<&std::path::Path>) -> bool {
    let Ok(exe) = std::env::current_exe() else { return false };
    let mut c = std::process::Command::new(exe);
    c.args(args).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    if let Some(config) = config {
        c.env("TACTILE_CONFIG", config);
    }
    c.status().is_ok_and(|s| s.success())
}

fn controle(nom: &str) -> Option<Id> {
    ETAT.with_borrow(|e| e.as_ref()?.controles.iter().find(|(n, _)| n == nom).map(|(_, id)| *id))
}

fn principale() -> Id {
    ETAT.with_borrow(|e| e.as_ref().map(|e| e.barres[0]).unwrap_or(std::ptr::null_mut()))
}

fn appli() -> Id {
    ETAT.with_borrow(|e| e.as_ref().and_then(|e| e.applis.first().copied()).unwrap_or(std::ptr::null_mut()))
}

fn noter(n: usize, ok: bool, quoi: &str) {
    if !ok {
        ECHECS.set(ECHECS.get() + 1);
    }
    eprintln!("AUTOTEST {n} {} {quoi}", if ok { "ok" } else { "ÉCHEC" });
}

fn cliquer(n: usize, nom: &str) -> bool {
    match controle(nom) {
        Some(b) => {
            unsafe { let _: () = msg_send![b, performClick: std::ptr::null_mut::<AnyObject>()]; }
            true
        }
        None => {
            noter(n, false, &format!("contrôle « {nom} » introuvable"));
            false
        }
    }
}

/// Règle un slider comme un doigt : nouvelle valeur, puis le message que la Touch Bar envoie.
fn glisser(nom: &str, valeur: f32) -> bool {
    let Some(item) = controle(nom) else { return false };
    unsafe {
        let slider: Id = msg_send![item, slider];
        let _: () = msg_send![slider, setDoubleValue: valeur as f64];
        let _: () = msg_send![cible(), glisse: item];
    }
    true
}

fn titre(nom: &str) -> String {
    controle(nom).map(|b| unsafe { let t: Id = msg_send![b, title]; (*(t as *const objc2_foundation::NSString)).to_string() }).unwrap_or_default()
}

fn a_une_image(nom: &str) -> bool {
    controle(nom).is_some_and(|b| { let i: Id = unsafe { msg_send![b, image] }; !i.is_null() })
}

/// Le badge « ⚠ config » est-il dans ce que montre la barre système ?
fn badge_montre() -> bool {
    unsafe {
        let idents: Id = msg_send![modale(), defaultItemIdentifiers];
        msg_send![idents, containsObject: &*vues::texte(&format!("{PREFIXE}.erreur"))]
    }
}

fn textedit_ouvert() -> bool {
    std::process::Command::new("/usr/bin/pgrep").args(["-x", "TextEdit"]).output().is_ok_and(|s| s.status.success())
}

/// Le lanceur de TextEdit montre-t-il l'icône avec le point « ouverte » ?
fn point_de_textedit() -> bool {
    let image: Id = controle("TextEdit").map_or(std::ptr::null_mut(), |b| unsafe { msg_send![b, image] });
    !image.is_null() && Some(image) == vues::icone_app("/System/Applications/TextEdit.app", true)
}

fn etat_vocal() -> String {
    std::fs::read_to_string(ETAT_VOCAL).unwrap_or_default()
}

fn lignes_conversation() -> usize {
    std::fs::read_to_string("/tmp/claude-voix/conversation.md").map(|t| t.lines().count()).unwrap_or(0)
}

fn verifier_ecoute(n: usize) {
    let e = etat_vocal();
    noter(n, e.starts_with('🎙'), &format!("Claude écoute (état « {e} »)"));
}

fn verifier_annule(n: usize, quoi: &str) {
    let (e, lignes) = (etat_vocal(), lignes_conversation());
    noter(n, e.is_empty() && lignes == CONVERSATION.get(), &format!("{quoi} (état « {e} », conversation inchangée : {})", lignes == CONVERSATION.get()));
}

fn cliquer_indicateur(n: usize) -> bool {
    let nom = ETAT.with_borrow(|e| e.as_ref()?.controles.iter().find(|(c, _)| c.contains("claude-voix/etat")).map(|(c, _)| c.clone()));
    match nom {
        Some(nom) => cliquer(n, &nom),
        None => { noter(n, false, "indicateur de Claude vocal introuvable"); false }
    }
}

/// Même chemin qu'un appui long reconnu : la commande longue du bouton.
fn appui_long(n: usize, nom: &str) -> bool {
    match controle(nom) {
        Some(b) => { super::declencher(super::etiquette(b), true); true }
        None => { noter(n, false, &format!("contrôle « {nom} » introuvable")); false }
    }
}

fn verifier_slider(n: usize, nom: &str, lire: fn() -> Result<f32, String>, vers_le_bas: bool) {
    let Ok(origine) = lire() else { return noter(n, false, &format!("{nom} illisible")) };
    let cible_ = if vers_le_bas || origine > 0.5 { (origine - 0.1).max(0.0) } else { origine + 0.1 };
    glisser(&format!("slider {nom}"), cible_);
    let relue = lire().unwrap_or(-1.0);
    glisser(&format!("slider {nom}"), origine);
    let restauree = lire().unwrap_or(-1.0);
    noter(n, (relue - cible_).abs() < 0.03 && (restauree - origine).abs() < 0.03,
        &format!("slider {nom} : {origine:.2} → {cible_:.2}, relu {relue:.2}, restauré {restauree:.2}"));
}

/// Écran qui doit être visible après l'action de l'étape n (comparé aux captures par scripts/verifier.sh).
fn attendu(n: usize) -> &'static str {
    match n {
        1 | 2 | 4 | 5 => "sous",
        7 | 8 => "apple",
        20 => "erreur",
        22..=25 => "riche",
        26 => "sous",
        27 => "application",
        28 | 29 => "erreur",
        31..=38 => "riche",
        39 => "importee",
        _ => "principale",
    }
}

pub(super) fn etape() {
    let n = ETAPE.get();
    ETAPE.set(n + 1);
    if REPRISES.get() == 0 {
        eprintln!("AUTOTEST {n} pas attendu={}", attendu(n));
    }
    let suivante = match n {
        0 => { noter(n, COURANTE.get() == principale(), "barre principale présentée"); true }
        1 => cliquer(n, "ouvrir sun.max.fill"),
        2 => { noter(n, COURANTE.get() != principale(), "sous-barre luminosité ouverte"); verifier_slider(n, "luminosite", luminosite::lire, false); true }
        3 => cliquer(n, "fermer"),
        4 => { noter(n, COURANTE.get() == principale(), "✕ ramène la barre principale"); cliquer(n, "ouvrir speaker.wave.2.fill") }
        5 => { noter(n, COURANTE.get() != principale(), "sous-barre volume ouverte"); verifier_slider(n, "volume", volume::lire, true); true }
        6 => cliquer(n, "fermer"),
        7 => { noter(n, COURANTE.get() == principale(), "✕ ramène la barre principale"); cliquer(n, "apple.logo") }
        8 => { eprintln!("AUTOTEST {n} info barre d'Apple demandée (voir la capture : ✳ doit être dans le Control Strip)"); true }
        9 => { unsafe { let _: () = msg_send![cible(), afficherBarre: std::ptr::null_mut::<AnyObject>()]; } noter(n, COURANTE.get() == principale(), "✳ ramène la barre Tactile"); true }
        10 => match controle("✳ Claude") {
            Some(_) => { CONVERSATION.set(lignes_conversation()); cliquer(n, "✳ Claude") }
            None => { eprintln!("AUTOTEST {n} info pas de bouton ✳ Claude, étapes vocales sautées"); ETAPE.set(17); true }
        },
        11 => { verifier_ecoute(n); cliquer_indicateur(n) }
        12 => { verifier_annule(n, "toucher l'indicateur annule"); cliquer(n, "✳ Claude") }
        13 => { verifier_ecoute(n); appui_long(n, "✳ Claude") }
        14 => { verifier_annule(n, "appui long sur ✳ Claude annule"); cliquer(n, "✳ Claude") }
        15 => { verifier_ecoute(n); cliquer(n, "✳ Claude") }
        16 => { let e = etat_vocal(); noter(n, !e.starts_with('🎙'), &format!("second appui envoie la question (état « {e} »)")); true }
        17 => match media::en_cours() {
            Ok(Some(m)) => { LECTURE.set(Some(m.en_lecture)); cliquer(n, "en_cours") }
            _ => { eprintln!("AUTOTEST {n} info rien ne joue, étapes musique sautées"); ETAPE.set(19); true }
        },
        18 => {
            let avant = LECTURE.get().unwrap_or(false);
            let apres = media::en_cours().ok().flatten().map(|m| m.en_lecture);
            noter(n, apres == Some(!avant), &format!("appui sur le titre : lecture {avant} → {apres:?}"));
            cliquer(n, "en_cours")
        }
        19 => {
            if let Some(avant) = LECTURE.get() {
                let apres = media::en_cours().ok().flatten().map(|m| m.en_lecture);
                noter(n, apres == Some(avant), &format!("second appui : lecture revenue à {avant}"));
            }
            true
        }
        20 => { super::afficher_erreur("essai, ligne 6 : valeur inconnue «truc»"); noter(n, controle("erreur").is_some(), "config refusée : « ⚠ config ligne 6 » dans la barre"); true }
        21 => {
            let config = ETAT.with_borrow(|e| e.as_ref().map(|e| e.config.clone()));
            super::installer(&config.unwrap());
            noter(n, controle("erreur").is_none() && COURANTE.get() == principale(), "config valide : l'erreur disparaît");
            true
        }
        22 => {
            super::installer(&crate::config::lire(BARRE_RICHE).expect("barre riche invalide"));
            let manquants: Vec<&str> = ["Pr", "ouvrir Export", "batterie", "keyboard", "TextEdit", "Quit"].into_iter().filter(|c| controle(c).is_none()).collect();
            noter(n, manquants.is_empty(), &format!("barre riche installée (manquants : {manquants:?})"));
            true
        }
        23 => {
            let script = ETAT.with_borrow(|e| e.as_ref().and_then(|e| e.controles.iter().find(|(c, _)| c.starts_with("script echo")).map(|(c, _)| c.clone()))).unwrap_or_default();
            let fond: Id = controle(&script).map_or(std::ptr::null_mut(), |b| unsafe { msg_send![b, bezelColor] });
            noter(n, titre(&script) == "ok" && !fond.is_null(), &format!("script JSON : texte « {} », fond coloré {}", titre(&script), !fond.is_null()));
            noter(n, a_une_image("Pr") && titre("Pr") == "Pr", "bouton avec image de fichier et texte");
            let pile = titre("batterie");
            noter(n, pile.trim_end_matches(" %").parse::<u8>().is_ok() && a_une_image("batterie"), &format!("batterie « {pile} » avec symbole"));
            noter(n, actions::executer(&Action::Touche(crate::actions::clavier::Touche::try_from("f15".to_string()).unwrap())).is_ok(), "raccourci f15 envoyé");
            noter(n, a_une_image("TextEdit") && !point_de_textedit(), "lanceur : icône de TextEdit, sans point (fermé)");
            cliquer(n, "keyboard") && cliquer(n, "TextEdit")
        }
        24 => {
            noter(n, textedit_ouvert(), "lanceur (action app) : TextEdit ouvert");
            noter(n, point_de_textedit(), "lanceur : point sous l'icône, TextEdit ouvert");
            cliquer(n, "Quit")
        }
        25 => {
            noter(n, !textedit_ouvert(), "action applescript : TextEdit fermé");
            noter(n, !point_de_textedit(), "lanceur : point retiré, TextEdit fermé");
            true
        }
        26 => {
            let config = ETAT.with_borrow(|e| e.as_ref().map(|e| e.config.clone()));
            let mut config = crate::config::charger().unwrap_or_else(|_| config.unwrap());
            config.applications = crate::config::lire(BARRE_APPLI).expect("barre d'application invalide").applications;
            super::installer(&config);
            noter(n, !appli().is_null() && COURANTE.get() == principale(), "barre de TextEdit construite, principale présentée");
            let ouvert = cliquer(n, "ouvrir sun.max.fill");
            if super::app_active().0 == "com.apple.loginwindow" {
                eprintln!("AUTOTEST {n} info écran verrouillé : loginwindow garde le premier plan, TextEdit au premier plan est simulé");
                APP_SIMULEE.set(Some(("com.apple.TextEdit".into(), "TextEdit".into())));
            }
            let lance = std::process::Command::new("/usr/bin/open").args(["-a", "TextEdit"]).status().is_ok_and(|s| s.success());
            noter(n, lance, "open -a TextEdit");
            ouvert
        }
        27 => {
            let courante = COURANTE.get();
            let (bundle, _) = super::app_active();
            noter(n, bundle == "com.apple.TextEdit", &format!("TextEdit au premier plan (vu : « {bundle} »)"));
            noter(n, courante != principale() && courante != appli(), "sous-barre gardée malgré le changement d'app");
            cliquer(n, "fermer")
        }
        28 => {
            noter(n, COURANTE.get() == appli(), "✕ ramène la barre de TextEdit (et non la principale)");
            super::afficher_erreur("essai, ligne 9 : valeur inconnue «batteri»");
            noter(n, COURANTE.get() == appli() && badge_montre(), "config refusée : « ⚠ config ligne 9 » aussi sur la barre de TextEdit");
            true
        }
        29 => {
            let quitte = std::process::Command::new("/usr/bin/osascript").args(["-e", "quit app \"TextEdit\""]).status().is_ok_and(|s| s.success());
            noter(n, quitte, "osascript : quit app \"TextEdit\"");
            APP_SIMULEE.set(None);
            // Sans changement d'app réel (écran verrouillé), aucune notification : on suit l'app comme elle.
            super::suivre_app();
            true
        }
        30 => {
            noter(n, !textedit_ouvert() && COURANTE.get() == principale(), "TextEdit fermé : retour à la barre principale");
            let config = ETAT.with_borrow(|e| e.as_ref().map(|e| e.config.clone()));
            super::installer(&crate::config::charger().unwrap_or_else(|_| config.unwrap()));
            noter(n, controle("Quit").is_none() && COURANTE.get() == principale(), "barre de l'utilisateur rétablie");
            true
        }
        31 => {
            let dossier = dossier_essai();
            let _ = std::fs::create_dir_all(&dossier);
            let ecrit = std::fs::write(dossier.join("valeur.txt"), "un").is_ok() && std::fs::write(dossier.join("valeur.json"), r#"{"t": 21.5}"#).is_ok();
            let serveur = std::process::Command::new("/usr/bin/python3")
                .args(["-m", "http.server", &PORT_HTTP.to_string(), "--bind", "127.0.0.1", "--directory"]).arg(&dossier)
                .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
            noter(n, ecrit && serveur.is_ok(), &format!("serveur HTTP local lancé sur le port {PORT_HTTP}"));
            SERVEUR_HTTP.set(serveur.ok());
            super::installer(&crate::config::lire(&barre_sources(&dossier)).expect("barre des sources invalide"));
            let manquants: Vec<&str> = ["texte commande", "texte http", "texte mcp", "Logo", "rectangle.3.group", "menubar.dock.rectangle", "camera.viewfinder", "camera.metering.matrix"]
                .into_iter().filter(|c| controle(c).is_none()).collect();
            noter(n, manquants.is_empty(), &format!("barre des sources installée (manquants : {manquants:?})"));
            true
        }
        32 => {
            let (c, h, m) = (titre("texte commande"), titre("texte http"), titre("texte mcp"));
            let mcp_ok = m.starts_with("MCP ") && m.len() == 12;
            if !patienter(n, c == "val un" && h == "21.5 °C" && mcp_ok, 15.0) { return; }
            noter(n, c == "val un", &format!("source commande : « {c} »"));
            noter(n, h == "21.5 °C", &format!("source http locale : « {h} »"));
            noter(n, mcp_ok, &format!("source mcp : « {m} »"));
            noter(n, std::fs::write(dossier_essai().join("valeur.txt"), "deux").is_ok(), "valeur de la commande changée");
            true
        }
        33 => {
            let c = titre("texte commande");
            if !patienter(n, c == "val deux", 12.0) { return; }
            noter(n, c == "val deux", &format!("source commande relue : « {c} »"));
            let pid = pid_mcp();
            PID_MCP.set(pid);
            if pid > 0 {
                super::tuer_groupe(pid);
            }
            noter(n, pid > 0, &format!("serveur MCP {pid} tué"));
            true
        }
        34 => {
            let m = titre("texte mcp");
            if !patienter(n, m == "⚠ source", 15.0) { return; }
            noter(n, m == "⚠ source", &format!("serveur MCP mort : « {m} »"));
            true
        }
        35 => {
            let m = titre("texte mcp");
            let relance = m.starts_with("MCP ");
            if !patienter(n, relance, 15.0) { return; }
            let pid = pid_mcp();
            noter(n, relance && pid > 0 && pid != PID_MCP.get(), &format!("serveur MCP relancé (pid {} → {pid}) : « {m} »", PID_MCP.get()));
            true
        }
        36 => {
            let image: Id = controle("Logo").map_or(std::ptr::null_mut(), |b| unsafe { msg_send![b, image] });
            let gabarit = !image.is_null() && unsafe { msg_send![image, isTemplate] };
            // 1 = NSImageOnly : le titre n'est pas dessiné.
            let seule = controle("Logo").is_some_and(|b| { let p: isize = unsafe { msg_send![b, imagePosition] }; p == 1 });
            noter(n, gabarit && seule, "logo perso : image de fichier monochrome (gabarit), seule");
            for (nom, voulue) in [("camera.viewfinder", Action::CaptureEcran), ("camera.metering.matrix", Action::CaptureZone), ("rectangle.3.group", Action::MissionControl), ("menubar.dock.rectangle", Action::Bureau)] {
                let vue = action_de(nom);
                noter(n, vue.as_ref() == Some(&voulue), &format!("bouton {nom} → {vue:?}"));
            }
            noter(n, actions::executer(&Action::MissionControl).is_ok(), "mission_control ouvert (open -b)");
            true
        }
        37 => {
            noter(n, actions::executer(&Action::MissionControl).is_ok(), "mission_control refermé");
            noter(n, actions::executer(&Action::Bureau).is_ok(), "bureau montré (open -b … 1)");
            true
        }
        38 => {
            noter(n, actions::executer(&Action::Bureau).is_ok(), "bureau rétabli");
            if std::env::var_os("TACTILE_CONFIG").is_none() {
                // L'import remplace le fichier de config : jamais celui de l'utilisateur.
                eprintln!("AUTOTEST {n} info sans TACTILE_CONFIG, import sauté (scripts/verifier.sh le fixe)");
                ETAPE.set(41);
            }
            true
        }
        39 => {
            let dossier = dossier_essai().join("import");
            let _ = std::fs::create_dir_all(&dossier);
            let logo = dossier.join("logo.png");
            let source = dossier.join("barre.toml");
            let zip = dossier.join("barre.zip");
            let _ = std::fs::remove_file(&zip);
            let pret = std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icone-1024.png"), &logo).is_ok()
                && std::fs::write(&source, barre_a_importer(&logo)).is_ok();
            let exporte = pret && cli(&["--exporter", &zip.to_string_lossy()], Some(&source));
            let est_zip = std::fs::read(&zip).is_ok_and(|o| o.starts_with(b"PK"));
            noter(n, exporte && est_zip, "tactile --exporter : barre à logo exportée en .zip");
            CONFIG_ORIGINALE.set(std::fs::read(crate::config::chemin()).ok());
            noter(n, cli(&["--importer", &zip.to_string_lossy(), "--oui"], None), "tactile --importer … --oui");
            true
        }
        40 => {
            let vue = controle("Importée").is_some();
            if !patienter(n, vue, 5.0) { return; }
            noter(n, vue && a_une_image("Importée") && COURANTE.get() == principale(), "barre importée rechargée à chaud, avec son logo");
            let remise = CONFIG_ORIGINALE.take().is_some_and(|o| std::fs::write(crate::config::chemin(), o).is_ok());
            noter(n, remise, "fichier de config d'origine réécrit");
            true
        }
        41 => {
            let partie = controle("Importée").is_none();
            if !patienter(n, partie, 5.0) { return; }
            noter(n, partie, "config d'origine rechargée à chaud");
            true
        }
        42 => {
            if let Some(mut serveur) = SERVEUR_HTTP.take() {
                let _ = serveur.kill();
                let _ = serveur.wait();
            }
            let config = ETAT.with_borrow(|e| e.as_ref().map(|e| e.config.clone()));
            super::installer(&crate::config::charger().unwrap_or_else(|_| config.unwrap()));
            noter(n, controle("texte mcp").is_none() && COURANTE.get() == principale(), "barre de l'utilisateur rétablie");
            let restants = std::process::Command::new("/usr/bin/pgrep").args(["-P", &std::process::id().to_string(), "-f", "horloge-mcp|http.server"]).output().map(|s| s.stdout.is_empty()).unwrap_or(false);
            noter(n, restants, "serveurs MCP et HTTP arrêtés");
            eprintln!("AUTOTEST fin {} échec(s)", ECHECS.get());
            false
        }
        _ => false,
    };
    if suivante {
        programmer();
    }
}
