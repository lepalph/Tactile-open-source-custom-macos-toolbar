mod actions;
mod barre;
mod config;

fn main() {
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
