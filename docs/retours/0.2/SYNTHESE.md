# Synthèse des retours sur la 0.2

Seconde vague, mêmes personas (rapports dans ce dossier). L'écran du Mac est resté verrouillé pendant les essais : tout ce qui dépend de l'app au premier plan ou d'une capture de fenêtre est à reprouver écran déverrouillé.

Verdicts : Lise lance une semaine d'essai (ses deux bloquants, barre par app et raccourcis, sont réglés) ; Nadia fait sa barre sans TOML ; Sam écrit une config anglaise complète du premier coup ; Marc ne trouve plus de bloquant.

## Lot R — exécution (sur la vraie barre)

| # | Problème | Source | Gravité | Preuve attendue |
|---|---|---|---|---|
| R1 | AZERTY : un chiffre se résout sur le pavé numérique (`4` → 86 au lieu de 21), `cmd+shift+4` ne lance pas la capture ; `cmd+\` introuvable ; `keycode` sans modificateur | Marc, Lise | majeur | test : sur la disposition courante, chaque chiffre donne le keycode de la rangée du haut ; `{ keycode = 21, modificateurs = "cmd+shift" }` ou `"cmd+shift+keycode:21"` |
| R2 | Fuites : chaque rechargement garde l'ancienne barre (14 → 32 Mo / 20 rechargements), image géante gardée (831 Mo), script JSON avec `image` (20 → 141 Mo en 60 s) | Marc | majeur | mesure avant/après dans le rapport ; images bornées (lecture redimensionnée), cache par chemin |
| R3 | Badge « ⚠ config » invisible sur une barre `[[application]]` | Marc | majeur | étape d'autotest |
| R4 | Fichier vide → Touch Bar noire sans esc ni message | Marc | majeur | barre par défaut + badge ; test |
| R5 | `[[application]]` : 0,5–0,7 % CPU (sondage 0,1 s) | Marc | mineur | observateur `NSWorkspaceDidActivateApplicationNotification` ou sondage à 0,5 s ; mesure |
| R6 | Boutons à texte court qui débordent sans message (B3) | Marc | mineur | avertissement aussi pour eux |

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
