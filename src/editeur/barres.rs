//! Ce que l'éditeur fait d'une barre entière : fusion d'une barre importée, barre toute faite qui garde les sources
//! encore citées, avertissements lisibles après l'enregistrement, élément météo prêt à poser.

use super::partage::Texte;
use crate::config::{self, Barre, Element, Source};
use serde_json::{json, Value};

/// Le nom de `[[source]]` cité par chaque élément (groupes compris), modifiable sur place.
fn chaque_source(elements: &mut [Element], f: &mut impl FnMut(&mut String)) {
    for e in elements {
        match e {
            Element::Bouton { source: Some(s), .. } | Element::Texte { source: Some(s), .. } => f(s),
            Element::Groupe { elements, .. } => chaque_source(elements, f),
            _ => {}
        }
    }
}

/// Un nom libre parmi `pris` : `meteo`, sinon `meteo 2`, `meteo 3`…
pub(super) fn nom_libre(nom: &str, pris: &[&str]) -> String {
    if !pris.contains(&nom) {
        return nom.to_string();
    }
    (2..).map(|n| format!("{nom} {n}")).find(|c| !pris.contains(&c.as_str())).expect("suite infinie")
}

/// « Ajouter à ma barre » : les éléments reçus vont à la fin de la principale, les barres d'application reçues
/// s'ajoutent à celle de la même app (ou deviennent une barre de plus), les sources s'ajoutent. Une source reçue
/// dont le nom est déjà pris est renommée (`meteo 2`) et les éléments reçus suivent ; identique, elle n'est pas doublée.
/// Le reste de ma barre (Control Strip compris) ne change pas.
pub(super) fn fusionner(mut mienne: Barre, mut recue: Barre) -> Barre {
    let mut renommees: Vec<(String, String)> = Vec::new();
    for source in recue.sources {
        match mienne.sources.iter().find(|s| s.nom == source.nom) {
            Some(deja) if *deja == source => {}
            Some(_) => {
                let pris: Vec<&str> = mienne.sources.iter().map(|s| s.nom.as_str()).collect();
                let nom = nom_libre(&source.nom, &pris);
                renommees.push((source.nom.clone(), nom.clone()));
                mienne.sources.push(Source { nom, ..source });
            }
            None => mienne.sources.push(source),
        }
    }
    let mut renommer = |s: &mut String| {
        if let Some((_, nouveau)) = renommees.iter().find(|(ancien, _)| ancien == s) {
            *s = nouveau.clone();
        }
    };
    chaque_source(&mut recue.elements, &mut renommer);
    mienne.elements.extend(recue.elements);
    for mut app in recue.applications {
        chaque_source(&mut app.elements, &mut renommer);
        match mienne.applications.iter_mut().find(|a| a.app.to_lowercase() == app.app.to_lowercase()) {
            Some(deja) => deja.elements.extend(app.elements),
            None => mienne.applications.push(app),
        }
    }
    mienne
}

/// Barre toute faite : elle remplace la principale ; les barres d'application de l'utilisateur restent, avec les
/// `[[source]]` qu'elles citent (sinon elles afficheraient « ⚠ source »). Les sources de la barre toute faite passent
/// devant ; une source gardée au même nom qu'elle n'est pas doublée.
pub(super) fn prereglage(actuelle: Option<Barre>, toute_faite: Barre) -> Barre {
    let Some(mut actuelle) = actuelle else { return toute_faite };
    let mut citees = Vec::new();
    for app in &mut actuelle.applications {
        chaque_source(&mut app.elements, &mut |s| citees.push(s.clone()));
    }
    let mut sources = toute_faite.sources;
    for s in actuelle.sources {
        if citees.contains(&s.nom) && !sources.iter().any(|t| t.nom == s.nom) {
            sources.push(s);
        }
    }
    Barre { applications: actuelle.applications, sources, ..toute_faite }
}

/// Un avis de `config::controler`, dit pour quelqu'un qui n'a jamais vu le fichier : ni numéro de ligne, ni
/// « bundle id ». Les deux cas que l'éditeur rencontre le plus (app absente, débordement) ont leur phrase dans les
/// deux langues ; les autres gardent le texte de `controler`, en français seulement (comme ses erreurs).
// shortcut: reconnaît les messages de `controler` à leur texte, verrouillé par le test `avis_lisibles` ; à remplacer
// par un genre d'avis dans `config::Avis` si d'autres cas doivent être réécrits.
pub(super) fn avis_humain(avis: &str) -> Texte {
    if let Some(debut) = avis.find("app « ")
        && let Some(fin) = avis[debut..].find(" » introuvable")
    {
        let app = &avis[debut + "app « ".len()..debut + fin];
        return Texte { fr: format!("l’app {app} n’est pas installée sur ce Mac"), en: format!("{app} isn’t installed on this Mac") };
    }
    if let Some(i) = avis.find(" déborde (") {
        let barre = &avis[..i];
        let caches: usize = avis[i..].split(") : ").nth(1).and_then(|s| s.split(' ').next()).and_then(|n| n.parse().ok()).unwrap_or(1);
        let (fr, en) = match caches {
            1 => ("1 élément ne tient pas : la Touch Bar le cachera".to_string(), "1 item doesn’t fit: the Touch Bar will hide it".to_string()),
            n => (format!("{n} éléments ne tiennent pas : la Touch Bar les cachera"), format!("{n} items don’t fit: the Touch Bar will hide them")),
        };
        return match barre.strip_prefix("la barre de « ").and_then(|a| a.strip_suffix(" »")) {
            Some(app) => Texte { fr: format!("barre de {app} : {fr}"), en: format!("{app} bar: {en}") },
            None => Texte { fr, en },
        };
    }
    let sans_ligne = match avis.strip_prefix("ligne ").and_then(|s| s.split_once(" : ")) {
        Some((n, suite)) if n.chars().all(|c| c.is_ascii_digit()) => suite,
        _ => avis,
    };
    Texte { fr: sans_ligne.to_string(), en: sans_ligne.to_string() }
}

/// Ce qui marchera mal dans une barre que l'éditeur vient d'écrire (rien de grave : le grave est refusé avant).
pub(super) fn avertissements(barre: &Barre) -> Vec<Texte> {
    let mut liste: Vec<Texte> = Vec::new();
    for a in config::controler(barre, None).into_iter().filter(|a| !a.grave) {
        let t = avis_humain(&a.to_string());
        if !liste.contains(&t) {
            liste.push(t);
        }
    }
    liste
}

/// La barre exécute-t-elle du code (commande shell, AppleScript, `script`, source `commande`, serveur MCP lancé) ?
/// Seulement alors l'import prévient qu'une commande peut tout faire, y compris effacer des fichiers.
pub(super) fn execute_du_code(barre: &Barre) -> bool {
    use crate::actions::Action;
    let code = |a: &Option<Action>| matches!(a, Some(Action::Shell(_) | Action::Applescript(_)));
    fn dans(elements: &[Element], code: &impl Fn(&Option<Action>) -> bool) -> bool {
        elements.iter().any(|e| match e {
            Element::Script { .. } => true,
            Element::Bouton { action, appui_long, .. } => code(action) || code(appui_long),
            Element::Lanceur { appui_long, .. } => code(appui_long),
            Element::Texte { action, .. } => code(action),
            Element::Groupe { elements, .. } => dans(elements, code),
            _ => false,
        })
    }
    dans(&barre.elements, &code)
        || barre.applications.iter().any(|a| dans(&a.elements, &code))
        || barre.sources.iter().any(|s| s.commande.is_some() || s.mcp.as_ref().is_some_and(|m| m.commande.is_some()))
}

/// `geocoding-api.open-meteo.com` : l'adresse de recherche d'une ville (nom encodé pour l'URL).
pub(super) fn adresse_geocodage(ville: &str, langue: &str) -> String {
    let nom: String = ville
        .trim()
        .bytes()
        .map(|o| if o.is_ascii_alphanumeric() || b"-_.~".contains(&o) { (o as char).to_string() } else { format!("%{o:02X}") })
        .collect();
    let langue = if langue == "en" { "en" } else { "fr" };
    format!("https://geocoding-api.open-meteo.com/v1/search?name={nom}&count=5&language={langue}&format=json")
}

/// Réponse du géocodage → `[{ nom, region, pays, latitude, longitude }]` (5 au plus, coordonnées valides seulement).
pub(super) fn villes(reponse: &Value) -> Vec<Value> {
    let Some(liste) = reponse["results"].as_array() else { return vec![] };
    liste
        .iter()
        .filter_map(|v| {
            let (lat, lon) = (v["latitude"].as_f64()?, v["longitude"].as_f64()?);
            ((-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon)).then(|| {
                json!({ "nom": v["name"].as_str().unwrap_or(""), "region": v["admin1"].as_str().unwrap_or(""), "pays": v["country"].as_str().unwrap_or(""), "latitude": lat, "longitude": lon })
            })
        })
        .take(5)
        .collect()
}

/// La météo d'une ville, d'après `exemples/meteo.toml` (symboles WMO, couleur selon la température) : la `[[source]]`
/// Open-Meteo à ces coordonnées et l'élément `texte` qui l'affiche. `nom` : nom de la source, déjà libre.
pub(super) fn meteo(nom: &str, latitude: f64, longitude: f64) -> Result<(Source, Element), String> {
    if !(-90.0..=90.0).contains(&latitude) || !(-180.0..=180.0).contains(&longitude) {
        return Err(format!("coordonnées invalides : {latitude}, {longitude}"));
    }
    let exemple = config::lire(include_str!("../../exemples/meteo.toml"))?;
    let mut source = exemple.sources.into_iter().next().ok_or("exemple sans source")?;
    let element = exemple.elements.into_iter().find(|e| matches!(e, Element::Texte { .. })).ok_or("exemple sans texte")?;
    source.nom = nom.to_string();
    source.http = Some(format!("https://api.open-meteo.com/v1/forecast?latitude={latitude:.2}&longitude={longitude:.2}&current=temperature_2m,weather_code"));
    let Element::Texte { source: _, gabarit, largeur, symbole_selon, couleur_selon, action } = element else { unreachable!() };
    Ok((source, Element::Texte { source: Some(nom.to_string()), gabarit, largeur, symbole_selon, couleur_selon, action }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn barre(toml: &str) -> Barre {
        config::lire(toml).unwrap()
    }

    #[test]
    fn fusion_des_elements_sources_et_applications() {
        let mienne = barre(
            "[[element]]\ntype = \"heure\"\n[[element]]\ntype = \"texte\"\nsource = \"meteo\"\n\
             [[source]]\nnom = \"meteo\"\nhttp = \"https://a.example/\"\n\
             [[application]]\napp = \"Safari\"\n[[application.element]]\ntype = \"souple\"\n",
        );
        let recue = barre(
            "masquer_control_strip = false\n[[element]]\ntype = \"texte\"\nsource = \"meteo\"\n\
             [[element]]\ntype = \"groupe\"\n[[element.element]]\ntype = \"bouton\"\ntexte = \"{x}\"\nsource = \"disque\"\n\
             [[source]]\nnom = \"meteo\"\nhttp = \"https://b.example/\"\n[[source]]\nnom = \"disque\"\ncommande = \"df\"\n\
             [[application]]\napp = \"safari\"\n[[application.element]]\ntype = \"texte\"\nsource = \"meteo\"\n\
             [[application]]\napp = \"Figma\"\n[[application.element]]\ntype = \"heure\"\n",
        );
        let f = fusionner(mienne.clone(), recue);
        assert!(f.masquer_control_strip, "le réglage de ma barre reste");
        assert_eq!(f.elements.len(), 4);
        assert_eq!(f.sources.iter().map(|s| s.nom.as_str()).collect::<Vec<_>>(), ["meteo", "meteo 2", "disque"]);
        // Mon texte cite toujours ma source ; le texte reçu suit la source renommée.
        assert!(matches!(&f.elements[1], Element::Texte { source: Some(s), .. } if s == "meteo"));
        assert!(matches!(&f.elements[2], Element::Texte { source: Some(s), .. } if s == "meteo 2"));
        assert!(matches!(&f.elements[3], Element::Groupe { elements, .. } if matches!(&elements[0], Element::Bouton { source: Some(s), .. } if s == "disque")));
        assert_eq!(f.applications.len(), 2, "Safari fusionnée sans casse, Figma ajoutée");
        assert!(matches!(&f.applications[0].elements[1], Element::Texte { source: Some(s), .. } if s == "meteo 2"));
        assert!(config::controler(&f, None).iter().all(|a| !a.message.contains("aucune [[source]]")));
        // Une source identique n'est pas doublée.
        let deux = fusionner(mienne.clone(), mienne.clone());
        assert_eq!(deux.sources.len(), 1);
        assert_eq!(deux.elements.len(), 4);
    }

    /// Lise (0.3) : « Simple » gardait la barre Resolve mais effaçait la source `disque` qu'elle cite.
    #[test]
    fn barre_toute_faite_garde_les_sources_des_applications() {
        let actuelle = barre(
            "[[element]]\ntype = \"texte\"\nsource = \"meteo\"\n\
             [[source]]\nnom = \"meteo\"\nhttp = \"https://a.example/\"\n[[source]]\nnom = \"disque\"\ncommande = \"df\"\n\
             [[application]]\napp = \"DaVinci Resolve\"\n[[application.element]]\ntype = \"groupe\"\n\
             [[application.element.element]]\ntype = \"texte\"\nsource = \"disque\"\n",
        );
        let simple = barre(include_str!("../../exemples/simple.toml"));
        let b = prereglage(Some(actuelle), simple.clone());
        assert_eq!(b.elements, simple.elements);
        assert_eq!(b.applications.len(), 1);
        assert_eq!(b.sources.iter().map(|s| s.nom.as_str()).collect::<Vec<_>>(), ["disque"], "meteo n'est plus citée");
        assert!(config::controler(&b, None).iter().all(|a| !a.message.contains("aucune [[source]]")), "{:?}", config::controler(&b, None));
        assert_eq!(prereglage(None, simple.clone()), simple);
    }

    /// Les phrases réécrites dépendent du texte de `controler` : ce test casse si ce texte change.
    #[test]
    fn avis_lisibles() {
        let b = barre(
            "[[element]]\ntype = \"lanceur\"\napp = \"AppQuiNexistePasDuTout\"\n\
             [[element]]\ntype = \"groupe\"\n[[element.element]]\ntype = \"lanceur\"\napp = \"AppQuiNexistePasDuTout\"\n\
             [[element]]\ntype = \"en_cours\"\nlargeur = 600\n[[element]]\ntype = \"en_cours\"\nlargeur = 600\n",
        );
        let avis = avertissements(&b);
        assert_eq!(
            avis,
            [
                Texte { fr: "l’app AppQuiNexistePasDuTout n’est pas installée sur ce Mac".into(), en: "AppQuiNexistePasDuTout isn’t installed on this Mac".into() },
                Texte { fr: "1 élément ne tient pas : la Touch Bar le cachera".into(), en: "1 item doesn’t fit: the Touch Bar will hide it".into() },
            ],
            "un avis par app absente, même citée deux fois"
        );
        // Import : le numéro de ligne d'un fichier jamais vu disparaît, « bundle id » aussi.
        let import = config::controler(&b, Some(&config::ecrire(&b).unwrap()));
        let premier = import[0].to_string();
        assert!(premier.starts_with("ligne ") && premier.contains("bundle id"), "{premier}");
        assert_eq!(avis_humain(&premier).fr, "l’app AppQuiNexistePasDuTout n’est pas installée sur ce Mac");
        assert_eq!(avis_humain("ligne 12 : toutes_les est ramené à 5 secondes, le minimum").fr, "toutes_les est ramené à 5 secondes, le minimum");
        assert_eq!(avis_humain("la barre de « Safari » déborde (environ 1159 pt pour 1004) : 2 éléments sur 13 seront cachés ; réduis").en, "Safari bar: 2 items don’t fit: the Touch Bar will hide them");
    }

    #[test]
    fn code_execute_seulement_si_commande() {
        assert!(!execute_du_code(&barre("[[element]]\ntype = \"lanceur\"\napp = \"Figma\"\n[[element]]\ntype = \"bouton\"\naction = \"capture_zone\"\n")));
        assert!(execute_du_code(&barre("[[element]]\ntype = \"groupe\"\n[[element.element]]\ntype = \"bouton\"\nappui_long = { shell = \"rm -rf ~\" }\n")));
        assert!(execute_du_code(&barre("[[element]]\ntype = \"heure\"\n[[source]]\nnom = \"d\"\ncommande = \"df\"\n")));
        assert!(execute_du_code(&barre("[[element]]\ntype = \"heure\"\n[[application]]\napp = \"Safari\"\n[[application.element]]\ntype = \"script\"\ncommande = \"date\"\nintervalle = 60\n")));
        assert!(!execute_du_code(&barre("[[element]]\ntype = \"heure\"\n[[source]]\nnom = \"m\"\nhttp = \"https://a.example/\"\n")));
    }

    #[test]
    fn meteo_d_une_ville() {
        assert_eq!(adresse_geocodage(" Saint-Étienne ", "fr"), "https://geocoding-api.open-meteo.com/v1/search?name=Saint-%C3%89tienne&count=5&language=fr&format=json");
        assert!(adresse_geocodage("a&count=500#", "de").contains("name=a%26count%3D500%23&count=5&language=fr"));
        let reponse = json!({ "results": [
            { "name": "Lyon", "admin1": "Auvergne-Rhône-Alpes", "country": "France", "latitude": 45.74846, "longitude": 4.84671 },
            { "name": "Faux", "latitude": 400.0, "longitude": 0.0 },
            { "name": "Sans coordonnées" } ] });
        let v = villes(&reponse);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0]["region"], "Auvergne-Rhône-Alpes");
        assert!(villes(&json!({ "generationtime_ms": 0.1 })).is_empty(), "aucun résultat : pas de clé results");

        let (source, element) = meteo("Météo Lyon", 45.74846, 4.84671).unwrap();
        assert_eq!(source.http.as_deref(), Some("https://api.open-meteo.com/v1/forecast?latitude=45.75&longitude=4.85&current=temperature_2m,weather_code"));
        assert_eq!(source.toutes_les, Some(600.0));
        let Element::Texte { source: Some(s), symbole_selon: Some(sym), couleur_selon: Some(_), .. } = &element else { panic!("{element:?}") };
        assert_eq!(s, "Météo Lyon");
        assert_eq!(sym.valeurs.get("0").map(String::as_str), Some("sun.max.fill"));
        let b = Barre { masquer_control_strip: true, elements: vec![element], applications: vec![], sources: vec![source] };
        assert!(config::controler(&b, None).is_empty(), "{:?}", config::controler(&b, None));
        assert!(meteo("m", 91.0, 0.0).is_err());
        assert!(meteo("m", f64::NAN, 0.0).is_err());
        assert_eq!(nom_libre("meteo", &["meteo", "meteo 2"]), "meteo 3");
    }

    /// Une barre météo faite par l'éditeur puis importée ailleurs : l'aperçu d'import dit l'adresse que lit sa source.
    #[test]
    fn barre_meteo_importee_liste_sa_source_http() {
        let (source, element) = meteo("Météo Lyon", 45.74846, 4.84671).unwrap();
        let b = Barre { masquer_control_strip: true, elements: vec![element], applications: vec![], sources: vec![source] };
        let paquet = super::super::partage::ouvrir_octets(config::ecrire(&b).unwrap().into_bytes(), None).unwrap();
        let http = paquet.commandes.iter().find(|c| c.valeur.starts_with("https://api.open-meteo.com/")).expect("source http listée");
        assert_eq!(http.ou.fr, "Source « Météo Lyon »");
        assert_eq!(http.quoi.fr, "Adresse lue toutes les 600 s");
        assert!(!http.alerte);
        assert!(!execute_du_code(&paquet.barre));
    }
}
