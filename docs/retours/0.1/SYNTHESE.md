# Synthèse des retours sur la 0.1

Quatre personas (rapports dans ce dossier) : Nadia (débutante), Marc (avancé, sur la vraie barre), Lise (BetterTouchTool), Sam (dev anglophone). Carnet ordonné ; chaque item dit la preuve qui le ferme.

## A. Bugs

| # | Bug | Source | Gravité | Preuve de correction |
|---|---|---|---|---|
| A1 | Symbole SF inexistant → panique au rechargement, Touch Bar noire, replante au relancement | Marc | bloquant | test : symbole inconnu → élément remplacé par son nom en texte + erreur journalisée ; la barre reste |
| A2 | Gatekeeper Sequoia : « clic droit → Ouvrir » ne marche plus pour une app ad hoc | Nadia | bloquant | README : chemin Réglages → Confidentialité et sécurité → « Ouvrir quand même », et commande `xattr -d com.apple.quarantine` en alternative ; icône d'app |
| A3 | Erreur de config invisible (journal caché), barre « qui ne marche plus » | Nadia, Marc | majeur | notification macOS à l'erreur + entrée de menu « Voir le journal » + `tactile --verifier fichier.toml` |
| A4 | Éléments qui débordent disparaissent sans message ; texte long avale la barre | Marc | majeur | largeur maximale par défaut pour `bouton` texte ; avertissement journalisé quand la somme des largeurs dépasse la barre |
| A5 | Scripts : stderr et code retour jetés ; script sans fin → orphelins à chaque rechargement | Marc | majeur | délai maximal (`delai`, 5 s par défaut) qui tue le processus ; stderr dans le journal ; test |
| A6 | `masquer_control_strip` : doc dit `true` par défaut, code dit `false` ; sans effet à chaud | Nadia, Marc | majeur | défaut `true` + représentation à chaud ; test |
| A7 | `cargo doc` : lien intra-doc cassé `[[element]]` config.rs:83 | Sam | mineur | `cargo doc` sans avertissement |
| A8 | `main.rs` ignore argv : `tactile --help` lance l'app | Sam | mineur | `--help`, `--version`, `--verifier` |

## B. Ergonomie

| # | Point | Source |
|---|---|---|
| B1 | Pas d'éditeur visuel : tout passe par un fichier caché et un vocabulaire (TOML, SF Symbols, hex) inconnu d'une débutante | Nadia |
| B2 | Lancer une app = `shell = "open -a Nom"` avec le nom anglais, jamais expliqué | Nadia, Lise |
| B3 | Anglais : 19 lignes en pied de README, erreurs traduites en français, CONTRIBUTING impose le français | Sam |
| B4 | Clés TOML françaises : frein international → alias anglais `#[serde(alias)]`, glossaire testé | Sam, Lise |
| B5 | Présentation GitHub : pas de bannière, badges, GIF, notes de release, gabarits d'issues, icône | Nadia, Sam |

## C. Personnalisation (par valeur)

| # | Fonction | Syntaxe souhaitée | Source |
|---|---|---|---|
| C1 | Raccourci clavier comme action | `action = { touche = "cmd+shift+4" }` | Marc, Lise |
| C2 | Lancer une app / ouvrir une URL | `action = { app = "Figma" }`, `{ url = "https://…" }` | Nadia, Lise |
| C3 | Barre par application | `[[application]] app = "com.adobe.PremierePro"` + ses `[[application.element]]`, `herite = true` | Lise, Marc |
| C4 | Apparence : image perso, couleur du texte, taille, icône + texte | `image = "~/x.png"`, `couleur_texte`, `taille`, `affichage = "icone"\|"texte"\|"les_deux"` | Lise, Marc |
| C5 | `largeur` sur `groupe` (libellé tronqué) | `largeur = 100` | Marc |
| C6 | Scripts riches : sortie JSON `{"texte","symbole","couleur"}` | automatique si la sortie commence par `{` | Lise |
| C7 | Widget batterie | `type = "batterie"` | Marc, Lise |
| C8 | AppleScript natif | `action = { applescript = "…" }` | Lise |
| C9 | Import de presets BTT | `tactile --importer x.bttpreset` | Lise |

## Ce que Tactile fait déjà mieux (à garder)

MIT, 14 Mo et 0,1 % CPU au repos, config texte rechargée à chaud, erreurs avec numéro de ligne, groupes, reprenabilité documentée.

## Découpage phase 3

- **3a-1 Solidité** : A1, A3, A4, A5, A6, A7, A8 (même fichiers, un agent, sur la vraie barre).
- **3a-2 Personnalisation** : C1, C2, C4, C5, C6, C7, C8, B4 (alias anglais), B2 (doc).
- **3a-3 Barre par application** : C3.
- **3b Éditeur visuel** : B1 (fenêtre « Personnaliser ma barre »).
- **3c Open source** : A2, B3, B5, README anglais + README.fr.md.
- Reporté : C9 (import BTT, après la 0.2).
