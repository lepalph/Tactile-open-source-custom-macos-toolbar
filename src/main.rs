mod actions;
mod barre;
mod config;
mod menu;

/// Lancée depuis le Finder, l'app n'a pas de terminal : ses messages vont dans ~/Library/Logs/Tactile.log.
fn journal_si_pas_de_terminal() {
    use std::io::IsTerminal;
    use std::os::fd::AsRawFd;
    unsafe extern "C" {
        fn dup2(source: i32, cible: i32) -> i32;
    }
    if std::io::stderr().is_terminal() {
        return;
    }
    let chemin = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("Library/Logs/Tactile.log");
    if let Ok(fichier) = std::fs::OpenOptions::new().create(true).append(true).open(chemin) {
        unsafe { dup2(fichier.as_raw_fd(), 2) };
        std::mem::forget(fichier);
    }
}

fn main() {
    journal_si_pas_de_terminal();
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
