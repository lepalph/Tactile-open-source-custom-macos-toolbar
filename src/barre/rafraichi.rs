//! Éléments qui se mettent à jour seuls : heure, titre en cours, scripts, batterie.
//! Le travail lent (commandes, media-control, pmset) tourne dans un fil à part ; le résultat revient par un canal.

use objc2::{class, msg_send};
use objc2_foundation::NSString;
use serde::Deserialize;
use std::os::unix::process::CommandExt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::elements::Icone;
use super::vues::{couleur, styler, titre};
use super::{batterie, Id, ETAT, PREFIXE};
use crate::actions::media;
use crate::config::{CouleurSelon, SymboleSelon};

pub(super) enum Rafraichi {
    Heure { bouton: Id, formateur: Id },
    EnCours { bouton: Id },
    Script { bouton: Id, commande: String, intervalle: Duration, delai: Duration, prochain: Instant, en_cours: bool, derniere_erreur: String, derniere_sortie: String, element: String },
    Batterie { bouton: Id, symbole: bool, prochain: Instant, en_cours: bool },
    /// Élément `texte`, ou bouton avec `source` (`lien.style` : sa couleur de texte et sa taille, à remettre après le titre).
    Source { bouton: Id, lien: Lien, derniere_erreur: String },
}

/// Ce qu'un élément affiche d'une `[[source]]`.
pub(super) struct Lien {
    /// Index dans `Barre::sources` ; `None` : aucune source de ce nom.
    pub source: Option<usize>,
    pub gabarit: String,
    pub symbole_selon: Option<SymboleSelon>,
    pub couleur_selon: Option<CouleurSelon>,
    pub style: Option<(Option<String>, Option<f64>)>,
    pub element: String,
}

/// Inscrit un élément lié à `source` (par son nom) ; il affiche « … » jusqu'à la première valeur.
pub(super) fn lier(etat: &mut super::Etat, bouton: Id, source: &str, mut lien: Lien) {
    lien.source = etat.config.sources.iter().position(|s| s.nom == source);
    titre(bouton, if lien.source.is_some() { "…" } else { SOURCE_EN_ERREUR });
    restyler(bouton, &lien);
    etat.rafraichis.push(Rafraichi::Source { bouton, lien, derniere_erreur: String::new() });
}

/// Ce qu'affiche un élément dont la source manque ou a échoué (l'erreur est au journal).
const SOURCE_EN_ERREUR: &str = "⚠ source";

fn restyler(bouton: Id, lien: &Lien) {
    if let Some((couleur_texte, taille)) = &lien.style {
        styler(bouton, couleur_texte.as_deref(), *taille);
    }
}

/// Une valeur (ou une erreur) de la source arrive : gabarit, symbole et couleur ; un champ absent est journalisé une fois.
fn afficher_source(bouton: Id, lien: &Lien, valeur: &Result<serde_json::Value, String>, derniere_erreur: &mut String) {
    let erreur = match valeur {
        Ok(v) => {
            let rendu = crate::sources::rendre(&lien.gabarit, lien.symbole_selon.as_ref(), lien.couleur_selon.as_ref(), v);
            if lien.style.is_some() {
                titre(bouton, &rendu.texte);
                restyler(bouton, lien);
            } else {
                appliquer(bouton, &Riche { texte: rendu.texte, symbole: rendu.symbole, couleur: rendu.couleur, ..Riche::default() }, &lien.element);
            }
            match rendu.manquants.as_slice() {
                [] => String::new(),
                m => format!("champ absent de la source : {}", m.join(", ")),
            }
        }
        Err(_) if lien.style.is_some() => {
            titre(bouton, SOURCE_EN_ERREUR);
            restyler(bouton, lien);
            String::new()
        }
        Err(_) => {
            appliquer(bouton, &Riche { texte: SOURCE_EN_ERREUR.into(), ..Riche::default() }, &lien.element);
            String::new()
        }
    };
    if erreur != *derniere_erreur {
        if !erreur.is_empty() {
            let element = lien.element.strip_prefix(PREFIXE).map_or(lien.element.as_str(), |e| e.trim_start_matches('.'));
            eprintln!("tactile : élément {element} : {erreur}");
        }
        *derniere_erreur = erreur;
    }
}

/// Sortie d'un script qui commence par `{` : il règle aussi son icône et ses couleurs. Tout est facultatif.
#[derive(Debug, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Riche {
    #[serde(default, alias = "text")]
    pub texte: String,
    #[serde(alias = "symbol")]
    pub symbole: Option<String>,
    pub image: Option<String>,
    #[serde(alias = "color")]
    pub couleur: Option<String>,
    #[serde(alias = "text_color")]
    pub couleur_texte: Option<String>,
}

/// JSON si la sortie commence par `{`, sinon le texte brut. Un JSON faux est affiché tel quel et signalé.
pub(super) fn analyser_sortie(brut: &str) -> Result<Riche, String> {
    let texte = brut.trim_end();
    if !texte.starts_with('{') {
        return Ok(Riche { texte: texte.to_string(), ..Riche::default() });
    }
    serde_json::from_str(texte).map_err(|e| format!("JSON illisible ({e})"))
}

/// Applique texte, icône et couleurs à un bouton de script. Les champs absents reviennent à l'ordinaire.
fn appliquer(bouton: Id, riche: &Riche, element: &str) {
    titre(bouton, &riche.texte);
    let icone = riche.symbole.as_deref().map(Icone::Symbole).or(riche.image.as_deref().map(|c| Icone::Image(c, false)));
    let image = icone.and_then(|i| match i.charger() {
        Some(image) => Some(image),
        None => {
            eprintln!("tactile : script : {} « {} » (élément {})", i.repli().1, i.nom(), element.strip_prefix(PREFIXE).map_or(element, |e| e.trim_start_matches('.')));
            None
        }
    });
    unsafe {
        let _: () = msg_send![bouton, setImage: image.unwrap_or(std::ptr::null_mut())];
        // 2 = image à gauche du texte, 1 = image seule, 0 = texte seul.
        let position: isize = match (image.is_some(), riche.texte.is_empty()) { (true, false) => 2, (true, true) => 1, (false, _) => 0 };
        let _: () = msg_send![bouton, setImagePosition: position];
        // Un script est sans bordure ; une couleur de fond la fait apparaître.
        let fond = riche.couleur.as_deref().and_then(couleur);
        let _: () = msg_send![bouton, setBordered: objc2::runtime::Bool::new(fond.is_some())];
        let _: () = msg_send![bouton, setBezelColor: fond.unwrap_or(std::ptr::null_mut())];
    }
    styler(bouton, riche.couleur_texte.as_deref(), None);
}

/// Ce qu'un fil renvoie : la sortie à afficher, et ce qui a cloché (vide si rien).
pub(crate) struct Sortie {
    pub texte: String,
    pub erreur: String,
}

/// Groupes de processus des scripts en cours. `None` : la barre a été remplacée, tout nouveau
/// processus est tué aussitôt ; sinon un orphelin par rechargement (vu avec `sleep 1000`).
/// Les sources (`curl`, commandes, serveurs MCP) s'y inscrivent aussi.
pub(crate) type Processus = Arc<Mutex<Option<Vec<i32>>>>;

unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}

/// Tue un groupe de processus entier : `zsh -c` et ses enfants.
pub(crate) fn tuer_groupe(pid: i32) {
    unsafe { kill(-pid, 9) };
}

pub(crate) fn tuer_tous(processus: &Processus) {
    if let Some(pids) = processus.lock().unwrap().take() {
        pids.iter().for_each(|&pid| tuer_groupe(pid));
    }
}

/// Inscrit un groupe à tuer avec la barre ; `false` (et le groupe est tué) si la barre est déjà remplacée.
pub(crate) fn inscrire(processus: &Processus, pid: i32) -> bool {
    match processus.lock().unwrap().as_mut() {
        Some(pids) => {
            pids.push(pid);
            true
        }
        None => {
            tuer_groupe(pid);
            false
        }
    }
}

pub(crate) fn retirer(processus: &Processus, pid: i32) {
    if let Some(pids) = processus.lock().unwrap().as_mut() {
        pids.retain(|&p| p != pid);
    }
}

/// Lance la commande dans son propre groupe de processus et la tue au-delà de `delai`.
pub(super) fn executer_script(commande: &str, delai: Duration, processus: &Processus) -> Sortie {
    executer(std::process::Command::new("/bin/zsh").args(["-c", commande]), None, delai, processus)
}

/// Lance `commande` dans son propre groupe (inscrit dans `processus`), lui écrit `entree`, la tue au-delà de `delai`.
pub(crate) fn executer(commande: &mut std::process::Command, entree: Option<&str>, delai: Duration, processus: &Processus) -> Sortie {
    let enfant = commande
        .process_group(0)
        .stdin(if entree.is_some() { std::process::Stdio::piped() } else { std::process::Stdio::null() })
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    let mut enfant = match enfant {
        Ok(e) => e,
        Err(e) => return Sortie { texte: String::new(), erreur: format!("{} : {e}", commande.get_program().to_string_lossy()) },
    };
    let pid = enfant.id() as i32;
    inscrire(processus, pid);
    // Petite entrée (en-têtes et corps d'une requête) : elle tient dans le tampon du tube, l'écrire d'un coup ne bloque pas.
    if let (Some(texte), Some(mut tube)) = (entree, enfant.stdin.take()) {
        use std::io::Write;
        let _ = tube.write_all(texte.as_bytes());
    }
    let (fini, attente) = std::sync::mpsc::channel::<()>();
    let garde = std::thread::spawn(move || {
        let tue = attente.recv_timeout(delai).is_err();
        if tue {
            tuer_groupe(pid);
        }
        tue
    });
    let sortie = enfant.wait_with_output();
    let _ = fini.send(());
    let tue = garde.join().unwrap_or(false);
    retirer(processus, pid);
    let Ok(sortie) = sortie else { return Sortie { texte: String::new(), erreur: "sortie illisible".into() } };
    let stderr = String::from_utf8_lossy(&sortie.stderr).trim().to_string();
    let erreur = if tue {
        format!("tué après {} s", delai.as_secs_f64())
    } else if !sortie.status.success() || !stderr.is_empty() {
        let code = sortie.status.code().map_or("signal".to_string(), |c| format!("code {c}"));
        if stderr.is_empty() { code } else { format!("{code}, {stderr}") }
    } else {
        String::new()
    };
    Sortie { texte: String::from_utf8_lossy(&sortie.stdout).into_owned(), erreur }
}

pub(super) fn rafraichir() {
    ETAT.with_borrow_mut(|etat| {
        let Some(etat) = etat.as_mut() else { return };
        while let Ok((k, valeur)) = etat.valeurs.try_recv() {
            for r in &mut etat.rafraichis {
                if let Rafraichi::Source { bouton, lien, derniere_erreur } = r
                    && lien.source == Some(k)
                {
                    afficher_source(*bouton, lien, &valeur, derniere_erreur);
                }
            }
        }
        while let Ok((i, sortie)) = etat.reception.try_recv() {
            match etat.rafraichis.get_mut(i) {
                Some(Rafraichi::Script { bouton, commande, en_cours, derniere_erreur, derniere_sortie, element, .. }) => {
                    *en_cours = false;
                    let mut erreur = sortie.erreur;
                    if sortie.texte != *derniere_sortie {
                        *derniere_sortie = sortie.texte;
                        match analyser_sortie(derniere_sortie) {
                            Ok(riche) => appliquer(*bouton, &riche, element),
                            Err(e) => {
                                titre(*bouton, derniere_sortie.trim_end());
                                erreur = if erreur.is_empty() { e } else { format!("{erreur}, {e}") };
                            }
                        }
                    }
                    // Une fois par changement, pas à chaque intervalle.
                    if erreur != *derniere_erreur {
                        if !erreur.is_empty() {
                            eprintln!("tactile : script « {commande} » : {erreur}");
                        }
                        *derniere_erreur = erreur;
                    }
                }
                Some(Rafraichi::Batterie { bouton, symbole, en_cours, .. }) => {
                    *en_cours = false;
                    let riche = match batterie::analyser(&sortie.texte) {
                        Some(b) => Riche { texte: b.texte(*symbole), symbole: symbole.then(|| b.symbole().to_string()), ..Riche::default() },
                        None => Riche { texte: "pile ?".into(), ..Riche::default() },
                    };
                    appliquer(*bouton, &riche, "batterie");
                }
                Some(Rafraichi::EnCours { bouton }) => titre(*bouton, &sortie.texte),
                _ => {}
            }
        }
        let maintenant = Instant::now();
        let media_du = maintenant >= etat.prochain_media;
        if media_du {
            etat.prochain_media = maintenant + Duration::from_secs(2);
        }
        for (i, r) in etat.rafraichis.iter_mut().enumerate() {
            match r {
                Rafraichi::Heure { bouton, formateur } => unsafe {
                    let date: Id = msg_send![class!(NSDate), date];
                    let s: Id = msg_send![*formateur, stringFromDate: date];
                    titre(*bouton, &(*(s as *const NSString)).to_string());
                },
                Rafraichi::EnCours { .. } if media_du => {
                    let envoi = etat.envoi.clone();
                    std::thread::spawn(move || {
                        let s = match media::en_cours() {
                            Ok(Some(m)) => {
                                let signe = if m.en_lecture { "♫" } else { "❚❚" };
                                let ligne = if m.artiste.is_empty() { format!("{signe}  {}", m.titre) } else { format!("{signe}  {}  ·  {}", m.titre, m.artiste) };
                                ligne.chars().take(44).collect()
                            }
                            _ => String::new(),
                        };
                        let _ = envoi.send((i, Sortie { texte: s, erreur: String::new() }));
                    });
                }
                Rafraichi::Script { commande, intervalle, delai, prochain, en_cours, .. } if !*en_cours && maintenant >= *prochain => {
                    *en_cours = true;
                    *prochain = maintenant + *intervalle;
                    let (envoi, commande, delai, processus) = (etat.envoi.clone(), commande.clone(), *delai, etat.processus.clone());
                    std::thread::spawn(move || {
                        let _ = envoi.send((i, executer_script(&commande, delai, &processus)));
                    });
                }
                Rafraichi::Batterie { prochain, en_cours, .. } if !*en_cours && maintenant >= *prochain => {
                    *en_cours = true;
                    *prochain = maintenant + batterie::INTERVALLE;
                    let envoi = etat.envoi.clone();
                    std::thread::spawn(move || {
                        let _ = envoi.send((i, Sortie { texte: batterie::lire(), erreur: String::new() }));
                    });
                }
                _ => {}
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn processus() -> Processus {
        Arc::new(Mutex::new(Some(Vec::new())))
    }

    fn vivants(motif: &str) -> usize {
        let ps = std::process::Command::new("/usr/bin/pgrep").args(["-f", motif]).output().unwrap();
        String::from_utf8_lossy(&ps.stdout).lines().count()
    }

    #[test]
    fn sortie_json_ou_texte_brut() {
        assert_eq!(analyser_sortie("ok\n"), Ok(Riche { texte: "ok".into(), ..Riche::default() }));
        assert_eq!(
            analyser_sortie("{\"texte\":\"ok\",\"couleur\":\"#00AA00\"}\n"),
            Ok(Riche { texte: "ok".into(), couleur: Some("#00AA00".into()), ..Riche::default() })
        );
        assert_eq!(
            analyser_sortie(r##"{"text": "12°", "symbol": "cloud.rain.fill", "image": "~/x.png", "color": "#1F3A5F", "text_color": "#FFFFFF"}"##),
            Ok(Riche { texte: "12°".into(), symbole: Some("cloud.rain.fill".into()), image: Some("~/x.png".into()), couleur: Some("#1F3A5F".into()), couleur_texte: Some("#FFFFFF".into()) })
        );
        assert_eq!(analyser_sortie("{}"), Ok(Riche::default()));
        assert!(analyser_sortie("{\"texte\": }").unwrap_err().starts_with("JSON illisible ("));
        assert!(analyser_sortie("{\"titre\": \"x\"}").unwrap_err().contains("unknown field `titre`"));
    }

    #[test]
    fn un_script_trop_long_est_tue_avec_son_groupe() {
        let debut = Instant::now();
        let sortie = executer_script("sleep 30; echo fini", Duration::from_millis(500), &processus());
        assert!(debut.elapsed() < Duration::from_secs(3), "{:?}", debut.elapsed());
        assert_eq!(sortie.erreur, "tué après 0.5 s");
        assert_eq!(sortie.texte, "");
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(vivants("sleep 30; echo fini"), 0, "le zsh survit");
        assert_eq!(vivants("^sleep 30$"), 0, "le sleep survit");
    }

    #[test]
    fn stderr_et_code_de_retour_remontent() {
        let p = processus();
        let sortie = executer_script("echo oups >&2; echo ok; exit 3", Duration::from_secs(5), &p);
        assert_eq!(sortie.texte, "ok\n");
        assert_eq!(sortie.erreur, "code 3, oups");
        assert_eq!(executer_script("echo bien", Duration::from_secs(5), &p).erreur, "");
        assert_eq!(p.lock().unwrap().as_ref().unwrap().len(), 0);
    }

    #[test]
    fn une_barre_remplacee_tue_ses_scripts() {
        let p = processus();
        let p2 = p.clone();
        let fil = std::thread::spawn(move || executer_script("sleep 31; echo fini", Duration::from_secs(60), &p2));
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(p.lock().unwrap().as_ref().unwrap().len(), 1);
        tuer_tous(&p);
        let sortie = fil.join().unwrap();
        assert_eq!(sortie.texte, "");
        assert_eq!(vivants("^sleep 31$"), 0);
        // Après le remplacement, un script qui démarre encore est tué tout de suite.
        let debut = Instant::now();
        executer_script("sleep 32", Duration::from_secs(60), &p);
        assert!(debut.elapsed() < Duration::from_secs(3));
    }
}
