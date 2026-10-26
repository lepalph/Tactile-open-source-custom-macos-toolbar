mod actions;
mod barre;
mod config;
mod editeur;
mod menu;

use std::path::PathBuf;

const AIDE: &str = "Tactile : ta Touch Bar, décrite dans ~/.config/tactile/barre.toml.

Usage :
  tactile                        lance la barre (icône dans la barre des menus)
  tactile --personnaliser        ouvre la fenêtre « Personnaliser ma barre », sans la Touch Bar
  tactile --verifier [fichier]   vérifie un fichier de barre sans toucher la Touch Bar
  tactile --version              affiche la version
  tactile --help                 affiche cette aide
Variables : TACTILE_CONFIG=fichier, TACTILE_OUVRIR=k, TACTILE_MASQUER=1, TACTILE_RETOUR=1, TACTILE_AUTOTEST=1 (voir CONTRIBUTING.md).
Journal : ~/Library/Logs/Tactile.log";

/// Ce que demande la ligne de commande.
#[derive(Debug, PartialEq)]
enum Commande {
    Lancer,
    Aide,
    Version,
    Verifier(Option<PathBuf>),
    Personnaliser,
    Inconnue(String),
}

fn commande(args: &[String]) -> Commande {
    match args {
        [] => Commande::Lancer,
        [a] if a == "--help" || a == "-h" => Commande::Aide,
        [a] if a == "--version" => Commande::Version,
        [a] if a == "--verifier" => Commande::Verifier(None),
        [a] if a == "--personnaliser" => Commande::Personnaliser,
        [a, fichier] if a == "--verifier" => Commande::Verifier(Some(PathBuf::from(fichier))),
        [a, ..] => Commande::Inconnue(a.clone()),
    }
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
        Commande::Aide => return println!("{AIDE}"),
        Commande::Version => return println!("tactile {}", env!("CARGO_PKG_VERSION")),
        Commande::Verifier(fichier) => match config::verifier(&fichier.unwrap_or_else(config::chemin)) {
            Ok(barre) => return println!("OK : {} éléments", barre.elements.len()),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        },
        // La fenêtre « Personnaliser ma barre » seule, sans toucher à la Touch Bar.
        Commande::Personnaliser => return editeur::lancer_seul(),
        Commande::Inconnue(a) => {
            eprintln!("tactile : argument inconnu « {a} »\n\n{AIDE}");
            std::process::exit(2);
        }
    }
    journal_si_sortie_perdue();
    let (envoi, recharges) = std::sync::mpsc::channel();
    // Un fichier invalide au lancement : barre par défaut, et l'erreur suit le même chemin qu'au rechargement.
    let config = config::charger().unwrap_or_else(|e| {
        let _ = envoi.send(Err(e));
        config::lire(include_str!("../exemples/barre.toml")).expect("exemple invalide")
    });
    config::surveiller(move |barre| {
        let _ = envoi.send(barre);
    });
    barre::lancer(config, recharges);
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
        assert_eq!(commande(&args(&["--truc"])), Commande::Inconnue("--truc".into()));
        assert_eq!(commande(&args(&["--help", "x"])), Commande::Inconnue("--help".into()));
    }

    #[test]
    fn aide_en_dix_lignes() {
        assert_eq!(AIDE.lines().count(), 10);
    }
}
