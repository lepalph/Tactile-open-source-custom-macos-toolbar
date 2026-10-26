# Retour de Lise sur la 0.2 : je migre de BetterTouchTool ?

Persona : Lise, 29 ans, monteuse vidéo, MacBook Pro 16" 2019 (Intel, Touch Bar), clavier AZERTY, quatre ans de BetterTouchTool. Suite de [mon retour sur la 0.1](../0.1/lise-btt.md).

## Contexte

Cette fois, je n'ai pas seulement lu : j'ai écrit ma barre de monteuse pour de vrai et je l'ai passée à `tactile --verifier` jusqu'au « OK ». Je n'ai pas lancé la barre elle-même (la Touch Bar de la machine de test sert à autre chose), donc tout ce qui se passe à l'appui est déduit du code, pas vu.

- Dépôt : `main` à `f6b9cdb`, section `Unreleased` du `CHANGELOG.md`.
- Lu : `CHANGELOG.md`, `README.fr.md`, `exemples/par-application.toml`, `exemples/barre.en.toml`, `exemples/developpeur.toml`, puis `src/config.rs`, `src/actions/mod.rs`, `src/actions/clavier.rs`, `src/barre/mod.rs`, `src/barre/elements.rs`, `src/barre/rafraichi.rs`, `src/barre/batterie.rs`, `src/barre/vues.rs`, `src/main.rs`.
- Compilé avec `mise exec -- cargo build --release -q`, puis `target/release/tactile --verifier <fichier>`.
- Mes fichiers (barre, 28 petites sondes, outils) sont dans le scratchpad de la session : `/private/tmp/claude-501/-Users-palphinho/03bc99cc-4742-4072-87c4-fe525a6d77ae/scratchpad/lise2/` (`barre-monteuse.toml`, `premier-jet.toml`, `premier-jet-element-egare.toml`, `sondes/*.toml`, `outils/resoudre.py`, `outils/largeurs.py`, `outils/symboles.swift`).
- Pour ce que `--verifier` ne vérifie pas, j'ai rejoué le code avec trois petits outils : `resoudre.py` refait `clavier.rs::code_du_caractere` sur ma disposition (French), `largeurs.py` refait `elements.rs::debordement` et `Application::elements_avec` pour chaque barre, et `symboles.swift` vérifie que mes 17 symboles SF existent (ils existent tous).

## Mes 5 priorités de la 0.1 : état

| # | Priorité 0.1 | État 0.2 | Preuve | Ce qui manque encore |
|---|---|---|---|---|
| 1 | Barre par application | **Fait, avec des limites** | `config.rs` `Application { app, herite, elements }`, `Barre::application_pour` ; `barre/mod.rs` `base()` et `suivre_app()` (l'app au premier plan est relue à chaque `tic`, toutes les 0,1 s) | `app` ne prend qu'une chaîne, pas de liste (sonde 01) ; pas de `barre = "apple"` pour garder la barre d'Apple dans le Finder ou Safari (sonde 02) ; `herite` est tout ou rien (les bords de la principale, découpés aux `souple`) |
| 2 | Raccourci clavier, lancer une app, URL, AppleScript | **Fait** | `actions/mod.rs` `Touche`, `Keycode`, `App`, `Url`, `Applescript` ; `clavier.rs` envoie un `CGEvent` avec les drapeaux des modificateurs, sans lancer de processus | Des trous sur AZERTY (voir les blocages) ; `keycode` sans modificateur ; pas de touche « + » |
| 3 | Apparence : image, couleur du texte, taille, affichage | **Fait pour les boutons** | `Element::Bouton { image, gabarit, couleur_texte, taille, affichage }` ; `vues.rs::image` (20 pt de haut), `vues.rs::styler` | Un `groupe` n'accepte ni `image` ni `couleur` (sondes 07 et 08) |
| 4 | Widgets `script` riches, `en_cours` avec pochette | **Moitié** : le JSON est fait, la pochette non | `rafraichi.rs` `Riche`, `analyser_sortie`, `appliquer` ; `EnCours` coupe toujours à 44 caractères (`.take(44)`) sans pochette | Pochette et défilement ; le JSON ne peut pas changer l'action du widget |
| 5 | Import des presets BTT | **Pas commencé** | Aucune occurrence de `bttpreset` ni d'import dans `src/` | Tout |

Trois sur cinq, plus la moitié d'une. C'est beaucoup plus que ce que j'attendais d'une seule version.

## Ma barre de monteuse

Trois barres dans un seul fichier :

- **Principale** (hors montage) : Apple, esc, groupe Son, puis lanceurs Resolve, Final Cut Pro, Frame.io, corbeille (AppleScript à l'appui long), titre en cours, puis espace disque (script JSON, rouge sous 10 %), batterie, heure.
- **DaVinci Resolve** (`herite = false`, toute la largeur) : esc, groupe Pages (Fusion, Couleur, Fairlight, Livrer), sélection, lame, couper, ripple delete, marqueur, J, K, L, zoom − et +, batterie, heure.
- **Final Cut Pro** (`herite = true`) : garde Apple, esc et Son à gauche, disque, batterie et heure à droite ; au milieu lame, couper, marqueur, J, K, L, zoom − et +, groupe Export.

Largeur estimée avec la formule de `elements.rs` (1004 pt disponibles, alerte au-delà de 5 %) : principale 1015 pt, Resolve 929 pt, Final Cut Pro 1007 pt. Ça tient, mais de peu.

### Le fichier complet

```toml
# Barre de Lise, monteuse. Tactile 0.2 (main, Unreleased).
# Principale hors montage ; DaVinci Resolve et Final Cut Pro ont la leur.
# Clavier : AZERTY (French). Raccourcis : ceux d'usine de Resolve 19 et FCP 11.

masquer_control_strip = true

# ───────────── Barre principale ─────────────

# La barre d'Apple à la main : pas de `barre = "apple"` automatique pour le Finder ou Safari.
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

# Groupe « Son » : les sliders restent en sous-barre (un toucher de plus qu'avec BTT).
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

# Lanceurs. Mes PNG si présents ; sinon Tactile affiche le texte (« Re », « FCP »).
[[element]]
type = "bouton"
texte = "Re"
image = "~/.config/tactile/icones/resolve.png"
affichage = "icone"
couleur = "#1B2A41"
largeur = 56
action = { app = "DaVinci Resolve" }

[[element]]
type = "bouton"
texte = "FCP"
image = "~/.config/tactile/icones/finalcut.png"
affichage = "icone"
couleur = "#2B2B2B"
largeur = 56
action = { app = "Final Cut Pro" }

[[element]]
type = "bouton"
texte = "Frame.io"
symbole = "link"
couleur = "#5B53FF"
couleur_texte = "#FFFFFF"
action = { url = "https://next.frame.io" }

# AppleScript : vider la corbeille (appui long, pour ne pas la vider par erreur).
[[element]]
type = "bouton"
symbole = "trash"
largeur = 56
appui_long = { applescript = 'tell application "Finder" to empty trash' }

[[element]]
type = "en_cours"
largeur = 200

[[element]]
type = "souple"

# Espace libre sur le disque, en rouge sous 10 % (sortie JSON). Toucher = « À propos de ce Mac > Stockage ».
[[element]]
type = "script"
commande = '''
df -k / | awk 'NR==2 { libre = $4 / $2 * 100; fond = libre < 10 ? ", \"couleur\": \"#B00020\", \"couleur_texte\": \"#FFFFFF\"" : ""; printf "{\"texte\": \"%d %%\", \"symbole\": \"internaldrive\"%s}\n", libre, fond }'
'''
intervalle = 120
largeur = 80
action = { url = "x-apple.systempreferences:com.apple.settings.Storage" }

[[element]]
type = "batterie"
largeur = 90

[[element]]
type = "heure"
format = "HH:mm"

# ───────────── DaVinci Resolve ─────────────
# herite = false : toute la largeur pour le montage ; batterie et heure sont donc recopiées.

[[application]]
app = "com.blackmagic-design.DaVinciResolve"

[[application.element]]
type = "bouton"
texte = "esc"
largeur = 50
action = "echap"

# Pages Fusion, Couleur, Fairlight, Livrer : ⇧5 à ⇧8. Sur AZERTY, `touche = "shift+6"` viserait
# le 6 du pavé numérique (keycode 88) et `keycode` n'accepte pas de modificateur : on nomme donc le
# caractère de la touche sans maj (( § è !). Propre à l'AZERTY ; portable mais plus lent :
# { applescript = 'tell application "System Events" to key code 22 using shift down' }.
[[application.element]]
type = "groupe"
texte = "Pages"
largeur = 80
  [[application.element.element]]
  type = "bouton"
  texte = "Fusion"
  action = { touche = "shift+(" }

  [[application.element.element]]
  type = "bouton"
  texte = "Couleur"
  action = { touche = "shift+§" }

  [[application.element.element]]
  type = "bouton"
  texte = "Fairlight"
  action = { touche = "shift+è" }

  [[application.element.element]]
  type = "bouton"
  texte = "Livrer"
  action = { touche = "shift+!" }

[[application.element]]
type = "souple"

[[application.element]]
type = "bouton"
symbole = "cursorarrow"
largeur = 50
action = { touche = "a" }          # mode sélection

[[application.element]]
type = "bouton"
symbole = "scissors"
largeur = 50
action = { touche = "b" }          # mode lame

[[application.element]]
type = "bouton"
texte = "Couper"
symbole = "scissors.badge.ellipsis"
couleur = "#7A2E2E"
action = { touche = "cmd+b" }      # couper à la tête de lecture (cmd+\ est introuvable en AZERTY)

[[application.element]]
type = "bouton"
symbole = "delete.left"
largeur = 50
action = { touche = "shift+suppr" }  # ripple delete

[[application.element]]
type = "bouton"
symbole = "bookmark.fill"
couleur = "#2E5C7A"
largeur = 50
action = { touche = "m" }          # marqueur

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

# « cmd++ » est refusé (le + sépare les touches) : c'est cmd+= qui zoome, comme dans le menu.
[[application.element]]
type = "bouton"
symbole = "plus.magnifyingglass"
largeur = 44
action = { touche = "cmd+=" }
appui_long = { touche = "shift+z" }   # tout voir

[[application.element]]
type = "souple"

[[application.element]]
type = "batterie"
symbole = false
largeur = 80

[[application.element]]
type = "heure"
format = "HH:mm"

# ───────────── Final Cut Pro ─────────────
# herite = true : la principale garde son début (Apple, esc, Son) et sa fin (disque, batterie, heure).

[[application]]
app = "com.apple.FinalCut"
herite = true

[[application.element]]
type = "bouton"
symbole = "scissors"
largeur = 50
action = { touche = "b" }          # outil lame

[[application.element]]
type = "bouton"
texte = "Couper"
couleur = "#7A2E2E"
largeur = 64
action = { touche = "cmd+b" }      # lame à la tête de lecture

[[application.element]]
type = "bouton"
symbole = "bookmark.fill"
couleur = "#2E5C7A"
largeur = 50
action = { touche = "m" }
appui_long = { touche = "alt+m" }  # marqueur + fenêtre de modification

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
type = "groupe"
texte = "Export"
symbole = "square.and.arrow.up"
largeur = 64
  [[application.element.element]]
  type = "bouton"
  texte = "Partager"
  action = { touche = "cmd+e" }

  [[application.element.element]]
  type = "bouton"
  texte = "Rendu"
  action = { touche = "ctrl+shift+r" }

  [[application.element.element]]
  type = "bouton"
  texte = "Compressor"
  action = { app = "Compressor" }
```

### Résultat de `--verifier`

```
$ target/release/tactile --verifier barre-monteuse.toml
OK : 13 éléments
```

Le chiffre ne compte que la barre principale. Les 16 éléments de Resolve et les 9 de Final Cut Pro n'apparaissent nulle part.

Le script disque testé à la main : `{"texte": "91 %", "symbole": "internaldrive"}` sur cette machine, et `{"texte": "5 %", "symbole": "internaldrive", "couleur": "#B00020", "couleur_texte": "#FFFFFF"}` avec un `df` simulé à 5 % libre. Les deux se lisent en JSON valide.

### Le chemin jusqu'au « OK »

J'ai d'abord écrit la barre avec la syntaxe que je souhaitais en 0.1 (`premier-jet.toml`), puis corrigé une erreur à la fois. Cinq passages :

| Passage | Message de `--verifier` | Vraie ligne | Correction |
|---|---|---|---|
| 1 | `ligne 108 : modificateur inconnu «  » ; attendu : cmd, shift, ctrl, alt, fn` | **204** (`cmd++`) | `cmd+=` |
| 2 | `ligne 223 : type invalide: sequence, attendu a string` | 223 | `app = [ "FCP", "iMovie" ]` → une seule app |
| 3 | `ligne 226 : champ inconnu «image», attendu parmi «text», «texte», …` | **281** (`image` sur le groupe Export) | `symbole` |
| 4 | `ligne 299 : type invalide: sequence, attendu a string` | 299 | `[[application]]` Finder + Safari `barre = "apple"` supprimée |
| 5 | `ligne 93 : champ inconnu «seuil_rouge», …` | 96 (début du bloc : correct) | supprimé |

Deux numéros de ligne sur cinq sont faux, et l'erreur de la barre principale (ligne 96) n'est sortie qu'en dernier, après celles des `[[application]]` situées plus bas dans le fichier.

## Surprises et blocages de syntaxe

Dans l'ordre où ils m'ont coûté du temps.

1. **Bug : la ligne d'erreur est fausse dans une `[[application]]`.** Si l'élément fautif n'est pas le premier de l'app, `--verifier` (et donc le « ⚠ config ligne N » de la barre) indique la ligne du *premier* `[[application.element]]` de l'app. Sondes minimales : `sondes/25-erreur-3e-element-app.toml` (erreur ligne 15, annoncée ligne 7) et `sondes/27-erreur-2e-app.toml` (erreur ligne 17, annoncée ligne 11). Cause dans `config.rs` : `element_fautif` ne descend pas dans `application.element` (il essaie l'`Application` entière), le span de toml pointe le premier élément, et `lire` garde le `max` des deux positions.
2. **Bug : l'ordre des erreurs ne suit pas le fichier.** `sondes/26-deux-erreurs.toml` : l'erreur de la ligne 3 (barre principale) ne sort qu'après celle de la ligne 8 (dans une app). Les erreurs d'`application` passent avant celles d'`element` : sans doute l'ordre alphabétique des clés. Avec une seule erreur affichée à la fois, je corrige dans le désordre.
3. **Piège silencieux : un `[[element]]` égaré dans un bloc `[[application]]`.** J'ai tapé `[[element]]` au lieu de `[[application.element]]` pour un `souple` au milieu de la barre Resolve. En TOML, c'est valide : l'élément part au bout de la barre *principale*. `--verifier` répond `OK : 14 éléments` au lieu de 13, c'est tout. Effet en chaîne : le dernier `souple` de la principale devient ce dernier élément, donc la barre Final Cut Pro (`herite = true`) perd disque, batterie et heure (744 pt au lieu de 1007 d'après `largeurs.py`). Fichier : `premier-jet-element-egare.toml`.
4. **AZERTY : les chiffres partent sur le pavé numérique.** `clavier.rs::code_du_caractere` cherche le premier keycode qui donne le caractère *sans modificateur*. Sur ma disposition, la rangée du haut donne `& é " ' ( § è ! ç à` sans maj, donc `"6"` se résout en keycode 88 (6 du pavé) et pas 22 (rejoué avec `outils/resoudre.py` : 4 → 86, 5 → 87, 6 → 88, 7 → 89, 8 → 91). `{ touche = "shift+6" }` passe `--verifier` mais envoie ⇧ + pavé 6 : Resolve ne changera probablement pas de page (je n'ai pas pu appuyer pour le voir). Ça touche aussi l'exemple `cmd+shift+4` du README pour les utilisateurs AZERTY. Détour trouvé : nommer le caractère de la touche sans maj (`shift+§` pour ⇧6). Il marche, mais il est propre à l'AZERTY et je ne l'aurais pas trouvé sans lire `clavier.rs`.
5. **AZERTY : un caractère qui demande un modificateur est introuvable, et seulement à l'appui.** `cmd+\` (couper dans Resolve) passe `--verifier` (`sondes/05-antislash.toml`), mais `\` n'existe pas sans modificateur en AZERTY (`resoudre.py` → `None`) : l'erreur n'arrivera qu'au toucher, dans le journal. Et `keycode` n'accepte pas de modificateur (`Action::Keycode(code) => clavier::touche(*code, 0)` ; `sondes/06-keycode-modif.toml` refusé). Il reste AppleScript, qui lance `osascript` à chaque appui.
6. **`cmd++` est refusé avec un message obscur** : `modificateur inconnu «  »`. Le `+` sert de séparateur, il faut deviner `cmd+=`. Pour une monteuse, « zoom + » s'écrit d'abord `cmd++`.
7. **Pas de liste d'apps.** Resolve et Final Cut Pro partagent dix raccourcis (lame, couper, marqueur, J/K/L, zoom) : je les ai écrits deux fois. Rien pour factoriser (pas de liste `app = [...]`, pas d'inclusion, pas de modèle).
8. **Pas de `barre = "apple"`.** Dans le Finder, Safari et Mail, je voulais la barre d'Apple automatiquement. Il me reste le bouton `barre_apple` à toucher moi-même, et le ✳ pour revenir.
9. **`herite` est tout ou rien.** Dans Resolve, je voulais garder la batterie et l'heure sans le groupe Son ni le bouton Apple : impossible, donc `herite = false` et je recopie batterie et heure. Le découpage aux `souple` est malin, mais il fait qu'ajouter un `souple` dans ma principale change toutes mes barres d'app.
10. **`--verifier` ne vérifie que la syntaxe.** Il dit `OK` pour un symbole mal écrit (`scisors`, sonde 11), une image absente (sonde 12), une app introuvable (`DaVinci Resolv`, sonde 13), un AppleScript faux (sonde 14), une couleur courte `#F00` qui sera ignorée sans un mot (sonde 10, `vues.rs::couleur` exige 6 chiffres), un mauvais bundle id (`com.blackmagicdesign.resolve`, sonde 24), une app déclarée deux fois (sonde 22, la première gagne), une barre qui déborde (le calcul de `debordement` n'est fait qu'au lancement, et seulement pour la principale : `barre/mod.rs::installer`). Les barres d'app ne sont jamais contrôlées en largeur, même au lancement.
11. **Messages à moitié traduits** : `type invalide: sequence, attendu a string`, `valeur invalide: map, attendu map with a single key`, et les listes de champs attendus mélangent français et anglais (`«text», «texte», «symbol», «symbole»…`). Petit détail aussi : `OK : 1 éléments`.
12. **Pas de seuil sur `batterie`** (`seuil` refusé, sonde 18), ni de minutes restantes : `batterie.rs` ne lit que le pourcentage et la charge. Pour ma batterie BTT rouge sous 15 %, il faut retourner au `script` JSON.
13. **Coût caché de `herite`** : chaque barre d'app reconstruit ses propres copies des éléments hérités, et `rafraichi.rs::rafraichir` fait tourner tous les rafraîchissements, barre visible ou pas. Avec mon fichier : le script disque tourne deux fois, `pmset` trois fois (principale, copie Resolve, copie FCP). C'est négligeable ici, mais sur mon Intel avec cinq apps et des scripts à 1 s, ça se multiplie.
14. **La fenêtre « Personnaliser ma barre » réécrit le fichier sans les commentaires** (`config.rs::ecrire` = en-tête fixe + `toml::to_string_pretty`). Mes explications AZERTY disparaîtraient au premier enregistrement depuis la fenêtre. La copie `.precedent` me sauve une fois, pas deux.
15. **Ce qui m'a fait plaisir.** `{ applescript = '…' }` avec des apostrophes autour : plus aucune triple imbrication de guillemets. `'''` pour un script sur plusieurs lignes, qui laisse passer le JSON tel quel. Les raccourcis partent sans processus (`CGEventPost` direct), ce qui compte au rythme d'une coupe. `affichage = "icone"` avec un `texte` de secours, qui s'affiche si mon PNG manque.

Ce que la syntaxe ne permet pas, et dont je ne parlerai plus avant que les gestes arrivent : maintenir K+L (avance lente) ou laisser L enfoncé, puisqu'une `touche` est un appui-relâché instantané ; enchaîner deux raccourcis (pas de liste d'actions, sonde 19) ; un appui long sur un `script` (sonde 09).

## Tableau comparatif mis à jour

Légende : **Oui** = fait et visible dans le code ; **Partiel** = possible avec un détour ou des limites ; **Non** = absent. ↑ = progrès depuis la 0.1. Importance de 1 à 5, comme en 0.1.

| Fonction BTT | 0.1 | 0.2 | Preuve (fichier source) | Importance |
|---|---|---|---|---|
| Bouton texte + icône + couleur de fond | Oui | **Oui** | `config.rs` `Element::Bouton` ; `elements.rs::construire` (`setBezelColor`) | 5 |
| Icône personnalisée (PNG, logo) | Non | **Oui** ↑ (boutons et JSON de script ; pas les groupes) | `config.rs` `image`, `gabarit` ; `vues.rs::image` ; `rafraichi.rs::Riche.image` | 4 |
| Couleur du texte, taille, icône seule / texte seul | Non | **Oui** ↑ | `config.rs` `couleur_texte`, `taille`, `affichage` ; `vues.rs::styler` ; `elements.rs::construire` | 3 |
| Largeur fixe | Oui | **Oui** (et sur les groupes) | `config.rs` `largeur` sur `Groupe` ; `vues.rs::largeur_fixe` | 3 |
| **Barre différente selon l'app active** | Non | **Oui** ↑ (une app par bloc, `herite` tout ou rien) | `config.rs` `Application`, `application_pour`, `elements_avec` ; `barre/mod.rs` `base`, `suivre_app` | **5** |
| Garder la barre d'Apple dans certaines apps | Partiel | **Partiel** (toujours à la main) | `actions/mod.rs` `BarreApple` ; `barre = "apple"` refusé (sonde 02) | 4 |
| **Raccourci clavier** (⌘K, ⇧⌘E…) | Non | **Oui** ↑ (trous AZERTY : chiffres, caractères à modificateur, pas de `+`) | `actions/clavier.rs` `Touche`, `raccourci`, `code_du_caractere` | **5** |
| Touche par code avec modificateurs | Non | **Partiel** ↑ (`keycode` sans modificateur) | `actions/mod.rs` `Keycode(code) => touche(*code, 0)` | 3 |
| Lancer une app | Partiel | **Oui** ↑ | `actions/mod.rs` `App` → `open -a`, erreur au journal | 4 |
| Ouvrir une URL | Partiel | **Oui** ↑ | `actions/mod.rs` `Url` → `open` | 3 |
| Exécuter un AppleScript | Partiel | **Oui** ↑ (texte en ligne ; un `.scpt` reste un `shell`) | `actions/mod.rs` `Applescript` → `osascript -e` | 4 |
| Widget shell, sortie texte, intervalle | Oui | **Oui** (+ `delai`, stderr au journal, processus tués au rechargement) | `config.rs` `Script` ; `rafraichi.rs::executer_script` | 5 |
| Widget script qui change son icône et sa couleur (JSON) | Non | **Oui** ↑ (pas l'action) | `rafraichi.rs` `Riche`, `analyser_sortie`, `appliquer` | 4 |
| Widget AppleScript à intervalle | Partiel | **Partiel** (`script` + `osascript`) | `config.rs` `Script` | 3 |
| Widget batterie prêt à l'emploi | Partiel | **Oui** ↑ pour le %, **Non** pour les minutes restantes et le seuil rouge | `config.rs` `Batterie` ; `barre/batterie.rs` (`pourcent`, `en_charge` seulement) | 3 |
| Météo, calendrier prêts à l'emploi | Partiel | **Partiel** (en `script`) | aucun type dédié dans `config.rs` | 3 |
| Date / heure formatée | Oui | **Oui** | `config.rs` `Heure` ; `rafraichi.rs` (`NSDateFormatter`) | 4 |
| Now Playing : titre + artiste | Oui | **Oui**, toujours 44 caractères sans défilement | `rafraichi.rs` `.take(44)` | 4 |
| Now Playing : pochette | Non | **Non** | `rafraichi.rs` : un titre seulement | 3 |
| Lecture / pause, suivant, précédent | Oui | **Oui** | `actions/mod.rs` | 4 |
| Groupe pliable | Oui | **Oui** (texte, symbole, largeur ; ni image ni couleur) | `config.rs` `Groupe` ; `elements.rs::depliable` ; sondes 07, 08 | 4 |
| Groupe imbriqué, retour au parent | Partiel | **Partiel** (✕ ramène toujours la barre de base) | `barre/mod.rs::declencher` `Commande::Fermer => base()` | 2 |
| Fermer le groupe après une action | Non | **Non** | rien dans `Groupe` | 2 |
| Appui long | Oui | **Oui**, 0,5 s fixe, boutons seulement | `elements.rs::bouton` (`setMinimumPressDuration: 0.5`) ; sonde 09 | 3 |
| Double appui | Non | **Non** | — | 1 |
| Slider volume / luminosité | Partiel | **Partiel** (sous-barre) | `elements.rs::slider` → `depliable` | 3 |
| Luminosité d'un écran externe | Non | **Non** | `README.fr.md` « Limites connues » | 2 |
| Slider personnalisé piloté par script | Non | **Non** | `config.rs` `Reglage` : `Luminosite`, `Volume` | 2 |
| Gestes sur la Touch Bar | Non | **Non** | `README.fr.md` « Limites connues » | 3 |
| Enchaîner plusieurs actions (macro) | — | **Non** | `action` = une seule `Action` ; sonde 19 | 2 |
| Boutons conditionnels, barre alternative ⇧ / ⌥ | Non | **Non** | rien dans `config.rs` | 3 |
| App switcher, emoji, Dock | Non | **Non** | — | 1 |
| Masquer le Control Strip, touche esc | Oui | **Oui** (`true` par défaut, comme documenté) | `config.rs` `masquer_control_strip` ; `Action::Echap` | 4 |
| Retour haptique | Non | **Non** | — | 1 |
| Éditeur visuel | Non | **Partiel** ↑ (barre principale seulement ; garde les `[[application]]` mais efface les commentaires) | `src/editeur/mod.rs` ; `config.rs::ecrire` ; `README.fr.md` « Une barre par application » | 3 |
| Vérifier un fichier sans le charger | — | **Partiel** (syntaxe seulement ; lignes fausses dans les apps) | `main.rs` `--verifier` ; `config.rs::lire`, `element_fautif` | 4 |
| Erreur visible sans ouvrir de journal | Non | **Oui** ↑ (« ⚠ config ligne N » dans la barre) | `barre/mod.rs::afficher_erreur` | 4 |
| Partager une config en anglais | Non | **Oui** ↑ | alias serde dans `config.rs`, `actions/mod.rs` ; `exemples/barre.en.toml` | 2 |
| **Import de mes presets BTT** | Non | **Non** | rien dans `src/` | **5** |
| Rechargement sans relancer | Oui | **Oui** | `config.rs::surveiller` | 4 |
| Lancer au démarrage | Oui | **Oui** | `menu.rs` | 4 |
| Mac Intel testé en vrai | Non | **Non** (Rosetta seulement) | `README.fr.md` « Vérifié sur » | 4 |

Bilan : 13 lignes passent de Non ou Partiel à Oui ou Partiel. Les deux lignes à 5 qui me bloquaient (barre par app, raccourcis) sont faites. Il reste une ligne à 5, l'import.

## Migrerait-elle ?

**Je lance l'essai, je ne désinstalle pas encore BTT.** En 0.1, j'avais écrit que si la barre par app et les raccourcis existaient, je garderais BTT installé mais désactivé une semaine et je retaperais mes barres à la main. Les deux existent, j'ai retapé mes barres en une heure environ, ce que j'avais prévu. Je tiens parole : semaine d'essai.

Ce qui m'empêche encore de désinstaller BTT, dans l'ordre :

1. **Les raccourcis à chiffres et à caractères spéciaux sur AZERTY.** C'est mon quotidien dans Resolve (pages ⇧4 à ⇧8, `cmd+\`) et chez beaucoup de monteurs français. Le détour `shift+§` marche mais ne se devine pas, et `cmd+\` n'a pas d'autre issue que l'AppleScript, plus lent. Tant qu'un `"shift+6"` qui passe `--verifier` peut envoyer la mauvaise touche, je n'ai pas confiance dans ma barre en séance.
2. **Pas de barre d'Apple automatique par app** (`barre = "apple"`) ni de liste d'apps. Dans Finder, Safari, Mail, BTT me rendait la barre d'Apple tout seul ; ici je dois toucher un bouton à chaque fois.
3. **Les erreurs mal situées et les pièges silencieux** : la ligne fausse dans une `[[application]]`, l'`[[element]]` égaré qui part dans la principale sans un mot, l'absence de contrôle de largeur des barres d'app. Avec trois barres et 300 lignes, c'est là que je vais perdre du temps chaque fois que je retouche.
4. **Pas d'import BTT.** Pour moi, c'est fait (j'ai retapé). Pour mes deux collègues monteuses à qui je voudrais le conseiller, non : elles ne retaperont pas.
5. **Mon Mac est un Intel**, et personne ne l'a encore fait tourner sur un vrai Intel. Je serai la première, en sachant ce que je risque.

Ensuite, pour le confort mais sans bloquer : pochette et défilement dans `en_cours`, sliders en ligne ou gestes à deux doigts, seuil de couleur sur `batterie`.

## 3 propositions

### 1. Des touches qui marchent en AZERTY

- Pour un chiffre, viser la rangée du haut (keycodes 18 à 29, ceux que macOS utilise pour ⌘1 en AZERTY) avant le pavé numérique ; plus généralement, chercher aussi le caractère avec maj ou option et ajouter ce modificateur, pour que `cmd+\` trouve `⌥⇧:`.
- Accepter des modificateurs avec un keycode : `{ keycode = 22, modificateurs = "shift" }`, ou `touche = "shift+#22"`.
- Accepter le `+` lui-même : `cmd+plus` (ou `cmd++` reconnu comme cas particulier), et un message « pour la touche +, écris cmd+plus » au lieu de `modificateur inconnu «  »`.
- `--verifier` résout chaque `touche` sur la disposition courante et prévient : « ligne 129 : "6" se tape sur le pavé numérique avec ta disposition (French) ».

### 2. `[[application]]` : moins de recopie, plus de garde-fous

```toml
[[application]]
app = ["com.blackmagic-design.DaVinciResolve", "com.apple.FinalCut"]   # une barre, deux apps
herite = ["batterie", "heure"]        # ou true / false comme aujourd'hui : garde ces types-là seulement

[[application]]
app = ["com.apple.finder", "com.apple.Safari", "com.apple.mail"]
barre = "apple"                       # la barre d'Apple tant que l'app est au premier plan
```

Et un avertissement à la lecture quand un `[[element]]` apparaît après le premier `[[application]]` : « ligne 141 : cet [[element]] va dans la barre principale ; voulais-tu [[application.element]] ? ». Avertir aussi d'une app déclarée deux fois.

### 3. Un `--verifier` qui vérifie ce qui casse à l'usage

- Corriger la ligne des erreurs dans les `[[application]]` (descendre dans `application.element` dans `element_fautif`, comme pour `element`), et lire les erreurs dans l'ordre du fichier.
- Prévenir (sans refuser) : symbole SF inconnu, image absente, couleur qui n'est pas `#RRGGBB`, app ou bundle id introuvable (LaunchServices), barre qui déborde, y compris chaque barre d'app avec ses éléments hérités.
- Un résumé par barre : `OK : principale 13 éléments ; DaVinci Resolve 16 ; Final Cut Pro 9 (+ 8 hérités) ; 2 avertissements`.
- Finir de traduire les messages de serde (`sequence`, `a string`, `map with a single key`) et ne lister que les noms de champs dans la langue du fichier.
