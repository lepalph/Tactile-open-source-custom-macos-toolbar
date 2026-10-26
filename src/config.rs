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
    /// `true` par défaut : toute la largeur pour la barre, sans le Control Strip d'Apple.
    #[serde(default = "vrai", alias = "hide_control_strip")]
    pub masquer_control_strip: bool,
    #[serde(rename = "element", alias = "item", default)]
    pub elements: Vec<Element>,
}

/// Les clés françaises sont la forme canonique ; chaque clé, type, réglage et action accepte
/// un alias anglais (glossaire dans le README, vérifié par le test `glossaire_du_readme`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", deny_unknown_fields)]
pub enum Element {
    #[serde(alias = "button")]
    Bouton {
        #[serde(alias = "text")]
        texte: Option<String>,
        #[serde(alias = "symbol")]
        symbole: Option<String>,
        /// Chemin d'une image (PNG, PDF, icns…), redimensionnée à 20 pt de haut ; `~` est accepté.
        image: Option<String>,
        /// `true` : l'image est un gabarit, teinté comme un symbole SF.
        #[serde(default, alias = "template")]
        gabarit: bool,
        #[serde(alias = "color")]
        couleur: Option<String>,
        #[serde(alias = "text_color")]
        couleur_texte: Option<String>,
        /// Taille de la police en points.
        #[serde(alias = "size")]
        taille: Option<f64>,
        #[serde(alias = "display")]
        affichage: Option<Affichage>,
        #[serde(alias = "width")]
        largeur: Option<f64>,
        action: Option<Action>,
        #[serde(alias = "long_press")]
        appui_long: Option<Action>,
    },
    Slider {
        #[serde(alias = "setting")]
        reglage: Reglage,
        #[serde(alias = "symbol")]
        symbole: Option<String>,
    },
    #[serde(alias = "clock")]
    Heure {
        format: Option<String>,
    },
    #[serde(alias = "now_playing")]
    EnCours {
        #[serde(alias = "width")]
        largeur: Option<f64>,
    },
    Script {
        #[serde(alias = "command")]
        commande: String,
        #[serde(alias = "interval")]
        intervalle: f64,
        /// Secondes au bout desquelles la commande est tuée (5 par défaut).
        #[serde(alias = "timeout")]
        delai: Option<f64>,
        #[serde(alias = "width")]
        largeur: Option<f64>,
        action: Option<Action>,
    },
    #[serde(alias = "space")]
    Espace {
        #[serde(alias = "width")]
        largeur: f64,
    },
    #[serde(alias = "flexible")]
    Souple,
    #[serde(alias = "group")]
    Groupe {
        #[serde(alias = "text")]
        texte: Option<String>,
        #[serde(alias = "symbol")]
        symbole: Option<String>,
        #[serde(alias = "width")]
        largeur: Option<f64>,
        #[serde(rename = "element", alias = "item", default)]
        elements: Vec<Element>,
    },
    #[serde(alias = "battery")]
    Batterie {
        /// `true` par défaut : un symbole de pile à côté du pourcentage ; `false` : texte seul, ⚡ en charge.
        #[serde(alias = "symbol")]
        symbole: Option<bool>,
        #[serde(alias = "width")]
        largeur: Option<f64>,
    },
}

/// Ce qu'un bouton montre quand il a un symbole (ou une image) et un texte.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Affichage {
    #[serde(alias = "icon")]
    Icone,
    #[serde(alias = "text")]
    Texte,
    #[serde(alias = "both")]
    LesDeux,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reglage {
    #[serde(alias = "brightness")]
    Luminosite,
    Volume,
}

fn vrai() -> bool {
    true
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
        #[serde(alias = "item", default)]
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
        bouton_symbole(None, None, action)
    }

    fn bouton_symbole(symbole: Option<&str>, largeur: Option<f64>, action: Action) -> Element {
        Element::Bouton {
            texte: None,
            symbole: symbole.map(String::from),
            image: None,
            gabarit: false,
            couleur: None,
            couleur_texte: None,
            taille: None,
            affichage: None,
            largeur,
            action: Some(action),
            appui_long: None,
        }
    }

    #[test]
    fn exemple_donne_la_barre_attendue() {
        let attendu = Barre {
            masquer_control_strip: true,
            elements: vec![
                bouton_symbole(Some("apple.logo"), Some(56.0), Action::BarreApple),
                Element::Slider {
                    reglage: Reglage::Luminosite,
                    symbole: Some("sun.max.fill".into()),
                },
                Element::Slider {
                    reglage: Reglage::Volume,
                    symbole: Some("speaker.wave.2.fill".into()),
                },
                Element::Souple,
                bouton_symbole(Some("backward.fill"), None, Action::Precedent),
                Element::EnCours {
                    largeur: Some(330.0),
                },
                bouton_symbole(Some("forward.fill"), None, Action::Suivant),
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
            (r#""echap""#, Action::Echap),
            (
                r#"{ touche = "cmd+shift+4" }"#,
                Action::Touche(crate::actions::clavier::Touche::try_from("cmd+shift+4".to_string()).unwrap()),
            ),
            ("{ keycode = 53 }", Action::Keycode(53)),
            (r#"{ app = "Figma" }"#, Action::App("Figma".into())),
            (r#"{ url = "https://example.org" }"#, Action::Url("https://example.org".into())),
            (
                r#"{ applescript = 'tell application "Music" to playpause' }"#,
                Action::Applescript("tell application \"Music\" to playpause".into()),
            ),
        ] {
            let texte = format!("[[element]]\ntype = \"bouton\"\naction = {ecrit}\n");
            assert_eq!(
                lire(&texte),
                Ok(Barre {
                    masquer_control_strip: true,
                    elements: vec![bouton(action)]
                }),
                "{ecrit}"
            );
        }
    }

    /// Tout le vocabulaire, une fois en français et une fois en anglais : les deux doivent donner la même barre.
    const TOUT_FR: &str = r##"
masquer_control_strip = false

[[element]]
type = "bouton"
texte = "Pr"
symbole = "scissors"
image = "~/pr.png"
gabarit = true
couleur = "#2D0A4E"
couleur_texte = "#FFFFFF"
taille = 13
affichage = "les_deux"
largeur = 90
action = { touche = "cmd+k" }
appui_long = { keycode = 53 }

[[element]]
type = "bouton"
affichage = "icone"
action = "luminosite_plus"
appui_long = "luminosite_moins"

[[element]]
type = "bouton"
affichage = "texte"
action = { luminosite = 0.5 }
appui_long = "volume_plus"

[[element]]
type = "bouton"
action = "volume_moins"
appui_long = { volume = 0.3 }

[[element]]
type = "bouton"
action = "basculer_muet"
appui_long = "lecture_pause"

[[element]]
type = "bouton"
action = "suivant"
appui_long = "precedent"

[[element]]
type = "bouton"
action = "veille"
appui_long = "echap"

[[element]]
type = "bouton"
action = "barre_apple"
appui_long = { shell = "uptime" }

[[element]]
type = "bouton"
action = { app = "Figma" }
appui_long = { url = "https://example.org" }

[[element]]
type = "bouton"
action = { applescript = "beep" }

[[element]]
type = "slider"
reglage = "luminosite"
symbole = "sun.max.fill"

[[element]]
type = "slider"
reglage = "volume"

[[element]]
type = "heure"
format = "HH:mm"

[[element]]
type = "en_cours"
largeur = 300

[[element]]
type = "script"
commande = "uptime"
intervalle = 2
delai = 1
largeur = 80
action = "suivant"

[[element]]
type = "espace"
largeur = 10

[[element]]
type = "souple"

[[element]]
type = "groupe"
texte = "Son"
symbole = "speaker.wave.2.fill"
largeur = 100
  [[element.element]]
  type = "batterie"
  symbole = false
  largeur = 80
"##;

    const TOUT_EN: &str = r##"
hide_control_strip = false

[[item]]
type = "button"
text = "Pr"
symbol = "scissors"
image = "~/pr.png"
template = true
color = "#2D0A4E"
text_color = "#FFFFFF"
size = 13
display = "both"
width = 90
action = { key = "cmd+k" }
long_press = { keycode = 53 }

[[item]]
type = "button"
display = "icon"
action = "brightness_up"
long_press = "brightness_down"

[[item]]
type = "button"
display = "text"
action = { brightness = 0.5 }
long_press = "volume_up"

[[item]]
type = "button"
action = "volume_down"
long_press = { volume = 0.3 }

[[item]]
type = "button"
action = "toggle_mute"
long_press = "play_pause"

[[item]]
type = "button"
action = "next"
long_press = "previous"

[[item]]
type = "button"
action = "sleep"
long_press = "esc"

[[item]]
type = "button"
action = "apple_bar"
long_press = { shell = "uptime" }

[[item]]
type = "button"
action = { app = "Figma" }
long_press = { url = "https://example.org" }

[[item]]
type = "button"
action = { applescript = "beep" }

[[item]]
type = "slider"
setting = "brightness"
symbol = "sun.max.fill"

[[item]]
type = "slider"
setting = "volume"

[[item]]
type = "clock"
format = "HH:mm"

[[item]]
type = "now_playing"
width = 300

[[item]]
type = "script"
command = "uptime"
interval = 2
timeout = 1
width = 80
action = "next"

[[item]]
type = "space"
width = 10

[[item]]
type = "flexible"

[[item]]
type = "group"
text = "Son"
symbol = "speaker.wave.2.fill"
width = 100
  [[item.item]]
  type = "battery"
  symbol = false
  width = 80
"##;

    #[test]
    fn alias_anglais_donnent_la_meme_barre() {
        let fr = lire(TOUT_FR).unwrap();
        assert_eq!(lire(TOUT_EN), Ok(fr.clone()));
        assert_eq!(fr.elements.len(), 18);
        assert_eq!(lire(include_str!("../exemples/barre.en.toml")), lire(PAR_DEFAUT));
        // Une erreur dans un [[item]] garde sa ligne.
        assert!(lire("[[item]]\ntype = \"clock\"\n\n[[item]]\ntype = \"button\"\naction = \"dance\"\n").unwrap_err().starts_with("ligne 4 : valeur inconnue «dance»"));
    }

    /// Chaque ligne du glossaire FR → EN du README est exercée par `TOUT_FR` / `TOUT_EN`.
    #[test]
    fn glossaire_du_readme() {
        let readme = include_str!("../README.md");
        let glossaire = readme.split("### Glossaire").nth(1).expect("section Glossaire").split("\n## ").next().unwrap();
        let mots = |texte: &str| texte.split(|c: char| !(c.is_alphanumeric() || c == '_')).map(String::from).collect::<std::collections::HashSet<_>>();
        let (fr, en) = (mots(TOUT_FR), mots(TOUT_EN));
        let lignes: Vec<(&str, &str)> = glossaire
            .lines()
            .filter(|l| l.starts_with("| `"))
            .map(|l| {
                let mut cases = l.split('|').map(|c| c.trim().trim_matches('`'));
                (cases.nth(1).unwrap(), cases.next().unwrap())
            })
            .collect();
        assert!(lignes.len() >= 45, "{} lignes", lignes.len());
        for (f, e) in lignes {
            assert!(fr.contains(f), "« {f} » n'est pas dans TOUT_FR");
            assert!(en.contains(e), "« {e} » n'est pas dans TOUT_EN");
        }
    }

    #[test]
    fn control_strip_masque_par_defaut() {
        assert!(lire("").unwrap().masquer_control_strip);
        assert!(!lire("masquer_control_strip = false\n").unwrap().masquer_control_strip);
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
    fn touche_inconnue_dit_la_ligne() {
        let erreur = lire("[[element]]\ntype = \"heure\"\n\n[[element]]\ntype = \"bouton\"\naction = { touche = \"cmd+truc\" }\n").unwrap_err();
        assert_eq!(erreur, "ligne 4 : touche inconnue « truc » ; attendu : cmd+shift+4, f5, keycode = 53");
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
                largeur: None,
                elements: vec![
                    bouton_symbole(Some("speaker.slash.fill"), None, Action::BasculerMuet),
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
    fn apparence_d_un_bouton() {
        let barre = lire("[[element]]\ntype = \"bouton\"\ntexte = \"Pr\"\nimage = \"~/premiere.png\"\ngabarit = true\ncouleur_texte = \"#FFFFFF\"\ntaille = 13\naffichage = \"icone\"\n").unwrap();
        assert_eq!(
            barre.elements[0],
            Element::Bouton {
                texte: Some("Pr".into()),
                symbole: None,
                image: Some("~/premiere.png".into()),
                gabarit: true,
                couleur: None,
                couleur_texte: Some("#FFFFFF".into()),
                taille: Some(13.0),
                affichage: Some(Affichage::Icone),
                largeur: None,
                action: None,
                appui_long: None,
            }
        );
        for (ecrit, attendu) in [("texte", Affichage::Texte), ("les_deux", Affichage::LesDeux)] {
            assert!(matches!(lire(&format!("[[element]]\ntype = \"bouton\"\naffichage = \"{ecrit}\"\n")).unwrap().elements[0], Element::Bouton { affichage: Some(a), .. } if a == attendu));
        }
        assert!(lire("[[element]]\ntype = \"bouton\"\naffichage = \"gros\"\n").unwrap_err().starts_with("ligne 1 : valeur inconnue «gros», attendu parmi «icon», «icone»"));
    }

    #[test]
    fn groupe_avec_largeur_et_batterie() {
        let barre = lire("[[element]]\ntype = \"groupe\"\ntexte = \"Export\"\nlargeur = 100\n\n[[element]]\ntype = \"batterie\"\n\n[[element]]\ntype = \"batterie\"\nsymbole = false\nlargeur = 80\n").unwrap();
        assert_eq!(barre.elements[0], Element::Groupe { texte: Some("Export".into()), symbole: None, largeur: Some(100.0), elements: vec![] });
        assert_eq!(barre.elements[1], Element::Batterie { symbole: None, largeur: None });
        assert_eq!(barre.elements[2], Element::Batterie { symbole: Some(false), largeur: Some(80.0) });
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
