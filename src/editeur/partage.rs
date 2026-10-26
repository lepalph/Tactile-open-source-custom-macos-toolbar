//! Exporter et importer une barre : un `.toml` seul, ou un `.zip` avec `barre.toml` et `icones/` quand elle a des logos.
//!
//! L'import lit un fichier venu d'ailleurs : rien n'y est cru sur parole. L'archive est lue en mémoire, jamais
//! extraite sous ses propres noms ; une entrée avec `..`, un chemin absolu ou un lien symbolique fait refuser
//! toute l'archive ; seuls `barre.toml` et `icones/<image>` sont pris, dans des limites de taille ; les logos sont
//! redessinés en PNG (`icones::importer_en_png`) ; et tout ce que la barre peut lancer est listé avant la confirmation.

use super::icones;
use crate::actions::Action;
use crate::config::{self, Barre, Element};
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

/// Quelque chose que la barre lance (programme, script, frappe, app, adresse non web), montré avant d'importer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(super) struct Commande {
    /// Où : « Barre principale, élément 3 « Build » ».
    pub ou: Texte,
    /// Quoi : « Commande shell », « Appui long : AppleScript »…
    pub quoi: Texte,
    /// Ce qui sera exécuté, tel qu'écrit dans le fichier.
    pub valeur: String,
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
    for app in &mut barre.applications {
        dans(&mut app.elements, f);
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

/// Tout ce que la barre peut lancer : actions `shell`, `applescript`, `touche`, `keycode`, `app`, `url` autre que
/// http(s), actions système (app d'Apple ou frappe simulée), les apps des `lanceur` et les commandes des `script`. Dans l'ordre de la barre, principale d'abord.
pub(super) fn commandes(barre: &Barre) -> Vec<Commande> {
    let mut liste = Vec::new();
    let principale = tx("Barre principale", "Main bar");
    parcourir(&barre.elements, &principale, "", &mut liste);
    for app in &barre.applications {
        let ou = tx(format!("Barre de « {} »", app.app), format!("“{}” bar", app.app));
        parcourir(&app.elements, &ou, "", &mut liste);
    }
    liste
}

fn parcourir(elements: &[Element], barre: &Texte, parent: &str, liste: &mut Vec<Commande>) {
    for (i, e) in elements.iter().enumerate() {
        let numero = format!("{parent}{}", i + 1);
        let (nom_fr, nom_en) = match e {
            Element::Bouton { texte: Some(t), .. } | Element::Groupe { texte: Some(t), .. } if !t.is_empty() => (format!(" « {t} »"), format!(" “{t}”")),
            _ => (String::new(), String::new()),
        };
        let ou = tx(format!("{}, élément {numero}{nom_fr}", barre.fr), format!("{}, item {numero}{nom_en}", barre.en));
        match e {
            Element::Bouton { action, appui_long, .. } => {
                if let Some(c) = action.as_ref().and_then(lancement) {
                    liste.push(Commande { ou: ou.clone(), ..c });
                }
                if let Some(c) = appui_long.as_ref().and_then(lancement) {
                    let quoi = tx(format!("{} (appui long)", c.quoi.fr), format!("{} (long press)", c.quoi.en));
                    liste.push(Commande { ou, quoi, ..c });
                }
            }
            Element::Lanceur { app, appui_long, .. } => {
                liste.push(Commande { ou: ou.clone(), quoi: tx("Ouvre l’app", "Opens the app"), valeur: app.clone() });
                if let Some(c) = appui_long.as_ref().and_then(lancement) {
                    let quoi = tx(format!("{} (appui long)", c.quoi.fr), format!("{} (long press)", c.quoi.en));
                    liste.push(Commande { ou, quoi, ..c });
                }
            }
            Element::Script { commande, intervalle, action, .. } => {
                let quoi = tx(format!("Commande relancée toutes les {intervalle} s"), format!("Command run every {intervalle} s"));
                liste.push(Commande { ou: ou.clone(), quoi, valeur: commande.clone() });
                if let Some(c) = action.as_ref().and_then(lancement) {
                    liste.push(Commande { ou, ..c });
                }
            }
            Element::Groupe { elements, .. } => parcourir(elements, barre, &format!("{numero} › "), liste),
            _ => {}
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
        Action::Url(u) if !["http://", "https://"].iter().any(|p| u.to_lowercase().starts_with(p)) => (tx("Ouvre l’adresse", "Opens the address"), u.clone()),
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
        _ => return None,
    };
    Some(Commande { ou: tx("", ""), quoi, valeur })
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
                if toml.is_some() {
                    return Err(tx("archive refusée : deux barre.toml", "archive refused: two barre.toml files"));
                }
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
    let mut barre = config::lire(texte).map_err(fr_seul).map_err(nettoyer)?;
    let mut sources = Vec::new();
    let mut absentes = Vec::new();
    chaque_image(&mut barre, &mut |i| {
        if let Some((_, copie)) = copies.iter().find(|(c, _)| c == i) {
            *i = copie.to_string_lossy().into_owned();
            sources.push(i.clone());
        } else if nom_d_icone(i).is_some() && !absentes.contains(i) {
            absentes.push(i.clone());
        }
    });
    let mut paquet = Paquet { barre, commandes: vec![], avis: vec![], sources, temporaire };
    // Le validateur partagé avec `--verifier` : une barre inutilisable est refusée, le reste devient un avis.
    for avis in config::controler(&paquet.barre, Some(texte)) {
        if avis.grave {
            return Err(fr_seul(avis.to_string()));
        }
        paquet.avis.push(fr_seul(avis.to_string()));
    }
    super::verifier(&paquet.barre.elements).map_err(fr_seul)?;
    for app in &paquet.barre.applications {
        super::verifier(&app.elements).map_err(|e| fr_seul(format!("application « {} », {e}", app.app)))?;
    }
    if !absentes.is_empty() {
        let liste = absentes.join(", ");
        paquet.avis.push(tx(format!("logo absent du fichier importé : {liste}"), format!("logo missing from the imported file: {liste}")));
    }
    paquet.commandes = commandes(&paquet.barre);
    Ok(paquet)
}

impl Paquet {
    /// Nombre de logos que l'installation copiera.
    pub fn logos(&self) -> usize {
        self.sources.len()
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
                eprintln!("{} {}", if fr { "attention :" } else { "warning:" }, a.dans(fr));
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
    if paquet.commandes.is_empty() {
        lignes.push(if fr { "Elle ne lance aucune commande." } else { "It runs no commands." }.into());
    } else {
        lignes.push(if fr { "Ce qu'elle peut lancer sur ton Mac :" } else { "What it can run on your Mac:" }.into());
        for c in &paquet.commandes {
            lignes.push(format!("  {} · {}{} {}", c.ou.dans(fr), c.quoi.dans(fr), if fr { " :" } else { ":" }, c.valeur));
        }
    }
    for a in &paquet.avis {
        lignes.push(format!("{} {}", if fr { "attention :" } else { "warning:" }, a.dans(fr)));
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
    #[cfg_attr(ci, ignore = "rendu AppKit dans un contexte bitmap : pas de session graphique sur un runner de CI")]
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
}
