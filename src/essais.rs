//! Outils des tests par propriétés : un hasard reproductible (xorshift, sans dépendance) et des barres générées.
//! Chaque test tire ses cas d'une graine fixe : un échec se rejoue à l'identique, la graine est dans le message.

use crate::actions::{clavier::Touche, Action};
use crate::config::{Affichage, Application, Barre, CouleurSelon, Element, Reglage, Source, SourceMcp, SymboleSelon};
use serde_json::json;

/// xorshift64* : assez de hasard pour varier les cas, et le même tirage à chaque exécution.
pub(crate) struct Hasard(u64);

impl Hasard {
    pub fn new(graine: u64) -> Self {
        Hasard(graine.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn suivant(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Un entier dans `0..n`.
    pub fn jusqua(&mut self, n: usize) -> usize {
        (self.suivant() % n as u64) as usize
    }

    pub fn pile(&mut self) -> bool {
        self.suivant() & 1 == 1
    }

    pub fn parmi<'a, T>(&mut self, liste: &'a [T]) -> &'a T {
        &liste[self.jusqua(liste.len())]
    }

    fn peut_etre<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> Option<T> {
        if self.pile() { Some(f(self)) } else { None }
    }

    /// Un texte piégeux pour le TOML et le JSON : guillemets, `#`, retours à la ligne, accents, émojis, en-têtes `[[…]]`.
    pub fn texte(&mut self) -> String {
        const PIEGES: [&str; 16] = ["", "a", "Build", "é à ç", "日本", "\"guillemets\"", "anti\\slash", "deux\nlignes", "#pas un commentaire", "'apostrophe'", "tab\t", "🎵 musique", "]]", "[[element]]", " = ", "\u{7f}\u{1}"];
        let mut t = String::new();
        for _ in 0..1 + self.jusqua(3) {
            t += self.parmi(&PIEGES);
        }
        t
    }

    /// Un nombre écrit par un humain ou par l'éditeur : entier, décimal, petit, grand.
    pub fn nombre(&mut self) -> f64 {
        match self.jusqua(4) {
            0 => *self.parmi(&[0.0, 0.1, 1.0, 56.0, 120.0, 1004.0, 0.25, 1e-3, 1e6]),
            1 => self.jusqua(2000) as f64,
            2 => self.jusqua(100_000) as f64 / 1000.0,
            _ => f64::from_bits(0x3FF0_0000_0000_0000 | (self.suivant() >> 12)) * self.jusqua(500) as f64,
        }
    }

    /// Une valeur de curseur (`volume`, `luminosite`) : n'importe quel f32 de 0 à 1, pas seulement des valeurs rondes.
    fn reel(&mut self) -> f32 {
        match self.jusqua(3) {
            0 => *self.parmi(&[0.0, 0.5, 1.0, 0.3, 0.0625]),
            _ => (self.suivant() >> 40) as f32 / (1u64 << 24) as f32,
        }
    }

    pub fn touche(&mut self) -> Touche {
        const MODS: [&str; 7] = ["cmd", "shift", "ctrl", "alt", "fn", "option", "maj"];
        const NOMS: [&str; 10] = ["a", "4", "esc", "f5", "espace", "keycode:21", "gauche", "/", "é", "tab"];
        let mut mots: Vec<&str> = (0..self.jusqua(3)).map(|_| *self.parmi(&MODS)).collect();
        mots.push(self.parmi(&NOMS));
        Touche::try_from(mots.join("+")).expect("touche générée valide")
    }

    pub fn action(&mut self) -> Action {
        const SIMPLES: [&str; 23] = [
            "luminosite_plus", "luminosite_moins", "volume_plus", "volume_moins", "basculer_muet", "lecture_pause", "suivant", "precedent", "veille", "echap",
            "barre_apple", "mission_control", "fenetres_app", "launchpad", "bureau", "spotlight", "capture_ecran", "capture_zone", "capture_options",
            "verrouiller", "emojis", "forcer_quitter", "lecture_pause",
        ];
        match self.jusqua(10) {
            0 => Action::Luminosite(self.reel()),
            1 => Action::Volume(self.reel()),
            2 => Action::Shell(self.texte()),
            3 => Action::Touche(self.touche()),
            4 => Action::Keycode(self.suivant() as u16),
            5 => Action::App(self.texte()),
            6 => Action::Url(self.texte()),
            7 => Action::Applescript(self.texte()),
            _ => serde_json::from_value(serde_json::Value::String(self.parmi(&SIMPLES).to_string())).expect("action simple"),
        }
    }

    fn affichage(&mut self) -> Affichage {
        *self.parmi(&[Affichage::Icone, Affichage::Texte, Affichage::LesDeux])
    }

    pub fn element(&mut self, profondeur: usize) -> Element {
        match self.jusqua(if profondeur < 2 { 12 } else { 11 }) {
            0 | 1 => Element::Bouton {
                texte: self.peut_etre(Self::texte),
                symbole: self.peut_etre(|h| h.parmi(&["play.fill", "sun.max", ""]).to_string()),
                image: self.peut_etre(|h| h.parmi(&["~/logo.png", "/tmp/x y.png", "icones/a.png"]).to_string()),
                gabarit: self.pile(),
                couleur: self.peut_etre(|h| h.parmi(&["#A259FF", "#000000", "violet"]).to_string()),
                couleur_texte: self.peut_etre(|_| "#FFFFFF".to_string()),
                taille: self.peut_etre(Self::nombre),
                affichage: self.peut_etre(Self::affichage),
                largeur: self.peut_etre(Self::nombre),
                action: self.peut_etre(Self::action),
                appui_long: self.peut_etre(Self::action),
                source: self.peut_etre(Self::texte),
            },
            2 => Element::Slider { reglage: *self.parmi(&[Reglage::Luminosite, Reglage::Volume]), symbole: self.peut_etre(|_| "speaker.wave.2".to_string()) },
            3 => Element::Heure { format: self.peut_etre(Self::texte) },
            4 => Element::EnCours { largeur: self.peut_etre(Self::nombre) },
            5 => Element::Script {
                commande: self.texte(),
                intervalle: self.nombre(),
                delai: self.peut_etre(Self::nombre),
                largeur: self.peut_etre(Self::nombre),
                action: self.peut_etre(Self::action),
            },
            6 => Element::Espace { largeur: self.nombre() },
            7 => Element::Souple,
            8 => Element::Batterie { symbole: self.peut_etre(Self::pile), largeur: self.peut_etre(Self::nombre) },
            10 => Element::Texte {
                source: self.peut_etre(Self::texte),
                gabarit: self.peut_etre(|h| format!("{{{}}} {}", h.parmi(&["texte", "a.b", "liste.0.nom"]), h.texte())),
                symbole_selon: self.peut_etre(|h| SymboleSelon {
                    champ: h.texte(),
                    valeurs: (0..h.jusqua(3)).map(|_| (h.texte(), "sun.max".to_string())).collect(),
                    defaut: h.peut_etre(|_| "cloud".to_string()),
                }),
                couleur_selon: self.peut_etre(|h| CouleurSelon { champ: h.texte(), seuils: (0..h.jusqua(3)).map(|_| h.nombre()).collect(), couleurs: vec!["#FF0000".into(); h.jusqua(4)] }),
                largeur: self.peut_etre(Self::nombre),
                action: self.peut_etre(Self::action),
            },
            9 => Element::Lanceur {
                app: self.texte(),
                texte: self.peut_etre(Self::texte),
                affichage: self.peut_etre(Self::affichage),
                largeur: self.peut_etre(Self::nombre),
                appui_long: self.peut_etre(Self::action),
            },
            _ => Element::Groupe {
                texte: self.peut_etre(Self::texte),
                symbole: self.peut_etre(|_| "ellipsis".to_string()),
                largeur: self.peut_etre(Self::nombre),
                elements: (0..self.jusqua(4)).map(|_| self.element(profondeur + 1)).collect(),
            },
        }
    }

    /// Une barre complète : principale (vide parfois), et jusqu'à trois barres d'application d'au moins un élément.
    pub fn barre(&mut self) -> Barre {
        Barre {
            masquer_control_strip: self.pile(),
            elements: (0..self.jusqua(7)).map(|_| self.element(0)).collect(),
            applications: (0..self.jusqua(3))
                .map(|_| Application { app: self.texte(), herite: self.pile(), elements: (0..1 + self.jusqua(3)).map(|_| self.element(0)).collect() })
                .collect(),
            sources: (0..self.jusqua(3)).map(|_| self.source()).collect(),
        }
    }

    /// Une `[[source]]` valide : exactement un de http, commande ou mcp (stdio ou url).
    pub fn source(&mut self) -> Source {
        let (http, commande, mcp) = match self.jusqua(3) {
            0 => (Some(format!("https://example.com/{}", self.jusqua(99))), None, None),
            1 => (None, Some(self.texte()), None),
            _ => {
                let stdio = self.pile();
                let arguments = self.peut_etre(|h| json!({ "ville": h.texte(), "jours": h.jusqua(9), "precis": h.pile(), "liste": [h.texte()] }));
                (None, None, Some(SourceMcp { commande: stdio.then(|| self.texte()), url: (!stdio).then(|| "https://mcp.example.com/mcp".to_string()), outil: self.texte(), arguments }))
            }
        };
        let entetes = (0..self.jusqua(3)).map(|k| (format!("X-{k}"), self.parmi(&["trousseau:meteo", "Bearer abc", ""]).to_string())).collect();
        Source { nom: self.texte(), http, commande, mcp, toutes_les: self.peut_etre(Self::nombre), entetes }
    }
}

/// Lance `f` en attrapant une panique : `Err(message)` au lieu d'arrêter le test, pour dire quel cas l'a provoquée.
pub(crate) fn sans_panique<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|e| {
        e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()
    })
}

/// Pic de mémoire du processus de test (`getrusage`, `ru_maxrss` : en octets sur macOS).
pub(crate) fn pic_memoire() -> i64 {
    #[repr(C)]
    struct Rusage {
        temps: [i64; 4],
        maxrss: i64,
        reste: [i64; 13],
    }
    unsafe extern "C" {
        fn getrusage(qui: i32, usage: *mut Rusage) -> i32;
    }
    let mut u = Rusage { temps: [0; 4], maxrss: 0, reste: [0; 13] };
    // 0 : RUSAGE_SELF.
    assert_eq!(unsafe { getrusage(0, &mut u) }, 0);
    u.maxrss
}
