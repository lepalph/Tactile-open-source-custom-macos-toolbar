//! Description de la barre, lue depuis ~/.config/tactile/barre.toml.

use crate::actions::Action;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Barre {
    #[serde(default)]
    pub masquer_control_strip: bool,
    #[serde(rename = "element", default)]
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Element {
    Bouton {
        texte: Option<String>,
        symbole: Option<String>,
        couleur: Option<String>,
        largeur: Option<f64>,
        #[serde(flatten)]
        action: Option<Action>,
        appui_long: Option<Action>,
    },
    Slider {
        reglage: Reglage,
        symbole: Option<String>,
    },
    Heure {
        format: Option<String>,
    },
    EnCours {
        largeur: Option<f64>,
    },
    Script {
        commande: String,
        intervalle: f64,
        largeur: Option<f64>,
        #[serde(flatten)]
        action: Option<Action>,
    },
    Espace {
        largeur: f64,
    },
    Groupe {
        texte: Option<String>,
        symbole: Option<String>,
        #[serde(rename = "element", default)]
        elements: Vec<Element>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reglage {
    Luminosite,
    Volume,
}

pub fn chemin() -> PathBuf {
    let maison = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(maison).join(".config/tactile/barre.toml")
}

pub fn lire(_texte: &str) -> Result<Barre, String> {
    Err("config::lire : à faire".into())
}
