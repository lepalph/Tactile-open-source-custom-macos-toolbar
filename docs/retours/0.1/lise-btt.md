# Retour de Lise : migrer de BetterTouchTool à Tactile ?

Persona : Lise, 29 ans, monteuse vidéo, MacBook Pro 16" 2019 (Intel, Touch Bar). Quatre ans de BetterTouchTool. Lecture seule du dépôt `lepalph/tactile-touch-bar` (v0.1.0), sans lancer l'app.

## Contexte

Ma barre BTT aujourd'hui :

- **Une barre par application.** Premiere Pro : couper (⌘K), marqueur (M), lier/délier, exporter (⌘M), rendu de la zone (↩), bascule des pistes audio. DaVinci : lame, ripple delete, couleur, Fusion, Fairlight, « livrer ». Finder, Safari, Mail gardent la barre d'Apple. Hors de ces apps, une barre « globale » : météo, batterie en minutes restantes, prochain rendez-vous, heure, Now Playing avec pochette, volume et luminosité inline.
- **Des boutons qui lancent des apps** (Premiere, Resolve, Frame.io dans Safari), **des AppleScript** (vider la corbeille, basculer la sortie son vers le casque), **des icônes à moi** (PNG des logos Adobe et Blackmagic), **des groupes pliables** (groupe « Export » dans Premiere, groupe « Son » dans la barre globale).
- **Deux gestes** : glisser à deux doigts sur la barre pour le volume, trois doigts pour la luminosité.
- **Pourquoi je lis Tactile** : la licence Standard de BTT couvre 2 ans de mises à jour, puis il faut repayer ; j'en suis à mon deuxième renouvellement. Tactile est MIT et gratuit. La question : est-ce que je perds quoi, et combien ?

Ce que j'ai lu : `README.md`, `SPEC.md`, `exemples/barre.toml`, `exemples/claude-vocal.toml`, `docs/ARCHITECTURE.md`, `docs/DECISIONS.md`, `docs/CHANTIER-0.2.md`, et surtout `src/config.rs`, `src/actions/mod.rs`, `src/barre/elements.rs`, `src/barre/rafraichi.rs` pour vérifier ce que l'app fait vraiment, pas ce que le README promet.

Sources BTT et MTMR consultées pour comparer :

- BTT, widgets Touch Bar : https://docs.folivora.ai/docs/touch-bar/widgets/ (batterie restante, date/heure, app switcher, sliders volume et luminosité, widget shell/Node/Python, widget AppleScript, emoji, slider AppleScript personnalisé, groupes).
- BTT, bases Touch Bar : https://docs.folivora.ai/docs/touch-bar/basics/ (barre globale ou par app, « Touch Bar Configuration For » par app, Control Strip, limites : on remplace la barre d'une app entièrement ou pas du tout).
- BTT, gestes Touch Bar : https://docs.folivora.ai/docs/touch-bar/gestures/ (glissement à 1, 2, 3, 4 doigts, gauche/droite, pourcentage transmis au script).
- BTT, conditions : https://docs.folivora.ai/docs/actions/conditions/ (Conditional Activation Groups, Advanced Trigger Conditions, variable `BTTActiveAppBundleIdentifier`).
- BTT, prix : https://folivora.ai/buy (Standard 15 $ pour 2 ans de mises à jour, Lifetime 25 $).
- Pochette dans BTT : fil Mac Power Users https://talk.macpowerusers.com/t/i-the-touchbar/4779?page=2 (config partagée avec titre, artiste et pochette ; l'import direct a fait planter BTT chez un utilisateur).
- MTMR, README : https://github.com/Toxblh/MTMR (`matchAppId`, `shellScriptTitledButton`, `appleScriptTitledButton`, `music`, `weather`, `battery`, `swipe`, `longTap`, `image` en base64 ou fichier, `openUrl`, `keyPress`).

## Tableau comparatif

Légende : **Oui** = fait et visible dans le code ; **Partiel** = possible avec un détour ; **Non** = absent. Importance pour moi de 1 (je m'en passe) à 5 (sans ça je ne migre pas). Effort = mon estimation en lisant l'architecture (`docs/ARCHITECTURE.md` donne les recettes) : faible (< 1 jour), moyen (2 à 5 jours), élevé (plus d'une semaine).

| Fonction BTT | Tactile aujourd'hui | Preuve dans le code | Importance | Effort |
|---|---|---|---|---|
| Bouton texte + icône + couleur de fond | **Oui** | `config.rs` `Element::Bouton { texte, symbole, couleur }` ; `elements.rs` `setImagePosition: 2` quand les deux sont donnés, `setBezelColor` | 5 | fait |
| Icône personnalisée (PNG, logo d'app) | **Non** : SF Symbols seulement | `vues.rs::symbole` → `imageWithSystemSymbolName:` ; aucun champ `image` dans `config.rs` | 4 | faible |
| Couleur du texte, taille de police, icône seule / texte seul | **Non** | `elements.rs` ne règle que `setBezelColor` ; pas de `couleur_texte` ni `taille` | 3 | faible |
| Largeur fixe d'un bouton | **Oui** | `largeur` sur `bouton`, `script`, `en_cours`, `espace` ; `vues.rs::largeur_fixe` | 3 | fait |
| **Barre différente selon l'app active** | **Non** | `config.rs` : une seule `Barre` avec un `Vec<Element>` ; aucun champ d'app, aucun observateur `NSWorkspace` dans `barre/` | **5** | moyen |
| Garder la barre d'Apple dans certaines apps (Finder, Safari) | **Partiel** : à la main | `action = "barre_apple"` (`actions/mod.rs` `BarreApple`), retour par ✳ ; rien d'automatique | 4 | faible une fois la barre par app faite |
| Bouton qui envoie un raccourci clavier (⌘K, ⇧⌘E…) | **Non** | `actions/clavier.rs` n'expose que `touche(53)` (esc) ; l'enum `Action` n'a pas de variante touche/raccourci | **5** | faible à moyen |
| Lancer une app | **Partiel** : `{ shell = "open -a 'DaVinci Resolve'" }` | `Action::Shell` → `zsh -c` | 4 | trivial (sucre) |
| Ouvrir une URL | **Partiel** : `{ shell = "open https://…" }` | idem | 3 | trivial (sucre) |
| Exécuter un AppleScript | **Partiel** : `{ shell = "osascript -e '…'" }` | idem ; les guillemets imbriqués TOML + zsh + AppleScript sont pénibles | 4 | faible |
| Widget shell script, sortie texte, intervalle | **Oui** | `Element::Script { commande, intervalle, largeur, action }` ; `rafraichi.rs` lance `zsh -c` dans un fil | 5 | fait |
| Widget script qui change aussi son **icône et sa couleur** (BTT : sortie JSON `text`, `icon_data`, `background_color`) | **Non** : texte seulement | `rafraichi.rs` : `titre(*bouton, sortie.trim_end())`, rien d'autre | 4 | moyen |
| Widget AppleScript (à intervalle) | **Partiel** : `script` + `osascript` | `Element::Script` | 3 | faible |
| Widget météo, batterie, calendrier prêts à l'emploi | **Partiel** : à écrire soi-même en `script` | aucun type dédié dans `config.rs` | 3 | faible (exemples fournis suffisent) |
| Date / heure formatée | **Oui** | `Element::Heure { format }` → `NSDateFormatter` | 4 | fait |
| Now Playing : titre + artiste | **Oui**, 44 caractères, sans défilement | `rafraichi.rs` `EnCours`, `.take(44)` ; dépend de `media-control` | 4 | fait |
| Now Playing : **pochette** | **Non** | `rafraichi.rs` ne met qu'un titre ; `media.rs` ne lit pas l'illustration | 3 | moyen |
| Lecture / pause, suivant, précédent | **Oui** | `Action::LecturePause / Suivant / Precedent` | 4 | fait |
| Groupe pliable | **Oui** | `Element::Groupe`, `elements.rs::depliable` (sous-barre + ✕) | 4 | fait |
| Groupe imbriqué, retour au groupe parent | **Partiel** : la config l'accepte, mais ✕ revient toujours à la barre principale | `construire` est récursif ; `Commande::Fermer` ramène la principale (`ARCHITECTURE.md`) | 2 | faible |
| Fermer le groupe après une action | **Non** | pas d'option sur `Groupe` | 2 | faible |
| Appui long | **Oui**, durée fixe 0,5 s | `appui_long` ; `NSPressGestureRecognizer` `setMinimumPressDuration: 0.5` | 3 | fait |
| Double appui | **Non** | aucun recognizer de tap multiple | 1 | faible |
| Slider volume / luminosité | **Partiel** : dans une sous-barre, pas inline | `elements.rs::slider` → `depliable` | 3 | moyen (limite des barres système, cf. `DECISIONS.md` n° 3) |
| Luminosité d'un écran externe (BTT : ctrl + slider) | **Non** | `README` : écran intégré seulement | 2 | moyen |
| Slider personnalisé piloté par script | **Non** | `Reglage` n'a que `Luminosite` et `Volume` | 2 | moyen |
| Gestes sur la Touch Bar (glisser 2 / 3 doigts) | **Non** (prévu, module 2 de `SPEC.md`) | `README` « Limites connues » | 3 | moyen |
| Boutons conditionnels (CAG : variable, modificateur maintenu) | **Non** | aucune notion de condition dans `config.rs` | 3 | moyen |
| Barre alternative quand ⇧ ou ⌥ est maintenu | **Non** | idem | 3 | moyen |
| App switcher, emoji, Dock | **Non** | — | 1 | moyen |
| Masquer le Control Strip, touche esc | **Oui** | `masquer_control_strip`, `Action::Echap` (demande l'Accessibilité) | 4 | fait |
| Retour haptique au toucher | **Non** | — | 1 | faible |
| Éditeur visuel | **Non** (chantier 3b) | `README` « Pas encore d'éditeur visuel » | 3 | élevé |
| **Import de mes presets BTT** | **Non** (module 7 de `SPEC.md`) | — | **5** pour migrer sans tout retaper | élevé |
| Rechargement sans relancer | **Oui**, 1 s | `config.rs::surveiller` | 4 | fait |
| Lancer au démarrage | **Oui** | `menu.rs` (`SMAppService`) | 4 | fait |

## Bloquants pour migrer

Par ordre, ce qui m'empêche aujourd'hui de désinstaller BTT :

1. **Une seule barre pour toutes les apps.** C'est tout mon usage : la barre Premiere n'a rien à voir avec la barre DaVinci ni avec la barre « bureau ». Avec Tactile v0.1, soit je mets tout dans des groupes (un groupe « Premiere », un groupe « DaVinci »), ce qui coûte un toucher à chaque fois et me laisse une barre encombrée, soit je change de fichier `barre.toml` à la main. Non.
2. **Pas de bouton « raccourci clavier ».** Mes boutons Premiere sont à 80 % des raccourcis (⌘K, M, ⇧⌘E…). Le détour `{ shell = "osascript -e 'tell application \"System Events\" to keystroke \"k\" using command down'" }` marche, mais : il lance un processus `osascript` à chaque appui (latence sensible au montage, où je coupe au rythme), il demande l'Accessibilité, et la triple imbrication de guillemets TOML / zsh / AppleScript est illisible. `clavier.rs` sait déjà envoyer un code de touche : il manque les modificateurs et une syntaxe.
3. **Pas d'import de mes presets.** Quatre ans de configuration BTT, exportables en JSON. Si je dois tout retaper en TOML, la migration prend une journée, et je ne la ferai pas « pour voir ».
4. **Pas d'icônes personnalisées.** Les logos Premiere et Resolve ne sont pas des SF Symbols. Avec un texte « Pr » et une couleur, je m'en sors, mais la barre est moins lisible.
5. **Les widgets ne peuvent changer que leur texte.** Ma météo BTT change d'icône (soleil, pluie), ma batterie passe en rouge sous 15 %. Dans Tactile, je peux mettre un emoji dans le texte (☀️ 12°) mais pas de couleur ni de symbole dynamique.

Non bloquants, mais je les noterai le premier jour : pas de pochette dans `en_cours`, 44 caractères sans défilement ; sliders en sous-barre (un toucher de plus) ; pas de gestes.

### Ce que je ferais avec les widgets `script` à la place

Tout ce que BTT appelle « widget » tient dans un `script` zsh, tant que la sortie est du texte :

```toml
# Batterie : minutes restantes, comme le widget BTT.
[[element]]
type = "script"
commande = "pmset -g batt | grep -o '[0-9]*%.*remaining' | sed 's/;.*remaining/ ·/' | sed 's/ (no estimate)//'"
intervalle = 60
largeur = 110

# Météo (wttr.in, sans clé d'API).
[[element]]
type = "script"
commande = "curl -s 'wttr.in/Paris?format=%c+%t' | tr -d '+'"
intervalle = 900
largeur = 90
action = { shell = "open https://wttr.in/Paris" }

# Prochain rendez-vous (icalBuddy, brew install ical-buddy).
[[element]]
type = "script"
commande = "icalBuddy -n -nc -li 1 -eep 'notes,url,location,attendees' -b '' eventsToday | head -1"
intervalle = 300
largeur = 220
action = { shell = "open -a Calendar" }

# Sortie audio actuelle, bascule casque / haut-parleurs (SwitchAudioSource, brew).
[[element]]
type = "script"
commande = "SwitchAudioSource -c | cut -c1-14"
intervalle = 5
largeur = 120
action = { shell = "SwitchAudioSource -n" }

# Lancer DaVinci Resolve (le bouton 'lancer une app' de BTT).
[[element]]
type = "bouton"
texte = "Resolve"
symbole = "film"
couleur = "#1B2A41"
action = { shell = "open -a 'DaVinci Resolve'" }
```

Limites de ce détour, vues dans `rafraichi.rs` : un script par élément et un fil par exécution (dix widgets à 1 s = dix `zsh` par seconde ; sur mon Intel ça se verra), texte coupé à la largeur fixe, pas de couleur ni d'icône pilotée par le script, et un script qui ne rend pas la main bloque son élément (`en_cours` reste vrai jusqu'au retour).

## Ce que Tactile fait mieux

- **Gratuit, MIT, sans compte, sans licence qui expire.** C'est la raison de ma lecture. BTT : 15 $ pour 2 ans, 25 $ à vie, et le renouvellement arrive toujours au mauvais moment.
- **Léger.** 0,5 % CPU et 13 Mo mesurés (`README`, `PLAN.md`). BTT est une app de gestes, fenêtres, presse-papier et Touch Bar en même temps ; je paie en mémoire pour des modules que je n'utilise pas sur un Mac qui rend déjà pendant les exports.
- **La configuration est un fichier texte lisible.** 55 lignes commentées pour la barre par défaut (`exemples/barre.toml`), que je peux versionner, diff-er, partager à une collègue. Mes presets BTT sont un JSON de plusieurs milliers de lignes que je n'ouvre jamais, et dont l'import plante parfois (cf. fil Mac Power Users cité).
- **Erreur de config = numéro de ligne, en français, et l'ancienne barre reste** (`config.rs::lire`, `element_fautif`, tests `type_inconnu`, `champ_mal_orthographie`). BTT ne me dit rien quand un widget est mal réglé : il n'affiche rien.
- **Rechargement en une seconde** sans passer par l'interface.
- **Les groupes marchent dans une barre système** (sous-barre + ✕, `DECISIONS.md` n° 3), là où MTMR les avait cassés. BTT fait pareil, mais c'est le point où Tactile dépasse déjà l'alternative gratuite.
- **Ça se répare.** `ARCHITECTURE.md` explique en quatre étapes comment ajouter une action ou un élément, et liste les API privées. Si une mise à jour de macOS casse quelque chose, je peux au moins lire pourquoi. BTT est une boîte noire ; quand Apple a bloqué MediaRemote (15.4), j'ai attendu la mise à jour de l'éditeur.
- **Universel Intel + Apple Silicon**, utile pour mon 2019 (note : la version Intel n'a été testée que sous Rosetta, `README`).

## 5 fonctionnalités prioritaires (avec TOML souhaité)

Les syntaxes suivent `exemples/barre.toml` : clés en français, `snake_case`, une action = un mot ou une table.

### 1. Barre par application

Le bloquant n° 1. Une table `[[application]]` par app, identifiée par bundle id ou par nom, avec ses propres `[[application.element]]` ; les `[[element]]` de tête restent la barre globale. `barre = "apple"` garde la barre d'Apple pour une app. `herite = true` ajoute les éléments globaux à droite (l'option BTT « Show global actions also if app-specific actions are available »).

```toml
masquer_control_strip = true

# Barre globale : inchangée.
[[element]]
type = "heure"
format = "HH:mm"

# Premiere Pro : ma barre de montage.
[[application]]
app = "com.adobe.PremierePro"        # ou app = "Adobe Premiere Pro"
herite = false

  [[application.element]]
  type = "bouton"
  texte = "Couper"
  symbole = "scissors"
  action = { touche = "cmd+k" }

  [[application.element]]
  type = "groupe"
  texte = "Export"
  symbole = "square.and.arrow.up"
    [[application.element.element]]
    type = "bouton"
    texte = "Media Encoder"
    action = { touche = "cmd+m" }

# Finder et Safari : la barre d'Apple, automatiquement.
[[application]]
app = ["com.apple.finder", "com.apple.Safari"]
barre = "apple"
```

Côté code : `Barre` gagne `applications: Vec<Application>` ; `barre/mod.rs` observe `NSWorkspaceDidActivateApplicationNotification` et appelle `installer` avec la bonne liste. La recette « Ajouter un type d'élément » d'`ARCHITECTURE.md` ne couvre pas ce cas, il faudra l'y ajouter.

### 2. Actions du quotidien : raccourci clavier, lancer une app, ouvrir une URL

Trois tables d'action, dans l'esprit de `{ volume = 0.3 }` et `{ shell = "…" }` :

```toml
[[element]]
type = "bouton"
symbole = "scissors"
action = { touche = "cmd+k" }              # modificateurs : cmd, shift, alt, ctrl, fn ; touches : a-z, 0-9, f1-f20, esc, tab, espace, retour, suppr, gauche…
appui_long = { touche = "shift+cmd+k" }

[[element]]
type = "bouton"
texte = "Resolve"
action = { app = "DaVinci Resolve" }       # = open -a, mais sans zsh ni guillemets

[[element]]
type = "bouton"
symbole = "link"
action = { url = "https://app.frame.io" }

[[element]]
type = "bouton"
texte = "Casque"
action = { applescript = "tell application \"System Events\" to …" }   # ou applescript = "~/.config/tactile/scripts/casque.scpt"
```

`touche` est la clé : `clavier.rs::touche(code)` existe, il manque les drapeaux de modificateurs sur le `CGEvent` et l'analyse de la chaîne. `app`, `url`, `applescript` sont du sucre sur `lancer`, mais ils enlèvent la triple imbrication de guillemets et rendent le fichier lisible.

### 3. Apparence : icône à moi, couleur du texte, taille

```toml
[[element]]
type = "bouton"
image = "~/.config/tactile/icones/premiere.png"   # PNG ou PDF ; exclusif avec symbole ; rendu à 20 pt de haut
texte = "Pr"
couleur = "#2D0A4E"
couleur_texte = "#FFFFFF"
taille = 13                                         # points ; défaut Apple si absent
affichage = "icone"                                 # "icone", "texte", "les_deux" (défaut : les_deux)
```

`image` : `NSImage initWithContentsOfFile:` à côté de `vues.rs::symbole`. `couleur_texte` : attributed title. Faible effort, et c'est ce qui rend une barre « à soi » plutôt qu'une barre de symboles gris.

### 4. Widgets `script` riches, et `en_cours` avec pochette

Un `script` dont la sortie commence par `{` est lue comme du JSON (BTT fait exactement ça, MTMR passe par `image`). Sinon : texte brut, comme aujourd'hui.

```toml
# Météo : le script écrit {"texte": "12°", "symbole": "cloud.rain.fill", "couleur": "#1F3A5F"}
[[element]]
type = "script"
commande = "~/.config/tactile/scripts/meteo.sh"
intervalle = 900
largeur = 90

# Batterie : {"texte": "1 h 40", "couleur": "#B4552F"} quand elle passe sous 15 %
[[element]]
type = "script"
commande = "~/.config/tactile/scripts/batterie.sh"
intervalle = 60
largeur = 100

# Now Playing : pochette et défilement du titre
[[element]]
type = "en_cours"
largeur = 330
pochette = true          # illustration à gauche, si media-control la donne
defilement = true        # fait défiler au lieu de couper à 44 caractères
```

Clés JSON acceptées : `texte`, `symbole`, `image` (chemin ou base64), `couleur`, `couleur_texte`. Dans `rafraichi.rs`, la branche `Script` fait aujourd'hui un seul `titre(...)` ; elle ferait le même aiguillage que `construire` pour un `Bouton`.

### 5. Import des presets BTT

Pas de TOML à écrire, une commande :

```
tactile --importer ~/Desktop/MaBarre.bttpreset > ~/.config/tactile/barre.toml
```

Ce qui se traduit : boutons (nom, icône, couleur de fond, action « raccourci clavier », « lancer une app », « ouvrir une URL », « script shell », « AppleScript »), groupes, widgets shell/AppleScript (en `script`, intervalle conservé), date/heure (en `heure`), sliders, et la portée par app (en `[[application]]`, bundle id conservé). Ce qui ne se traduit pas est écrit en commentaire TOML à l'endroit où ça aurait dû être, avec la raison (`# BTT : widget Emoji, pas d'équivalent`). Le format BTT est un JSON documenté par l'éditeur (`SPEC.md` le note) ; l'import est le module 7 de la spec, je le remonte en 5 parce que sans lui, personne qui a quatre ans de BTT ne « teste » Tactile.

## Remarques

- **Clés en français et public international.** Pour moi, `bouton`, `couleur`, `appui_long` se lisent sans effort et le README anglais les traduit. Mais un Américain qui cherche « touch bar replacement free » sur GitHub ouvrira `barre.toml`, verra `masquer_control_strip` et refermera. Les configurations partagées sur les forums (le réflexe BTT et MTMR) ne circuleront pas si elles ne se lisent pas. Proposition à bas coût : des **alias anglais acceptés par serde** (`#[serde(rename = "bouton", alias = "button")]`, idem pour chaque champ et chaque action), sans rien changer au français qui reste la forme canonique ; un fichier `exemples/bar.en.toml` ; et des messages d'erreur dans la langue du fichier (si la première clé lue est anglaise, répondre en anglais). Risque : deux orthographes dans la nature ; un `tactile --verifier` qui peut réécrire en une seule langue (`--en`, `--fr`) le règle.
- **Mon Mac est un Intel.** Le README dit que la version Intel n'a tourné que sous Rosetta. Avant de migrer je veux un retour d'un vrai Intel 2019, ou je le fais moi en sachant que je suis la première.
- **`echap` demande l'Accessibilité, `media-control` demande Homebrew.** Deux prérequis que BTT n'a pas (il demande l'Accessibilité aussi, mais une fois pour tout). À dire clairement dans l'installation, avec un ordre : Tactile, puis Accessibilité, puis `brew install media-control`.
- **Dix widgets à intervalle court = dix processus `zsh` par intervalle** (`rafraichi.rs`). Sur un Intel qui exporte, je veux soit un `intervalle` minimum plus élevé par défaut pour `script` (1 s plutôt que 0,1), soit un mot dans le README : « un script par seconde coûte autant qu'un processus par seconde ».
- **Pas de gestes.** Je m'en passerais si les sliders étaient inline ; là, régler le volume coûte deux touchers (ouvrir le slider, glisser) contre un glissement à deux doigts dans BTT. Si les gestes arrivent (module 2), `glisser_2_doigts = "volume"` en tête de fichier me suffirait.
- **Ce que je ferais demain matin si les points 1 et 2 existaient** : je garderais BTT installé mais désactivé une semaine, Tactile avec mes deux barres Premiere et DaVinci retapées à la main (vingt boutons, une heure), et je verrais si le mot « renouveler » me manque.
