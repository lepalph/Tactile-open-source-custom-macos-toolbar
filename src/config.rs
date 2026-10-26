//! Description de la barre, lue depuis ~/.config/tactile/barre.toml.

use crate::actions::Action;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};

/// Barre utilisée tant que l'utilisateur n'a pas écrit la sienne.
const PAR_DEFAUT: &str = include_str!("../exemples/barre.toml");

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Barre {
    #[serde(default)]
    pub masquer_control_strip: bool,
    #[serde(rename = "element", default)]
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", deny_unknown_fields)]
pub enum Element {
    Bouton {
        texte: Option<String>,
        symbole: Option<String>,
        couleur: Option<String>,
        largeur: Option<f64>,
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
        /// Secondes au bout desquelles la commande est tuée (5 par défaut).
        delai: Option<f64>,
        largeur: Option<f64>,
        action: Option<Action>,
    },
    Espace {
        largeur: f64,
    },
    Souple,
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

pub fn lire(texte: &str) -> Result<Barre, String> {
    toml::from_str(texte).map_err(|e| {
        let message = traduire(e.message());
        let debut = e.span().map(|s| s.start).max(element_fautif(texte));
        match debut.and_then(|d| texte.get(..d)) {
            Some(avant) => format!("ligne {} : {message}", avant.matches('\n').count() + 1),
            None => message,
        }
    })
}

/// Début du premier `[[element]]` invalide : serde ne situe pas les erreurs internes
/// d'un élément (action, champ manquant…) et toml pointe alors le premier élément.
fn element_fautif(texte: &str) -> Option<usize> {
    #[derive(Deserialize)]
    struct Brut {
        element: Vec<toml::Spanned<toml::Table>>,
    }
    let brut: Brut = toml::from_str(texte).ok()?;
    brut.element
        .into_iter()
        .find(|e| {
            toml::Value::Table(e.get_ref().clone())
                .try_into::<Element>()
                .is_err()
        })
        .map(|e| e.span().start)
}

pub fn charger() -> Result<Barre, String> {
    charger_chemin(&chemin())
}

fn charger_chemin(chemin: &Path) -> Result<Barre, String> {
    if matches!(std::fs::metadata(chemin), Err(e) if e.kind() == std::io::ErrorKind::NotFound) {
        return lire(PAR_DEFAUT);
    }
    verifier(chemin)
}

/// Lit un fichier de barre sans repli sur la barre par défaut : `tactile --verifier`.
pub fn verifier(chemin: &Path) -> Result<Barre, String> {
    std::fs::read_to_string(chemin)
        .map_err(|e| format!("impossible de lire {} : {e}", chemin.display()))
        .and_then(|texte| lire(&texte).map_err(|e| format!("{}, {e}", chemin.display())))
}

/// Relit la barre à chaque modification du fichier (vérifiée toutes les secondes).
pub fn surveiller(rappel: impl Fn(Result<Barre, String>) + Send + 'static) -> JoinHandle<()> {
    surveiller_chemin(chemin(), rappel)
}

fn surveiller_chemin(
    chemin: PathBuf,
    rappel: impl Fn(Result<Barre, String>) + Send + 'static,
) -> JoinHandle<()> {
    let mut derniere = date_modification(&chemin);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let actuelle = date_modification(&chemin);
            if actuelle != derniere {
                derniere = actuelle;
                rappel(charger_chemin(&chemin));
            }
        }
    })
}

fn date_modification(chemin: &Path) -> Option<SystemTime> {
    std::fs::metadata(chemin).and_then(|m| m.modified()).ok()
}

/// Met en français les messages de serde les plus courants ; le reste passe tel quel.
fn traduire(message: &str) -> String {
    let mut texte = message.to_string();
    for (anglais, francais) in [
        ("unknown variant", "valeur inconnue"),
        ("unknown field", "champ inconnu"),
        ("missing field", "champ manquant"),
        ("invalid type", "type invalide"),
        ("invalid value", "valeur invalide"),
        ("expected one of", "attendu parmi"),
        ("` or `", "` ou `"),
        ("there are no fields", "aucun champ n'est permis ici"),
        ("expected", "attendu"),
    ] {
        texte = texte.replace(anglais, francais);
    }
    let mut ouvert = false;
    texte
        .chars()
        .map(|c| match c {
            '`' => {
                ouvert = !ouvert;
                if ouvert { '«' } else { '»' }
            }
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bouton(action: Action) -> Element {
        Element::Bouton {
            texte: None,
            symbole: None,
            couleur: None,
            largeur: None,
            action: Some(action),
            appui_long: None,
        }
    }

    #[test]
    fn exemple_donne_la_barre_attendue() {
        let attendu = Barre {
            masquer_control_strip: true,
            elements: vec![
                Element::Bouton {
                    texte: None,
                    symbole: Some("apple.logo".into()),
                    couleur: None,
                    largeur: Some(56.0),
                    action: Some(Action::BarreApple),
                    appui_long: None,
                },
                Element::Slider {
                    reglage: Reglage::Luminosite,
                    symbole: Some("sun.max.fill".into()),
                },
                Element::Slider {
                    reglage: Reglage::Volume,
                    symbole: Some("speaker.wave.2.fill".into()),
                },
                Element::Souple,
                Element::Bouton {
                    texte: None,
                    symbole: Some("backward.fill".into()),
                    couleur: None,
                    largeur: None,
                    action: Some(Action::Precedent),
                    appui_long: None,
                },
                Element::EnCours {
                    largeur: Some(330.0),
                },
                Element::Bouton {
                    texte: None,
                    symbole: Some("forward.fill".into()),
                    couleur: None,
                    largeur: None,
                    action: Some(Action::Suivant),
                    appui_long: None,
                },
                Element::Souple,
                Element::Heure {
                    format: Some("HH:mm".into()),
                },
            ],
        };
        assert_eq!(lire(PAR_DEFAUT), Ok(attendu));
    }

    #[test]
    fn chaque_forme_d_action() {
        for (ecrit, action) in [
            (r#""luminosite_plus""#, Action::LuminositePlus),
            (r#""luminosite_moins""#, Action::LuminositeMoins),
            ("{ luminosite = 0.5 }", Action::Luminosite(0.5)),
            (r#""volume_plus""#, Action::VolumePlus),
            (r#""volume_moins""#, Action::VolumeMoins),
            ("{ volume = 0.3 }", Action::Volume(0.3)),
            ("{ volume = 1 }", Action::Volume(1.0)),
            (r#""basculer_muet""#, Action::BasculerMuet),
            (r#""lecture_pause""#, Action::LecturePause),
            (r#""suivant""#, Action::Suivant),
            (r#""precedent""#, Action::Precedent),
            (r#""veille""#, Action::Veille),
            (
                r#"{ shell = "open -a Safari" }"#,
                Action::Shell("open -a Safari".into()),
            ),
        ] {
            let texte = format!("[[element]]\ntype = \"bouton\"\naction = {ecrit}\n");
            assert_eq!(
                lire(&texte),
                Ok(Barre {
                    masquer_control_strip: false,
                    elements: vec![bouton(action)]
                }),
                "{ecrit}"
            );
        }
    }

    #[test]
    fn type_inconnu() {
        let erreur =
            lire("[[element]]\ntype = \"heure\"\n\n[[element]]\ntype = \"truc\"\n").unwrap_err();
        assert!(
            erreur.starts_with("ligne 5 : valeur inconnue «truc», attendu parmi «bouton»"),
            "{erreur}"
        );
    }

    #[test]
    fn action_inconnue() {
        let erreur = lire("[[element]]\ntype = \"heure\"\n\n[[element]]\ntype = \"bouton\"\naction = \"danser\"\n").unwrap_err();
        assert!(
            erreur.starts_with("ligne 4 : valeur inconnue «danser», attendu parmi"),
            "{erreur}"
        );
    }

    #[test]
    fn champ_mal_orthographie() {
        let erreur = lire("[[element]]\ntype = \"bouton\"\nactoin = \"suivant\"\n").unwrap_err();
        assert!(
            erreur.starts_with("ligne 1 : champ inconnu «actoin»"),
            "{erreur}"
        );
    }

    #[test]
    fn groupe_imbrique() {
        let texte = r##"
masquer_control_strip = true

[[element]]
type = "slider"
reglage = "luminosite"
symbole = "sun.max.fill"

[[element]]
type = "bouton"
texte = "✳ Claude"
couleur = "#B4552F"
largeur = 120
action = { shell = "~/.local/bin/claude-voix" }
appui_long = { shell = "~/.local/bin/claude-voix" }

[[element]]
type = "groupe"
symbole = "speaker.wave.2.fill"
  [[element.element]]
  type = "bouton"
  symbole = "speaker.slash.fill"
  action = "basculer_muet"

  [[element.element]]
  type = "slider"
  reglage = "volume"
"##;
        let barre = lire(texte).unwrap();
        assert!(barre.masquer_control_strip);
        assert_eq!(barre.elements.len(), 3);
        assert_eq!(
            barre.elements[2],
            Element::Groupe {
                texte: None,
                symbole: Some("speaker.wave.2.fill".into()),
                elements: vec![
                    Element::Bouton {
                        texte: None,
                        symbole: Some("speaker.slash.fill".into()),
                        couleur: None,
                        largeur: None,
                        action: Some(Action::BasculerMuet),
                        appui_long: None,
                    },
                    Element::Slider {
                        reglage: Reglage::Volume,
                        symbole: None
                    },
                ],
            }
        );
    }

    #[test]
    fn script_avec_delai() {
        let barre = lire("[[element]]\ntype = \"script\"\ncommande = \"uptime\"\nintervalle = 2\ndelai = 1.5\n").unwrap();
        assert_eq!(
            barre.elements[0],
            Element::Script { commande: "uptime".into(), intervalle: 2.0, delai: Some(1.5), largeur: None, action: None }
        );
    }

    #[test]
    fn fichier_absent_donne_la_barre_par_defaut() {
        assert_eq!(
            charger_chemin(Path::new("/nexiste/pas/barre.toml")),
            lire(PAR_DEFAUT)
        );
    }

    #[test]
    fn verifier_dit_la_ligne_ou_l_absence_du_fichier() {
        let fichier = std::env::temp_dir().join(format!("tactile-verifier-{}.toml", std::process::id()));
        std::fs::write(&fichier, "[[element]]\ntype = \"heure\"\n\n[[element]]\ntype = \"truc\"\n").unwrap();
        let erreur = verifier(&fichier).unwrap_err();
        std::fs::remove_file(&fichier).unwrap();
        assert!(erreur.starts_with(&format!("{}, ligne 5 : valeur inconnue «truc»", fichier.display())), "{erreur}");
        assert!(verifier(&fichier).unwrap_err().starts_with("impossible de lire"));
    }

    #[test]
    fn surveiller_voit_une_modification() {
        let fichier =
            std::env::temp_dir().join(format!("tactile-essai-{}.toml", std::process::id()));
        std::fs::write(&fichier, "").unwrap();
        let (envoi, reception) = std::sync::mpsc::channel();
        surveiller_chemin(fichier.clone(), move |barre| {
            let _ = envoi.send(barre);
        });

        std::thread::sleep(Duration::from_millis(100));
        std::fs::write(&fichier, "masquer_control_strip = true\n").unwrap();
        let barre = reception.recv_timeout(Duration::from_secs(3));
        std::fs::remove_file(&fichier).unwrap();
        assert!(barre.unwrap().unwrap().masquer_control_strip);
    }
}
