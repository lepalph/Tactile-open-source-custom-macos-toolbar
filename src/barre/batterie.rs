//! Élément `batterie` : pourcentage et charge, lus dans `pmset -g batt` toutes les 30 s.

use std::time::Duration;

pub(super) const INTERVALLE: Duration = Duration::from_secs(30);

#[derive(Debug, PartialEq)]
pub(super) struct Batterie {
    pub pourcent: u8,
    pub en_charge: bool,
}

/// La sortie brute de `pmset -g batt` (lue dans un fil, hors du fil principal).
pub(super) fn lire() -> String {
    std::process::Command::new("/usr/bin/pmset")
        .args(["-g", "batt"])
        .output()
        .map(|s| String::from_utf8_lossy(&s.stdout).into_owned())
        .unwrap_or_default()
}

/// `None` sans batterie (Mac de bureau) ou si la sortie change de forme.
pub(super) fn analyser(pmset: &str) -> Option<Batterie> {
    let (avant, _) = pmset.split_once('%')?;
    let pourcent = avant.rsplit(|c: char| !c.is_ascii_digit()).next()?.parse().ok()?;
    Some(Batterie { pourcent, en_charge: pmset.contains("'AC Power'") })
}

impl Batterie {
    /// Symbole SF : niveau par quart, éclair en charge.
    pub fn symbole(&self) -> &'static str {
        match (self.en_charge, self.pourcent) {
            (true, _) => "battery.100percent.bolt",
            (_, 88..) => "battery.100percent",
            (_, 63..) => "battery.75percent",
            (_, 38..) => "battery.50percent",
            (_, 13..) => "battery.25percent",
            _ => "battery.0percent",
        }
    }

    /// Texte seul (`symbole = false`) : l'éclair dit la charge.
    pub fn texte(&self, avec_symbole: bool) -> String {
        if self.en_charge && !avec_symbole { format!("⚡ {} %", self.pourcent) } else { format!("{} %", self.pourcent) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyse_pmset() {
        let secteur = "Now drawing from 'AC Power'\n -InternalBattery-0 (id=16384099)\t100%; charged; 0:00 remaining present: true\n";
        let batterie = "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=16384099)\t47%; discharging; 3:20 remaining present: true\n";
        assert_eq!(analyser(secteur), Some(Batterie { pourcent: 100, en_charge: true }));
        assert_eq!(analyser(batterie), Some(Batterie { pourcent: 47, en_charge: false }));
        assert_eq!(analyser("Now drawing from 'AC Power'\n"), None);
        assert_eq!(analyser(""), None);
        let b = analyser(batterie).unwrap();
        assert_eq!((b.texte(true), b.texte(false), b.symbole()), ("47 %".into(), "47 %".into(), "battery.50percent"));
        let s = analyser(secteur).unwrap();
        assert_eq!((s.texte(true), s.texte(false), s.symbole()), ("100 %".into(), "⚡ 100 %".into(), "battery.100percent.bolt"));
        assert_eq!(Batterie { pourcent: 5, en_charge: false }.symbole(), "battery.0percent");
    }

    #[test]
    fn chaque_symbole_de_pile_existe() {
        for pourcent in [0, 13, 38, 63, 88, 100] {
            for en_charge in [false, true] {
                let nom = Batterie { pourcent, en_charge }.symbole();
                assert!(super::super::vues::symbole(nom).is_some(), "{nom}");
            }
        }
    }

    #[test]
    #[cfg_attr(ci, ignore = "pas de batterie sur un runner de CI")]
    fn la_vraie_sortie_se_lit() {
        assert!(analyser(&lire()).is_some());
    }
}
