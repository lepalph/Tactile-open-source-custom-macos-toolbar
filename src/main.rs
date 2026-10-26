mod actions;
mod barre;
mod config;
mod editeur;
#[cfg(test)]
mod essais;
mod menu;
mod sources;

use std::path::PathBuf;

const AIDE: &str = "Tactile : ta Touch Bar, décrite dans ~/.config/tactile/barre.toml.
Usage :
  tactile                        lance la barre (icône dans la barre des menus)
  tactile --personnaliser        ouvre la fenêtre « Personnaliser ma barre », sans la Touch Bar
  tactile --verifier [fichier]   vérifie un fichier de barre (syntaxe, symboles, apps, largeur)
  tactile --exporter <fichier>   exporte ta barre : un .toml, ou un .zip avec ses logos
  tactile --importer <fichier> [--oui]   montre ce que la barre lance, puis remplace la tienne
  tactile --version | --help     version, cette aide (--check, --customize, --export, --import, --yes marchent aussi)
Variables : TACTILE_CONFIG=fichier, TACTILE_OUVRIR=k, TACTILE_MASQUER=1, TACTILE_RETOUR=1, TACTILE_AUTOTEST=1, TACTILE_MESURE=1 (voir CONTRIBUTING.md).
Journal : ~/Library/Logs/Tactile.log";

const HELP: &str = "Tactile: your Touch Bar, described in ~/.config/tactile/barre.toml.
Usage:
  tactile                        starts the bar (menu bar icon)
  tactile --customize            opens the \"Customize my bar\" window, without the Touch Bar
  tactile --check [file]         checks a bar file (syntax, symbols, apps, width)
  tactile --export <file>        exports your bar: a .toml, or a .zip with its logos
  tactile --import <file> [--yes]   shows what the bar runs, then replaces yours
  tactile --version | --help     version, this help (--verifier, --personnaliser, --exporter, --importer, --oui work too)
Variables: TACTILE_CONFIG=file, TACTILE_OUVRIR=k, TACTILE_MASQUER=1, TACTILE_RETOUR=1, TACTILE_AUTOTEST=1, TACTILE_MESURE=1 (see CONTRIBUTING.md).
Log: ~/Library/Logs/Tactile.log";

/// Le français si la langue du système l'est : `LC_ALL`, `LC_MESSAGES` puis `LANG` (comme les outils Unix),
/// sinon la première langue préférée de macOS (`AppleLanguages`, lue seulement si besoin). Tout le reste : anglais.
fn en_francais(variable: impl Fn(&str) -> Option<String>, langue_macos: impl FnOnce() -> Option<String>) -> bool {
    let locale = ["LC_ALL", "LC_MESSAGES", "LANG"].iter().find_map(|v| variable(v).filter(|l| !l.is_empty() && l != "C" && l != "POSIX"));
    locale.or_else(langue_macos).is_some_and(|l| l.starts_with("fr"))
}

fn aide() -> &'static str {
    let macos = || unsafe {
        let langues: *mut objc2::runtime::AnyObject = objc2::msg_send![objc2::class!(NSLocale), preferredLanguages];
        let premiere: *const objc2_foundation::NSString = objc2::msg_send![langues, firstObject];
        premiere.as_ref().map(|l| l.to_string())
    };
    if en_francais(|v| std::env::var(v).ok(), macos) { AIDE } else { HELP }
}

/// Ce que demande la ligne de commande.
#[derive(Debug, PartialEq)]
enum Commande {
    Lancer,
    Aide,
    Version,
    Verifier(Option<PathBuf>),
    Personnaliser,
    Exporter(PathBuf),
    /// Le fichier, et `--oui` : sans confirmation.
    Importer(PathBuf, bool),
    Inconnue(String),
}

fn commande(args: &[String]) -> Commande {
    match args {
        [] => Commande::Lancer,
        [a] if a == "--help" || a == "-h" => Commande::Aide,
        [a] if a == "--version" => Commande::Version,
        [a] if a == "--verifier" || a == "--check" => Commande::Verifier(None),
        [a] if a == "--personnaliser" || a == "--customize" => Commande::Personnaliser,
        [a, fichier] if a == "--verifier" || a == "--check" => Commande::Verifier(Some(PathBuf::from(fichier))),
        [a, fichier] if (a == "--exporter" || a == "--export") && !fichier.starts_with("--") => Commande::Exporter(PathBuf::from(fichier)),
        [a, fichier] if (a == "--importer" || a == "--import") && !fichier.starts_with("--") => Commande::Importer(PathBuf::from(fichier), false),
        [a, x, y] if a == "--importer" || a == "--import" => match (oui(x), oui(y)) {
            (false, true) if !x.starts_with("--") => Commande::Importer(PathBuf::from(x), true),
            (true, false) if !y.starts_with("--") => Commande::Importer(PathBuf::from(y), true),
            _ => Commande::Inconnue(a.clone()),
        },
        [a, ..] => Commande::Inconnue(a.clone()),
    }
}

fn oui(a: &str) -> bool {
    a == "--oui" || a == "--yes"
}

/// Journal de Tactile quand il n'y a pas de terminal ; « Voir le journal » dans le menu l'ouvre.
pub fn chemin_journal() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("Library/Logs/Tactile.log")
}

/// Lancée depuis le Finder, la sortie d'erreur part dans /dev/null : on l'envoie dans ~/Library/Logs/Tactile.log.
/// Un terminal ou une redirection choisie (script, test) est laissé tel quel.
fn journal_si_sortie_perdue() {
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::MetadataExt;
    unsafe extern "C" {
        fn dup2(source: i32, cible: i32) -> i32;
    }
    let rdev = |chemin: &str| std::fs::metadata(chemin).map(|m| (m.dev(), m.ino(), m.rdev())).ok();
    if rdev("/dev/fd/2") != rdev("/dev/null") {
        return;
    }
    if let Ok(fichier) = std::fs::OpenOptions::new().create(true).append(true).open(chemin_journal()) {
        unsafe { dup2(fichier.as_raw_fd(), 2) };
        std::mem::forget(fichier);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match commande(&args) {
        Commande::Lancer => {}
        Commande::Aide => return println!("{}", aide()),
        Commande::Version => return println!("tactile {}", env!("CARGO_PKG_VERSION")),
        Commande::Verifier(fichier) => std::process::exit(verifier(&fichier.unwrap_or_else(config::chemin))),
        // La fenêtre « Personnaliser ma barre » seule, sans toucher à la Touch Bar.
        Commande::Personnaliser => return editeur::lancer_seul(),
        Commande::Exporter(fichier) => std::process::exit(editeur::exporter_cli(&fichier, aide() == AIDE)),
        Commande::Importer(fichier, oui) => std::process::exit(editeur::importer_cli(&fichier, oui, aide() == AIDE)),
        Commande::Inconnue(a) => {
            let aide = aide();
            let erreur = if aide == AIDE { format!("tactile : argument inconnu « {a} »") } else { format!("tactile: unknown argument \"{a}\"") };
            eprintln!("{erreur}\n\n{aide}");
            std::process::exit(2);
        }
    }
    journal_si_sortie_perdue();
    let (envoi, recharges) = std::sync::mpsc::channel();
    // Un fichier invalide au lancement : barre par défaut, et l'erreur suit le même chemin qu'au rechargement.
    let config = config::charger().inspect(journaliser_avis).unwrap_or_else(|e| {
        let _ = envoi.send(Err(e));
        config::lire(include_str!("../exemples/barre.toml")).expect("exemple invalide")
    });
    config::surveiller(move |barre| {
        // La barre d'abord : les avis (apps, images, symboles) se calculent ensuite, hors du chemin du rechargement.
        let pour_avis = barre.as_ref().ok().cloned();
        let _ = envoi.send(barre);
        barre::reveiller();
        if let Some(b) = pour_avis {
            journaliser_avis(&b);
        }
    });
    barre::lancer(config, recharges);
}

/// `tactile --verifier` : la syntaxe (`config::verifier`), puis le validateur partagé (`config::controler`).
/// Code de sortie : 0 si la barre se lancerait (avertissements « attention : » compris), 1 sinon.
fn verifier(chemin: &std::path::Path) -> i32 {
    let barre = match config::verifier(chemin) {
        Ok(barre) => barre,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let avis = config::controler(&barre, std::fs::read_to_string(chemin).ok().as_deref());
    for a in &avis {
        if a.grave {
            eprintln!("{}, {a}", chemin.display());
        } else {
            eprintln!("attention : {a}");
        }
    }
    if avis.iter().any(|a| a.grave) {
        return 1;
    }
    let applis = match barre.applications.len() {
        0 => String::new(),
        n => format!(", {} d'application", config::nombre(n, "barre")),
    };
    println!("OK : {}{applis}", config::nombre(barre.elements.len(), "élément"));
    0
}

/// Au chargement de la barre, les avis du validateur partagé vont au journal.
fn journaliser_avis(barre: &config::Barre) {
    let texte = std::fs::read_to_string(config::chemin()).ok();
    for a in config::controler(barre, texte.as_deref()) {
        eprintln!("tactile : attention, {a}");
    }
}

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(liste: &[&str]) -> Vec<String> {
        liste.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn chaque_argument() {
        assert_eq!(commande(&args(&[])), Commande::Lancer);
        assert_eq!(commande(&args(&["--help"])), Commande::Aide);
        assert_eq!(commande(&args(&["-h"])), Commande::Aide);
        assert_eq!(commande(&args(&["--version"])), Commande::Version);
        assert_eq!(commande(&args(&["--verifier"])), Commande::Verifier(None));
        assert_eq!(commande(&args(&["--verifier", "x.toml"])), Commande::Verifier(Some("x.toml".into())));
        assert_eq!(commande(&args(&["--personnaliser"])), Commande::Personnaliser);
        // Les alias anglais.
        assert_eq!(commande(&args(&["--check"])), Commande::Verifier(None));
        assert_eq!(commande(&args(&["--check", "x.toml"])), Commande::Verifier(Some("x.toml".into())));
        assert_eq!(commande(&args(&["--customize"])), Commande::Personnaliser);
        assert_eq!(commande(&args(&["--truc"])), Commande::Inconnue("--truc".into()));
        assert_eq!(commande(&args(&["--help", "x"])), Commande::Inconnue("--help".into()));
        assert_eq!(commande(&args(&["--customize", "x"])), Commande::Inconnue("--customize".into()));
        // Exporter et importer, en français et en anglais ; --oui avant ou après le fichier.
        assert_eq!(commande(&args(&["--exporter", "b.zip"])), Commande::Exporter("b.zip".into()));
        assert_eq!(commande(&args(&["--export", "b.toml"])), Commande::Exporter("b.toml".into()));
        assert_eq!(commande(&args(&["--importer", "b.zip"])), Commande::Importer("b.zip".into(), false));
        assert_eq!(commande(&args(&["--import", "b.zip", "--yes"])), Commande::Importer("b.zip".into(), true));
        assert_eq!(commande(&args(&["--importer", "--oui", "b.zip"])), Commande::Importer("b.zip".into(), true));
        assert_eq!(commande(&args(&["--exporter"])), Commande::Inconnue("--exporter".into()));
        assert_eq!(commande(&args(&["--importer", "--oui"])), Commande::Inconnue("--importer".into()));
        assert_eq!(commande(&args(&["--importer", "b.zip", "--truc"])), Commande::Inconnue("--importer".into()));
        assert_eq!(commande(&args(&["--importer", "--oui", "--yes"])), Commande::Inconnue("--importer".into()));
    }

    #[test]
    fn aide_en_dix_lignes() {
        assert!(AIDE.lines().count() <= 10 && HELP.lines().count() <= 10);
        // Chaque option de l'aide existe, dans les deux langues.
        for option in ["--personnaliser", "--customize", "--verifier", "--check", "--version", "--help"] {
            assert!(AIDE.contains(option) && HELP.contains(option), "{option}");
            assert_ne!(commande(&args(&[option])), Commande::Inconnue(option.into()));
        }
        for option in ["--exporter", "--export", "--importer", "--import", "--oui", "--yes"] {
            assert!(AIDE.contains(option) && HELP.contains(option), "{option}");
        }
    }

    #[test]
    fn aide_dans_la_langue_du_systeme() {
        let env = |paires: &'static [(&'static str, &'static str)]| move |v: &str| paires.iter().find(|p| p.0 == v).map(|p| p.1.to_string());
        let jamais = || -> Option<String> { panic!("AppleLanguages lue alors qu'une locale est posée") };
        assert!(en_francais(env(&[("LANG", "fr_FR.UTF-8")]), jamais));
        assert!(!en_francais(env(&[("LANG", "en_US.UTF-8")]), jamais));
        // LC_ALL l'emporte sur LANG ; C et POSIX ne disent rien de la langue.
        assert!(!en_francais(env(&[("LC_ALL", "de_DE.UTF-8"), ("LANG", "fr_FR.UTF-8")]), jamais));
        assert!(en_francais(env(&[("LANG", "C")]), || Some("fr-FR".into())));
        assert!(!en_francais(env(&[]), || Some("en-US".into())));
        assert!(!en_francais(env(&[]), || None));
    }
}
