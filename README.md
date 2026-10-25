# Tactile

Tactile remplace la Touch Bar des MacBook Pro par une barre que tu décris dans un fichier TOML. C'est une app native Apple Silicon écrite en Rust, sans icône dans le Dock, légère au repos (mesuré : 0,5 % de CPU et 13 Mo de RAM).

![La barre par défaut : , luminosité, volume, ⏪, titre en cours, ⏩, heure](docs/barre.png)

Ce qu'elle sait faire aujourd'hui :

- boutons avec texte, symbole SF et couleur, avec une action à l'appui et une autre à l'appui long ;
- curseurs de luminosité et de volume, qui s'ouvrent en sous-barre (✕ pour revenir) ;
- titre en cours de lecture, quelle que soit l'app (Spotify, Musique, Chrome…), avec ⏪ ⏯ ⏩ ;
- heure, widget de script (sortie d'une commande, rafraîchie à intervalle régulier), groupes qui s'ouvrent en sous-barre ;
- aller-retour avec la barre d'Apple : le bouton  rend la barre d'Apple (avec les boutons propres à chaque app), l'icône ✳ ajoutée au Control Strip ramène celle de Tactile ;
- rechargement à chaud : le fichier de config est relu dès qu'il est enregistré.

Tactile ne couvre que la Touch Bar. BetterTouchTool en fait beaucoup plus (gestes, fenêtres, raccourcis…) ; Tactile est gratuit et open source, et ses autres modules viendront un par un (voir [SPEC.md](SPEC.md)).

## Installation

1. Télécharge `Tactile.zip` depuis la page [Releases](../../releases/latest) et décompresse-le.
2. Glisse `Tactile.app` dans le dossier Applications.
3. Premier lancement : clic droit sur `Tactile.app` → Ouvrir, puis Ouvrir dans la fenêtre d'avertissement. L'app n'est pas notarisée par Apple, un double-clic seul est refusé la première fois.
4. Autorise Tactile dans Réglages Système → Confidentialité et sécurité → Accessibilité. C'est nécessaire pour l'action `echap`, qui simule une frappe clavier.

Pour la lancer à l'ouverture de session : Réglages Système → Général → Ouverture, ajoute `Tactile.app`.

### Titre en cours : `media-control` (optionnel)

```
brew install media-control
```

Depuis macOS 15.4, le framework privé MediaRemote refuse aux apps non autorisées de lire ce qui joue, et ses commandes répondent « vrai » sans rien faire. [media-control](https://github.com/ungive/media-control) passe par un binaire qu'Apple autorise encore. Tactile l'appelle à `/opt/homebrew/bin/media-control` pour l'élément `en_cours` et pour les actions `lecture_pause`, `suivant` et `precedent`. Sans lui, le titre reste vide et ces boutons ne font rien ; le reste de la barre marche.

## Configuration

La barre se décrit dans `~/.config/tactile/barre.toml`. Tant que ce fichier n'existe pas, Tactile affiche la barre par défaut, [exemples/barre.toml](exemples/barre.toml) : copie-la à cet endroit pour partir d'elle.

```
mkdir -p ~/.config/tactile
cp exemples/barre.toml ~/.config/tactile/barre.toml
```

Les éléments s'affichent de gauche à droite, dans l'ordre du fichier. Chaque modification est prise en compte dans la seconde. Si le fichier contient une erreur, Tactile garde la barre en place et écrit l'erreur, avec son numéro de ligne, sur sa sortie d'erreur (visible en lançant l'exécutable de `Tactile.app/Contents/MacOS/` depuis un terminal). Au démarrage, une config invalide donne la barre par défaut.

```toml
[[element]]
type = "bouton"
symbole = "moon.fill"
texte = "Veille"
couleur = "#3A3A3C"
action = "veille"
appui_long = { shell = "open -a 'Activity Monitor'" }
```

### Éléments

| `type` | Champs | Ce que ça affiche |
|---|---|---|
| `bouton` | `texte`, `symbole` (nom d'un symbole SF), `couleur` (`"#RRGGBB"`), `largeur` (points), `action`, `appui_long` (action, après 0,5 s) | Un bouton. Tous les champs sont facultatifs. |
| `slider` | `reglage` (`"luminosite"` ou `"volume"`), `symbole` | Un bouton qui ouvre une sous-barre avec un curseur ; ✕ la referme. |
| `heure` | `format` (format de date Apple, `"HH:mm"` par défaut) | L'heure, mise à jour en continu. |
| `en_cours` | `largeur` | Le titre et l'artiste en cours de lecture (44 caractères au plus, rafraîchi toutes les 2 s) ; un appui met en pause ou relance. Demande `media-control`. |
| `script` | `commande` (obligatoire), `intervalle` (secondes, obligatoire, 0,1 au minimum), `largeur`, `action` | La sortie de la commande, lancée avec `zsh -c` à chaque intervalle. |
| `espace` | `largeur` (obligatoire) | Un vide de largeur fixe. |
| `souple` | aucun | Un vide qui prend toute la place libre : deux `souple` centrent ce qui est entre eux. |
| `groupe` | `texte`, `symbole`, puis des sous-éléments `[[element.element]]` | Un bouton qui ouvre une sous-barre avec ses éléments ; ✕ la referme. |

Au niveau du fichier, `masquer_control_strip = true` est accepté, mais la barre occupe pour l'instant toujours toute la largeur, Control Strip compris.

### Actions

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

Un bouton ✳ Claude : tu parles, Claude Code répond à voix haute. Transcription (whisper.cpp) et voix (Piper) tournent en local. Installation et réglages : [extras/claude-voix](extras/claude-voix/README.md), éléments à ajouter à ta barre : [exemples/claude-vocal.toml](exemples/claude-vocal.toml).

## Compiler depuis les sources

Il faut Rust stable (le dépôt fournit un `mise.toml` : `mise install`) et un Mac Apple Silicon.

```
cargo build --release      # binaire seul : target/release/tactile
cargo test                 # règle puis restaure vraiment la luminosité et le volume
scripts/empaqueter.sh      # target/Tactile.app et target/Tactile.zip
```

Les tests de luminosité et de volume ne font rien quand la variable `CI` est définie (pas d'écran ni de son sur les machines d'intégration continue).

## Limites connues

- MacBook Pro Apple Silicon avec Touch Bar uniquement.
- Tactile repose sur des API privées d'Apple (barre système, icône du Control Strip via DFRFoundation, luminosité via DisplayServices). Une mise à jour de macOS peut les casser.
- L'app n'est pas notarisée : le premier lancement passe par clic droit → Ouvrir.
- `masquer_control_strip` n'a pas encore d'effet (voir plus haut).
- Le curseur de luminosité règle l'écran intégré seulement.
- Pas encore de gestes sur la Touch Bar, de raccourcis clavier ni de gestes trackpad : ce sont les modules suivants.

## Remerciements

[MTMR](https://github.com/Toxblh/MTMR) (licence MIT) a servi de référence pour l'approche des API privées de la Touch Bar : présentation d'une barre système et icône dans le Control Strip.

## Licence

[MIT](LICENSE).

---

## English

Tactile replaces the MacBook Pro Touch Bar with a bar you describe in a TOML file. It is a native Apple Silicon app written in Rust, with no Dock icon and a small footprint at rest (measured: 0.5 % CPU, 13 MB RAM).

It supports buttons (text, SF Symbol, color, tap and long-press actions), brightness and volume sliders in sub-bars, the track currently playing in any app with ⏪ ⏯ ⏩, a clock, script widgets, groups, and switching back and forth with Apple's own bar (the  button shows Apple's bar, the ✳ icon in the Control Strip brings Tactile back). The config file is reloaded as soon as it is saved.

**Install.** Download `Tactile.zip` from [Releases](../../releases/latest), move `Tactile.app` to Applications, and open it the first time with right-click → Open (the app is not notarized). Allow it under System Settings → Privacy & Security → Accessibility (needed by the `echap` action). For the now-playing title, `brew install media-control`: since macOS 15.4, MediaRemote no longer lets unapproved apps read what is playing.

**Configure.** Edit `~/.config/tactile/barre.toml`; start from [exemples/barre.toml](exemples/barre.toml). Element types and actions are listed in the French tables above (keys are French: `bouton` = button, `heure` = clock, `en_cours` = now playing, `souple` = flexible space, `groupe` = group).

**Build.** `cargo build --release`, `cargo test`, then `scripts/empaqueter.sh` to get `target/Tactile.app` and `target/Tactile.zip`.

**Limits.** Apple Silicon MacBook Pro with a Touch Bar only; relies on private Apple APIs that a macOS update may break; not notarized.

Thanks to [MTMR](https://github.com/Toxblh/MTMR) (MIT), whose approach to the private Touch Bar APIs served as a reference. MIT licensed.
