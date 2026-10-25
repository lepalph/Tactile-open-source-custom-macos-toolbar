mod actions;
mod barre;
mod config;

use actions::Action;
use config::{Barre, Element, Reglage};

fn barre_provisoire() -> Barre {
    let bouton = |symbole: &str, action: Action| Element::Bouton {
        texte: None,
        symbole: Some(symbole.into()),
        couleur: None,
        largeur: Some(56.0),
        action: Some(action),
        appui_long: None,
    };
    let claude = Action::Shell("~/.local/bin/claude-voix".into());
    Barre {
        masquer_control_strip: true,
        elements: vec![
            Element::Slider { reglage: Reglage::Luminosite, symbole: None },
            Element::Slider { reglage: Reglage::Volume, symbole: None },
            bouton("backward.fill", Action::Precedent),
            Element::EnCours { largeur: Some(330.0) },
            bouton("forward.fill", Action::Suivant),
            Element::Heure { format: Some("HH:mm".into()) },
            Element::Script { commande: "cat /tmp/claude-voix/etat 2>/dev/null".into(), intervalle: 0.3, largeur: None, action: Some(claude.clone()) },
            Element::Bouton { texte: Some("✳ Claude".into()), symbole: None, couleur: Some("#B4552F".into()), largeur: Some(120.0), action: Some(claude.clone()), appui_long: Some(claude) },
        ],
    }
}

fn main() {
    barre::lancer(barre_provisoire());
}

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}
