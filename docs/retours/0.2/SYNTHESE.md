# Synthèse des retours sur la 0.2

Seconde vague, mêmes personas (rapports dans ce dossier). L'écran du Mac est resté verrouillé pendant les essais : tout ce qui dépend de l'app au premier plan ou d'une capture de fenêtre est à reprouver écran déverrouillé.

Verdicts : Lise lance une semaine d'essai (ses deux bloquants, barre par app et raccourcis, sont réglés) ; Nadia fait sa barre sans TOML ; Sam écrit une config anglaise complète du premier coup ; Marc ne trouve plus de bloquant.

## Lot R — exécution (sur la vraie barre)

| # | Problème | Source | Gravité | Preuve attendue |
|---|---|---|---|---|
| R1 | AZERTY : un chiffre se résout sur le pavé numérique (`4` → 86 au lieu de 21), `cmd+shift+4` ne lance pas la capture ; `cmd+\` introuvable ; `keycode` sans modificateur | Marc, Lise | majeur | **Fait.** Tests `un_chiffre_vise_la_rangee_du_haut` (table fixe : `cmd+shift+4` → 21 + cmd + shift, sur toute disposition), `azerty_chiffres_antislash_et_caracteres_a_option` (disposition French chargée explicitement : `4` → 21 + Maj, `cmd+4` → 21 sans Maj, `\` → 47 + ⌥⇧, `[` → 23 + ⌥⇧, `/` → 47 + Maj et non 75), `keycode_avec_modificateurs` (`"cmd+shift+keycode:21"`, aller-retour) |
| R2 | Fuites : chaque rechargement garde l'ancienne barre (14 → 32 Mo / 20 rechargements), image géante gardée (831 Mo), script JSON avec `image` (20 → 141 Mo en 60 s) | Marc | majeur | **Fait.** Mesures `footprint` ci-dessous ; test `image_redimensionnee_ou_absente` (40 px de haut, même objet au 2e appel) |
| R3 | Badge « ⚠ config » invisible sur une barre `[[application]]` | Marc | majeur | **Fait.** Autotest 28 : « ⚠ config ligne 9 » aussi sur la barre de TextEdit (vu en capture) |
| R4 | Fichier vide → Touch Bar noire sans esc ni message | Marc | majeur | **Fait.** Test `config_sans_element_donne_la_barre_par_defaut` ; fichier vide lancé en vrai : barre par défaut + « ⚠ config vide » (capture) |
| R5 | `[[application]]` : 0,5–0,7 % CPU (sondage 0,1 s) | Marc | mineur | **Fait** (observateur). CPU avec une `[[application]]` : 0,5–0,7 % → 0,1–0,2 % |
| R6 | Boutons à texte court qui débordent sans message (B3) | Marc | mineur | **Fait.** Test `debordement_compte_les_elements_caches` : 15 boutons « B1 »… → 2 cachés annoncés (3 réels), aucun avant |

Mesures R2 et R5 (`footprint -p`, `top -stats cpu`, processus neuf, `TACTILE_CONFIG` dans un dossier temporaire, écran verrouillé) :

| Cas | Avant | Après |
|---|---|---|
| Barre par défaut, 20 rechargements (`touch`) | 14 → 32 Mo | 14 → 17 Mo (18 Mo après 40, 60 et 80 : palier) |
| Bouton `image` PNG 10 000 × 10 000, 2 constructions, puis barre par défaut | 400 → 790 → 792 Mo | 20 → 20 → 22 Mo |
| Script JSON avec `image` (PNG 1024²), `intervalle = 0.3`, 60 s | 59 → 429 → 796 Mo | 12 → 12 → 12 Mo |
| CPU, barre par défaut + une `[[application]]` | 0,5–0,7 % | 0,1–0,2 % |
| CPU, barre par défaut (témoin) | 0,1–0,2 % | — |

## Lot C — config, `--verifier`, ligne de commande

| # | Problème | Source | Gravité |
|---|---|---|---|
| C1 | `[[element]]` et `[[item]]` dans un même fichier → « duplicate field », alors que le README promet le mélange | Sam, Marc | majeur |
| C2 | `--verifier` ne contrôle que la syntaxe : couleur (`red`, `#F00`), symbole SF, image, app introuvable, débordement, app en double, fichier vide passent | Sam, Lise, Marc, Nadia | majeur |
| C3 | Ligne d'erreur fausse dans une `[[application]]` (`element_fautif` ne descend pas) ; erreurs des applications avant celles de la principale | Lise | majeur |
| C4 | `[[element]]` égaré après un bloc `[[application]]` part dans la principale sans avertissement | Lise | mineur |
| C5 | Messages serde à moitié traduits ; « 1 éléments » | Sam, Lise | mineur |
| C6 | `--help` et les options seulement en français ; `--check` refusé | Sam | mineur |
| C7 | `Cargo.toml` 0.1.0 sans métadonnées ; tests matériels « ok » sous `CI` au lieu d'ignorés ; test glossaire lit seulement README.fr.md ; CONTRIBUTING donne 3 des 8 endroits pour ajouter une action | Sam | mineur |

## Lot E — éditeur

| # | Problème | Source | Gravité |
|---|---|---|---|
| E1 | Fichier abîmé : l'éditeur s'ouvre vide, « Enregistrer » écrit `element = []` et répond « Enregistré » ; la Touch Bar se vide (perte de données) | Nadia | majeur |
| E2 | Fermer la fenêtre, choisir une barre toute faite, « Revenir » : changements perdus sans confirmation ni message | Nadia | majeur |
| E3 | Nom d'app tapé à la main non vérifié (« Photoshop » au lieu de « Adobe Photoshop 2025 ») | Nadia | majeur |
| E4 | Enregistrer efface les commentaires d'un fichier écrit à la main ; une seule copie `.precedent` | Marc | majeur |

## Reportés après la 0.2

`barre = "apple"` et `app = [liste]` (Lise), import BTT, essai sur Mac Intel, messages d'erreur en anglais.
