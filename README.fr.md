<p align="center">
  <img src="assets/icone-1024.png" width="96" alt="Icône de Tactile">
</p>

<h1 align="center">Tactile</h1>

<p align="center">
  <strong>Une Touch Bar à ton goût, gratuite et open source, décrite dans un petit fichier texte ou réglée dans une fenêtre.</strong>
</p>

<p align="center">
  <a href="https://github.com/lepalph/tactile-touch-bar/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/lepalph/tactile-touch-bar/ci.yml?branch=main&label=CI" alt="État de la CI"></a>
  <a href="https://github.com/lepalph/tactile-touch-bar/releases/latest"><img src="https://img.shields.io/github/v/release/lepalph/tactile-touch-bar?label=release" alt="Dernière version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="Licence MIT"></a>
  <img src="https://img.shields.io/badge/macOS-13%2B%20%C2%B7%20Intel%20%26%20Apple%20Silicon-lightgrey" alt="macOS 13 ou plus récent, Intel et Apple Silicon">
</p>

<p align="center">🇬🇧 <a href="README.md">Read in English</a></p>

Tactile remplace la Touch Bar de ton MacBook Pro par la barre que tu décris : boutons, curseurs de luminosité et de volume, musique en cours, heure, commandes à toi. Tu passes de ta barre à celle d'Apple d'un toucher.

![La barre par défaut de Tactile](docs/barre.png)

![La fenêtre Personnaliser ma barre, avec la barre Musique](docs/editeur-musique.png)

## Pourquoi Tactile

Si tu connais [BetterTouchTool](https://folivora.ai) ou [MTMR](https://github.com/Toxblh/MTMR), tu connais l'idée. Tactile fait moins de choses :

- **Gratuit, sous licence MIT.** Pas de licence à renouveler, pas de compte, aucune donnée envoyée. BetterTouchTool est payant et fait aussi les gestes, les fenêtres et le reste ; Tactile ne fait que la Touch Bar.
- **Léger.** Rust natif, pas d'Electron, pas de moteur web dans la barre. Avec la barre par défaut, il consomme au repos de 0,0 à 0,2 % de CPU et 14 Mo de mémoire (mesuré avec `top` sur un M1 13 pouces, macOS 15.8). La version 0.2.0 se télécharge en un zip de 2 Mo.
- **Un fichier texte qui se versionne.** Ta barre est `~/.config/tactile/barre.toml`. Enregistre, la barre change dans la seconde. Une erreur ne casse rien : ta barre actuelle reste en place et l'erreur dit la ligne à corriger.
- **Ou une fenêtre, si tu préfères ne pas toucher au fichier.** « Personnaliser ma barre » montre ta Touch Bar à sa vraie taille, se règle à la souris et écrit le même fichier.
- **Des groupes qui marchent.** Un groupe ou un slider ouvre une vraie sous-barre avec un ✕ pour la fermer. MTMR passe par des popovers, qui ne s'ouvrent pas dans une barre système ([pourquoi](docs/DECISIONS.md)).

## Compatibilité

| | |
|---|---|
| Mac | MacBook Pro avec Touch Bar, Intel (2016 à 2020) ou Apple Silicon (M1 et M2, 13 pouces) |
| macOS | 13 Ventura ou plus récent |
| Vérifié sur | MacBook Pro M1 13 pouces, macOS 15.8. La version Intel a été testée sous Rosetta, pas encore sur un vrai Mac Intel : tes retours sont bienvenus. |

L'app est universelle : Apple Silicon et Intel dans le même `Tactile.app`.

## Installation

1. Télécharge `Tactile.zip` dans la [dernière version](https://github.com/lepalph/tactile-touch-bar/releases/latest) et décompresse-le.
2. Glisse `Tactile.app` dans **Applications**.
3. Ouvre-la une première fois à la main. L'app n'est pas notarisée par Apple, donc macOS bloque le premier lancement :
   - **macOS 15 Sequoia et plus récent :** double-clique Tactile. macOS dit qu'il « n'a pas pu vérifier » l'app ; clique **Terminé** (pas Placer dans la corbeille). Ouvre **Réglages Système → Confidentialité et sécurité**, descends tout en bas, clique **Ouvrir quand même** et confirme avec Touch ID ou ton mot de passe. C'est à faire une seule fois.
   - **macOS 13 et 14 :** clic droit sur `Tactile.app` → **Ouvrir**, puis **Ouvrir**.
   - **Ou, depuis le Terminal, sur toutes les versions :** `xattr -d com.apple.quarantine /Applications/Tactile.app`, puis ouvre l'app normalement.
4. Ta Touch Bar change, et une icône apparaît dans la barre des menus, en haut de l'écran.

Pourquoi pas de notarisation : elle demande un compte développeur Apple payant, que le projet n'a pas. L'app est signée ad hoc, et chaque nouvelle version donne le SHA-256 du zip.

Facultatif :

- Pour la musique en cours : `brew install media-control` (voir [pourquoi](#pourquoi-media-control)).
- Pour les touches simulées (actions `echap`, `touche`, `keycode`), autorise Tactile dans **Réglages Système → Confidentialité et sécurité → Accessibilité**.

Tactile ne demande rien d'autre au lancement. L'accès au micro n'est demandé que si tu installes l'[extra Claude vocal](#extra--claude-vocal).

## Prise en main

### Le menu

L'icône de la barre des menus donne accès à tout :

| Entrée | Effet |
|---|---|
| Personnaliser ma barre… | Ouvre la fenêtre de réglage (ci-dessous) : pas besoin de toucher au fichier. |
| Afficher ma barre | Remet ta barre (si elle a disparu, c'est ici). |
| Barre d'Apple | Rend la barre d'Apple ; l'icône ✳ du Control Strip ramène la tienne. |
| Ouvrir la configuration | Ouvre ton fichier de barre (il est créé s'il n'existe pas). |
| Voir le journal | Ouvre `~/Library/Logs/Tactile.log` : erreurs de config, scripts en échec. |
| Lancer au démarrage | Tactile s'ouvre avec ta session. |
| Quitter Tactile | Rend la barre d'Apple et ferme l'app. |

### Personnaliser ma barre

Dans le menu, choisis **Personnaliser ma barre…**. La fenêtre montre ta Touch Bar en haut, à sa vraie taille. Elle prend la langue du Mac ; FR / EN en haut à droite.

- À gauche, **Ajouter** : un clic ajoute un bouton, un curseur, l'heure, la musique en cours, le résultat d'une commande, la batterie, un espace ou un groupe. Les trois **barres toutes faites** (Simple, Musique, Développeur) remplacent ta barre par un modèle que tu peux ensuite retoucher.
- Dans l'aperçu, clique un élément pour le régler, glisse-le pour le déplacer. Au clavier : flèches pour passer d'un élément à l'autre, ⌥ + flèches pour le déplacer, suppr pour le retirer.
- À droite, les réglages de l'élément choisi : texte, symbole (cherche dans la liste, ou tape le nom exact d'un symbole SF), couleurs du fond et du texte, taille du texte, symbole seul ou texte seul, largeur, et ce qui se passe au toucher : lancer une application (le bouton **Choisir…** ouvre ton dossier Applications), ouvrir une adresse web, envoyer un raccourci clavier (`cmd+shift+4`), une commande shell ou AppleScript, ou une action système (luminosité, volume, lecture…). Un appui long peut faire autre chose. Ce que la fenêtre ne sait pas régler (une `image`, un `keycode`) est gardé tel quel à l'enregistrement.
- Un nom d'application tapé à la main est vérifié : s'il n'existe pas sur ton Mac, la fenêtre le dit sous le champ et propose le plus proche (« Photoshop » → « Adobe Photoshop 2025 »). **Choisir…** reste le plus sûr.
- **Enregistrer** écrit le fichier et ta Touch Bar change dans la seconde. Si quelque chose cloche (un symbole qui n'existe pas, une couleur mal écrite, une barre vide), rien n'est écrit et la fenêtre te dit quoi corriger. Tant que tu n'as pas enregistré, « Modifié » s'affiche à côté du titre, et la fenêtre te demande confirmation avant de fermer, de charger une barre toute faite ou de revenir en arrière.
- **Revenir à la version précédente** remonte d'un enregistrement à chaque clic. Les 5 dernières versions sont gardées à côté de ta barre (`barre.toml.precedent`, puis `.precedent-2` à `.precedent-5`) ; la version que tu quittes passe en dernier, donc rien n'est perdu.
- Si tu avais écrit ton fichier à la main avec des commentaires, le premier enregistrement depuis la fenêtre en garde une copie intacte, `barre.toml.manuel`, que la fenêtre ne remplace jamais.
- Si ton fichier contient une erreur, la fenêtre ne montre pas de barre vide : elle affiche la ligne en cause et désactive Enregistrer. Tu peux **ouvrir le fichier** pour le corriger, puis le **relire**, ou **repartir d'une barre toute faite** ; dans ce cas, ton fichier est d'abord copié en `barre.toml.abime-AAAAMMJJ-HHMMSS`.

Pour ouvrir la fenêtre seule, sans prendre la Touch Bar : `tactile --personnaliser`.

### Modifier le fichier à la main

La fenêtre écrit `~/.config/tactile/barre.toml`, avec un bref en-tête : tu peux passer de l'une à l'autre quand tu veux. Dans le menu, choisis **Ouvrir la configuration** ; chaque bloc `[[element]]` (ou `[[item]]`, en anglais) est un élément de la barre, de gauche à droite ; les deux se mélangent dans un même fichier et gardent l'ordre du fichier. Enregistre : la barre change dans la seconde.

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

Les symboles sont ceux d'Apple ([SF Symbols](https://developer.apple.com/sf-symbols/)) : `moon.fill`, `play.fill`, `bolt.fill`… Un symbole mal orthographié s'affiche en texte.

Si le fichier contient une erreur, ta barre actuelle reste en place, un élément rouge « ⚠ config ligne N » apparaît en tête de barre jusqu'à la correction, et l'erreur s'écrit dans le journal (menu → Voir le journal). Pour vérifier un fichier sans toucher la barre : `tactile --verifier` (ou `tactile --verifier autre.toml`). Au-delà de la syntaxe, il prévient de ce qui casserait à l'usage : symbole SF inconnu, couleur qui n'est pas en `#RRGGBB`, image ou app introuvable, barre qui déborde.

## Référence de la configuration

Les clés françaises sont la forme de référence, celle qu'écrit l'éditeur et qu'emploient les messages d'erreur. Chaque clé, type, réglage et action a aussi un alias anglais, et les deux formes peuvent se mélanger dans un fichier : voir le [glossaire](#glossaire--les-clés-en-anglais).

### Réglage général

`masquer_control_strip`, en tête de fichier, vaut `true` par défaut. `true` donne toute la largeur à ta barre (1004 points) ; `false` garde le Control Strip d'Apple à droite (il reste 685 points, et une ✕ système à gauche).

Un bouton, l'heure ou le titre en cours sans `largeur` ne dépasse pas 200 points (le texte est coupé avec …). Ce qui ne tient pas dans la barre n'est pas affiché : le journal le dit (« la barre principale déborde … n éléments seront cachés »).

### Les éléments

| `type` | Champs | Ce que ça affiche |
|---|---|---|
| `bouton` | `texte`, `symbole` (nom d'un symbole SF) ou `image` (chemin d'un PNG, PDF, icns… ; `~` accepté ; `gabarit = true` pour une image monochrome teintée comme un symbole), `couleur` (fond, `"#RRGGBB"`), `couleur_texte`, `taille` (police, en points), `affichage` (`"icone"`, `"texte"` ou `"les_deux"`, défaut), `largeur` (points), `action`, `appui_long` (action, après 0,5 s) | Un bouton. Tous les champs sont facultatifs. Une image introuvable s'affiche en texte, comme un symbole inconnu. |
| `slider` | `reglage` (obligatoire : `"luminosite"` ou `"volume"`), `symbole` | Un bouton qui ouvre une sous-barre avec un curseur ; ✕ la referme. |
| `heure` | `format` (format de date Apple, `"HH:mm"` par défaut) | L'heure, mise à jour en continu. |
| `en_cours` | `largeur` | Le titre et l'artiste en cours de lecture (44 caractères au plus, rafraîchi toutes les 2 s) ; un appui met en pause ou relance. Demande `media-control`. |
| `script` | `commande` (obligatoire), `intervalle` (secondes, obligatoire ; en dessous de 0,1, ramené à 0,1), `delai` (secondes, 5 par défaut), `largeur` (120 par défaut), `action` | La sortie de la commande, lancée avec `zsh -c` à chaque intervalle et tuée au-delà de `delai`. Un texte plus large que l'élément est coupé. Code de retour non nul ou texte sur stderr : une ligne dans le journal. Si la sortie commence par `{`, elle est lue comme du JSON : `{"texte": "12°", "symbole": "cloud.rain.fill", "image": "~/pluie.png", "couleur": "#1F3A5F", "couleur_texte": "#FFFFFF"}`, chaque clé facultative ; le script règle ainsi son icône et ses couleurs. |
| `batterie` | `symbole` (`true` par défaut : une pile à côté du pourcentage, avec un éclair en charge ; `false` : texte seul, `⚡ 85 %` en charge), `largeur` (96 par défaut) | Le niveau de la batterie, relu toutes les 30 s (`pmset -g batt`). |
| `espace` | `largeur` (obligatoire) | Un vide de largeur fixe. |
| `souple` | aucun | Un vide qui prend toute la place libre : deux `souple` centrent ce qui est entre eux. |
| `groupe` | `texte`, `symbole`, `largeur` (72 par défaut), puis des sous-éléments `[[element.element]]` | Un bouton qui ouvre une sous-barre avec ses éléments ; ✕ la referme. |
| `lanceur` | `app` (obligatoire : le nom du dossier `.app`, un bundle id ou un chemin, comme l'action `app`), `texte`, `affichage` (comme un bouton), `largeur`, `appui_long` | L'icône de l'app. Un appui l'ouvre ou la met au premier plan ; un petit point sous l'icône dit qu'elle est ouverte, comme dans le Dock. Une app introuvable affiche son nom en texte, et `tactile --verifier` le signale. |

#### Un tiroir d'apps

Des lanceurs dans un groupe font un tiroir d'apps : une icône de la barre principale ouvre une sous-barre d'apps, ✕ la referme. Exemple complet : [exemples/lanceur.toml](exemples/lanceur.toml).

```toml
[[element]]
type = "lanceur"
app = "Safari"

[[element]]
type = "groupe"
symbole = "square.grid.2x2"

  [[element.element]]
  type = "lanceur"
  app = "Calendar"

  [[element.element]]
  type = "lanceur"
  app = "com.apple.systempreferences"
  texte = "Réglages"
  appui_long = "launchpad"
```

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
| `{ touche = "cmd+shift+4" }` | Simule un raccourci clavier (Accessibilité). Modificateurs : `cmd`, `shift`, `ctrl`, `alt` (ou `option`), `fn`. Touche : une lettre, un chiffre ou un signe du clavier (résolu sur ta disposition, AZERTY compris), `f1` à `f20`, `esc`, `tab`, `espace`, `entree`, `suppr`, `gauche`, `droite`, `haut`, `bas`, `debut`, `fin`, `pageup`, `pagedown` (noms anglais acceptés : `space`, `enter`, `delete`, `left`…). |
| `{ keycode = 53 }` | Simule une touche par son code macOS, pour celles qui n'ont pas de nom. |
| `{ app = "Figma" }` | Ouvre une app par son nom ou son identifiant (`com.figma.Desktop`), comme `open -a`. Voir [trouver le nom d'une app](#trouver-le-nom-dune-app). |
| `{ url = "https://…" }` | Ouvre l'adresse dans le navigateur par défaut (ou un fichier, un dossier). |
| `{ applescript = 'tell application "Music" to playpause' }` | Lance l'AppleScript avec `osascript`. Avec des apostrophes autour, les guillemets du script restent lisibles. Son erreur éventuelle va au journal. |
| `"barre_apple"` | Rend la barre d'Apple ; l'icône ✳ du Control Strip ramène celle de Tactile. |
| `"mission_control"`, `"fenetres_app"`, `"bureau"` | Mission Control, les fenêtres de l'app au premier plan (App Exposé) ou le bureau, par l'app Mission Control d'Apple. |
| `"launchpad"` | Ouvre le Launchpad. |
| `"capture_options"` | La palette de capture et d'enregistrement de l'écran (⌘⇧5), par l'app Capture d'écran d'Apple. |
| `"spotlight"`, `"capture_ecran"`, `"capture_zone"`, `"emojis"`, `"verrouiller"`, `"forcer_quitter"` | Spotlight (⌘Espace), capture de tout l'écran (⌘⇧3) ou d'une zone (⌘⇧4), la palette des émojis et symboles (⌃⌘Espace), verrouiller l'écran (⌃⌘Q), la fenêtre « Forcer à quitter » (⌥⌘⎋). Envoie le raccourci par défaut de macOS (Accessibilité) ; si tu l'as changé dans Réglages Système, écris le tien avec `{ touche = "…" }`. |
| `{ shell = "commande" }` | Lance la commande avec `zsh -c`, sans attendre sa fin. `~` est accepté. |

#### Trouver le nom d'une app

`app` prend le nom du dossier `.app` dans Applications, sans l'extension, en anglais quand l'app en a un : `"TextEdit"`, `"Activity Monitor"`, `"DaVinci Resolve"`. Pour le lire : clic droit sur l'app dans le Finder → Lire les informations ; le nom en haut est celui à écrire. Si deux apps portent le même nom, l'identifiant est plus sûr : `osascript -e 'id of app "Figma"'` l'affiche (`com.figma.Desktop`). Une app introuvable est signalée par `tactile --verifier` et dans le journal (menu → Voir le journal). Un bundle id marche aussi : il est ouvert avec `open -b`.

### Glossaire : les clés en anglais

Le fichier s'écrit en français (c'est la forme de référence, celle de l'éditeur), mais chaque clé, type, réglage et action a un alias anglais, pour partager une barre avec qui ne lit pas le français. Les deux formes peuvent se mélanger, `[[element]]` et `[[item]]` compris : les éléments gardent l'ordre dans lequel ils sont écrits. Les messages d'erreur restent en français.

| Français | Anglais |
|---|---|
| `masquer_control_strip` | `hide_control_strip` |
| `element` | `item` |
| `bouton` | `button` |
| `heure` | `clock` |
| `en_cours` | `now_playing` |
| `espace` | `space` |
| `souple` | `flexible` |
| `groupe` | `group` |
| `batterie` | `battery` |
| `texte` | `text` |
| `symbole` | `symbol` |
| `gabarit` | `template` |
| `couleur` | `color` |
| `couleur_texte` | `text_color` |
| `taille` | `size` |
| `affichage` | `display` |
| `icone` | `icon` |
| `les_deux` | `both` |
| `largeur` | `width` |
| `appui_long` | `long_press` |
| `reglage` | `setting` |
| `luminosite` | `brightness` |
| `commande` | `command` |
| `intervalle` | `interval` |
| `delai` | `timeout` |
| `luminosite_plus` | `brightness_up` |
| `luminosite_moins` | `brightness_down` |
| `volume_plus` | `volume_up` |
| `volume_moins` | `volume_down` |
| `basculer_muet` | `toggle_mute` |
| `lecture_pause` | `play_pause` |
| `suivant` | `next` |
| `precedent` | `previous` |
| `veille` | `sleep` |
| `echap` | `esc` |
| `barre_apple` | `apple_bar` |
| `touche` | `key` |
| `slider` | `slider` |
| `script` | `script` |
| `format` | `format` |
| `image` | `image` |
| `action` | `action` |
| `volume` | `volume` |
| `shell` | `shell` |
| `keycode` | `keycode` |
| `app` | `app` |
| `url` | `url` |
| `applescript` | `applescript` |
| `herite` | `inherit` |
| `lanceur` | `launcher` |
| `mission_control` | `mission_control` |
| `fenetres_app` | `app_windows` |
| `launchpad` | `launchpad` |
| `bureau` | `show_desktop` |
| `spotlight` | `spotlight` |
| `capture_ecran` | `screenshot` |
| `capture_zone` | `screenshot_area` |
| `capture_options` | `screenshot_options` |
| `verrouiller` | `lock_screen` |
| `emojis` | `emoji` |
| `forcer_quitter` | `force_quit` |

Dans la sortie JSON d'un `script` aussi : `text`, `symbol`, `color`, `text_color`.

### Un exemple complet en anglais

[exemples/barre.en.toml](exemples/barre.en.toml) est la barre par défaut écrite avec les clés anglaises ; [exemples/barre.toml](exemples/barre.toml), la même en français. Les autres exemples de [exemples/](exemples/) : [simple.toml](exemples/simple.toml) et [developpeur.toml](exemples/developpeur.toml), les barres Simple et Développeur de l'éditeur ; [lanceur.toml](exemples/lanceur.toml), des lanceurs, un tiroir d'apps et des raccourcis système. Pour partir de l'un d'eux, copie-le vers `~/.config/tactile/barre.toml`.

## Une barre par application

Une table `[[application]]` donne sa propre barre à une app : quand elle passe au premier plan, sa barre remplace la principale ; quand tu la quittes, la principale revient. Les `[[element]]` du haut du fichier restent la barre principale.

```toml
[[application]]
app = "com.apple.Safari"        # bundle id, ou nom affiché ("Safari")
herite = true                   # défaut false

[[application.element]]
type = "bouton"
texte = "Nouvel onglet"
action = { touche = "cmd+t" }
```

- `app` : l'identifiant de l'app (bundle id) ou son nom affiché, majuscules et minuscules confondues. L'identifiant est le plus sûr ; pour le trouver : `osascript -e 'id of app "Safari"'` (affiche `com.apple.Safari`).
- `herite = false` (défaut) : les éléments de l'app remplacent toute la barre. `herite = true` : ils remplacent seulement la zone entre le premier et le dernier `souple` de la barre principale ; ce qui est avant et après (luminosité, heure…) reste. Sans `souple` dans la principale, ils s'ajoutent à sa suite.
- Chaque `[[application.element]]` s'écrit comme un `[[element]]` ; un groupe y prend ses sous-éléments en `[[application.element.element]]`. Une `[[application]]` sans `app` ou sans élément est refusée, avec sa ligne. Un `[[element]]` écrit après une `[[application]]` va quand même dans la barre principale : `tactile --verifier` le signale, car tu voulais sans doute `[[application.element]]`. Il signale aussi deux `[[application]]` pour la même app (seule la première compte) et une barre d'app qui déborde.
- Une sous-barre ouverte (curseur, groupe) reste en place quand tu changes d'app ; son ✕ ramène la barre de l'app au premier plan. L'icône ✳ ramène aussi la barre de l'app en cours.

Exemple complet : [exemples/par-application.toml](exemples/par-application.toml). La fenêtre « Personnaliser ma barre » ne règle que la barre principale, mais garde tes `[[application]]` à l'enregistrement.

## Extra : Claude vocal

Un bouton ✳ Claude : tu parles, Claude Code te répond à voix haute. La transcription (whisper.cpp) et la voix (Piper) tournent sur ton Mac ; seule la réponse passe par Claude Code et ton compte Claude. La première fois, macOS demande l'accès au micro pour Tactile. Installation : [extras/claude-voix](extras/claude-voix/README.md) ; éléments à ajouter à ta barre : [exemples/claude-vocal.toml](exemples/claude-vocal.toml).

## Ligne de commande

Le binaire de l'app est `/Applications/Tactile.app/Contents/MacOS/tactile`.

| Commande | Effet |
|---|---|
| `tactile` | Lance la barre (icône dans la barre des menus). |
| `tactile --personnaliser` | Ouvre la fenêtre « Personnaliser ma barre » seule, sans prendre la Touch Bar. Alias anglais : `--customize`. |
| `tactile --verifier [fichier]` | Vérifie un fichier de barre sans toucher la Touch Bar (voir plus bas). Alias anglais : `--check`. Sans fichier, vérifie ta `barre.toml`. |
| `tactile --version` | Affiche la version. |
| `tactile --help` | Affiche l'aide, en français si la langue du système l'est (`LC_ALL`, `LANG`, puis la langue de macOS), sinon en anglais. |

`tactile --verifier` lit le fichier comme la barre, puis cherche ce qui casserait à l'usage. Une erreur de syntaxe affiche le fichier, la ligne et la raison, et sort avec le code 1. Un problème que la barre supporterait affiche une ligne qui commence par `attention :` et sort avec le code 0 : symbole SF inconnu, couleur qui n'est pas en `#RRGGBB`, image introuvable, app non installée, barre (principale ou d'une app) qui déborde, deux `[[application]]` pour la même app, `[[element]]` après une `[[application]]`. Une barre vide est une erreur. Si tout va bien : `OK : n éléments`, et le nombre de barres d'application. La barre fait les mêmes contrôles quand elle charge le fichier et écrit les avertissements au journal.

```
$ tactile --verifier barre.toml
attention : ligne 4 : la couleur « #F00 » doit s'écrire #RRGGBB : elle sera ignorée
attention : ligne 11 : cet [[element]] est écrit après un [[application]] : il va dans la barre principale ; voulais-tu [[application.element]] ?
OK : 2 éléments, 1 barre d'application
```

Un argument inconnu affiche l'aide et sort avec le code 2. `TACTILE_CONFIG=/chemin/fichier.toml` remplace `~/.config/tactile/barre.toml` partout (barre, surveillance du fichier, éditeur) : pratique pour essayer sans toucher à ta vraie barre. Les autres variables `TACTILE_*` servent au développement : voir [CONTRIBUTING.fr.md](CONTRIBUTING.fr.md).

## Dépannage

Les erreurs s'écrivent dans `~/Library/Logs/Tactile.log` (menu → **Voir le journal**).

| Problème | Solution |
|---|---|
| macOS refuse d'ouvrir Tactile | Voir l'étape 3 de l'[installation](#installation) : sur macOS 15, **Réglages Système → Confidentialité et sécurité → Ouvrir quand même** ; ou `xattr -d com.apple.quarantine /Applications/Tactile.app`. |
| Ma barre a disparu, je vois celle d'Apple | Touche l'icône ✳ dans la barre d'Apple, ou menu → Afficher ma barre. |
| Ma modification n'apparaît pas | Il y a une erreur dans le fichier : un élément rouge « ⚠ config ligne N » le dit dans la barre, et menu → Voir le journal donne le détail. Ouvre **Personnaliser ma barre…** et enregistre : l'erreur s'affiche dans la fenêtre. `tactile --verifier` vérifie le fichier depuis un terminal. |
| Un élément de la fin a disparu | La barre déborde : `tactile --verifier` et le journal disent quelle barre et combien d'éléments sont cachés. Réduis des `largeur` ou retire un élément. |
| Un élément est parti dans la barre principale au lieu de celle d'une app | Il est écrit `[[element]]` après une `[[application]]` : écris `[[application.element]]`. `tactile --verifier` donne sa ligne. |
| `attention : …` dans le journal ou après `--verifier` | La barre marche, mais quelque chose ne s'affichera pas comme écrit : symbole SF inconnu, couleur hors `#RRGGBB` (`#F00` et `red` sont refusés), image ou app introuvable, app décrite deux fois. La ligne est donnée. |
| Un `script` reste vide | Son erreur est dans le journal (code de retour, stderr, « tué après 5 s »). Lancé depuis le Finder, le PATH ne contient pas `/opt/homebrew/bin` : écris le chemin complet des outils. |
| La musique en cours reste vide | Installe `media-control` (`brew install media-control`) ; il faut aussi que quelque chose joue. |
| La touche esc ou un raccourci ne fait rien | Autorise Tactile dans **Réglages Système → Confidentialité et sécurité → Accessibilité**. |
| `{ app = "…" }` n'ouvre rien | Le nom est celui du dossier `.app` ([trouver le nom d'une app](#trouver-le-nom-dune-app)) ; le journal dit « Unable to find application ». |

![Une erreur affichée dans la fenêtre de réglage](docs/editeur-erreur.png)

### Pourquoi media-control

Depuis macOS 15.4, Apple empêche les apps de lire ce qui joue (le framework MediaRemote répond sans rien faire). [media-control](https://github.com/ungive/media-control) passe par un programme qu'Apple autorise encore. Sans lui, le titre reste vide et ⏪ ⏯ ⏩ ne font rien ; le reste de la barre marche.

## Désinstaller

1. Menu de Tactile → décoche **Lancer au démarrage**, puis **Quitter Tactile**.
2. Mets `Tactile.app` à la corbeille.
3. Supprime ta config et le journal : `rm -r ~/.config/tactile ~/Library/Logs/Tactile.log`.

## Limites connues

- Tactile utilise des API privées d'Apple (barre système, Control Strip, luminosité). Une mise à jour de macOS peut les casser ; elles sont listées dans [ARCHITECTURE.md](docs/ARCHITECTURE.md#api-privées-utilisées).
- Le curseur de luminosité règle l'écran intégré seulement.
- Le menu et les messages d'erreur sont en français ; `--help` et la fenêtre de réglage parlent les deux langues.
- Pas de notarisation (voir l'[installation](#installation)).
- Gestes sur la Touch Bar et gestes trackpad viendront ensuite (voir [SPEC.md](SPEC.md)).

## Contribuer

Les contributions sont bienvenues, en français ou en anglais. Le code est fait pour être repris :

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) : où est quoi, comment ajouter une action ou un élément, les API privées utilisées et les pièges déjà rencontrés.
- [docs/DECISIONS.md](docs/DECISIONS.md) : pourquoi les choix qui surprennent ont été faits.
- [CONTRIBUTING.fr.md](CONTRIBUTING.fr.md) : style, tests, captures, commits ([version anglaise](CONTRIBUTING.md)).

Compiler depuis les sources (Rust stable ; le dépôt fournit un `mise.toml`) :

```
cargo build --release      # target/release/tactile, pour essayer (--help, --version, --verifier)
CI=1 cargo test            # sans CI=1, les tests règlent puis restaurent vraiment luminosité et volume
scripts/empaqueter.sh      # target/Tactile.app universelle et target/Tactile.zip
scripts/verifier.sh        # actionne chaque contrôle sur la vraie Touch Bar et vérifie l'effet (écran allumé)
```

Les changements sont listés dans [CHANGELOG.md](CHANGELOG.md) (en anglais).

## Remerciements

- [MTMR](https://github.com/Toxblh/MTMR) (licence MIT), dont l'approche des API privées de la Touch Bar a servi de référence.
- [media-control](https://github.com/ungive/media-control), qui fait de nouveau marcher la musique en cours sur les macOS récents.

## Licence

[MIT](LICENSE). Fais-en ce que tu veux.
