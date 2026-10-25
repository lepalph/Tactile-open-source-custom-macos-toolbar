mod actions;
mod barre;
mod config;
mod menu;

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
    let chemin = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("Library/Logs/Tactile.log");
    if let Ok(fichier) = std::fs::OpenOptions::new().create(true).append(true).open(chemin) {
        unsafe { dup2(fichier.as_raw_fd(), 2) };
        std::mem::forget(fichier);
    }
}

fn main() {
    journal_si_sortie_perdue();
    let config = config::charger().unwrap_or_else(|e| {
        eprintln!("tactile : {e} ; barre par défaut");
        config::lire(include_str!("../exemples/barre.toml")).expect("exemple invalide")
    });
    let (envoi, recharges) = std::sync::mpsc::channel();
    config::surveiller(move |barre| {
        let _ = envoi.send(barre);
    });
    barre::lancer(config, recharges);
}

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}
