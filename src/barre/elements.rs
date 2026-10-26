//! Construction de chaque type d'élément de la config en item de Touch Bar.

use objc2::runtime::Bool;
use objc2::{class, msg_send, sel};
use std::time::{Duration, Instant};

use super::rafraichi::{self, Lien, Rafraichi};
use super::vues::{barre_touch, couleur, icone_app, image, item_vue, lacher, largeur_fixe, largeur_max, styler, symbole, texte};
use super::{cible, Commande, Etat, Id, PREFIXE};
use crate::actions::{self, Action};
use crate::config::{Affichage, Element, Reglage};

/// Ce qui illustre un bouton : un symbole SF, ou un fichier image (gabarit ou non).
#[derive(Clone, Copy)]
pub(super) enum Icone<'a> {
    Symbole(&'a str),
    Image(&'a str, bool),
    /// L'icône d'une app (nom, bundle id ou chemin), avec le point si elle est ouverte.
    App(&'a str),
}

impl<'a> Icone<'a> {
    pub(super) fn charger(self) -> Option<Id> {
        match self {
            Icone::Symbole(nom) => symbole(nom),
            Icone::Image(chemin, gabarit) => image(chemin, gabarit),
            Icone::App(app) => actions::chemin_app(app).and_then(|c| icone_app(&c, actions::app_ouverte(&c))),
        }
    }

    pub(super) fn nom(self) -> &'a str {
        match self {
            Icone::Symbole(nom) | Icone::Image(nom, _) | Icone::App(nom) => nom,
        }
    }

    /// Ce qui s'affiche en texte quand l'icône manque, et le mot du journal.
    pub(super) fn repli(self) -> (&'a str, &'static str) {
        match self {
            Icone::Symbole(nom) => (nom, "symbole SF inconnu"),
            Icone::Image(chemin, _) => (chemin.rsplit('/').next().unwrap_or(chemin), "image introuvable"),
            Icone::App(app) => (app.rsplit('/').next().unwrap_or(app).trim_end_matches(".app"), "app introuvable"),
        }
    }
}

pub(super) fn enregistrer(etat: &mut Etat, court: Option<Commande>, long: Option<Commande>) -> usize {
    etat.commandes.push((court, long));
    etat.commandes.len() - 1
}

/// `element` est l'identifiant de l'item (`fr.palph.tactile.3.1` = premier enfant du troisième élément), pour le journal.
pub(super) fn bouton(titre: Option<&str>, icone: Option<Icone>, index: usize, avec_appui_long: bool, element: &str) -> Id {
    // Un symbole inconnu ou une image absente n'est pas une raison de perdre la barre : son nom s'affiche en texte.
    let (titre, image) = match icone.map(|i| (i, i.charger())) {
        Some((_, Some(i))) => (titre, Some(i)),
        Some((icone, None)) => {
            let (nom, quoi) = icone.repli();
            eprintln!("tactile : {quoi} « {} » (élément {})", icone.nom(), element.strip_prefix(PREFIXE).map_or(element, |e| e.trim_start_matches('.')));
            (titre.or(Some(nom)), None)
        }
        None => (titre, None),
    };
    unsafe {
        let b: Id = match image {
            Some(i) => msg_send![class!(NSButton), buttonWithImage: i, target: cible(), action: sel!(appui:)],
            None => msg_send![class!(NSButton), buttonWithTitle: &*texte(titre.unwrap_or("")), target: cible(), action: sel!(appui:)],
        };
        if let (Some(t), Some(_)) = (titre, image) {
            let _: () = msg_send![b, setTitle: &*texte(t)];
            let _: () = msg_send![b, setImagePosition: 2isize];
        }
        let _: () = msg_send![b, setTag: index as isize];
        if avec_appui_long {
            let r: Id = msg_send![class!(NSPressGestureRecognizer), alloc];
            let r: Id = msg_send![r, initWithTarget: cible(), action: sel!(appuiLong:)];
            let _: () = msg_send![r, setMinimumPressDuration: 0.5f64];
            let _: () = msg_send![r, setAllowedTouchTypes: 1usize];
            let _: () = msg_send![b, addGestureRecognizer: r];
            lacher(r);
        }
        b
    }
}

pub(super) fn bouton_texte(etat: &mut Etat, action: Option<Action>) -> Id {
    let index = enregistrer(etat, action.map(Commande::Systeme), None);
    let b = bouton(Some(""), None, index, false, "");
    unsafe { let _: () = msg_send![b, setBordered: Bool::NO]; }
    b
}

/// Bouton replié (symbole + chevron) qui ouvre une sous-barre faite de ✕ suivi de `items`.
pub(super) fn depliable(identifiant: &str, nom: Option<&str>, libelle: Option<&str>, largeur: Option<f64>, items: Vec<Id>, etat: &mut Etat) -> Id {
    let fermer = enregistrer(etat, Some(Commande::Fermer), None);
    let croix = bouton(None, Some(Icone::Symbole("xmark")), fermer, false, identifiant);
    etat.controles.push(("fermer".into(), croix));
    largeur_fixe(croix, 64.0);
    let mut contenu = vec![item_vue(&format!("{identifiant}.fermer"), croix)];
    contenu.extend(items);
    etat.barres.push(barre_touch(&format!("{identifiant}.barre"), contenu));
    let ouvrir = enregistrer(etat, Some(Commande::Ouvrir(etat.barres.len() - 1)), None);
    let titre = format!("{}  ›", libelle.unwrap_or(""));
    let b = bouton(Some(titre.trim_start()), nom.map(Icone::Symbole), ouvrir, false, identifiant);
    etat.controles.push((format!("ouvrir {}", nom.or(libelle).unwrap_or("")), b));
    largeur_fixe(b, largeur.unwrap_or(LARGEUR_GROUPE));
    item_vue(identifiant, b)
}

pub(super) fn slider(identifiant: &str, reglage: Reglage, nom: &str, etat: &mut Etat) -> Id {
    let (valeur, tag, petit, grand) = match reglage {
        Reglage::Luminosite => (actions::luminosite::lire(), 0isize, "sun.min.fill", "sun.max.fill"),
        Reglage::Volume => (actions::volume::lire(), 1, "speaker.fill", "speaker.wave.3.fill"),
    };
    let item = unsafe {
        let item: Id = msg_send![class!(NSSliderTouchBarItem), alloc];
        let item: Id = msg_send![item, initWithIdentifier: &*texte(&format!("{identifiant}.slider"))];
        let s: Id = msg_send![item, slider];
        let _: () = msg_send![s, setMinValue: 0.0f64];
        let _: () = msg_send![s, setMaxValue: 1.0f64];
        let _: () = msg_send![s, setDoubleValue: valeur.unwrap_or(0.5) as f64];
        let _: () = msg_send![s, setTag: tag];
        let _: () = msg_send![s, setContinuous: Bool::YES];
        if let (Some(petit), Some(grand)) = (symbole(petit), symbole(grand)) {
            let acc_min: Id = msg_send![class!(NSSliderAccessory), accessoryWithImage: petit];
            let acc_max: Id = msg_send![class!(NSSliderAccessory), accessoryWithImage: grand];
            let _: () = msg_send![item, setMinimumValueAccessory: acc_min];
            let _: () = msg_send![item, setMaximumValueAccessory: acc_max];
        }
        let _: () = msg_send![item, setMinimumSliderWidth: 600.0f64];
        let _: () = msg_send![item, setTarget: cible()];
        let _: () = msg_send![item, setAction: sel!(glisse:)];
        item
    };
    let nom_reglage = if tag == 0 { "luminosite" } else { "volume" };
    etat.controles.push((format!("slider {nom_reglage}"), item));
    depliable(identifiant, Some(nom), None, None, vec![item], etat)
}

pub(super) fn construire(elements: &[Element], prefixe: &str, etat: &mut Etat) -> Vec<Id> {
    let mut items = Vec::new();
    for (i, element) in elements.iter().enumerate() {
        let ident = format!("{prefixe}.{}", i + 1);
        let item = match element {
            Element::Bouton { texte: t, symbole: s, image: i, gabarit, couleur: c, couleur_texte, taille, affichage, largeur, action, appui_long, source } => {
                let index = enregistrer(etat, action.clone().map(Commande::Systeme), appui_long.clone().map(Commande::Systeme));
                // Une image posée passe avant le symbole (l'éditeur le dit à côté du choix d'image).
                let icone = i.as_deref().map(|chemin| Icone::Image(chemin, *gabarit)).or(s.as_deref().map(Icone::Symbole));
                let (titre, icone) = selon(*affichage, t.as_deref(), icone);
                let b = bouton(titre, icone, index, appui_long.is_some(), &ident);
                etat.controles.push((t.clone().or(s.clone()).or(i.clone()).unwrap_or_default(), b));
                if let Some(col) = c.as_deref().and_then(couleur) {
                    unsafe { let _: () = msg_send![b, setBezelColor: col]; }
                }
                styler(b, couleur_texte.as_deref(), *taille);
                match largeur {
                    Some(l) => largeur_fixe(b, *l),
                    None if t.is_some() => largeur_max(b, LARGEUR_MAX_TEXTE),
                    None => {}
                }
                if let Some(source) = source {
                    let gabarit = t.clone().unwrap_or_else(|| "{texte}".into());
                    let style = Some((couleur_texte.clone(), *taille));
                    rafraichi::lier(etat, b, source, Lien { source: None, gabarit, symbole_selon: None, couleur_selon: None, style, element: ident.clone() });
                }
                item_vue(&ident, b)
            }
            Element::Texte { source, gabarit, symbole_selon, couleur_selon, largeur, action } => {
                let b = bouton_texte(etat, action.clone());
                largeur_fixe(b, largeur_script(*largeur));
                etat.controles.push((format!("texte {}", source.as_deref().unwrap_or("")), b));
                let lien = Lien {
                    source: None,
                    gabarit: gabarit.clone().unwrap_or_else(|| "{texte}".into()),
                    symbole_selon: symbole_selon.clone(),
                    couleur_selon: couleur_selon.clone(),
                    style: None,
                    element: ident.clone(),
                };
                rafraichi::lier(etat, b, source.as_deref().unwrap_or(""), lien);
                item_vue(&ident, b)
            }
            Element::Slider { reglage, symbole: s } => {
                let defaut = if *reglage == Reglage::Luminosite { "sun.max.fill" } else { "speaker.wave.2.fill" };
                slider(&ident, *reglage, s.as_deref().unwrap_or(defaut), etat)
            }
            Element::Heure { format } => {
                let b = bouton_texte(etat, None);
                let formateur: Id = unsafe { msg_send![class!(NSDateFormatter), new] };
                unsafe { let _: () = msg_send![formateur, setDateFormat: &*texte(format.as_deref().unwrap_or("HH:mm"))]; }
                largeur_max(b, LARGEUR_MAX_TEXTE);
                etat.rafraichis.push(Rafraichi::Heure { bouton: b, formateur });
                item_vue(&ident, b)
            }
            Element::EnCours { largeur } => {
                let b = bouton_texte(etat, Some(Action::LecturePause));
                etat.controles.push(("en_cours".into(), b));
                match largeur {
                    Some(l) => largeur_fixe(b, *l),
                    None => largeur_max(b, LARGEUR_MAX_TEXTE),
                }
                etat.rafraichis.push(Rafraichi::EnCours { bouton: b });
                item_vue(&ident, b)
            }
            Element::Script { commande, intervalle, delai, largeur, action } => {
                let b = bouton_texte(etat, action.clone());
                largeur_fixe(b, largeur_script(*largeur));
                etat.controles.push((format!("script {commande}"), b));
                etat.rafraichis.push(Rafraichi::Script {
                    bouton: b,
                    commande: commande.clone(),
                    intervalle: Duration::from_secs_f64(intervalle.max(0.1)),
                    delai: Duration::from_secs_f64(delai.unwrap_or(DELAI_SCRIPT).max(0.1)),
                    prochain: Instant::now(),
                    en_cours: false,
                    derniere_erreur: String::new(),
                    derniere_sortie: String::new(),
                    element: ident.clone(),
                });
                item_vue(&ident, b)
            }
            Element::Batterie { symbole: s, largeur } => {
                let b = bouton_texte(etat, None);
                largeur_fixe(b, largeur.unwrap_or(LARGEUR_BATTERIE));
                etat.controles.push(("batterie".into(), b));
                etat.rafraichis.push(Rafraichi::Batterie { bouton: b, symbole: s.unwrap_or(true), prochain: Instant::now(), en_cours: false });
                item_vue(&ident, b)
            }
            Element::Espace { largeur } => {
                let vue: Id = unsafe { msg_send![class!(NSView), new] };
                largeur_fixe(vue, *largeur);
                let item = item_vue(&ident, vue);
                lacher(vue);
                item
            }
            Element::Lanceur { app, texte: t, affichage, largeur, appui_long } => {
                let index = enregistrer(etat, Some(Commande::Systeme(Action::App(app.clone()))), appui_long.clone().map(Commande::Systeme));
                let (titre, icone) = selon(*affichage, t.as_deref(), Some(Icone::App(app)));
                let b = bouton(titre, icone, index, appui_long.is_some(), &ident);
                etat.controles.push((t.clone().unwrap_or(app.clone()), b));
                if let Some(chemin) = icone.and_then(|_| actions::chemin_app(app)) {
                    etat.lanceurs.push((b, chemin));
                }
                match largeur {
                    Some(l) => largeur_fixe(b, *l),
                    None if titre.is_some() => largeur_max(b, LARGEUR_MAX_TEXTE),
                    None => {}
                }
                item_vue(&ident, b)
            }
            Element::Souple => std::ptr::null_mut(),
            Element::Groupe { texte: t, symbole: s, largeur, elements: enfants } => {
                let sous_items = construire(enfants, &ident, etat);
                depliable(&ident, s.as_deref(), t.as_deref(), *largeur, sous_items, etat)
            }
        };
        items.push(item);
    }
    items
}

/// Le titre et l'icône que montre un bouton ou un lanceur selon son `affichage` (sans texte, l'icône reste, et inversement).
fn selon<'a>(affichage: Option<Affichage>, texte: Option<&'a str>, icone: Option<Icone<'a>>) -> (Option<&'a str>, Option<Icone<'a>>) {
    match affichage {
        Some(Affichage::Texte) if texte.is_some() => (texte, None),
        Some(Affichage::Icone) if icone.is_some() => (None, icone),
        _ => (texte, icone),
    }
}

/// Met le point « ouverte » sous l'icône de chaque lanceur dont l'app tourne, et l'enlève sinon, à chaque lancement
/// ou fin d'une app (`appLanceeOuQuittee:`) ; `construire` dessine déjà l'état du moment.
pub(super) fn marquer_lanceurs(lanceurs: &[(Id, String)]) {
    for (b, chemin) in lanceurs {
        if let Some(i) = icone_app(chemin, actions::app_ouverte(chemin)) {
            let actuelle: Id = unsafe { msg_send![*b, image] };
            if actuelle != i {
                unsafe { let _: () = msg_send![*b, setImage: i]; }
            }
        }
    }
}

/// Largeur d'un widget script sans `largeur` : sa sortie change, et une sortie qui grandit
/// pousserait ses voisins hors de la barre (la Touch Bar retire ce qui dépasse).
pub(super) const LARGEUR_SCRIPT: f64 = 120.0;

/// Secondes avant qu'un script sans `delai` soit tué.
pub(super) const DELAI_SCRIPT: f64 = 5.0;

pub(super) fn largeur_script(largeur: Option<f64>) -> f64 {
    largeur.unwrap_or(LARGEUR_SCRIPT)
}

/// Largeur au-delà de laquelle un texte sans `largeur` (bouton, heure, titre en cours) est coupé avec « … ».
pub(super) const LARGEUR_MAX_TEXTE: f64 = 200.0;

/// Largeur du bouton qui ouvre un groupe ou un slider, sans `largeur`.
pub(super) const LARGEUR_GROUPE: f64 = 72.0;

/// Largeur de l'élément batterie sans `largeur` : « ⚡ 100 % » avec son symbole.
pub(super) const LARGEUR_BATTERIE: f64 = 96.0;

/// Largeur de la barre en points : toute la Touch Bar, ou ce qu'il reste à côté du Control Strip.
pub(crate) fn largeur_barre(masquer_control_strip: bool) -> f64 {
    if masquer_control_strip { 1004.0 } else { 685.0 }
}

/// Largeur approximative d'un élément (texte : ~9 pt par caractère plus les marges ; symbole seul : 72, mesuré).
/// Un bouton n'est jamais plus étroit qu'un bouton à symbole : 15 boutons « B1 »… en cachaient 3 sans message.
fn largeur_estimee(element: &Element) -> f64 {
    let texte = |t: &str| (t.chars().count() as f64 * 9.0 + 24.0).min(LARGEUR_MAX_TEXTE);
    match element {
        Element::Bouton { largeur: Some(l), .. } | Element::EnCours { largeur: Some(l) } | Element::Espace { largeur: l } | Element::Groupe { largeur: Some(l), .. } => *l,
        Element::Bouton { texte: t, symbole: s, image: i, .. } => t.as_deref().map_or(LARGEUR_GROUPE, |t| texte(t).max(LARGEUR_GROUPE)) + if t.is_some() && (s.is_some() || i.is_some()) { 24.0 } else { 0.0 },
        Element::Lanceur { largeur: Some(l), .. } => *l,
        Element::Lanceur { texte: t, .. } => t.as_deref().map_or(LARGEUR_GROUPE, |t| texte(t).max(LARGEUR_GROUPE) + 24.0),
        Element::Heure { format } => texte(format.as_deref().unwrap_or("HH:mm")),
        Element::EnCours { largeur: None } => LARGEUR_MAX_TEXTE,
        Element::Script { largeur, .. } | Element::Texte { largeur, .. } => largeur_script(*largeur),
        Element::Batterie { largeur, .. } => largeur.unwrap_or(LARGEUR_BATTERIE),
        Element::Slider { .. } | Element::Groupe { .. } => LARGEUR_GROUPE,
        Element::Souple => 0.0,
    }
}

/// La Touch Bar retire en silence ce qui dépasse : largeur totale estimée et nombre d'éléments
/// qui seront cachés (du premier qui dépasse `disponible` jusqu'au dernier), ou `None` si tout tient.
pub(crate) fn debordement(elements: &[Element], disponible: f64) -> Option<(f64, usize)> {
    const ESPACEMENT: f64 = 8.0;
    // L'estimation est grossière (la barre par défaut tient à 3 % près) : on ne prévient qu'au-delà de 5 %.
    const TOLERANCE: f64 = 1.05;
    let largeurs: Vec<f64> = elements.iter().map(|e| largeur_estimee(e) + ESPACEMENT).collect();
    let mut cumul = 0.0;
    let premier_cache = largeurs.iter().position(|l| {
        cumul += l;
        cumul > disponible * TOLERANCE
    })?;
    Some((largeurs.iter().sum(), elements.len() - premier_cache))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbole_inconnu_donne_un_bouton_texte() {
        super::super::preparer_cible();
        let b = bouton(None, Some(Icone::Symbole("symbole.qui.nexiste.pas")), 0, false, "fr.palph.tactile.3");
        let titre: Id = unsafe { msg_send![b, title] };
        let image: Id = unsafe { msg_send![b, image] };
        assert_eq!(unsafe { (*(titre as *const objc2_foundation::NSString)).to_string() }, "symbole.qui.nexiste.pas");
        assert!(image.is_null());
    }

    #[test]
    fn image_introuvable_donne_un_bouton_texte() {
        super::super::preparer_cible();
        let b = bouton(Some("Pr"), Some(Icone::Image("~/icones/premiere.png", false)), 0, false, "fr.palph.tactile.4");
        let titre: Id = unsafe { msg_send![b, title] };
        let image: Id = unsafe { msg_send![b, image] };
        assert_eq!(unsafe { (*(titre as *const objc2_foundation::NSString)).to_string() }, "Pr");
        assert!(image.is_null());
        let b = bouton(None, Some(Icone::Image("~/icones/premiere.png", false)), 0, false, "fr.palph.tactile.4");
        let titre: Id = unsafe { msg_send![b, title] };
        assert_eq!(unsafe { (*(titre as *const objc2_foundation::NSString)).to_string() }, "premiere.png");
        let b = bouton(Some("Pr"), Some(Icone::Image("/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/GenericApplicationIcon.icns", false)), 0, false, "");
        let image: Id = unsafe { msg_send![b, image] };
        assert!(!image.is_null());
    }

    /// Une image qui annonce 30000 × 30000 px (bouton ou image d'un script JSON, même chemin `Icone::Image`) : repli texte.
    #[test]
    fn image_geante_donne_un_bouton_texte() {
        super::super::preparer_cible();
        let chemin = std::env::temp_dir().join(format!("tactile-bouton-geante-{}.png", std::process::id()));
        std::fs::write(&chemin, super::super::png_annonce(30000, 30000)).unwrap();
        let avant = super::super::memoire_max();
        let b = bouton(None, Some(Icone::Image(&chemin.to_string_lossy(), false)), 0, false, "fr.palph.tactile.5");
        let titre: Id = unsafe { msg_send![b, title] };
        let image: Id = unsafe { msg_send![b, image] };
        assert_eq!(unsafe { (*(titre as *const objc2_foundation::NSString)).to_string() }, chemin.file_name().unwrap().to_string_lossy());
        assert!(image.is_null());
        assert!(super::super::memoire_max() - avant < 64 << 20);
        std::fs::remove_file(&chemin).unwrap();
    }

    #[test]
    fn debordement_compte_les_elements_caches() {
        let bouton = |texte: Option<&str>, largeur: Option<f64>| Element::Bouton {
            texte: texte.map(String::from),
            symbole: Some("bolt.fill".into()),
            image: None,
            gabarit: false,
            couleur: None,
            couleur_texte: None,
            taille: None,
            affichage: None,
            largeur,
            action: None,
            appui_long: None,
            source: None,
        };
        let defaut = crate::config::lire(include_str!("../../exemples/barre.toml")).unwrap();
        assert_eq!(debordement(&defaut.elements, largeur_barre(true)), None);
        // La barre par défaut avec en_cours à 280 et les deux éléments de Claude vocal tient sur la vraie barre (vu en capture).
        let mut avec_claude = defaut.elements.clone();
        avec_claude[5] = Element::EnCours { largeur: Some(280.0) };
        avec_claude.extend(crate::config::lire(include_str!("../../exemples/claude-vocal.toml")).unwrap().elements);
        assert_eq!(debordement(&avec_claude, largeur_barre(true)), None);
        assert_eq!(debordement(&[bouton(None, Some(100_000.0)), bouton(None, None)], 1004.0).map(|d| d.1), Some(2));
        let vingt_cinq: Vec<Element> = (0..25).map(|_| bouton(None, None)).collect();
        let (total, caches) = debordement(&vingt_cinq, 1004.0).unwrap();
        assert_eq!(total, 25.0 * 80.0);
        assert_eq!(caches, 25 - 13); // la vraie barre en cache 13 : l'estimation est à un élément près
        // 15 boutons à texte court : la vraie barre en cache 3 ; l'estimation en annonce 2 (avant : aucun message).
        let courts: Vec<Element> = (1..=15).map(|n| match bouton(Some(&format!("B{n}")), None) {
            Element::Bouton { symbole: _, texte, image, gabarit, couleur, couleur_texte, taille, affichage, largeur, action, appui_long, source } =>
                Element::Bouton { symbole: None, texte, image, gabarit, couleur, couleur_texte, taille, affichage, largeur, action, appui_long, source },
            autre => autre,
        }).collect();
        assert_eq!(debordement(&courts, 1004.0).map(|d| d.1), Some(2));
        // Un texte long sans largeur est plafonné, il ne cache pas tout seul ses voisins.
        let long = "x".repeat(200);
        assert_eq!(debordement(&[bouton(Some(&long), None), bouton(None, None)], 1004.0), None);
    }

    #[test]
    fn un_script_a_toujours_une_largeur_fixe() {
        assert_eq!(largeur_script(None), LARGEUR_SCRIPT);
        assert_eq!(largeur_script(Some(80.0)), 80.0);
    }
}
