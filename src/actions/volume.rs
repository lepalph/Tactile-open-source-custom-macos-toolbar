//! Volume de la sortie audio par défaut (0.0 à 1.0) et coupure du son.

use super::Resultat;
use std::ffi::c_void;

#[repr(C)]
struct Adresse {
    selecteur: u32,
    portee: u32,
    element: u32,
}

const fn code(s: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*s)
}

const SYSTEME: u32 = 1;
const SORTIE_PAR_DEFAUT: u32 = code(b"dOut");
const VOLUME_PRINCIPAL_VIRTUEL: u32 = code(b"vmvc");
const MUET: u32 = code(b"mute");
const PORTEE_GLOBALE: u32 = code(b"glob");
const PORTEE_SORTIE: u32 = code(b"outp");
const ELEMENT_PRINCIPAL: u32 = 0;

#[link(name = "CoreAudio", kind = "framework")]
unsafe extern "C" {
    fn AudioObjectGetPropertyData(
        objet: u32,
        adresse: *const Adresse,
        taille_qualif: u32,
        qualif: *const c_void,
        taille: *mut u32,
        donnees: *mut c_void,
    ) -> i32;
    fn AudioObjectSetPropertyData(
        objet: u32,
        adresse: *const Adresse,
        taille_qualif: u32,
        qualif: *const c_void,
        taille: u32,
        donnees: *const c_void,
    ) -> i32;
}

// Le volume principal virtuel est fourni par AudioToolbox.
#[link(name = "AudioToolbox", kind = "framework")]
unsafe extern "C" {}

fn lire_propriete<T: Default>(objet: u32, selecteur: u32, portee: u32, nom: &str) -> Resultat<T> {
    let adresse = Adresse { selecteur, portee, element: ELEMENT_PRINCIPAL };
    let mut valeur = T::default();
    let mut taille = size_of::<T>() as u32;
    let statut = unsafe {
        AudioObjectGetPropertyData(objet, &adresse, 0, std::ptr::null(), &mut taille, (&raw mut valeur).cast())
    };
    match statut {
        0 => Ok(valeur),
        s => Err(format!("volume : lecture de {nom} impossible (OSStatus {s})")),
    }
}

fn ecrire_propriete<T>(objet: u32, selecteur: u32, valeur: T, nom: &str) -> Resultat<()> {
    let adresse = Adresse { selecteur, portee: PORTEE_SORTIE, element: ELEMENT_PRINCIPAL };
    let statut = unsafe {
        AudioObjectSetPropertyData(objet, &adresse, 0, std::ptr::null(), size_of::<T>() as u32, (&raw const valeur).cast())
    };
    match statut {
        0 => Ok(()),
        s => Err(format!("volume : réglage de {nom} impossible (OSStatus {s})")),
    }
}

fn sortie() -> Resultat<u32> {
    match lire_propriete::<u32>(SYSTEME, SORTIE_PAR_DEFAUT, PORTEE_GLOBALE, "la sortie par défaut")? {
        0 => Err("volume : aucune sortie audio par défaut".into()),
        id => Ok(id),
    }
}

pub fn lire() -> Resultat<f32> {
    lire_propriete(sortie()?, VOLUME_PRINCIPAL_VIRTUEL, PORTEE_SORTIE, "volume")
}

pub fn regler(valeur: f32) -> Resultat<()> {
    ecrire_propriete(sortie()?, VOLUME_PRINCIPAL_VIRTUEL, valeur.clamp(0.0, 1.0), "volume")
}

pub fn est_muet() -> Resultat<bool> {
    lire_propriete::<u32>(sortie()?, MUET, PORTEE_SORTIE, "muet").map(|m| m != 0)
}

pub fn basculer_muet() -> Resultat<()> {
    let muet = est_muet()?;
    ecrire_propriete(sortie()?, MUET, u32::from(!muet), "muet")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Volume et muet touchent la même sortie : un seul test, pour qu'ils ne tournent pas en parallèle.
    #[test]
    #[cfg_attr(ci, ignore = "no audio output on a CI runner")]
    fn volume_et_muet_restaures() {
        let origine = lire().unwrap();
        let cible = (origine - 0.1).max(0.0);
        regler(cible).unwrap();
        let relue = lire();
        regler(origine).unwrap();
        assert!((relue.unwrap() - cible).abs() < 0.02, "volume relu différent de {cible}");
        assert!((lire().unwrap() - origine).abs() < 0.02);

        let muet = est_muet().unwrap();
        basculer_muet().unwrap();
        let bascule = est_muet();
        basculer_muet().unwrap();
        assert_eq!(bascule.unwrap(), !muet);
        assert_eq!(est_muet().unwrap(), muet);
    }
}
