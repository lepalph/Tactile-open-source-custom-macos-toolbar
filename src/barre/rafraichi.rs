//! Éléments qui se mettent à jour seuls : heure, titre en cours, scripts.
//! Le travail lent (commandes, media-control) tourne dans un fil à part ; le résultat revient par un canal.

use objc2::{class, msg_send};
use objc2_foundation::NSString;
use std::time::{Duration, Instant};

use super::vues::titre;
use super::{Id, ETAT};
use crate::actions::media;

pub(super) enum Rafraichi {
    Heure { bouton: Id, formateur: Id },
    EnCours { bouton: Id },
    Script { bouton: Id, commande: String, intervalle: Duration, prochain: Instant, en_cours: bool },
}

pub(super) fn rafraichir() {
    ETAT.with_borrow_mut(|etat| {
        let Some(etat) = etat.as_mut() else { return };
        while let Ok((i, sortie)) = etat.reception.try_recv() {
            match etat.rafraichis.get_mut(i) {
                Some(Rafraichi::Script { bouton, en_cours, .. }) => {
                    *en_cours = false;
                    titre(*bouton, sortie.trim_end());
                }
                Some(Rafraichi::EnCours { bouton }) => titre(*bouton, &sortie),
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
                        let _ = envoi.send((i, s));
                    });
                }
                Rafraichi::Script { commande, intervalle, prochain, en_cours, .. } if !*en_cours && maintenant >= *prochain => {
                    *en_cours = true;
                    *prochain = maintenant + *intervalle;
                    let (envoi, commande) = (etat.envoi.clone(), commande.clone());
                    std::thread::spawn(move || {
                        let sortie = std::process::Command::new("/bin/zsh").args(["-c", &commande]).output();
                        let s = sortie.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
                        let _ = envoi.send((i, s));
                    });
                }
                _ => {}
            }
        }
    });
}
