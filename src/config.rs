//! Description de la barre, lue depuis ~/.config/tactile/barre.toml (ou `TACTILE_CONFIG`).

use crate::actions::Action;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use toml::de::{DeTable, DeValue, ValueDeserializer};
use toml::Spanned;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};

/// Barre utilisée tant que l'utilisateur n'a pas écrit la sienne.
const PAR_DEFAUT: &str = include_str!("../exemples/barre.toml");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Barre {
    /// `true` par défaut : toute la largeur pour la barre, sans le Control Strip d'Apple.
    #[serde(default = "vrai", alias = "hide_control_strip")]
    pub masquer_control_strip: bool,
    #[serde(rename = "element", alias = "item", default)]
    pub elements: Vec<Element>,
    /// Barres propres à une app, montrées quand elle passe au premier plan.
    #[serde(rename = "application", default, skip_serializing_if = "Vec::is_empty")]
    pub applications: Vec<Application>,
    /// Sources de données (`[[source]]`) qu'un élément `texte` ou un bouton affiche par leur `nom`.
    #[serde(rename = "source", default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<Source>,
}

/// `[[source]]` : une donnée relue toutes les `toutes_les` secondes, par HTTP, une commande ou un serveur MCP (un seul des trois).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    #[serde(alias = "name")]
    pub nom: String,
    /// `https://…` (ou `http://localhost`), lue par `/usr/bin/curl`.
    pub http: Option<String>,
    /// Commande zsh dont la sortie (JSON ou texte) est la valeur.
    #[serde(alias = "command")]
    pub commande: Option<String>,
    pub mcp: Option<SourceMcp>,
    /// Secondes entre deux lectures : 60 par défaut, 5 au minimum.
    #[serde(alias = "every")]
    pub toutes_les: Option<f64>,
    /// En-têtes HTTP ; `trousseau:service` y est remplacé à l'exécution par le mot de passe du trousseau.
    #[serde(default, alias = "headers", skip_serializing_if = "BTreeMap::is_empty")]
    pub entetes: BTreeMap<String, String>,
}

/// `mcp = { commande = "…", outil = "…", arguments = { … } }` (serveur stdio) ou `{ url = "https://…/mcp", outil = "…" }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceMcp {
    #[serde(alias = "command")]
    pub commande: Option<String>,
    pub url: Option<String>,
    #[serde(alias = "tool")]
    pub outil: String,
    pub arguments: Option<serde_json::Value>,
}

/// `symbole_selon = { champ = "a.b", valeurs = { "0" = "sun.max.fill" }, defaut = "cloud.fill" }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SymboleSelon {
    #[serde(alias = "field")]
    pub champ: String,
    #[serde(default, alias = "values")]
    pub valeurs: BTreeMap<String, String>,
    #[serde(alias = "default")]
    pub defaut: Option<String>,
}

/// `couleur_selon = { champ = "a.b", seuils = [10, 25], couleurs = ["#…", "#…", "#…"] }` : une couleur de plus que de seuils.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CouleurSelon {
    #[serde(alias = "field")]
    pub champ: String,
    #[serde(alias = "thresholds")]
    pub seuils: Vec<f64>,
    #[serde(alias = "colors")]
    pub couleurs: Vec<String>,
}

/// `[[application]]` : la barre montrée quand cette app est au premier plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Application {
    /// Bundle id (`com.apple.Safari`) ou nom affiché (`Safari`), sans tenir compte de la casse.
    pub app: String,
    /// `true` : les éléments remplacent seulement la zone entre le premier et le dernier `souple` de la principale.
    #[serde(default, alias = "inherit")]
    pub herite: bool,
    #[serde(rename = "element", alias = "item", default)]
    pub elements: Vec<Element>,
}

impl Barre {
    /// Index de la première `[[application]]` qui désigne l'app active (bundle id ou nom, sans casse).
    pub fn application_pour(&self, bundle: &str, nom: &str) -> Option<usize> {
        let (bundle, nom) = (bundle.to_lowercase(), nom.to_lowercase());
        self.applications.iter().position(|a| {
            let app = a.app.to_lowercase();
            !app.is_empty() && (app == bundle || app == nom)
        })
    }
}

impl Application {
    /// Les éléments de sa barre : les siens seuls ; avec `herite`, ceux de la principale jusqu'au premier
    /// `souple` inclus, les siens, puis ceux de la principale depuis le dernier `souple` (sans `souple` : les siens à la suite).
    pub fn elements_avec(&self, principale: &[Element]) -> Vec<Element> {
        if !self.herite {
            return self.elements.clone();
        }
        let souple = |e: &Element| *e == Element::Souple;
        match (principale.iter().position(souple), principale.iter().rposition(souple)) {
            (Some(premier), Some(dernier)) => [&principale[..=premier], &self.elements[..], &principale[dernier..]].concat(),
            _ => [principale, &self.elements[..]].concat(),
        }
    }
}

/// Les clés françaises sont la forme canonique ; chaque clé, type, réglage et action accepte
/// un alias anglais (glossaire dans le README, vérifié par le test `glossaire_du_readme`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
        /// Nom d'une `[[source]]` : `texte` devient un gabarit (`{champ.sous_champ}`) ; `gabarit` est déjà le booléen de l'image.
        source: Option<String>,
    },
    /// Texte vivant, lié à une `[[source]]`.
    #[serde(alias = "text")]
    Texte {
        source: Option<String>,
        /// `{a.b.c}` est remplacé par la valeur à ce chemin ; `{texte}` par défaut (sortie qui n'est pas du JSON).
        #[serde(alias = "template")]
        gabarit: Option<String>,
        #[serde(alias = "symbol_by")]
        symbole_selon: Option<SymboleSelon>,
        #[serde(alias = "color_by")]
        couleur_selon: Option<CouleurSelon>,
        #[serde(alias = "width")]
        largeur: Option<f64>,
        action: Option<Action>,
    },
    /// Curseur dans une sous-barre : luminosité ou volume (`reglage`), ou n'importe quel réglage pilotable par une commande
    /// (`regler`, avec `{valeur}`, et `lire` pour la valeur actuelle). `curseur` est accepté comme nom de type.
    #[serde(alias = "curseur")]
    Slider {
        #[serde(alias = "setting")]
        reglage: Option<Reglage>,
        #[serde(alias = "symbol")]
        symbole: Option<String>,
        /// Commande zsh qui affiche la valeur actuelle (un nombre), relue à l'ouverture et toutes les `toutes_les` s.
        #[serde(alias = "read")]
        lire: Option<String>,
        /// Commande zsh lancée quand le doigt bouge (au plus toutes les 0,1 s) et au lâcher ; `{valeur}` y devient le nombre.
        #[serde(alias = "set")]
        regler: Option<String>,
        min: Option<f64>,
        max: Option<f64>,
        #[serde(alias = "step")]
        pas: Option<f64>,
        #[serde(alias = "every")]
        toutes_les: Option<f64>,
        style: Option<StyleCurseur>,
        /// Nombre de barres ou de points (grille, points, égaliseur) et de retours haptiques.
        #[serde(alias = "notches")]
        crans: Option<u32>,
        /// Une couleur par niveau, une de plus que de `seuils` (dans les unités du curseur, entre `min` et `max`).
        #[serde(default, alias = "colors", skip_serializing_if = "Vec::is_empty")]
        couleurs: Vec<String>,
        #[serde(default, alias = "thresholds", skip_serializing_if = "Vec::is_empty")]
        seuils: Vec<f64>,
        #[serde(default, alias = "haptic", skip_serializing_if = "std::ops::Not::not")]
        haptique: bool,
    },
    /// Sélecteur à choix (contrôle segmenté) : l'option touchée remplace `{valeur}` dans l'action, protégée pour le shell.
    #[serde(alias = "choice")]
    Choix {
        options: Vec<String>,
        action: Option<Action>,
        /// Commande zsh qui affiche l'option actuelle, relue toutes les `toutes_les` s.
        #[serde(alias = "read")]
        lire: Option<String>,
        #[serde(alias = "every")]
        toutes_les: Option<f64>,
        #[serde(alias = "width")]
        largeur: Option<f64>,
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
    /// L'icône d'une app ; l'appui la lance ou la met au premier plan, un point dessous dit qu'elle est ouverte.
    #[serde(alias = "launcher")]
    Lanceur {
        /// Nom, bundle id ou chemin du `.app`, comme l'action `app`.
        app: String,
        #[serde(alias = "text")]
        texte: Option<String>,
        #[serde(alias = "display")]
        affichage: Option<Affichage>,
        #[serde(alias = "width")]
        largeur: Option<f64>,
        #[serde(alias = "long_press")]
        appui_long: Option<Action>,
    },
}

#[cfg(test)]
impl Element {
    /// Un slider de luminosité ou de volume, sans style.
    pub fn slider(reglage: Reglage, symbole: Option<String>) -> Element {
        Element::Slider {
            reglage: Some(reglage),
            symbole,
            lire: None,
            regler: None,
            min: None,
            max: None,
            pas: None,
            toutes_les: None,
            style: None,
            crans: None,
            couleurs: Vec::new(),
            seuils: Vec::new(),
            haptique: false,
        }
    }
}

/// Ce qu'un bouton montre quand il a un symbole (ou une image) et un texte.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Affichage {
    #[serde(alias = "icon")]
    Icone,
    #[serde(alias = "text")]
    Texte,
    #[serde(alias = "both")]
    LesDeux,
}

/// Le dessin d'un curseur. `classique` est le curseur d'Apple ; les autres sont dessinés par Tactile.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StyleCurseur {
    #[default]
    #[serde(alias = "classic")]
    Classique,
    #[serde(alias = "grid")]
    Grille,
    #[serde(alias = "gauge")]
    Jauge,
    #[serde(alias = "dots")]
    Points,
    #[serde(alias = "equalizer")]
    Egaliseur,
    #[serde(alias = "stepper")]
    PasAPas,
    #[serde(alias = "dial")]
    Cadran,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reglage {
    #[serde(alias = "brightness")]
    Luminosite,
    Volume,
}

fn vrai() -> bool {
    true
}

/// `TACTILE_CONFIG=chemin` remplace ~/.config/tactile/barre.toml (essais sans toucher sa vraie barre).
pub fn chemin() -> PathBuf {
    if let Some(chemin) = std::env::var_os("TACTILE_CONFIG").filter(|c| !c.is_empty()) {
        return PathBuf::from(chemin);
    }
    let maison = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(maison).join(".config/tactile/barre.toml")
}

/// Le fichier complet tel que l'éditeur l'écrit : un en-tête fixe, puis la barre en TOML.
pub fn ecrire(barre: &Barre) -> Result<String, String> {
    let corps = toml::to_string_pretty(barre).map_err(|e| format!("écriture TOML : {e}"))?;
    Ok(format!(
        "# Ta barre Tactile, de gauche à droite. Écrite par « Personnaliser ma barre ».\n\
         # Tu peux aussi la modifier à la main : Tactile la recharge dès qu'elle est enregistrée.\n\
         # Les types d'éléments et les actions sont décrits dans le README.\n\n{corps}"
    ))
}

pub fn lire(texte: &str) -> Result<Barre, String> {
    let situer = |debut: Option<usize>, message: String| match debut {
        Some(d) => format!("ligne {} : {message}", ligne(texte, d)),
        None => message,
    };
    let document = document(texte).map_err(|e| situer(e.span().map(|s| s.start), traduire(e.message())))?;
    let racine = Spanned::new(0..texte.len(), DeValue::Table(document.clone()));
    if let Some((debut, nombre)) = non_fini(&racine) {
        return Err(situer(Some(debut), format!("« {nombre} » n'est pas un nombre fini : écris une valeur comme 0.5 ou 60")));
    }
    let barre = Barre::deserialize(ValueDeserializer::from(racine.clone())).map_err(|e| match (premiere_erreur::<Barre>(&racine), gabarit_de_bouton(&racine)) {
        (Some((_, message)), Some((debut, cle))) if message.contains("expected a boolean") => situer(
            Some(debut),
            format!("« {cle} » : sur un bouton, le texte lié à une source s'écrit texte / text ; gabarit / template est l'interrupteur monochrome de l'image (true ou false)"),
        ),
        (Some((debut, message)), _) => situer(Some(debut), traduire(&message)),
        (None, _) => situer(e.span().map(|s| s.start), traduire(e.message())),
    })?;
    if let Some(k) = barre.applications.iter().position(|a| a.elements.is_empty()) {
        let debut = position(&document, &[("application", k)]);
        return Err(situer(debut, format!("[[application]] « {} » sans élément : ajoute au moins un [[application.element]]", barre.applications[k].app)));
    }
    for (k, source) in barre.sources.iter().enumerate() {
        let sortes = [source.http.is_some(), source.commande.is_some(), source.mcp.is_some()].iter().filter(|&&s| s).count();
        let mcp = source.mcp.as_ref().is_none_or(|m| m.commande.is_some() != m.url.is_some());
        if sortes != 1 || !mcp {
            let table = valeur_au(&document, &[("source", k)]);
            let message = if sortes != 1 {
                format!("écris un seul de http, {} ou mcp", ecrite(table, &["commande", "command"]))
            } else {
                let mcp = match table.map(Spanned::get_ref) {
                    Some(DeValue::Table(t)) => t.get("mcp"),
                    _ => None,
                };
                format!("mcp demande {} ou url, pas les deux", ecrite(mcp, &["commande", "command"]))
            };
            return Err(situer(table.map(|t| t.span().start), format!("[[source]] « {} » : {message}", source.nom)));
        }
    }
    if let Some((chemin, message)) = curseur_mal_forme(&barre) {
        return Err(situer(position(&document, &chemin), message.into()));
    }
    Ok(barre)
}

/// Le premier `slider` sans réglage, ou avec `reglage` et `regler` à la fois (chemin dans le fichier, message).
fn curseur_mal_forme(barre: &Barre) -> Option<(Vec<(&'static str, usize)>, &'static str)> {
    fn chercher(elements: &[Element], chemin: &mut Vec<(&'static str, usize)>) -> Option<(Vec<(&'static str, usize)>, &'static str)> {
        for (i, e) in elements.iter().enumerate() {
            chemin.push(("element", i));
            let message = match e {
                Element::Slider { reglage: Some(_), regler: Some(_), .. } => Some("slider : écris reglage (luminosite ou volume) ou regler (une commande avec {valeur}), pas les deux"),
                Element::Slider { reglage: None, regler: None, .. } => Some("slider sans réglage : écris reglage = \"luminosite\" ou \"volume\", ou regler = \"une commande avec {valeur}\""),
                Element::Slider { reglage: Some(_), lire: Some(_), .. } => Some("slider : lire ne sert qu'avec regler (Tactile lit lui-même la luminosité et le volume)"),
                Element::Groupe { elements, .. } => {
                    if let Some(trouve) = chercher(elements, chemin) {
                        return Some(trouve);
                    }
                    None
                }
                _ => None,
            };
            if let Some(m) = message {
                return Some((chemin.clone(), m));
            }
            chemin.pop();
        }
        None
    }
    chercher(&barre.elements, &mut Vec::new())
        .or_else(|| barre.applications.iter().enumerate().find_map(|(k, a)| chercher(&a.elements, &mut vec![("application", k)])))
}

/// Le premier `nan` ou `inf` du fichier (position, texte) : valides en TOML, mais ni une largeur ni une durée,
/// et le JSON de l'éditeur n'a pas de quoi les écrire.
fn non_fini(valeur: &Spanned<DeValue>) -> Option<(usize, String)> {
    match valeur.get_ref() {
        DeValue::Float(f) if !f.as_str().parse::<f64>().is_ok_and(f64::is_finite) => Some((valeur.span().start, f.as_str().to_string())),
        DeValue::Array(liste) => liste.iter().filter_map(non_fini).min_by_key(|n| n.0),
        DeValue::Table(table) => table.values().filter_map(non_fini).min_by_key(|n| n.0),
        _ => None,
    }
}

/// Le premier `gabarit` / `template` écrit en texte sur un bouton (position, clé écrite) : serde dirait seulement
/// « attendu true ou false », alors que l'auteur voulait le gabarit d'un élément `texte`.
fn gabarit_de_bouton(valeur: &Spanned<DeValue>) -> Option<(usize, String)> {
    match valeur.get_ref() {
        DeValue::Array(liste) => liste.iter().filter_map(gabarit_de_bouton).min_by_key(|g| g.0),
        DeValue::Table(table) => {
            let bouton = matches!(table.get("type").map(Spanned::get_ref), Some(DeValue::String(t)) if t == "bouton" || t == "button");
            let texte = |cle: &&str| table.get(*cle).filter(|v| matches!(v.get_ref(), DeValue::String(_))).map(|v| (v.span().start, cle.to_string()));
            let ici = if bouton { ["gabarit", "template"].iter().find_map(texte) } else { None };
            ici.or_else(|| table.values().filter_map(gabarit_de_bouton).min_by_key(|g| g.0))
        }
        _ => None,
    }
}

/// Numéro de ligne (à partir de 1) d'une position dans le texte.
fn ligne(texte: &str, debut: usize) -> usize {
    texte.get(..debut).unwrap_or(texte).matches('\n').count() + 1
}

/// Le fichier en arbre TOML dont chaque valeur garde sa position, `[[item]]` fondus dans `[[element]]`.
fn document(texte: &str) -> Result<DeTable<'_>, toml::de::Error> {
    let mut document = DeTable::parse(texte)?.into_inner();
    fusionner(&mut document);
    Ok(document)
}

/// `[[element]]` et `[[item]]` (ou leurs formes `application.` et les enfants d'un groupe) dans une même table :
/// une seule liste `element`, dans l'ordre du fichier. Sans ça, serde voit deux fois le même champ.
fn fusionner(table: &mut DeTable) {
    let est_liste = |t: &DeTable, cle: &str| matches!(t.get(cle).map(Spanned::get_ref), Some(DeValue::Array(_)));
    if est_liste(table, "element") && est_liste(table, "item") {
        let (cle, element) = table.remove_entry("element").expect("présent");
        let span = element.span();
        if let (DeValue::Array(mut tous), Some(DeValue::Array(items))) = (element.into_inner(), table.remove("item").map(Spanned::into_inner)) {
            for item in items {
                tous.push(item);
            }
            tous.sort_by_key(|v| v.span().start);
            table.insert(cle, Spanned::new(span, DeValue::Array(tous)));
        }
    }
    for cle in ["element", "item", "application"] {
        if let Some(DeValue::Array(liste)) = table.get_mut(cle).map(Spanned::get_mut) {
            for valeur in liste.iter_mut() {
                if let DeValue::Table(t) = valeur.get_mut() {
                    fusionner(t);
                }
            }
        }
    }
}

/// Les valeurs d'une liste `[[cle]]` (vide si absente). Après `fusionner`, `item` n'existe plus à côté d'`element`.
fn tables<'a, 'i>(table: &'a DeTable<'i>, cle: &str) -> &'a [Spanned<DeValue<'i>>] {
    let cles: &[&str] = if cle == "element" { &["element", "item"] } else { &[cle] };
    match cles.iter().find_map(|c| table.get(*c)).map(Spanned::get_ref) {
        Some(DeValue::Array(liste)) => liste,
        _ => &[],
    }
}

/// La table au bout de `chemin` : `[("application", 1), ("element", 0)]` = premier élément de la deuxième app.
fn valeur_au<'a, 'i>(table: &'a DeTable<'i>, chemin: &[(&str, usize)]) -> Option<&'a Spanned<DeValue<'i>>> {
    let ((cle, k), reste) = chemin.split_first()?;
    let valeur = tables(table, cle).get(*k)?;
    match valeur.get_ref() {
        _ if reste.is_empty() => Some(valeur),
        DeValue::Table(t) => valeur_au(t, reste),
        _ => None,
    }
}

/// Début de la table au bout de `chemin`.
fn position(table: &DeTable, chemin: &[(&str, usize)]) -> Option<usize> {
    valeur_au(table, chemin).map(|v| v.span().start)
}

/// La clé telle que l'utilisateur l'a écrite dans `table` (`every` ou `toutes_les`) ; la française sans fichier.
fn ecrite<'a>(table: Option<&Spanned<DeValue>>, noms: &[&'a str]) -> &'a str {
    let present = |n: &&&str| matches!(table.map(Spanned::get_ref), Some(DeValue::Table(t)) if t.contains_key(**n));
    noms.iter().find(present).unwrap_or(&noms[0])
}

/// Première erreur dans l'ordre du fichier, avec sa position. serde s'arrête à la première dans l'ordre
/// alphabétique des clés (`application` avant `element`) et ne situe pas celles d'une table imbriquée :
/// chaque table est donc relue seule, ses enfants (`element`, `application`) à part.
fn premiere_erreur<T: DeserializeOwned>(valeur: &Spanned<DeValue>) -> Option<(usize, String)> {
    let erreur = T::deserialize(ValueDeserializer::from(valeur.clone())).err()?;
    let debut = valeur.span().start;
    let situee = |e: toml::de::Error| (e.span().map_or(debut, |s| s.start.max(debut)), e.message().to_string());
    let DeValue::Table(table) = valeur.get_ref() else { return Some(situee(erreur)) };
    let mut erreurs: Vec<(usize, String)> = tables(table, "element").iter().filter_map(premiere_erreur::<Element>).collect();
    erreurs.extend(tables(table, "application").iter().filter_map(premiere_erreur::<Application>));
    erreurs.extend(tables(table, "source").iter().filter_map(premiere_erreur::<Source>));
    let mut seule = table.clone();
    for cle in ["element", "item", "application", "source"] {
        seule.remove(cle);
    }
    if let Err(e) = T::deserialize(ValueDeserializer::from(Spanned::new(valeur.span(), DeValue::Table(seule)))) {
        erreurs.push(situee(e));
    }
    Some(erreurs.into_iter().min_by_key(|e| e.0).unwrap_or_else(|| situee(erreur)))
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

/// `#RRGGBB` (six chiffres hexadécimaux) : la seule forme de couleur, pour la barre, l'éditeur et `--verifier`.
/// `#F00` est refusé : le sélecteur de couleur de l'éditeur ne sait pas l'afficher.
pub fn rgb(hexa: &str) -> Option<u32> {
    let h = hexa.strip_prefix('#').filter(|h| h.len() == 6 && h.chars().all(|c| c.is_ascii_hexdigit()))?;
    u32::from_str_radix(h, 16).ok()
}

/// « 1 élément », « 2 éléments » (0 reste au singulier, comme en français).
pub fn nombre(n: usize, mot: &str) -> String {
    format!("{n} {mot}{}", if n > 1 { "s" } else { "" })
}

/// Un problème que `lire` laisse passer mais qui se verra à l'usage.
#[derive(Debug, Clone, PartialEq)]
pub struct Avis {
    /// `true` : la barre serait inutilisable (vide). `--verifier` sort en erreur, l'éditeur refuse d'enregistrer.
    pub grave: bool,
    /// Ligne du fichier, quand `controler` a eu le texte.
    pub ligne: Option<usize>,
    /// « élément 3 », « application « Safari », élément 2 » : le lieu quand la ligne n'est pas connue (éditeur).
    pub lieu: String,
    pub message: String,
}

impl std::fmt::Display for Avis {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match (self.ligne, self.lieu.as_str()) {
            (Some(n), _) => write!(f, "ligne {n} : {}", self.message),
            (None, "") => f.write_str(&self.message),
            (None, lieu) => write!(f, "{lieu} : {}", self.message),
        }
    }
}

/// Le validateur partagé par `--verifier`, l'éditeur et la barre (au chargement, dans le journal) : ce que la syntaxe
/// accepte mais qui casse à l'usage (couleur, symbole SF, image, app introuvable, débordement de chaque barre, app
/// décrite deux fois, barre vide). Avec le texte du fichier, chaque avis a sa ligne et un `[[element]]` écrit
/// après un `[[application]]` est signalé. Les avis sont dans l'ordre du fichier, ceux de toute la barre à la fin.
pub fn controler(barre: &Barre, texte: Option<&str>) -> Vec<Avis> {
    let sources = barre.sources.iter().map(|s| s.nom.clone()).collect();
    let mut c = Controle { document: texte.and_then(|t| document(t).ok().map(|d| (t, d))), avis: Vec::new(), sources };
    for (k, source) in barre.sources.iter().enumerate() {
        let (ici, lieu) = ([("source", k)], format!("source « {} »", source.nom));
        if barre.sources[..k].iter().any(|s| s.nom == source.nom) {
            c.signaler(false, &ici, &lieu, format!("le nom « {} » est déjà pris par une autre source : renomme-la", source.nom));
        }
        for url in [source.http.as_deref(), source.mcp.as_ref().and_then(|m| m.url.as_deref())].into_iter().flatten() {
            if !crate::sources::url_permise(url) {
                c.signaler(false, &ici, &lieu, format!("« {url} » : seules les adresses https:// (et http://localhost) sont lues ; la source restera vide"));
            }
        }
        if source.toutes_les.is_some_and(|t| t < crate::sources::MINIMUM) {
            c.signaler(false, &ici, &lieu, format!("{} est ramené à {} secondes, le minimum", c.cle(&ici, &["toutes_les", "every"]), crate::sources::MINIMUM));
        }
        for nom in source.entetes.iter().filter(|(_, v)| crate::sources::cle_en_clair(v)).map(|(n, _)| n) {
            c.signaler(false, &ici, &lieu, format!("l'en-tête « {nom} » semble contenir une clé en clair : range-la dans le trousseau et écris trousseau:service"));
        }
    }
    if barre.elements.is_empty() {
        c.signaler(true, &[], "", "aucun élément : la barre principale serait vide, ajoute au moins un [[element]]".into());
    }
    // Avec l'en-tête tel qu'écrit : `[[item]]` et `[[element]]` peuvent se mêler dans la même liste.
    let egares: Vec<(usize, &str)> = match &c.document {
        Some((t, d)) => match tables(d, "application").first() {
            Some(app) => tables(d, "element")
                .iter()
                .enumerate()
                .filter(|(_, e)| e.span().start > app.span().start)
                .map(|(i, e)| (i, if t.get(e.span().start..).is_some_and(|l| l.starts_with("[[item]]")) { "item" } else { "element" }))
                .collect(),
            None => vec![],
        },
        None => vec![],
    };
    for (i, entete) in egares {
        c.signaler(false, &[("element", i)], &format!("élément {}", i + 1), format!("cet [[{entete}]] est écrit après un [[application]] : il va dans la barre principale ; voulais-tu [[application.{entete}]] ?"));
    }
    c.elements(&barre.elements, &mut Vec::new(), "");
    let largeur = crate::barre::largeur_barre(barre.masquer_control_strip);
    c.deborde(&barre.elements, largeur, &[], "la barre principale");
    c.rapides(&barre.elements, &[], "la barre principale");
    // Chaque app par son bundle id quand elle est installée : `Finder` et `com.apple.finder` sont la même.
    let identites: Vec<String> = barre.applications.iter().map(|a| crate::actions::bundle_app(&a.app).unwrap_or_else(|| a.app.clone()).to_lowercase()).collect();
    for (k, app) in barre.applications.iter().enumerate() {
        let lieu = format!("application « {} »", app.app);
        if let Some(j) = identites[..k].iter().position(|i| *i == identites[k]) {
            let ou = c.ligne(&[("application", j)]).map_or(format!("l'application n° {}", j + 1), |n| format!("ligne {n}"));
            let premiere = &barre.applications[j].app;
            let qui = if premiere.to_lowercase() == app.app.to_lowercase() { String::new() } else { format!(" est la même app que « {premiere} », qui") };
            c.signaler(false, &[("application", k)], &lieu, format!("« {} »{qui} a déjà sa barre ({ou}) : celle-ci ne sera jamais montrée", app.app));
        }
        if !crate::actions::app_existe(&app.app) {
            c.signaler(false, &[("application", k)], &lieu, app_introuvable(&app.app, " : sa barre ne sera pas montrée"));
        }
        c.elements(&app.elements, &mut vec![("application", k)], &format!("{lieu}, "));
        c.deborde(&app.elements_avec(&barre.elements), largeur, &[("application", k)], &format!("la barre de « {} »", app.app));
        c.rapides(&app.elements, &[("application", k)], &format!("la barre de « {} »", app.app));
    }
    c.avis.sort_by_key(|a| a.ligne.unwrap_or(usize::MAX));
    c.avis
}

/// L'avis d'une app introuvable, suivi de `suite` (ce qui se passera). Un bundle id bien formé (deux points au moins,
/// sans espace) est sans doute juste, pour une app pas encore installée : pas de conseil d'écriture.
fn app_introuvable(app: &str, suite: &str) -> String {
    if app.matches('.').count() >= 2 && !app.contains([' ', '/']) {
        format!("aucune app installée n'a le bundle id « {app} »{suite}")
    } else {
        let suite = if suite.is_empty() { " :".to_string() } else { format!("{suite} ;") };
        format!("app « {app} » introuvable{suite} écris le nom du dossier .app sans l'extension, ou son bundle id")
    }
}

struct Controle<'a> {
    document: Option<(&'a str, DeTable<'a>)>,
    avis: Vec<Avis>,
    /// Noms des `[[source]]`, pour les éléments qui en citent une.
    sources: Vec<String>,
}

impl Controle<'_> {
    /// La clé telle qu'écrite dans la table au bout de `chemin` (la française sans le fichier).
    fn cle<'n>(&self, chemin: &[(&str, usize)], noms: &[&'n str]) -> &'n str {
        ecrite(self.document.as_ref().and_then(|(_, d)| valeur_au(d, chemin)), noms)
    }

    fn ligne(&self, chemin: &[(&str, usize)]) -> Option<usize> {
        let (texte, document) = self.document.as_ref()?;
        position(document, chemin).map(|p| ligne(texte, p))
    }

    /// `chemin` situe l'avis dans le fichier, `lieu` le nomme quand le fichier n'est pas connu.
    fn signaler(&mut self, grave: bool, chemin: &[(&str, usize)], lieu: &str, message: String) {
        let ligne = self.ligne(chemin);
        self.avis.push(Avis { grave, ligne, lieu: lieu.to_string(), message });
    }

    fn deborde(&mut self, elements: &[Element], largeur: f64, chemin: &[(&str, usize)], barre: &str) {
        if let Some((total, caches)) = crate::barre::debordement(elements, largeur) {
            let verbe = if caches > 1 { "seront cachés" } else { "sera caché" };
            let message = format!("{barre} déborde (environ {total:.0} pt pour {largeur:.0}) : {} sur {} {verbe} ; réduis des largeurs ou retire un élément", nombre(caches, "élément"), elements.len());
            self.signaler(false, chemin, "", message);
        }
    }

    /// Mesuré en 0.3 (`docs/ARCHITECTURE.md`, Mesures) : un script toutes les 0,3 s coûte à lui seul 0,4 % de CPU,
    /// presque tout le budget de la barre. Avis au-delà de 2 scripts sous 1 s, ou de 5 lancements par seconde (un
    /// script seul à 0,3 s, comme `exemples/claude-vocal.toml`, est un choix assumé : pas d'avis).
    fn rapides(&mut self, elements: &[Element], chemin: &[(&str, usize)], barre: &str) {
        fn frequences(elements: &[Element], liste: &mut Vec<f64>) {
            for e in elements {
                match e {
                    Element::Script { intervalle, .. } if *intervalle < 1.0 => liste.push(1.0 / intervalle.max(0.1)),
                    Element::Groupe { elements, .. } => frequences(elements, liste),
                    _ => {}
                }
            }
        }
        let mut liste = Vec::new();
        frequences(elements, &mut liste);
        let par_seconde: f64 = liste.iter().sum();
        if liste.len() > 2 || par_seconde > 5.0 {
            let message = format!(
                "{barre} lance {} plus d'une fois par seconde (environ {par_seconde:.0} lancements par seconde) : ces rafraîchissements rapides coûtent du CPU (un script toutes les 0,3 s prend à lui seul 0,4 %) ; passe leur intervalle à 1 s ou plus",
                nombre(liste.len(), "script")
            );
            self.signaler(false, chemin, "", message);
        }
    }

    fn lecture(&mut self, chemin: &[(&str, usize)], ici: &str, toutes_les: Option<f64>) {
        if toutes_les.is_some_and(|t| t < crate::barre::LECTURE_MIN) {
            let cle = self.cle(chemin, &["toutes_les", "every"]);
            self.signaler(false, chemin, ici, format!("{cle} est ramené à {} seconde, le minimum", crate::barre::LECTURE_MIN));
        }
    }

    /// Bornes, pas, crans, couleurs par niveau et `{valeur}` d'un curseur : ce qui se lit mais se dessinerait ou se réglerait mal.
    fn curseur(&mut self, chemin: &[(&str, usize)], ici: &str, element: &Element) {
        let Element::Slider { reglage, regler, min, max, pas, toutes_les, crans, couleurs, seuils, .. } = element else { return };
        let (reglage, min, max, pas, toutes_les, crans) = (*reglage, *min, *max, *pas, *toutes_les, *crans);
        if reglage.is_none() {
            let (a, b) = (min.unwrap_or(0.0), max.unwrap_or(100.0));
            if a >= b {
                self.signaler(false, chemin, ici, format!("min ({a}) doit être plus petit que max ({b}) : le curseur ira de 0 à 100"));
            } else if pas.is_some_and(|p| p <= 0.0 || p > b - a) {
                let cle = self.cle(chemin, &["pas", "step"]);
                self.signaler(false, chemin, ici, format!("{cle} doit être plus grand que 0 et pas plus grand que max − min ({}) : il sera ignoré", b - a));
            }
            if let Some(r) = regler.as_deref().filter(|r| !crate::barre::contient_valeur(r)) {
                let cle = self.cle(chemin, &["regler", "set"]);
                self.signaler(false, chemin, ici, format!("{cle} ne contient pas {{valeur}} : « {r} » recevra toujours la même chose"));
            }
        }
        self.lecture(chemin, ici, toutes_les);
        if let Some(c) = crans.filter(|c| !(2..=40).contains(c)) {
            let cle = self.cle(chemin, &["crans", "notches"]);
            self.signaler(false, chemin, ici, format!("{cle} = {c} est ramené entre 2 et 40"));
        }
        for c in couleurs.iter().filter(|c| rgb(c).is_none()) {
            self.signaler(false, chemin, ici, format!("la couleur « {c} » doit s'écrire #RRGGBB : elle sera ignorée"));
        }
        if couleurs.len() > 1 && couleurs.len() != seuils.len() + 1 {
            let cle = self.cle(chemin, &["couleurs", "colors"]);
            self.signaler(false, chemin, ici, format!("{cle} : {} pour {} : il faut une couleur de plus que de seuils", nombre(couleurs.len(), "couleur"), nombre(seuils.len(), "seuil")));
        }
        let (a, b, _) = crate::barre::bornes(reglage, min, max, pas);
        let cle = self.cle(chemin, &["seuils", "thresholds"]);
        if let Some(s) = seuils.iter().find(|s| **s <= a || **s >= b) {
            self.signaler(false, chemin, ici, format!("{cle} : {s} est hors du curseur (de {a} à {b}, bornes exclues)"));
        } else if seuils.windows(2).any(|w| w[0] >= w[1]) {
            self.signaler(false, chemin, ici, format!("{cle} : écris-les du plus petit au plus grand"));
        }
    }

    fn elements(&mut self, elements: &[Element], chemin: &mut Vec<(&'static str, usize)>, lieu: &str) {
        for (i, element) in elements.iter().enumerate() {
            chemin.push(("element", i));
            let ici = format!("{lieu}élément {}", i + 1);
            let (symbole, couleurs, image, actions) = match element {
                Element::Bouton { symbole, couleur, couleur_texte, image, action, appui_long, .. } => (symbole, vec![couleur, couleur_texte], image, vec![action, appui_long]),
                Element::Slider { symbole, .. } | Element::Groupe { symbole, .. } => (symbole, vec![], &None, vec![]),
                Element::Script { action, .. } | Element::Choix { action, .. } => (&None, vec![], &None, vec![action]),
                Element::Lanceur { appui_long, .. } => (&None, vec![], &None, vec![appui_long]),
                _ => (&None, vec![], &None, vec![]),
            };
            if let Element::Lanceur { app, .. } = element
                && !crate::actions::app_existe(app)
            {
                self.signaler(false, chemin, &ici, app_introuvable(app, " : le lanceur affichera son nom en texte"));
            }
            if let Some(s) = symbole.as_deref().filter(|s| !s.is_empty() && crate::barre::symbole(s).is_none()) {
                self.signaler(false, chemin, &ici, format!("le symbole SF « {s} » n'existe pas : le bouton affichera son nom en texte"));
            }
            for c in couleurs.into_iter().flatten().filter(|c| rgb(c).is_none()) {
                self.signaler(false, chemin, &ici, format!("la couleur « {c} » doit s'écrire #RRGGBB : elle sera ignorée"));
            }
            if let Some(i) = image.as_deref().filter(|i| crate::barre::image(i, false).is_none()) {
                self.signaler(false, chemin, &ici, format!("image « {i} » introuvable ou illisible : le bouton affichera du texte"));
            }
            for action in actions.into_iter().flatten() {
                if let Action::App(app) = action
                    && !crate::actions::app_existe(app)
                {
                    self.signaler(false, chemin, &ici, app_introuvable(app, ""));
                }
            }
            if let Element::Groupe { elements, .. } = element {
                self.elements(elements, chemin, &format!("{lieu}groupe {}, ", i + 1));
            }
            let source = match element {
                Element::Bouton { source, .. } | Element::Texte { source, .. } => source.as_deref(),
                _ => None,
            };
            if let Some(s) = source.filter(|s| !self.sources.iter().any(|n| n == s)) {
                self.signaler(false, chemin, &ici, format!("aucune [[source]] ne s'appelle « {s} » : l'élément affichera ⚠ source"));
            }
            let gabarit = match element {
                Element::Bouton { texte, source: Some(_), .. } => texte.as_deref(),
                Element::Texte { gabarit, .. } => gabarit.as_deref(),
                _ => None,
            };
            for v in gabarit.into_iter().flat_map(crate::sources::variables).filter(|v| crate::sources::variable_impossible(v)) {
                let possibles = "variables possibles : {texte} ou {text} (sortie qui n'est pas du JSON), sinon le chemin d'un champ du JSON, comme {current.temperature_2m}";
                self.signaler(false, chemin, &ici, format!("{{{v}}} n'existe pas ; {possibles}"));
            }
            self.curseur(chemin, &ici, element);
            if let Element::Choix { options, action, toutes_les, .. } = element {
                if options.is_empty() {
                    self.signaler(false, chemin, &ici, "aucune option : le choix sera vide ; écris options = [\"1×\", \"2×\"]".into());
                }
                if let Some(Action::Shell(c) | Action::Applescript(c)) = action
                    && !crate::barre::contient_valeur(c)
                {
                    self.signaler(false, chemin, &ici, "l'action ne contient pas {valeur} : chaque option lancera la même chose".into());
                }
                self.lecture(chemin, &ici, *toutes_les);
            }
            if let Element::Texte { source, gabarit, symbole_selon, couleur_selon, .. } = element {
                if source.is_none() && gabarit.is_some() {
                    self.signaler(false, chemin, &ici, format!("un {} sans source n'affiche rien : ajoute source = \"…\"", self.cle(chemin, &["gabarit", "template"])));
                }
                let symboles = symbole_selon.iter().flat_map(|s| s.valeurs.values().chain(&s.defaut));
                for s in symboles.filter(|s| crate::barre::symbole(s).is_none()) {
                    self.signaler(false, chemin, &ici, format!("le symbole SF « {s} » n'existe pas : il ne sera pas affiché"));
                }
                if let Some(c) = couleur_selon {
                    for couleur in c.couleurs.iter().filter(|c| rgb(c).is_none()) {
                        self.signaler(false, chemin, &ici, format!("la couleur « {couleur} » doit s'écrire #RRGGBB : elle sera ignorée"));
                    }
                    if c.couleurs.len() != c.seuils.len() + 1 {
                        self.signaler(false, chemin, &ici, format!("{} : {} pour {} : il faut une couleur de plus que de seuils", self.cle(chemin, &["couleur_selon", "color_by"]), nombre(c.couleurs.len(), "couleur"), nombre(c.seuils.len(), "seuil")));
                    }
                }
            }
            chemin.pop();
        }
    }
}

/// Relit la barre à chaque modification du fichier, vue par sa date, relevée toutes les 0,2 s (un `stat` : le CPU au repos
/// ne bouge pas, et la barre suit l'enregistrement en moins de 300 ms ; à 1 s, il fallait parfois une seconde).
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
            std::thread::sleep(Duration::from_millis(200));
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

/// Met en français les messages de toml et de serde ; ce qui est entre `…` (un nom, une valeur) ne change pas
/// et passe entre « ». Un message absent de la table garde ses mots anglais.
fn traduire(message: &str) -> String {
    // Ce que serde a trouvé (`invalid type: sequence`) puis ce qu'il attendait (`expected a string`).
    const TROUVE: [(&str, &str); 6] = [("sequence", "une liste"), ("map", "une table"), ("string ", "le texte "), ("integer ", "l'entier "), ("floating point ", "le nombre "), ("boolean ", "le booléen ")];
    const ATTENDU: [(&str, &str); 13] = [
        ("one of", "parmi"),
        ("a string", "un texte"),
        ("a boolean", "true ou false"),
        ("a sequence", "une liste de tables"),
        ("string or map", "un texte ou une table"),
        ("map with a single key", "une table à une seule clé, comme { volume = 0.5 }"),
        ("f64", "un nombre"),
        ("f32", "un nombre"),
        ("u16", "un entier de 0 à 65535"),
        ("variant identifier", "un nom de type"),
        ("internally tagged enum Element", "une table avec un type"),
        ("literal string", "un texte entre guillemets"),
        ("newline", "un retour à la ligne"),
    ];
    const MESSAGES: [(&str, &str); 14] = [
        ("unknown variant", "valeur inconnue"),
        ("unknown field", "champ inconnu"),
        ("missing field", "champ manquant"),
        ("duplicate field", "champ en double"),
        ("duplicate key", "clé en double"),
        ("there are no fields", "aucun champ n'est permis ici"),
        ("invalid basic string", "texte entre guillemets mal fermé"),
        ("invalid literal string", "texte entre apostrophes mal fermé"),
        ("missing escaped value", "caractère inconnu après \\"),
        ("unclosed array table", "en-tête [[…]] mal fermé"),
        ("unclosed table", "en-tête […] mal fermé"),
        ("string values must be quoted", "un texte doit être entre guillemets"),
        ("unexpected key or value", "clé ou valeur en trop"),
        ("` or `", "` ou `"),
    ];
    let mut texte = message.to_string();
    for (prefixe, francais) in [("invalid type: ", "type invalide : "), ("invalid value: ", "valeur invalide : ")] {
        for (anglais, mot) in TROUVE {
            texte = texte.replace(&format!("{prefixe}{anglais}"), &format!("{francais}{mot}"));
        }
        texte = texte.replace(prefixe, francais);
    }
    for (anglais, francais) in ATTENDU {
        texte = texte.replace(&format!("expected {anglais}"), &format!("attendu {francais}"));
    }
    for (anglais, francais) in MESSAGES {
        texte = texte.replace(anglais, francais);
    }
    texte = texte.replace("expected", "attendu");
    let mut ouvert = false;
    let texte: String = texte
        .chars()
        .map(|c| match c {
            '`' => {
                ouvert = !ouvert;
                if ouvert { '«' } else { '»' }
            }
            c => c,
        })
        .collect();
    // Un champ manquant n'a pas d'écriture à reprendre : ses deux noms (les champs obligatoires qui ont un alias).
    const REQUIS: [(&str, &str); 9] = [
        ("nom", "name"),
        ("outil", "tool"),
        ("reglage", "setting"),
        ("commande", "command"),
        ("intervalle", "interval"),
        ("largeur", "width"),
        ("champ", "field"),
        ("seuils", "thresholds"),
        ("couleurs", "colors"),
    ];
    REQUIS.iter().fold(texte, |t, (fr, en)| t.replace(&format!("champ manquant «{fr}»"), &format!("champ manquant «{fr}» (ou «{en}»)")))
}

/// Une barre qui exerce chaque nouveauté de la 0.2 (dont ce que l'éditeur ne sait pas éditer : image, keycode).
#[cfg(test)]
pub(crate) const BARRE_COMPLETE: &str = r##"
[[element]]
type = "bouton"
texte = "Pr"
image = "~/premiere.png"
gabarit = true
couleur = "#222222"
couleur_texte = "#FFFFFF"
taille = 13
affichage = "les_deux"
largeur = 90
action = { touche = "cmd+shift+4" }
appui_long = { keycode = 53 }

[[element]]
type = "bouton"
symbole = "safari"
action = { url = "https://example.org" }

[[element]]
type = "bouton"
texte = "Figma"
action = { app = "Figma" }

[[element]]
type = "bouton"
texte = "Pause"
action = { applescript = "tell application \"Music\" to playpause" }

[[element]]
type = "bouton"
texte = "30 %"
action = { luminosite = 0.3 }
appui_long = { volume = 0.5 }

[[element]]
type = "script"
commande = "uptime"
intervalle = 2
delai = 1.5
largeur = 120
action = "echap"

[[element]]
type = "slider"
symbole = "music.note"
lire = "osascript -e 'tell application \"Spotify\" to sound volume'"
regler = "osascript -e 'tell application \"Spotify\" to set sound volume to {valeur}'"
min = 0
max = 100
pas = 5
toutes_les = 2
style = "grille"
crans = 12
couleurs = ["#30D158", "#FFD60A", "#FF453A"]
seuils = [60, 85]
haptique = true

[[element]]
type = "choix"
options = ["1×", "1,5×", "2×"]
action = { shell = "echo {valeur} > /tmp/vitesse" }
lire = "cat /tmp/vitesse"
toutes_les = 10
largeur = 150

[[element]]
type = "groupe"
texte = "Export"
largeur = 100
  [[element.element]]
  type = "batterie"
  symbole = false
  largeur = 80

[[element]]
type = "groupe"
symbole = "square.grid.2x2"
  [[element.element]]
  type = "lanceur"
  app = "Safari"
  [[element.element]]
  type = "lanceur"
  app = "com.apple.TextEdit"
  texte = "Texte"
  affichage = "texte"
  largeur = 90
  appui_long = "launchpad"

[[element]]
type = "bouton"
action = "mission_control"
appui_long = "fenetres_app"

[[element]]
type = "bouton"
action = "bureau"
appui_long = "spotlight"

[[element]]
type = "bouton"
action = "capture_ecran"
appui_long = "capture_zone"

[[element]]
type = "bouton"
action = "capture_options"
appui_long = "verrouiller"

[[element]]
type = "bouton"
action = "emojis"
appui_long = "forcer_quitter"

[[element]]
type = "souple"

[[element]]
type = "espace"
largeur = 10

[[element]]
type = "texte"
source = "meteo"
gabarit = "{current.temperature_2m} °C"
symbole_selon = { champ = "current.weather_code", valeurs = { "0" = "sun.max.fill" }, defaut = "cloud.fill" }
couleur_selon = { champ = "current.temperature_2m", seuils = [10, 25], couleurs = ["#0A84FF", "#30D158", "#FF9F0A"] }

[[element]]
type = "bouton"
texte = "{heure}"
source = "horloge"

[[source]]
nom = "meteo"
http = "https://api.open-meteo.com/v1/forecast?latitude=48.85&longitude=2.35&current=temperature_2m,weather_code"
toutes_les = 600
entetes = { Authorization = "Bearer trousseau:meteo" }

[[source]]
nom = "horloge"
mcp = { commande = "python3 horloge-mcp.py", outil = "heure", arguments = { ville = "Paris", jours = 2 } }

[[application]]
app = "com.apple.Safari"
herite = true

[[application.element]]
type = "bouton"
texte = "Nouvel onglet"
action = { touche = "cmd+t" }

[[application.element]]
type = "groupe"
texte = "Plus"
  [[application.element.element]]
  type = "heure"
"##;

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
            source: None,
        }
    }

    #[test]
    fn exemple_donne_la_barre_attendue() {
        let attendu = Barre {
            masquer_control_strip: true,
            elements: vec![
                bouton_symbole(Some("apple.logo"), Some(56.0), Action::BarreApple),
                Element::slider(Reglage::Luminosite, Some("sun.max.fill".into())),
                Element::slider(Reglage::Volume, Some("speaker.wave.2.fill".into())),
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
            applications: vec![],
            sources: vec![],
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
            (r#""mission_control""#, Action::MissionControl),
            (r#""fenetres_app""#, Action::FenetresApp),
            (r#""launchpad""#, Action::Launchpad),
            (r#""bureau""#, Action::Bureau),
            (r#""spotlight""#, Action::Spotlight),
            (r#""capture_ecran""#, Action::CaptureEcran),
            (r#""capture_zone""#, Action::CaptureZone),
            (r#""capture_options""#, Action::CaptureOptions),
            (r#""verrouiller""#, Action::Verrouiller),
            (r#""emojis""#, Action::Emojis),
            (r#""forcer_quitter""#, Action::ForcerQuitter),
        ] {
            let texte = format!("[[element]]\ntype = \"bouton\"\naction = {ecrit}\n");
            assert_eq!(
                lire(&texte),
                Ok(Barre {
                    masquer_control_strip: true,
                    elements: vec![bouton(action)],
                    applications: vec![],
                    sources: vec![],
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
appui_long = "mission_control"

[[element]]
type = "bouton"
action = "fenetres_app"
appui_long = "launchpad"

[[element]]
type = "bouton"
action = "bureau"
appui_long = "spotlight"

[[element]]
type = "bouton"
action = "capture_ecran"
appui_long = "capture_zone"

[[element]]
type = "bouton"
action = "capture_options"
appui_long = "verrouiller"

[[element]]
type = "bouton"
action = "emojis"
appui_long = "forcer_quitter"

[[element]]
type = "lanceur"
app = "Safari"
texte = "Web"
affichage = "les_deux"
largeur = 80
appui_long = "launchpad"

[[element]]
type = "slider"
reglage = "luminosite"
symbole = "sun.max.fill"

[[element]]
type = "slider"
reglage = "volume"

[[element]]
type = "choix"
options = ["1×", "1,5×", "2×"]
action = { shell = "echo {valeur}" }
lire = "echo 1×"
toutes_les = 10
largeur = 150

[[element]]
type = "groupe"
symbole = "slider.horizontal.3"

  [[element.element]]
  type = "curseur"
  symbole = "music.note"
  lire = "echo 40"
  regler = "echo {valeur}"
  min = 0
  max = 100
  pas = 5
  toutes_les = 2
  style = "classique"
  crans = 12
  couleurs = ["#30D158", "#FFD60A", "#FF453A"]
  seuils = [60, 85]
  haptique = true

  [[element.element]]
  type = "slider"
  regler = "echo {valeur}"
  style = "grille"

  [[element.element]]
  type = "slider"
  regler = "echo {valeur}"
  style = "jauge"

  [[element.element]]
  type = "slider"
  regler = "echo {valeur}"
  style = "points"

  [[element.element]]
  type = "slider"
  regler = "echo {valeur}"
  style = "egaliseur"

  [[element.element]]
  type = "slider"
  regler = "echo {valeur}"
  style = "pas_a_pas"

  [[element.element]]
  type = "slider"
  regler = "echo {valeur}"
  style = "cadran"

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

[[element]]
type = "texte"
source = "meteo"
gabarit = "{current.temperature_2m} °C"
symbole_selon = { champ = "current.weather_code", valeurs = { "0" = "sun.max.fill" }, defaut = "cloud.fill" }
couleur_selon = { champ = "current.temperature_2m", seuils = [10, 25], couleurs = ["#0A84FF", "#30D158", "#FF9F0A"] }
largeur = 110
action = "suivant"

[[application]]
app = "com.apple.Safari"
herite = true
  [[application.element]]
  type = "bouton"
  texte = "{heure}"
  source = "horloge"
  action = { touche = "cmd+t" }

[[source]]
nom = "meteo"
http = "https://example.org/meteo.json"
toutes_les = 600
entetes = { Authorization = "Bearer trousseau:meteo" }

[[source]]
nom = "horloge"
mcp = { commande = "python3 horloge-mcp.py", outil = "heure", arguments = { ville = "Paris" } }

[[source]]
nom = "distante"
mcp = { url = "https://example.org/mcp", outil = "heure" }

[[source]]
nom = "uptime"
commande = "uptime"
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
long_press = "mission_control"

[[item]]
type = "button"
action = "app_windows"
long_press = "launchpad"

[[item]]
type = "button"
action = "show_desktop"
long_press = "spotlight"

[[item]]
type = "button"
action = "screenshot"
long_press = "screenshot_area"

[[item]]
type = "button"
action = "screenshot_options"
long_press = "lock_screen"

[[item]]
type = "button"
action = "emoji"
long_press = "force_quit"

[[item]]
type = "launcher"
app = "Safari"
text = "Web"
display = "both"
width = 80
long_press = "launchpad"

[[item]]
type = "slider"
setting = "brightness"
symbol = "sun.max.fill"

[[item]]
type = "slider"
setting = "volume"

[[item]]
type = "choice"
options = ["1×", "1,5×", "2×"]
action = { shell = "echo {valeur}" }
read = "echo 1×"
every = 10
width = 150

[[item]]
type = "group"
symbol = "slider.horizontal.3"

  [[item.item]]
  type = "slider"
  symbol = "music.note"
  read = "echo 40"
  set = "echo {valeur}"
  min = 0
  max = 100
  step = 5
  every = 2
  style = "classic"
  notches = 12
  colors = ["#30D158", "#FFD60A", "#FF453A"]
  thresholds = [60, 85]
  haptic = true

  [[item.item]]
  type = "slider"
  set = "echo {valeur}"
  style = "grid"

  [[item.item]]
  type = "slider"
  set = "echo {valeur}"
  style = "gauge"

  [[item.item]]
  type = "slider"
  set = "echo {valeur}"
  style = "dots"

  [[item.item]]
  type = "slider"
  set = "echo {valeur}"
  style = "equalizer"

  [[item.item]]
  type = "slider"
  set = "echo {valeur}"
  style = "stepper"

  [[item.item]]
  type = "slider"
  set = "echo {valeur}"
  style = "dial"

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

[[item]]
type = "text"
source = "meteo"
template = "{current.temperature_2m} °C"
symbol_by = { field = "current.weather_code", values = { "0" = "sun.max.fill" }, default = "cloud.fill" }
color_by = { field = "current.temperature_2m", thresholds = [10, 25], colors = ["#0A84FF", "#30D158", "#FF9F0A"] }
width = 110
action = "next"

[[application]]
app = "com.apple.Safari"
inherit = true
  [[application.item]]
  type = "button"
  text = "{heure}"
  source = "horloge"
  action = { key = "cmd+t" }

[[source]]
name = "meteo"
http = "https://example.org/meteo.json"
every = 600
headers = { Authorization = "Bearer trousseau:meteo" }

[[source]]
name = "horloge"
mcp = { command = "python3 horloge-mcp.py", tool = "heure", arguments = { ville = "Paris" } }

[[source]]
name = "distante"
mcp = { url = "https://example.org/mcp", tool = "heure" }

[[source]]
name = "uptime"
command = "uptime"
"##;

    #[test]
    fn alias_anglais_donnent_la_meme_barre() {
        let fr = lire(TOUT_FR).unwrap();
        assert_eq!(lire(TOUT_EN), Ok(fr.clone()));
        assert_eq!((fr.elements.len(), fr.sources.len()), (27, 4));
        assert_eq!((fr.applications.len(), fr.applications[0].herite), (1, true));
        assert_eq!(lire(include_str!("../exemples/barre.en.toml")), lire(PAR_DEFAUT));
        // Une erreur dans un [[item]] garde sa ligne.
        assert!(lire("[[item]]\ntype = \"clock\"\n\n[[item]]\ntype = \"button\"\naction = \"dance\"\n").unwrap_err().starts_with("ligne 4 : valeur inconnue «dance»"));
    }

    /// Chaque ligne du glossaire FR → EN du README est exercée par `TOUT_FR` / `TOUT_EN`.
    #[test]
    fn glossaire_du_readme() {
        let mots = |texte: &str| texte.split(|c: char| !(c.is_alphanumeric() || c == '_')).map(String::from).collect::<std::collections::HashSet<_>>();
        let (fr, en) = (mots(TOUT_FR), mots(TOUT_EN));
        let mut glossaires = Vec::new();
        for (readme, section) in [(include_str!("../README.fr.md"), "### Glossaire"), (include_str!("../README.md"), "### English aliases")] {
            let glossaire = readme.split(section).nth(1).expect(section).split("\n## ").next().unwrap();
            let lignes: Vec<(&str, &str)> = glossaire
                .lines()
                .filter(|l| l.starts_with("| `"))
                .map(|l| {
                    let mut cases = l.split('|').map(|c| c.trim().trim_matches('`'));
                    (cases.nth(1).unwrap(), cases.next().unwrap())
                })
                .collect();
            assert!(lignes.len() >= 45, "{section} : {} lignes", lignes.len());
            for &(f, e) in &lignes {
                assert!(fr.contains(f), "{section} : « {f} » n'est pas dans TOUT_FR");
                assert!(en.contains(e), "{section} : « {e} » n'est pas dans TOUT_EN");
            }
            glossaires.push(lignes);
        }
        // README.md et README.fr.md donnent le même glossaire.
        assert_eq!(glossaires[0], glossaires[1]);
    }

    #[test]
    fn sources_lues_et_controlees() {
        let barre = lire(TOUT_FR).unwrap();
        assert_eq!(barre.sources[1].mcp.as_ref().unwrap().arguments, Some(serde_json::json!({ "ville": "Paris" })));
        assert_eq!(barre.sources[0].entetes["Authorization"], "Bearer trousseau:meteo");
        // Une source écrit un seul de http, commande ou mcp ; mcp, commande ou url.
        let erreur = lire("[[element]]\ntype = \"heure\"\n\n[[source]]\nnom = \"x\"\nhttp = \"https://a\"\ncommande = \"b\"\n").unwrap_err();
        assert_eq!(erreur, "ligne 4 : [[source]] « x » : écris un seul de http, commande ou mcp");
        let erreur = lire("[[source]]\nnom = \"x\"\nmcp = { outil = \"t\" }\n").unwrap_err();
        assert_eq!(erreur, "ligne 1 : [[source]] « x » : mcp demande commande ou url, pas les deux");
        assert!(lire("[[source]]\nnom = \"x\"\nhtp = \"https://a\"\n").unwrap_err().starts_with("ligne 3 : champ inconnu «htp»"));
        let texte = "[[element]]\ntype = \"texte\"\nsource = \"metoe\"\n\n[[element]]\ntype = \"texte\"\ngabarit = \"{a}\"\n\n[[element]]\ntype = \"texte\"\nsource = \"m\"\ncouleur_selon = { champ = \"t\", seuils = [1], couleurs = [\"bleu\"] }\nsymbole_selon = { champ = \"c\", valeurs = { \"0\" = \"soleil\" } }\n\n[[element]]\ntype = \"bouton\"\nsource = \"inconnue\"\n\n[[source]]\nnom = \"m\"\nhttp = \"http://example.org/x\"\ntoutes_les = 1\nentetes = { Authorization = \"Bearer sk_live_0123456789abcdefghij\", Accept = \"application/json\" }\n\n[[source]]\nnom = \"m\"\nmcp = { url = \"https://example.org/mcp\", outil = \"t\" }\n";
        let avis: Vec<String> = controler(&lire(texte).unwrap(), Some(texte)).iter().map(|a| a.to_string()).collect();
        assert_eq!(
            avis,
            [
                "ligne 1 : aucune [[source]] ne s'appelle « metoe » : l'élément affichera ⚠ source",
                "ligne 5 : un gabarit sans source n'affiche rien : ajoute source = \"…\"",
                "ligne 9 : le symbole SF « soleil » n'existe pas : il ne sera pas affiché",
                "ligne 9 : la couleur « bleu » doit s'écrire #RRGGBB : elle sera ignorée",
                "ligne 9 : couleur_selon : 1 couleur pour 1 seuil : il faut une couleur de plus que de seuils",
                "ligne 15 : aucune [[source]] ne s'appelle « inconnue » : l'élément affichera ⚠ source",
                "ligne 19 : « http://example.org/x » : seules les adresses https:// (et http://localhost) sont lues ; la source restera vide",
                "ligne 19 : toutes_les est ramené à 5 secondes, le minimum",
                "ligne 19 : l'en-tête « Authorization » semble contenir une clé en clair : range-la dans le trousseau et écris trousseau:service",
                "ligne 25 : le nom « m » est déjà pris par une autre source : renomme-la",
            ]
        );
        // La clé elle-même n'apparaît dans aucun avis.
        assert!(avis.iter().all(|a| !a.contains("sk_live")));
    }

    /// Curseurs libres et choix : un slider règle une chose (luminosité, volume ou commande), et `controler` signale
    /// bornes, pas, seuils, couleurs et `{valeur}` absents.
    #[test]
    fn curseurs_et_choix_controles() {
        let erreur = lire("[[element]]\ntype = \"bouton\"\n\n[[element]]\ntype = \"curseur\"\nsymbole = \"music.note\"\n").unwrap_err();
        assert!(erreur.starts_with("ligne 4 : slider sans réglage"), "{erreur}");
        let erreur = lire("[[element]]\ntype = \"groupe\"\n  [[element.element]]\n  type = \"slider\"\n  reglage = \"volume\"\n  regler = \"x {valeur}\"\n").unwrap_err();
        assert!(erreur.starts_with("ligne 3 : slider : écris reglage"), "{erreur}");
        assert!(lire("[[element]]\ntype = \"slider\"\nsetting = \"volume\"\nread = \"echo 3\"\n").unwrap_err().contains("lire ne sert qu'avec regler"));
        assert!(lire("[[element]]\ntype = \"slider\"\nreglage = \"volume\"\nstyle = \"vague\"\n").unwrap_err().starts_with("ligne 1 : valeur inconnue «vague»"));
        let texte = "[[element]]\ntype = \"slider\"\nregler = \"echo 1\"\nmin = 10\nmax = 5\n\n[[element]]\ntype = \"slider\"\nset = \"echo {value}\"\nstep = 200\nthresholds = [50, 120]\ncolors = [\"vert\", \"#FFD60A\"]\nnotches = 99\nevery = 0.1\n\n[[element]]\ntype = \"slider\"\nreglage = \"volume\"\nstyle = \"grille\"\nseuils = [80, 60]\n\n[[element]]\ntype = \"choix\"\noptions = []\naction = { shell = \"echo fixe\" }\n";
        let avis: Vec<String> = controler(&lire(texte).unwrap(), Some(texte)).iter().map(|a| a.to_string()).collect();
        assert_eq!(
            avis,
            [
                "ligne 1 : min (10) doit être plus petit que max (5) : le curseur ira de 0 à 100",
                "ligne 1 : regler ne contient pas {valeur} : « echo 1 » recevra toujours la même chose",
                "ligne 7 : step doit être plus grand que 0 et pas plus grand que max − min (100) : il sera ignoré",
                "ligne 7 : every est ramené à 0.5 seconde, le minimum",
                "ligne 7 : notches = 99 est ramené entre 2 et 40",
                "ligne 7 : la couleur « vert » doit s'écrire #RRGGBB : elle sera ignorée",
                "ligne 7 : colors : 2 couleurs pour 2 seuils : il faut une couleur de plus que de seuils",
                "ligne 7 : thresholds : 120 est hors du curseur (de 0 à 100, bornes exclues)",
                "ligne 16 : seuils : écris-les du plus petit au plus grand",
                "ligne 22 : aucune option : le choix sera vide ; écris options = [\"1×\", \"2×\"]",
                "ligne 22 : l'action ne contient pas {valeur} : chaque option lancera la même chose",
            ]
        );
        // Le modèle d'exemple passe sans avis, et traverse TOML ⇄ JSON.
        let exemple = include_str!("../exemples/curseurs.toml");
        let barre = lire(exemple).unwrap();
        assert_eq!(controler(&barre, Some(exemple)).iter().filter(|a| !a.message.contains("introuvable")).count(), 0);
        let json = serde_json::to_string(&barre).unwrap();
        assert_eq!(serde_json::from_str::<Barre>(&json).unwrap(), barre);
    }

    /// Une variable qu'aucune valeur ne peut avoir, dans le gabarit d'un `texte` ou le texte d'un bouton lié (Sam, B2).
    #[test]
    fn variables_de_gabarit_controlees() {
        let texte = "[[element]]\ntype = \"texte\"\nsource = \"s\"\ngabarit = \"{text} {Text}\"\n\n[[element]]\ntype = \"bouton\"\nsource = \"s\"\ntexte = \"{a..b}\"\n\n[[element]]\ntype = \"bouton\"\ntexte = \"{Text} sans source\"\n\n[[source]]\nnom = \"s\"\ncommande = \"echo\"\n";
        let avis: Vec<String> = controler(&lire(texte).unwrap(), Some(texte)).iter().map(|a| a.to_string()).collect();
        let possibles = "variables possibles : {texte} ou {text} (sortie qui n'est pas du JSON), sinon le chemin d'un champ du JSON, comme {current.temperature_2m}";
        assert_eq!(avis, [format!("ligne 1 : {{Text}} n'existe pas ; {possibles}"), format!("ligne 6 : {{a..b}} n'existe pas ; {possibles}")]);
    }

    /// Les messages citent la clé telle qu'écrite, alias anglais compris (Sam, B3 et B4).
    #[test]
    fn messages_avec_la_cle_ecrite() {
        let texte = "[[item]]\ntype = \"text\"\ntemplate = \"{a}\"\n\n[[item]]\ntype = \"text\"\nsource = \"s\"\ncolor_by = { field = \"t\", thresholds = [1], colors = [\"#000000\"] }\n\n[[application]]\napp = \"Safari\"\n[[application.item]]\ntype = \"flexible\"\n\n[[item]]\ntype = \"clock\"\n\n[[source]]\nname = \"s\"\ncommand = \"echo\"\nevery = 1\n";
        let avis: Vec<String> = controler(&lire(texte).unwrap(), Some(texte)).iter().map(|a| a.to_string()).collect();
        assert_eq!(
            avis,
            [
                "ligne 1 : un template sans source n'affiche rien : ajoute source = \"…\"",
                "ligne 5 : color_by : 1 couleur pour 1 seuil : il faut une couleur de plus que de seuils",
                "ligne 15 : cet [[item]] est écrit après un [[application]] : il va dans la barre principale ; voulais-tu [[application.item]] ?",
                "ligne 18 : every est ramené à 5 secondes, le minimum",
            ]
        );
        let erreur = lire("[[source]]\nname = \"x\"\nhttp = \"https://a\"\ncommand = \"b\"\n").unwrap_err();
        assert_eq!(erreur, "ligne 1 : [[source]] « x » : écris un seul de http, command ou mcp");
        let erreur = lire("[[source]]\nname = \"x\"\nmcp = { command = \"a\", url = \"https://b\", tool = \"t\" }\n").unwrap_err();
        assert_eq!(erreur, "ligne 1 : [[source]] « x » : mcp demande command ou url, pas les deux");
        // Un champ manquant n'a pas été écrit : ses deux noms.
        let erreur = lire("[[source]]\nname = \"x\"\nmcp = { command = \"a\" }\n").unwrap_err();
        assert!(erreur.starts_with("ligne 3 : champ manquant «outil» (ou «tool»)"), "{erreur}");
        // `template` texte sur un bouton : ce qu'il faut écrire, pas « attendu true ou false ».
        let erreur = lire("[[element]]\ntype = \"heure\"\n\n[[item]]\ntype = \"button\"\nsource = \"s\"\ntemplate = \"{texte}\"\n").unwrap_err();
        assert_eq!(erreur, "ligne 7 : « template » : sur un bouton, le texte lié à une source s'écrit texte / text ; gabarit / template est l'interrupteur monochrome de l'image (true ou false)");
    }

    /// La même app sous son nom et son bundle id (Lise, sonde 22) ; un bundle id bien formé d'une app absente
    /// n'a pas besoin du conseil d'écriture (Lise, bug 4).
    #[test]
    fn meme_app_sous_deux_noms_et_bundle_id_absent() {
        let texte = "[[element]]\ntype = \"lanceur\"\napp = \"com.example.absente\"\n\n[[element]]\ntype = \"bouton\"\naction = { app = \"com.example.absente\" }\n\n[[application]]\napp = \"Finder\"\n[[application.element]]\ntype = \"souple\"\n\n[[application]]\napp = \"com.apple.finder\"\n[[application.element]]\ntype = \"souple\"\n\n[[application]]\napp = \"com.example.absente\"\n[[application.element]]\ntype = \"souple\"\n";
        let avis: Vec<String> = controler(&lire(texte).unwrap(), Some(texte)).iter().map(|a| a.to_string()).collect();
        assert_eq!(
            avis,
            [
                "ligne 1 : aucune app installée n'a le bundle id « com.example.absente » : le lanceur affichera son nom en texte",
                "ligne 5 : aucune app installée n'a le bundle id « com.example.absente »",
                "ligne 14 : « com.apple.finder » est la même app que « Finder », qui a déjà sa barre (ligne 9) : celle-ci ne sera jamais montrée",
                "ligne 19 : aucune app installée n'a le bundle id « com.example.absente » : sa barre ne sera pas montrée",
            ]
        );
    }

    #[test]
    fn barre_par_application() {
        let barre = lire("[[element]]\ntype = \"heure\"\n\n[[application]]\napp = \"com.apple.Safari\"\n[[application.element]]\ntype = \"bouton\"\ntexte = \"Nouvel onglet\"\naction = { touche = \"cmd+t\" }\n\n[[application]]\napp = \"Aperçu\"\n[[application.element]]\ntype = \"souple\"\n").unwrap();
        assert_eq!(barre.elements.len(), 1);
        assert_eq!(barre.applications.len(), 2);
        assert!(!barre.applications[0].herite);
        assert_eq!(barre.applications[0].elements.len(), 1);
        // Par bundle id ou par nom affiché, sans casse (accents compris).
        assert_eq!(barre.application_pour("com.apple.safari", "Safari"), Some(0));
        assert_eq!(barre.application_pour("com.example.x", "COM.APPLE.SAFARI"), Some(0));
        assert_eq!(barre.application_pour("com.apple.Preview", "APERÇU"), Some(1));
        assert_eq!(barre.application_pour("com.apple.finder", "Finder"), None);
        assert_eq!(barre.application_pour("", ""), None);
    }

    #[test]
    fn application_sans_app_ou_sans_element() {
        let erreur = lire("[[element]]\ntype = \"heure\"\n\n[[application]]\nherite = true\n[[application.element]]\ntype = \"souple\"\n").unwrap_err();
        assert!(erreur.starts_with("ligne 4 : champ manquant «app»"), "{erreur}");
        let erreur = lire("[[element]]\ntype = \"heure\"\n\n[[application]]\napp = \"Safari\"\n").unwrap_err();
        assert_eq!(erreur, "ligne 4 : [[application]] « Safari » sans élément : ajoute au moins un [[application.element]]");
        let erreur = lire("[[application]]\napp = \"Safari\"\n[[application.element]]\ntype = \"truc\"\n").unwrap_err();
        assert!(erreur.contains("valeur inconnue «truc»"), "{erreur}");
        let erreur = lire("[[application]]\napp = \"Safari\"\nherites = true\n[[application.element]]\ntype = \"souple\"\n").unwrap_err();
        assert!(erreur.starts_with("ligne 3 : champ inconnu «herites»"), "{erreur}");
    }

    #[test]
    fn element_et_item_se_melangent_dans_l_ordre_du_fichier() {
        let texte = "[[element]]\ntype = \"heure\"\n\n[[item]]\ntype = \"flexible\"\n\n[[element]]\ntype = \"espace\"\nlargeur = 5\n\n[[item]]\ntype = \"group\"\n  [[item.element]]\n  type = \"souple\"\n  [[item.item]]\n  type = \"clock\"\n\n[[application]]\napp = \"Safari\"\n[[application.item]]\ntype = \"souple\"\n[[application.element]]\ntype = \"heure\"\n";
        let barre = lire(texte).unwrap();
        let heure = || Element::Heure { format: None };
        let groupe = Element::Groupe { texte: None, symbole: None, largeur: None, elements: vec![Element::Souple, heure()] };
        assert_eq!(barre.elements, [heure(), Element::Souple, Element::Espace { largeur: 5.0 }, groupe]);
        assert_eq!(barre.applications[0].elements, [Element::Souple, heure()]);
        // Une erreur dans le mélange garde sa ligne.
        let erreur = lire("[[element]]\ntype = \"heure\"\n\n[[item]]\ntype = \"clock\"\n\n[[element]]\ntype = \"truc\"\n").unwrap_err();
        assert!(erreur.starts_with("ligne 8 : valeur inconnue «truc»"), "{erreur}");
    }

    #[test]
    fn erreur_situee_dans_une_application_et_dans_l_ordre_du_fichier() {
        // Troisième élément d'une app (Lise, sonde 25) : la ligne de cet élément, pas celle du premier.
        let erreur = lire("[[element]]\ntype = \"heure\"\n\n[[application]]\napp = \"Safari\"\n[[application.element]]\ntype = \"souple\"\n[[application.element]]\ntype = \"souple\"\n[[application.element]]\ntype = \"bouton\"\naction = \"danser\"\n").unwrap_err();
        assert!(erreur.starts_with("ligne 10 : valeur inconnue «danser»"), "{erreur}");
        // Deuxième app (sonde 27).
        let erreur = lire("[[application]]\napp = \"Safari\"\n[[application.element]]\ntype = \"souple\"\n\n[[application]]\napp = \"Notes\"\n[[application.element]]\ntype = \"souple\"\n[[application.element]]\ntype = \"bouton\"\ntexte = 3\n").unwrap_err();
        assert!(erreur.starts_with("ligne 10 : type invalide : l'entier «3», attendu un texte"), "{erreur}");
        // Enfant d'un groupe dans une app.
        let erreur = lire("[[application]]\napp = \"Safari\"\n[[application.element]]\ntype = \"groupe\"\n  [[application.element.element]]\n  type = \"souple\"\n  [[application.element.element]]\n  type = \"truc\"\n").unwrap_err();
        assert!(erreur.starts_with("ligne 8 : valeur inconnue «truc»"), "{erreur}");
        // Deux erreurs (sonde 26) : celle de la principale, plus haut, sort la première.
        let erreur = lire("[[element]]\ntype = \"bouton\"\nactoin = \"suivant\"\n\n[[application]]\napp = \"Safari\"\n[[application.element]]\ntype = \"truc\"\n").unwrap_err();
        assert!(erreur.starts_with("ligne 1 : champ inconnu «actoin»"), "{erreur}");
    }

    #[test]
    fn messages_de_serde_en_francais() {
        for (texte, attendu) in [
            ("[[element]]\ntype = \"bouton\"\ntexte = \"a\n", "ligne 3 : texte entre guillemets mal fermé"),
            ("[[element]]\ntype = \"bouton\"\ntexte = [\"a\"]\n", "ligne 1 : type invalide : une liste, attendu un texte"),
            ("[[element]]\ntype = \"bouton\"\naction = { app = \"x\", url = \"y\" }\n", "ligne 1 : valeur invalide : une table, attendu une table à une seule clé"),
            ("[[element]]\ntype = \"bouton\"\ntexte = \"a\"\ntexte = \"b\"\n", "ligne 4 : clé en double"),
            ("[[element]]\ntype = \"bouton\"\nlargeur = \"grand\"\n", "ligne 1 : type invalide : le texte \"grand\", attendu un nombre"),
            ("[[element]]\ntype = \"bouton\"\ngabarit = 1\n", "ligne 1 : type invalide : l'entier «1», attendu true ou false"),
            ("[[element]]\ntype = \"bouton\"\naction = { keycode = -1 }\n", "ligne 1 : valeur invalide : l'entier «-1», attendu un entier de 0 à 65535"),
            ("[[element]]\ntype = \"script\"\n", "ligne 1 : champ manquant «commande»"),
            ("[[element]\ntype = \"souple\"\n", "ligne 1 : en-tête [[…]] mal fermé"),
            ("element = [\"x\"]\n", "ligne 1 : type invalide : le texte \"x\", attendu une table avec un type"),
        ] {
            let erreur = lire(texte).unwrap_err();
            assert!(erreur.starts_with(attendu), "{erreur}");
            for anglais in ["expected", "invalid ", "string", "sequence", "map", "duplicate", "unclosed"] {
                assert!(!erreur.contains(anglais), "« {anglais} » dans : {erreur}");
            }
        }
        assert_eq!((nombre(0, "élément"), nombre(1, "élément"), nombre(2, "élément")), ("0 élément".into(), "1 élément".into(), "2 éléments".into()));
    }

    #[test]
    fn controler_signale_ce_qui_casse_a_l_usage() {
        let texte = "[[element]]\ntype = \"bouton\"\nsymbole = \"scisors\"\ncouleur = \"#F00\"\ncouleur_texte = \"red\"\nimage = \"~/nexiste-pas.png\"\naction = { app = \"AppQuiNexistePas\" }\n\n[[application]]\napp = \"Safari\"\n[[application.element]]\ntype = \"souple\"\n\n[[element]]\ntype = \"souple\"\n\n[[application]]\napp = \"safari\"\n[[application.element]]\ntype = \"groupe\"\n  [[application.element.element]]\n  type = \"bouton\"\n  symbole = \"nexiste.pas\"\n";
        let barre = lire(texte).unwrap();
        let avis: Vec<String> = controler(&barre, Some(texte)).iter().map(|a| a.to_string()).collect();
        assert_eq!(
            avis,
            [
                "ligne 1 : le symbole SF « scisors » n'existe pas : le bouton affichera son nom en texte",
                "ligne 1 : la couleur « #F00 » doit s'écrire #RRGGBB : elle sera ignorée",
                "ligne 1 : la couleur « red » doit s'écrire #RRGGBB : elle sera ignorée",
                "ligne 1 : image « ~/nexiste-pas.png » introuvable ou illisible : le bouton affichera du texte",
                "ligne 1 : app « AppQuiNexistePas » introuvable : écris le nom du dossier .app sans l'extension, ou son bundle id",
                "ligne 14 : cet [[element]] est écrit après un [[application]] : il va dans la barre principale ; voulais-tu [[application.element]] ?",
                "ligne 17 : « safari » a déjà sa barre (ligne 9) : celle-ci ne sera jamais montrée",
                "ligne 21 : le symbole SF « nexiste.pas » n'existe pas : le bouton affichera son nom en texte",
            ]
        );
        assert!(controler(&barre, Some(texte)).iter().all(|a| !a.grave));
        // Sans le texte (éditeur) : le lieu au lieu de la ligne.
        let avis = controler(&barre, None);
        assert_eq!(avis[0].to_string(), "élément 1 : le symbole SF « scisors » n'existe pas : le bouton affichera son nom en texte");
        assert_eq!(avis.last().unwrap().to_string(), "application « safari », groupe 1, élément 1 : le symbole SF « nexiste.pas » n'existe pas : le bouton affichera son nom en texte");
        // Une barre juste ne donne rien ; une barre vide est grave.
        assert_eq!(controler(&lire(PAR_DEFAUT).unwrap(), Some(PAR_DEFAUT)), []);
        let vide = controler(&lire("").unwrap(), Some(""));
        assert_eq!((vide.len(), vide[0].grave), (1, true));
        assert!(rgb("#2D0A4E") == Some(0x2D0A4E) && rgb("#F00").is_none() && rgb("2D0A4E").is_none() && rgb("#GG0000").is_none());
    }

    #[test]
    fn controler_mesure_chaque_barre() {
        let boutons = |n: usize| (0..n).map(|_| "[[element]]\ntype = \"bouton\"\ntexte = \"Bouton\"\nlargeur = 100\n\n").collect::<String>();
        let texte = format!("{}[[element]]\ntype = \"souple\"\n\n[[application]]\napp = \"com.apple.Safari\"\nherite = true\n{}", boutons(5), boutons(8).replace("[[element]]", "[[application.element]]"));
        let barre = lire(&texte).unwrap();
        let avis: Vec<String> = controler(&barre, Some(&texte)).iter().map(|a| a.to_string()).collect();
        assert_eq!(avis, ["ligne 29 : la barre de « com.apple.Safari » déborde (environ 1420 pt pour 1004) : 5 éléments sur 15 seront cachés ; réduis des largeurs ou retire un élément"]);
        let texte = boutons(10);
        let avis = controler(&lire(&texte).unwrap(), Some(&texte));
        assert_eq!(avis[0].to_string(), "la barre principale déborde (environ 1080 pt pour 1004) : 1 élément sur 10 sera caché ; réduis des largeurs ou retire un élément");
    }

    #[test]
    fn herite_remplace_la_zone_entre_les_souples() {
        let heure = || Element::Heure { format: None };
        let espace = |l: f64| Element::Espace { largeur: l };
        let principale = [espace(1.0), Element::Souple, espace(2.0), Element::Souple, espace(3.0)];
        let mut app = Application { app: "Safari".into(), herite: false, elements: vec![heure()] };
        assert_eq!(app.elements_avec(&principale), [heure()]);
        app.herite = true;
        assert_eq!(app.elements_avec(&principale), [espace(1.0), Element::Souple, heure(), Element::Souple, espace(3.0)]);
        // Un seul souple : les éléments de l'app se placent à côté de lui, centrés entre deux souples.
        assert_eq!(app.elements_avec(&[espace(1.0), Element::Souple, espace(3.0)]), [espace(1.0), Element::Souple, heure(), Element::Souple, espace(3.0)]);
        // Sans souple : à la suite de la principale.
        assert_eq!(app.elements_avec(&[espace(1.0)]), [espace(1.0), heure()]);
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
                    Element::slider(Reglage::Volume, None),
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
                source: None,
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
    fn lanceur_seul_ou_dans_un_groupe() {
        let barre = lire("[[element]]\ntype = \"lanceur\"\napp = \"Safari\"\n\n[[element]]\ntype = \"groupe\"\nsymbole = \"square.grid.2x2\"\n  [[element.element]]\n  type = \"lanceur\"\n  app = \"AppQuiNexistePas\"\n  texte = \"X\"\n  appui_long = \"launchpad\"\n").unwrap();
        assert_eq!(barre.elements[0], Element::Lanceur { app: "Safari".into(), texte: None, affichage: None, largeur: None, appui_long: None });
        let Element::Groupe { elements, .. } = &barre.elements[1] else { panic!("{:?}", barre.elements[1]) };
        assert_eq!(elements[0], Element::Lanceur { app: "AppQuiNexistePas".into(), texte: Some("X".into()), affichage: None, largeur: None, appui_long: Some(Action::Launchpad) });
        assert!(lire("[[element]]\ntype = \"lanceur\"\n").unwrap_err().starts_with("ligne 1 : champ manquant «app»"));
        // Une app introuvable : l'avis de --verifier, avec la ligne du lanceur dans le groupe.
        let texte = "[[element]]\ntype = \"lanceur\"\napp = \"Safari\"\n\n[[element]]\ntype = \"groupe\"\n  [[element.element]]\n  type = \"lanceur\"\n  app = \"AppQuiNexistePas\"\n";
        let avis: Vec<String> = controler(&lire(texte).unwrap(), Some(texte)).iter().map(|a| a.to_string()).collect();
        assert_eq!(avis, ["ligne 7 : app « AppQuiNexistePas » introuvable : le lanceur affichera son nom en texte ; écris le nom du dossier .app sans l'extension, ou son bundle id"]);
    }

    #[test]
    fn aller_retour_toml() {
        for texte in [
            PAR_DEFAUT,
            include_str!("../exemples/barre.en.toml"),
            include_str!("../exemples/claude-vocal.toml"),
            include_str!("../exemples/simple.toml"),
            include_str!("../exemples/developpeur.toml"),
            include_str!("../exemples/par-application.toml"),
            include_str!("../exemples/lanceur.toml"),
            include_str!("../exemples/meteo.toml"),
            include_str!("../exemples/mcp.toml"),
            include_str!("../exemples/vitrine.toml"),
            include_str!("../exemples/curseurs.toml"),
            BARRE_COMPLETE,
        ] {
            let barre = lire(texte).unwrap();
            let ecrit = ecrire(&barre).unwrap();
            assert_eq!(lire(&ecrit), Ok(barre), "{ecrit}");
        }
    }

    #[test]
    fn chemin_suit_tactile_config() {
        // Lue à chaque appel : posée puis retirée, sans effet sur les autres tests.
        unsafe { std::env::set_var("TACTILE_CONFIG", "/tmp/essai-tactile/barre.toml") };
        let essai = chemin();
        unsafe { std::env::remove_var("TACTILE_CONFIG") };
        assert_eq!(essai, PathBuf::from("/tmp/essai-tactile/barre.toml"));
        assert!(chemin().ends_with(".config/tactile/barre.toml"));
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

/// Propriétés vérifiées sur des centaines de barres générées et de fichiers abîmés (voir `crate::essais`).
#[cfg(test)]
mod proprietes {
    use super::*;
    use crate::essais::{sans_panique, Hasard};

    /// TOML ⇄ Barre ⇄ JSON : toute barre que l'éditeur peut produire revient à l'identique, et son texte est stable.
    #[test]
    fn aller_retour_de_barres_generees() {
        for graine in 0..400 {
            let barre = Hasard::new(graine).barre();
            let toml = ecrire(&barre).unwrap_or_else(|e| panic!("graine {graine} : {e}"));
            let relue = lire(&toml).unwrap_or_else(|e| panic!("graine {graine} : {e}\n{toml}"));
            assert_eq!(relue, barre, "graine {graine}, TOML :\n{toml}");
            assert_eq!(ecrire(&relue).unwrap(), toml, "graine {graine} : texte instable");
            let json = serde_json::to_string(&barre).unwrap();
            assert_eq!(serde_json::from_str::<Barre>(&json).unwrap(), barre, "graine {graine}, JSON : {json}");
        }
    }

    /// `nan` et `inf` sont du TOML valide mais pas une largeur ni une durée : la page de l'éditeur les recevrait en
    /// `null` (JSON) et ne pourrait plus enregistrer, la barre ferait une durée infinie. Refusés à la lecture, situés.
    #[test]
    fn nombres_non_finis_refuses() {
        for (texte, ligne) in [
            ("[[element]]\ntype = \"espace\"\nlargeur = nan\n", 3),
            ("[[element]]\ntype = \"script\"\ncommande = \"date\"\nintervalle = inf\n", 4),
            ("[[element]]\ntype = \"souple\"\n\n[[element]]\ntype = \"bouton\"\naction = { volume = -inf }\n", 6),
            ("[[element]]\ntype = \"groupe\"\n  [[element.element]]\n  type = \"batterie\"\n  largeur = +nan\n", 5),
        ] {
            let erreur = lire(texte).unwrap_err();
            assert!(erreur.starts_with(&format!("ligne {ligne} : ")) && erreur.contains("nombre fini"), "{erreur}\n{texte}");
        }
        assert!(lire("[[element]]\ntype = \"espace\"\nlargeur = 1e3\n").is_ok());
        assert!(lire("[[element]]\ntype = \"bouton\"\ntexte = \"nan\"\n").is_ok(), "un texte « nan » n'est pas un nombre");
    }

    /// `[[item]]` (alias anglais) mélangé à `[[element]]` au hasard, dans la principale comme dans les barres
    /// d'application : même barre, dans l'ordre du fichier. Seuls les en-têtes sans sous-table sont renommés
    /// (`[element.action]` s'accroche au dernier `[[element]]` : c'est la règle de TOML, pas un alias).
    #[test]
    fn element_et_item_melanges_au_hasard() {
        let mut h = Hasard::new(77);
        let mut renommes = 0;
        for graine in 0..300 {
            let barre = Hasard::new(graine).barre();
            let toml = ecrire(&barre).unwrap();
            let lignes: Vec<&str> = toml.lines().collect();
            let melange: Vec<String> = lignes
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    let (entete, prefixe) = match *l {
                        "[[element]]" => ("[[item]]", "element."),
                        "[[application.element]]" => ("[[application.item]]", "application.element."),
                        _ => return l.to_string(),
                    };
                    let suivant = lignes[i + 1..].iter().find(|s| s.starts_with('[') && s.ends_with(']'));
                    if suivant.is_some_and(|s| s.trim_start_matches('[').starts_with(prefixe)) || !h.pile() {
                        return l.to_string();
                    }
                    renommes += 1;
                    entete.to_string()
                })
                .collect();
            let melange = melange.join("\n");
            assert_eq!(lire(&melange).unwrap_or_else(|e| panic!("graine {graine} : {e}\n{melange}")), barre, "graine {graine}\n{melange}");
        }
        assert!(renommes > 300, "{renommes}");
    }

    /// Fonctionnel 0.3 : un script à 0,3 s coûte 0,4 % de CPU à lui seul ; au-delà de 2 scripts sous 1 s ou de
    /// 5 lancements par seconde, `controler` le dit.
    #[test]
    fn rafraichissements_rapides_signales() {
        let script = |i: f64| format!("[[element]]\ntype = \"script\"\ncommande = \"date\"\nintervalle = {i}\n");
        let rapide = |texte: &str| controler(&lire(texte).unwrap(), None).iter().any(|a| a.message.contains("rafraîchissements rapides coûtent du CPU"));
        assert!(!rapide(&script(0.5)), "un script à 0,5 s : 2 par seconde");
        assert!(!rapide(&[script(0.9), script(0.9), script(5.0)].concat()));
        assert!(!rapide(&script(0.3)), "un script à 0,3 s : 3,3 par seconde, choix assumé (Claude vocal)");
        assert!(rapide(&script(0.15)), "un script à 0,15 s : 6,7 par seconde");
        assert!(rapide(&[script(0.3), script(0.3)].concat()), "deux à 0,3 s : 6,7 par seconde");
        assert!(rapide(&[script(0.9), script(0.9), script(0.9)].concat()), "3 scripts sous 1 s");
        let groupe = format!("[[element]]\ntype = \"heure\"\n[[application]]\napp = \"Finder\"\n[[application.element]]\ntype = \"groupe\"\n{}", script(0.1).replace("[[element]]", "[[application.element.element]]"));
        let avis = controler(&lire(&groupe).unwrap(), Some(&groupe));
        let a = avis.iter().find(|a| a.message.contains("coûtent du CPU")).expect("script dans un groupe d'une barre d'app");
        assert!(a.message.starts_with("la barre de « Finder » lance 1 script plus d'une fois par seconde (environ 10 lancements par seconde)"), "{}", a.message);
        assert_eq!(a.ligne, Some(3));
    }

    /// Un fichier tronqué, une ligne en double, un caractère en trop ou en moins : `lire` répond par une erreur située,
    /// jamais par une panique (la barre tournerait sans config, l'éditeur fermerait).
    #[test]
    fn lire_ne_panique_jamais_sur_un_toml_abime() {
        let mut textes: Vec<String> = [PAR_DEFAUT, BARRE_COMPLETE, include_str!("../exemples/par-application.toml"), include_str!("../exemples/lanceur.toml")]
            .map(String::from)
            .into();
        textes.extend((1000..1040).map(|g| ecrire(&Hasard::new(g).barre()).unwrap()));
        let mut h = Hasard::new(7);
        let mut cas = 0;
        for texte in &textes {
            for _ in 0..60 {
                let abime = abimer(&mut h, texte);
                cas += 1;
                let resultat = sans_panique(|| lire(&abime)).unwrap_or_else(|p| panic!("panique « {p} » sur :\n{abime}"));
                match resultat {
                    Err(message) => {
                        if let Some(n) = message.split("ligne ").nth(1).and_then(|s| s.split(' ').next()).and_then(|n| n.parse::<usize>().ok()) {
                            assert!(n >= 1 && n <= abime.lines().count() + 1, "ligne {n} hors du fichier : {message}\n{abime}");
                        }
                    }
                    Ok(barre) if h.jusqua(8) == 0 => {
                        sans_panique(|| controler(&barre, Some(&abime))).unwrap_or_else(|p| panic!("controler : panique « {p} » sur :\n{abime}"));
                    }
                    Ok(_) => {}
                }
            }
        }
        assert!(cas > 2000);
    }

    /// Une mutation au hasard d'un texte TOML, toujours sur des frontières de caractère.
    fn abimer(h: &mut Hasard, texte: &str) -> String {
        let car: Vec<char> = texte.chars().collect();
        let i = h.jusqua(car.len() + 1);
        let lignes: Vec<&str> = texte.lines().collect();
        match h.jusqua(6) {
            0 => car[..i].iter().collect(),
            1 => car.iter().enumerate().filter(|(k, _)| *k != i).map(|(_, c)| c).collect(),
            2 => {
                let mut c = car.clone();
                c.insert(i, *h.parmi(&['[', ']', '=', '"', '\'', '#', '\n', '{', '}', ',', '.', '+', '-', '\\', 'é', '\0', 'x', '9']));
                c.into_iter().collect()
            }
            3 if !lignes.is_empty() => {
                let k = h.jusqua(lignes.len());
                let mut l = lignes.clone();
                l.insert(k, lignes[k]);
                l.join("\n")
            }
            4 if lignes.len() > 1 => {
                let mut l = lignes.clone();
                let (a, b) = (h.jusqua(l.len()), h.jusqua(l.len()));
                l.swap(a, b);
                l.join("\n")
            }
            _ => {
                // Une valeur remplacée par une autre d'un autre type ou hors bornes.
                let v = h.parmi(&["nan", "inf", "-inf", "1e308", "-1", "99999999999999999999", "\"\"", "[]", "{}", "true", "1979-05-27"]);
                match texte.match_indices(" = ").nth(h.jusqua(texte.matches(" = ").count().max(1))) {
                    Some((p, _)) => {
                        let fin = texte[p + 3..].find('\n').map_or(texte.len(), |f| p + 3 + f);
                        format!("{}{v}{}", &texte[..p + 3], &texte[fin..])
                    }
                    None => texte.to_string(),
                }
            }
        }
    }
}
