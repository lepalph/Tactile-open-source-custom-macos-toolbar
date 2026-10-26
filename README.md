# Tactile

**Une Touch Bar à ton goût, gratuite et open source.** Tactile remplace la Touch Bar de ton MacBook Pro par la barre que tu décris : boutons, curseurs de luminosité et de volume, musique en cours, heure, commandes à toi. Tu passes de ta barre à celle d'Apple d'un toucher.

![La barre par défaut de Tactile](docs/barre.png)

Au repos, Tactile consomme 0,5 % de CPU et 13 Mo de mémoire (mesuré). L'app est écrite en Rust et tourne nativement sur les Mac Intel comme Apple Silicon. Elle est sous licence MIT, sans compte, et n'envoie aucune donnée.

## Compatibilité

| | |
|---|---|
| Mac | MacBook Pro avec Touch Bar, Intel (2016 à 2020) ou Apple Silicon (M1 et M2, 13 pouces) |
| macOS | 13 Ventura ou plus récent |
| Vérifié sur | MacBook Pro M1 13 pouces, macOS 15.8. La version Intel a été testée sous Rosetta, pas encore sur un vrai Mac Intel : tes retours sont bienvenus. |

## Installation

1. Télécharge `Tactile.zip` dans les [Releases](../../releases/latest) et décompresse-le.
2. Glisse `Tactile.app` dans **Applications**.
3. Premier lancement : **clic droit sur Tactile.app → Ouvrir**, puis **Ouvrir**. L'app n'est pas notarisée par Apple ; un double-clic seul est refusé la première fois.
4. Ta Touch Bar change, et une icône apparaît dans la barre des menus, en haut de l'écran.

Facultatif :

- Pour la musique en cours : `brew install media-control` (voir [pourquoi](#pourquoi-media-control)).
- Pour la touche esc simulée (action `echap`), autorise Tactile dans Réglages Système → Confidentialité et sécurité → Accessibilité.

## Le menu de Tactile

L'icône de la barre des menus donne accès à tout :

| Entrée | Effet |
|---|---|
| Afficher ma barre | Remet ta barre (si elle a disparu, c'est ici). |
| Barre d'Apple | Rend la barre d'Apple ; l'icône ✳ du Control Strip ramène la tienne. |
| Ouvrir la configuration | Ouvre ton fichier de barre (il est créé s'il n'existe pas). |
| Voir le journal | Ouvre `~/Library/Logs/Tactile.log` : erreurs de config, scripts en échec. |
| Lancer au démarrage | Tactile s'ouvre avec ta session. |
| Quitter Tactile | Rend la barre d'Apple et ferme l'app. |

## Personnaliser ta barre

Dans le menu, choisis **Ouvrir la configuration**. Le fichier `~/.config/tactile/barre.toml` s'ouvre ; chaque bloc `[[element]]` est un élément de la barre, de gauche à droite. Enregistre : la barre change dans la seconde.

Par exemple, pour ajouter un bouton qui met le Mac en veille, et ouvre le Moniteur d'activité si tu le tiens appuyé :

```toml
[[element]]
type = "bouton"
symbole = "moon.fill"
texte = "Veille"
couleur = "#3A3A3C"
action = "veille"
appui_long = { shell = "open -a 'Activity Monitor'" }
```

Les symboles sont ceux d'Apple ([SF Symbols](https://developer.apple.com/sf-symbols/)) : `moon.fill`, `play.fill`, `bolt.fill`… Un symbole mal orthographié s'affiche en texte. Si le fichier contient une erreur, ta barre actuelle reste en place, un élément rouge « ⚠ config ligne N » apparaît en tête de barre jusqu'à la correction, et l'erreur s'écrit dans le journal (menu → Voir le journal). Pour vérifier un fichier sans toucher la barre : `tactile --verifier` (ou `tactile --verifier autre.toml`).

### Les éléments

| `type` | Champs | Ce que ça affiche |
|---|---|---|
| `bouton` | `texte`, `symbole` (nom d'un symbole SF), `couleur` (`"#RRGGBB"`), `largeur` (points), `action`, `appui_long` (action, après 0,5 s) | Un bouton. Tous les champs sont facultatifs. |
| `slider` | `reglage` (`"luminosite"` ou `"volume"`), `symbole` | Un bouton qui ouvre une sous-barre avec un curseur ; ✕ la referme. |
| `heure` | `format` (format de date Apple, `"HH:mm"` par défaut) | L'heure, mise à jour en continu. |
| `en_cours` | `largeur` | Le titre et l'artiste en cours de lecture (44 caractères au plus, rafraîchi toutes les 2 s) ; un appui met en pause ou relance. Demande `media-control`. |
| `script` | `commande` (obligatoire), `intervalle` (secondes, obligatoire, 0,1 au minimum), `delai` (secondes, 5 par défaut), `largeur` (120 par défaut), `action` | La sortie de la commande, lancée avec `zsh -c` à chaque intervalle et tuée au-delà de `delai`. Un texte plus large que l’élément est coupé. Code de retour non nul ou texte sur stderr : une ligne dans le journal. |
| `espace` | `largeur` (obligatoire) | Un vide de largeur fixe. |
| `souple` | aucun | Un vide qui prend toute la place libre : deux `souple` centrent ce qui est entre eux. |
| `groupe` | `texte`, `symbole`, puis des sous-éléments `[[element.element]]` | Un bouton qui ouvre une sous-barre avec ses éléments ; ✕ la referme. |

En tête de fichier, `masquer_control_strip = true` (par défaut) donne toute la largeur à ta barre (1004 points) ; `false` garde le Control Strip d'Apple à droite (il reste 685 points, et une ✕ système à gauche). Un bouton, l'heure ou le titre en cours sans `largeur` ne dépasse pas 200 points (le texte est coupé avec …). Ce qui ne tient pas dans la barre n'est pas affiché : le journal le dit (« la barre déborde … n élément(s) seront cachés »).

### Les actions

Une action s'écrit soit comme un mot (`action = "suivant"`), soit comme une table (`action = { volume = 0.3 }`).

| Action | Effet |
|---|---|
| `"luminosite_plus"`, `"luminosite_moins"` | Luminosité de l'écran intégré, d'un cran (1/16). |
| `{ luminosite = 0.5 }` | Luminosité fixée, de 0 à 1. |
| `"volume_plus"`, `"volume_moins"` | Volume de la sortie audio, d'un cran (1/16). |
| `{ volume = 0.3 }` | Volume fixé, de 0 à 1. |
| `"basculer_muet"` | Coupe ou rétablit le son. |
| `"lecture_pause"`, `"suivant"`, `"precedent"` | Commande la lecture en cours, quelle que soit l'app (demande `media-control`). |
| `"veille"` | Met le Mac en veille (`pmset sleepnow`). |
| `"echap"` | Simule la touche esc (demande l'autorisation Accessibilité). |
| `"barre_apple"` | Rend la barre d'Apple ; l'icône ✳ du Control Strip ramène celle de Tactile. |
| `{ shell = "commande" }` | Lance la commande avec `zsh -c`, sans attendre sa fin. `~` est accepté. |

### Extra : Claude vocal

Un bouton ✳ Claude : tu parles, Claude Code te répond à voix haute. La transcription (whisper.cpp) et la voix (Piper) tournent sur ton Mac. Installation : [extras/claude-voix](extras/claude-voix/README.md) ; éléments à ajouter à ta barre : [exemples/claude-vocal.toml](exemples/claude-vocal.toml).

## Dépannage

| Problème | Solution |
|---|---|
| macOS refuse d'ouvrir Tactile | Clic droit sur l'app → Ouvrir, une fois. |
| Ma barre a disparu, je vois celle d'Apple | Touche l'icône ✳ dans la barre d'Apple, ou menu → Afficher ma barre. |
| Ma modification n'apparaît pas | Il y a une erreur dans le fichier : un élément rouge « ⚠ config ligne N » le dit dans la barre, et menu → Voir le journal donne le détail. `tactile --verifier` vérifie le fichier depuis un terminal. |
| Un élément de la fin a disparu | La barre déborde : le journal dit combien d'éléments sont cachés. Réduis des `largeur` ou retire un élément. |
| Un `script` reste vide | Son erreur est dans le journal (code de retour, stderr, « tué après 5 s »). Lancé depuis le Finder, le PATH ne contient pas `/opt/homebrew/bin` : écris le chemin complet des outils. |
| La musique en cours reste vide | Installe `media-control` (`brew install media-control`) ; il faut aussi que quelque chose joue. |
| La touche esc ne fait rien | Autorise Tactile dans Réglages Système → Confidentialité et sécurité → Accessibilité. |

### Pourquoi media-control

Depuis macOS 15.4, Apple empêche les apps de lire ce qui joue (le framework MediaRemote répond sans rien faire). [media-control](https://github.com/ungive/media-control) passe par un programme qu'Apple autorise encore. Sans lui, le titre reste vide et ⏪ ⏯ ⏩ ne font rien ; le reste de la barre marche.

## Désinstaller

1. Menu de Tactile → décoche **Lancer au démarrage**, puis **Quitter Tactile**.
2. Mets `Tactile.app` à la corbeille.
3. Supprime ta config et le journal : `rm -r ~/.config/tactile ~/Library/Logs/Tactile.log`.

## Contribuer

Le code est fait pour être repris :

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) : où est quoi, comment ajouter une action ou un élément en quatre étapes, les API privées utilisées et les pièges déjà rencontrés.
- [docs/DECISIONS.md](docs/DECISIONS.md) : pourquoi les choix qui surprennent ont été faits.
- [CONTRIBUTING.md](CONTRIBUTING.md) : style, tests, captures, commits.

Compiler depuis les sources (Rust stable ; le dépôt fournit un `mise.toml`) :

```
cargo build --release      # target/release/tactile, pour essayer (--help, --version, --verifier)
cargo test                 # attention : règle puis restaure vraiment luminosité et volume
scripts/empaqueter.sh      # target/Tactile.app universelle et target/Tactile.zip
scripts/verifier.sh        # actionne chaque contrôle sur la vraie Touch Bar et vérifie l'effet (écran allumé)
```

## Limites connues

- Tactile utilise des API privées d'Apple (barre système, Control Strip, luminosité). Une mise à jour de macOS peut les casser ; elles sont listées dans [ARCHITECTURE.md](docs/ARCHITECTURE.md#api-privées-utilisées).
- Pas encore d'éditeur visuel : la barre se règle dans le fichier TOML.
- Le curseur de luminosité règle l'écran intégré seulement.
- Gestes sur la Touch Bar, raccourcis clavier et gestes trackpad viendront ensuite (voir [SPEC.md](SPEC.md)).

## Remerciements

[MTMR](https://github.com/Toxblh/MTMR) (licence MIT), dont l'approche des API privées de la Touch Bar a servi de référence.

## Licence

[MIT](LICENSE). Fais-en ce que tu veux.

---

## English

**Your own Touch Bar, free and open source.** Tactile replaces your MacBook Pro Touch Bar with the bar you describe: buttons, brightness and volume sliders, now playing, a clock, your own commands. One tap switches between your bar and Apple's.

At rest it uses 0.5 % CPU and 13 MB of memory (measured). It is written in Rust and runs natively on Intel and Apple Silicon Macs. MIT licensed, no account, no data sent anywhere.

**Compatibility.** MacBook Pro with a Touch Bar (Intel 2016 to 2020, or Apple Silicon), macOS 13 or later. Tested on an M1 13-inch running macOS 15.8; the Intel build was tested under Rosetta only.

**Install.** Download `Tactile.zip` from [Releases](../../releases/latest), move `Tactile.app` to Applications, and open it the first time with right-click → Open (the app is not notarized). For now playing, `brew install media-control`: since macOS 15.4, Apple no longer lets apps read what is playing.

**Menu.** The menu bar icon offers: show my bar, Apple's bar, open the configuration, launch at login, quit.

**Customize.** Menu → *Ouvrir la configuration* opens `~/.config/tactile/barre.toml`; save it and the bar updates within a second. Keys are French: `bouton` = button, `texte` = text, `symbole` = SF Symbol, `couleur` = colour, `appui_long` = long press, `heure` = clock, `en_cours` = now playing, `script` = command output, `espace` = fixed space, `souple` = flexible space, `groupe` = group. Errors are written to `~/Library/Logs/Tactile.log`.

**Contribute.** Start with [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) (code map, step-by-step recipes, private APIs, known pitfalls) and [docs/DECISIONS.md](docs/DECISIONS.md). Build with `cargo build --release`; `scripts/empaqueter.sh` produces the universal app.

**Limits.** Relies on private Apple APIs that a macOS update may break; no visual editor yet; not notarized.

Thanks to [MTMR](https://github.com/Toxblh/MTMR) (MIT). MIT licensed.
