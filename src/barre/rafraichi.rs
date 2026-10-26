//! Éléments qui se mettent à jour seuls : heure, titre en cours, scripts.
//! Le travail lent (commandes, media-control) tourne dans un fil à part ; le résultat revient par un canal.

use objc2::{class, msg_send};
use objc2_foundation::NSString;
use std::os::unix::process::CommandExt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::vues::titre;
use super::{Id, ETAT};
use crate::actions::media;

pub(super) enum Rafraichi {
    Heure { bouton: Id, formateur: Id },
    EnCours { bouton: Id },
    Script { bouton: Id, commande: String, intervalle: Duration, delai: Duration, prochain: Instant, en_cours: bool, derniere_erreur: String },
}

/// Ce qu'un fil renvoie : la sortie à afficher, et ce qui a cloché (vide si rien).
pub(super) struct Sortie {
    pub texte: String,
    pub erreur: String,
}

/// Groupes de processus des scripts en cours. `None` : la barre a été remplacée, tout nouveau
/// processus est tué aussitôt ; sinon un orphelin par rechargement (vu avec `sleep 1000`).
pub(super) type Processus = Arc<Mutex<Option<Vec<i32>>>>;

unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}

/// Tue un groupe de processus entier : `zsh -c` et ses enfants.
fn tuer_groupe(pid: i32) {
    unsafe { kill(-pid, 9) };
}

pub(super) fn tuer_tous(processus: &Processus) {
    if let Some(pids) = processus.lock().unwrap().take() {
        pids.iter().for_each(|&pid| tuer_groupe(pid));
    }
}

/// Lance la commande dans son propre groupe de processus et la tue au-delà de `delai`.
pub(super) fn executer_script(commande: &str, delai: Duration, processus: &Processus) -> Sortie {
    let enfant = std::process::Command::new("/bin/zsh")
        .args(["-c", commande])
        .process_group(0)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    let enfant = match enfant {
        Ok(e) => e,
        Err(e) => return Sortie { texte: String::new(), erreur: format!("zsh : {e}") },
    };
    let pid = enfant.id() as i32;
    match processus.lock().unwrap().as_mut() {
        Some(pids) => pids.push(pid),
        None => tuer_groupe(pid),
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
    if let Some(pids) = processus.lock().unwrap().as_mut() {
        pids.retain(|&p| p != pid);
    }
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
        while let Ok((i, sortie)) = etat.reception.try_recv() {
            match etat.rafraichis.get_mut(i) {
                Some(Rafraichi::Script { bouton, commande, en_cours, derniere_erreur, .. }) => {
                    *en_cours = false;
                    titre(*bouton, sortie.texte.trim_end());
                    // Une fois par changement, pas à chaque intervalle.
                    if sortie.erreur != *derniere_erreur {
                        if !sortie.erreur.is_empty() {
                            eprintln!("tactile : script « {commande} » : {}", sortie.erreur);
                        }
                        *derniere_erreur = sortie.erreur;
                    }
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
