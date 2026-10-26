# Retour de Marc, testeur « avancé » — Tactile 0.2 (branche main, section Unreleased)

## Contexte

- Marc, 41 ans, dev backend. MacBook Pro 13" M1 Touch Bar, macOS 15.8, disposition clavier **French (AZERTY)**. C'est lui qui a écrit le retour 0.1 ; il revient vérifier ses bugs et pousser les nouveautés dans leurs coins.
- Build : `mise exec -- cargo build --release -q` (binaire de 2,0 Mo, contre 1,3 Mo en 0.1). Tactile lancé depuis un terminal, stderr vers un journal à part. La config a été modifiée à chaud dans `~/.config/tactile/barre.toml` ; elle était sauvegardée avant et a été restaurée à la fin (voir la dernière section).
- Lu avant de tester : CHANGELOG (Unreleased), README, `docs/retours/0.1/SYNTHESE.md`, `exemples/` (dont `par-application.toml` et `barre.en.toml`), puis `src/config.rs`, `barre/{mod,elements,rafraichi,vues,batterie,autotest}.rs`, `actions/{mod,clavier}.rs` et `editeur/mod.rs`.
- **Limite importante : l'écran était verrouillé pendant toute la session** (`CGSSessionScreenIsLocked = 1`, app au premier plan : `com.apple.loginwindow`). La Touch Bar, elle, s'affiche et se capture normalement (`screencapture -b`), donc tout ce qui est visuel sur la barre a été vu. En revanche :
  - je ne peux pas toucher la barre : l'accès Accessibilité m'est refusé (`kAXErrorAPIDisabled`). Les actions (`touche`, `app`, `url`, `applescript`) ont été exercées par l'autotest intégré (`TACTILE_AUTOTEST=1`, qui appelle `performClick:` dans le processus) et par lecture du code ;
  - la fenêtre de l'éditeur ne se dessine pas (sa capture est blanche). Je l'ai pilotée avec `TACTILE_EDITEUR_JS` et j'ai vérifié les fichiers écrits, sans voir l'interface ;
  - `masquer_control_strip = false` : sous verrouillage, même la barre d'Apple est noire, donc le test n'est pas concluant (voir B4).
- Captures : `docs/retours/0.2/captures/marc-*.png`.

## Anciens bugs

| 0.1 | État | Preuve |
|---|---|---|
| **B1** symbole SF faux → panique, barre noire | **Corrigé** | `terminal.fil`, `symbole.qui.nexiste.pas` (avec texte), `speaker.wav.2` (slider), `hammer.filll` (groupe), `pas.un.symbole` (sous-élément) : plus de panique, ni à chaud ni au lancement. Le nom s'affiche en texte et le journal dit `symbole SF inconnu « terminal.fil » (élément 1)`, avec `élément 4.1` pour un sous-élément (`marc-symboles-faux.png`). Petit reste : un slider au symbole faux n'affiche que « › », sans nom. Pas de badge « ⚠ config » pour ce cas, ce qui est logique : la config est valide. |
| **B2** texte long qui avale la barre | **Corrigé** | Bouton de 400 caractères et `heure` au format `EEEE d MMMM yyyy 'à' HH:mm:ss zzzz` : coupés avec « … » à 200 pt, et les voisins restent visibles (`marc-texte-long-tronque.png`). |
| **B3** débordement silencieux | **Partiel** | 25 symboles : `la barre déborde (environ 2000 pt pour 1004) : 12 élément(s) sur 25 seront cachés`, et c'est juste. 20 boutons « Bouton n » : 10 cachés annoncés, 10 cachés réels. **Mais** 15 boutons à texte court (« B1 »… « B15 ») : 3 cachés et **aucun message** (`marc-15-boutons-3-caches-sans-message.png`). L'estimation compte 9 pt × 2 caractères + 24 = 42 pt, alors qu'un bouton fait au moins ~72 pt à l'écran. Même chose pour 8 scripts (7 × 120 + 130 pt) : le 8e est caché sans message, sous la tolérance de 5 %. Avec `largeur = 100000`, le journal dit « 2 élément(s) sur 2 seront cachés » alors que le premier s'affiche, en bloc gris vide qui remplit la barre (`marc-largeur-enorme.png`). |
| **B4** `masquer_control_strip = false` | **Non vérifiable ici** | À chaud, la barre change maintenant : `true → false` donne une barre noire, `→ true` la ramène, `→ false` la rend noire de nouveau. Le placement est donc bien réappliqué, ce qui n'était pas le cas en 0.1. Au lancement avec `false` : barre noire aussi. Sous écran verrouillé, la barre d'Apple (`TACTILE_MASQUER=1`) et la Touch Bar sans Tactile sont **elles aussi noires** : je ne peux pas dire si c'est un bug. À refaire écran déverrouillé. Le journal annonce correctement `environ 815 pt pour 685 : 3 élément(s) sur 9 seront cachés`. |
| **B5** un orphelin par rechargement | **Corrigé** | `sleep 1000; echo fini`, `intervalle = 1` : `tué après 5 s` au journal. Après 3 rechargements, une seule paire `zsh -c` + `sleep 1000` est vivante, celle de la barre courante (`pgrep -fl sleep`). `delai = 2.5` sur `sleep 3` : `tué après 2.5 s`. Un `<defunct>` passager apparaît puis disparaît ; il ne s'accumule pas sur 24 s. |
| **B6** stderr et code retour jetés | **Corrigé (journal)** | `code 127, zsh:1: command not found: commande_inexistante_xyz`, `code 0, sur stderr`, `code 1`. C'est écrit une fois par changement, pas à chaque intervalle. La case reste vide, sans « ⚠ » : il faut toujours penser à ouvrir le journal. |
| **B7** `intervalle = 0` accepté en silence | **Toujours là** (dans le fichier) | `intervalle = 0` et `delai = -3` : `--verifier` répond OK, rien au journal, ramené à 0,1 s. L'éditeur, lui, refuse un intervalle < 0,1 : les deux chemins ne valident pas la même chose. |
| **B8** couleur invalide ignorée | **Toujours là** (dans le fichier) | `couleur = "rouge"`, `couleur_texte = "#FFF"` : bouton gris, rien au journal, `--verifier` OK (`marc-apparence.png`). `largeur = -50` : le bouton disparaît sans message. L'éditeur refuse les couleurs mal écrites, pas le fichier. |
| **B9** bruit AppKit dans le journal | **Toujours là** | `This NSLayoutConstraint is being configured with a constant that exceeds internal limits` à chaque lancement, et `Expected min height of view … got a height of 36` dès qu'une sortie fait deux lignes. |
| Friction : libellé de groupe tronqué, pas de `largeur` | **Corrigé** | `largeur = 180` + « Développement » + symbole : tout tient (`marc-apparence.png`). |
| Friction : fichier supprimé sans message | **Toujours là** | `rm barre.toml` : la barre par défaut revient, rien au journal. |

## Nouveaux bugs

### N1 — Fuites mémoire : les anciennes barres et les images ne sont jamais libérées — **majeur** (proche du bloquant sur un Mac 8 Go)

- Étapes et mesures (`top -stats mem`, processus neuf à chaque fois) :
  1. barre par défaut, 20 rechargements (un commentaire ajouté à chaque fois) : **14 Mo → 32 Mo**, soit environ +0,9 Mo par rechargement ;
  2. un bouton `image` = PNG de 10 000 × 10 000 px : 431 Mo. On recharge la même barre une deuxième fois : **830 Mo**. Retour à la barre par défaut : **831 Mo**, rien n'est rendu ;
  3. un `script` à `intervalle = 1` qui alterne `{"texte":"ok","image":"…/icone.png","couleur":"#B00020"}` et du JSON cassé : **20 Mo → 80 Mo → 141 Mo** en 30 puis 60 s, soit environ +2 Mo/s. Témoin : le même script avec `symbole` à la place de `image` reste stable (11 → 12 Mo en 40 s).
- Attendu : mémoire stable, l'ancienne barre libérée au rechargement.
- Obtenu : tout s'accumule. Au rythme du cas 3, un script météo ou CI avec une icône de fichier remplit la RAM d'un Mac 8 Go dans la journée. Le cas 1 touche tout le monde, et chaque « Enregistrer » de l'éditeur est un rechargement.
- Piste : `vues::image` fait `alloc` + `initWithContentsOfFile` (+1) sans jamais relâcher, à chaque construction de barre et à chaque nouvelle sortie JSON ; `installer` remplace `Etat` mais les `NSTouchBar`, boutons et items (`Id` bruts) ne sont jamais relâchés.

### N2 — AZERTY : `cmd+shift+4` envoie le 4 du pavé numérique — **majeur** (à confirmer écran déverrouillé)

- Étapes : sur la disposition « French » (celle de ce Mac), résoudre les caractères comme `clavier::code_du_caractere` (UCKeyTranslate sans modificateur, codes 0 à 127, premier trouvé). Je l'ai reproduit avec un petit programme Swift qui suit le même algorithme.
- Obtenu : `4 → 86`, `1 → 83`, `/ → 75`, c'est-à-dire les touches du **pavé numérique**. En AZERTY, la rangée du haut donne `& é " ' (`… sans Maj, donc le premier code qui produit « 4 » est celui du pavé. `[` est introuvable (il faut ⌥⇧).
- Attendu : `cmd+shift+4`, l'exemple vedette du README et du CHANGELOG, doit déclencher la capture de zone, qui est liée au keycode 21.
- Conséquence probable : les raccourcis système liés au keycode (⌘⇧3, 4, 5) ne réagissent pas en AZERTY. Les raccourcis de menu (⌘1 dans une app) peuvent marcher, puisque le pavé produit aussi « 1 ». Je n'ai pas pu appuyer pour le prouver (écran verrouillé, pas d'Accessibilité). L'autotest n'essaie que `f15`, qui ne dépend pas de la disposition.
- Aggravant : une touche introuvable (`cmd+[` en AZERTY) passe `--verifier` et le rechargement ; l'erreur n'apparaît qu'à l'appui, dans le journal.

### N3 — Mélanger `[[element]]` et `[[item]]` dans un même fichier est refusé — **majeur**

- Étapes : `exemples/barre.toml`, puis on colle à la fin un bloc copié de `barre.en.toml` : `[[item]]` + `type = "battery"`.
- Attendu (README, glossaire) : « Les deux formes peuvent se mélanger ».
- Obtenu : toute la config est refusée avec `ligne 1 : duplicate field «element»`. La ligne est fausse (le bloc est en ligne 58) et le message est en anglais. Même chose avec `[[application.element]]` + `[[application.item]]`. Le mélange **à l'intérieur** d'un élément marche, lui (`type = "button"`, `texte`, `color`, `largeur`, `[[item.element]]` : barre correcte à la capture).
- Un doublon FR/EN dans un même élément (`width` + `largeur`, `symbol` + `symbole`, `hide_control_strip` + `masquer_control_strip`) est refusé : c'est le bon comportement. Mais le message reste `duplicate field «largeur»`, non traduit (ligne juste dans ce cas).

### N4 — Le badge « ⚠ config » est invisible quand une barre d'application est affichée — **majeur**

- Étapes : `[[application]]` qui correspond à l'app au premier plan (ici `com.apple.loginwindow`), puis une erreur dans le fichier (`type = "batteri"`).
- Attendu : le badge rouge, comme sur la barre principale (`marc-badge-config.png`).
- Obtenu : la barre de l'app reste telle quelle, sans badge ; seul le journal parle (`marc-badge-absent-barre-app.png`). `afficher_erreur` n'ajoute le bouton qu'à `barres[0]`.
- Pourquoi c'est majeur pour moi : mon premier usage de `[[application]]`, c'est VS Code. Or c'est justement dans VS Code que j'édite `barre.toml`. Pendant que j'écris la config, le retour visuel ajouté en 0.2 n'apparaît donc jamais.

### N5 — « Enregistrer » dans l'éditeur efface les commentaires d'un fichier écrit à la main — **majeur** (pour qui écrit sa config)

- Étapes : `TACTILE_CONFIG=<scratch>/barre.toml tactile --personnaliser` sur un fichier commenté (avec `image`, `keycode`, un script à `intervalle = 0.3`, une `[[application]]`). Le pilote JS ajoute un bouton puis clique « Enregistrer ».
- Ce qui marche : `image`, `gabarit`, `keycode`, `delai` et l'`[[application]]` traversent l'aller-retour. « Revenir à la version précédente » échange bien les deux fichiers : un retour donne l'original octet pour octet (commentaires compris), un deuxième retour redonne la version enregistrée, et `barre.toml.precedent` est alors identique à l'original (`cmp`). Un symbole faux est refusé : « Pas enregistré : élément 4 : le symbole « pas.un.symbole » n'existe pas », et rien n'est écrit.
- Le problème : le fichier enregistré n'a plus **aucun** de mes commentaires. Un en-tête générique les remplace, `gabarit = false` s'ajoute sur chaque bouton, et `action = { keycode = 105 }   # F13` devient une sous-table `[element.action]`. Une seule copie `.precedent` existe : au deuxième enregistrement, la version commentée est perdue pour de bon.
- Attendu : prévenir avant d'écraser un fichier écrit à la main, ou au moins garder une copie horodatée la première fois.

### N6 — Un fichier vide donne une Touch Bar entièrement noire, sans message — **majeur**

- Étapes : `: > ~/.config/tactile/barre.toml` (un éditeur qui enregistre un tampon vide, un `>` à la place de `>>`…).
- Obtenu : Touch Bar entièrement noire (aucun pixel allumé sur la capture), donc plus d'esc ni de bouton  ; rien au journal ; `tactile --verifier` répond `OK : 0 éléments`. On s'en sort par le menu de la barre des menus, mais c'est exactement le symptôme du bloquant de la 0.1.
- Attendu : refuser une barre sans élément (badge + journal), ou au moins garder esc.

### N7 — `couleur_texte` aligne le texte à droite sur Apple Silicon — **mineur**

- Étapes : `texte = "Centré ?"`, `couleur_texte = "#FFD700"`, `largeur = 200`, à côté d'un bouton témoin sans couleur.
- Obtenu : le texte coloré est collé à droite, le témoin est centré (`marc-couleur-texte-alignee-droite.png`). Même chose pour la sortie JSON d'un script avec `couleur_texte`.
- Cause probable : `vues::styler` fait `setAlignment: 2`. Sur arm64, `NSTextAlignment` utilise les valeurs iOS (centre = 1, droite = 2) ; sur Intel, 2 = centre. Il vaut mieux utiliser la constante `NSTextAlignment::Center`.

### N8 — Un JSON cassé après un JSON valide garde l'image et la couleur précédentes — **mineur**

- Étapes : script qui alterne `{"texte":"ok","image":"…","couleur":"#B00020"}` et `{"texte":"cassé"`.
- Obtenu : le texte brut `{"texte":"cassé"` s'affiche sur fond rouge, avec l'icône de la sortie d'avant (`marc-json-casse-garde-style.png`). Si on passe par du texte brut entre les deux, tout est bien remis à zéro.
- En plus : à chaque alternance, l'erreur change (`""` ↔ `JSON illisible`) et elle est réécrite au journal. 29 lignes en 60 s pour un seul script, soit environ 40 000 par jour.

### N9 — Sortie de script sur plusieurs lignes — **mineur**

- `printf 'ligne1\nligne2\n'` : les deux lignes s'affichent l'une sur l'autre, rognées, et AppKit écrit `Expected min height … 36` (B9). En 0.1, seule la première ligne comptait.
- `echo '{"texte":"l1"}'; echo ligne2` : `JSON illisible (trailing characters at line 2 column 1)`. Un script qui affiche un message de débogage après son JSON casse son affichage. Lire seulement la première ligne (comme le dit le README de la 0.1) réglerait les deux cas.

### N10 — `--verifier` ne vérifie que la syntaxe — **mineur** (mais trompeur)

- Il répond `OK` pour un symbole faux, `couleur = "rouge"`, `intervalle = 0`, `largeur = -50`, `taille = 1000000`, `keycode = 9999`, `url = ""`, `app = ""`, deux `[[application]]` pour la même app, et un fichier vide. L'éditeur, lui, refuse les symboles, les couleurs et l'intervalle : il y a trois validations différentes (fichier, rechargement, éditeur).
- `OK : 1 éléments` pour `par-application.toml` : il ne compte que la barre principale, pas les 2 applications (et il manque l'accord du pluriel).
- `tactile --verifier a b` répond `argument inconnu « --verifier »`, ce qui est trompeur.
- Fichier absent : `impossible de lire …`, alors que Tactile, lui, prendrait la barre par défaut. Il faudrait le dire.

### Autres mineurs

- `tactile --version` affiche `tactile 0.1.0` sur la 0.2 : impossible de savoir quelle version on teste.
- Messages encore en anglais : `duplicate field`, `attendu a boolean`, `attendu map with a single key` (deux actions dans une table), `EOF while parsing`, `unknown field … expected one of` (JSON).
- `touche = "cmd++"` : impossible d'écrire la touche « + » (`modificateur inconnu «  »`).
- Deux `[[application]]` pour la même app : la première gagne en silence. `app = ""` est accepté et ne correspond jamais à rien.
- `image = "icone.png"` (chemin relatif) : cherché depuis le dossier courant de Tactile, pas depuis celui de la config ; ce n'est pas documenté. Un fichier qui existe mais n'est pas une image est annoncé « image introuvable ».
- `taille = 40` rogne le texte en haut et en bas ; `taille = -5` est accepté (texte plus petit), `taille = 0` revient au défaut. Rien n'est borné ni signalé.
- Le badge « ⚠ config » prend environ 140 pt : sur une barre déjà pleine, il pousse un élément de plus hors de la barre, et le débordement n'est pas recalculé.

## Frictions

- **README en retard sur la 0.2** : « Limites connues : Pas encore d'éditeur visuel », « *Limits* : no visual editor yet », « celle de l'éditeur à venir » dans le glossaire, et la ligne *Menu* de la section anglaise ne cite ni « Customize » ni « View the log ».
- **Les cases de scripts en échec restent vides.** Le journal est enfin utile (B6), mais rien dans la barre ne dit d'aller le lire.
- **CPU ×3 dès qu'il y a une `[[application]]`** : l'app au premier plan est interrogée toutes les 0,1 s (voir les mesures). Ça reste bas, mais une notification `NSWorkspaceDidActivateApplicationNotification` coûterait zéro au repos.
- **Le format écrit par l'éditeur** (`[element.action]` en sous-table, `gabarit = false` partout, `delai = 1.0`) est moins lisible et fait des diffs bruyants pour qui garde sa config dans git (mes dotfiles).
- **`TACTILE_EDITEUR_JS`** est très pratique pour automatiser, mais l'aide de `--help` ne le cite pas, contrairement aux autres variables.
- La batterie, les alias anglais, `affichage`, `largeur` sur groupe et `herite` font ce que dit le README, sans surprise. C'est noté, parce que c'est rare.

## Ce qui a marché (vérifié)

- Barre 100 % anglaise (`barre.en.toml`) identique à la barre par défaut. Mélange FR/EN dans un même élément accepté, y compris dans `[[item.element]]`.
- `[[application]]` : bundle id (`com.apple.loginwindow`), nom affiché en majuscules (`LOGINWINDOW`), `herite = true` (« Gauche » et l'heure gardées, la zone entre les `souple` remplacée), app inexistante → barre principale (`marc-application-herite.png`).
- Images : `~` étendu, `gabarit = true` teinte bien, chemin faux ou fichier non image → nom en texte + journal, PNG 10 000 × 10 000 affiché correctement (mais voir N1).
- `batterie` avec et sans symbole (`⚡ 100 %` en charge), `affichage = "icone" | "texte"` et `"icone"` sans icône (le texte reste).
- Scripts JSON : valide (symbole + fond + couleur du texte), cassé (affiché brut + journal), partiel (`{"text":"partiel"}`), champ inconnu (`titre`) refusé avec la liste des champs permis, type faux (`"texte": 12`), JSON précédé d'espaces traité comme du texte.
- Autotest intégré : 0 échec sur 29 étapes, dont `{ touche = "f15" }` envoyé, `{ app = "TextEdit" }` qui ouvre TextEdit, `{ applescript = 'tell application "TextEdit" to quit' }` qui le ferme, et une sous-barre gardée pendant un changement d'app, dont la ✕ ramène la barre de l'app (app simulée, écran verrouillé). Le test du slider de volume était trivial (volume à 0,00 → 0,00).
- Erreurs pour `app` et `applescript` : les messages `open « -a » « AppQuiNexistePas » : … Unable to find application` et `osascript … error` sont produits par `actions::attendre` (lu dans le code et le test unitaire, pas déclenchés au doigt).
- `--help` (10 lignes), argument inconnu → code 2, `--verifier` sur un fichier invalide → code 1 avec la ligne, `TACTILE_CONFIG` respecté par `--verifier` et l'éditeur, et le vrai `~/.config/tactile` n'a pas été touché pendant les essais de l'éditeur (aucun `.precedent` créé).

## Mesures

`top -l 3 -s 3 -pid … -stats cpu,mem` (puis 5 échantillons pour confirmer), processus neuf, écran verrouillé :

| Barre | CPU | Mémoire |
|---|---|---|
| Par défaut (au repos) | 0,1–0,2 % | 14 Mo |
| Par défaut + une `[[application]]` active (`herite`) | 0,5–0,7 % | 14 Mo |
| Par défaut + `batterie` + 2 scripts à 1 s (`date`, `uptime`) | 0,7 % | 14 Mo |
| Après 20 rechargements de la barre par défaut | — | 32 Mo (N1) |
| Après 2 rechargements d'une barre avec un PNG 10 000² | — | 830 Mo (N1) |
| Script JSON avec `image`, 60 s | — | 20 → 141 Mo (N1) |

Le « 0,5 % CPU, 13 Mo » du README tient au repos ; il ne tient plus avec une `[[application]]` (CPU), ni après une journée de rechargements ou d'images (mémoire).

## Manques restants (classés par valeur pour Marc)

1. **Une seule validation**, partagée par `--verifier`, le rechargement et l'éditeur : symboles, couleurs, intervalle, largeurs, tailles, fichier vide, touche résolue sur la disposition. Aujourd'hui, `--verifier OK` ne garantit rien de ce qu'on verra.
2. **Un éditeur qui respecte un fichier écrit à la main** : avertir avant d'écraser des commentaires, ou garder une copie horodatée.
3. **« ⚠ » dans la case d'un script en échec** (code ≠ 0, tué, JSON illisible), le détail restant dans le journal.
4. **`tactile --recharger` / `SIGHUP`** pour les configs générées (toujours absent).
5. **Afficher la touche résolue** : `tactile --verifier` qui écrit `cmd+shift+4 → keycode 21 (AZERTY)` lèverait N2 d'un coup d'œil.
6. **Appui long sur `script` et `groupe`, état basculé** (muet : symbole qui change) : toujours absents.
7. **`locale` pour `heure`** (ici `Monday 26 October`, alors que je voudrais du français sans changer la langue du système).

## 3 propositions concrètes

1. **Libérer ce qui est créé (N1).** Garder les barres et items en `Retained<…>` dans `Etat` plutôt qu'en `Id` bruts, pour qu'un remplacement les relâche, et faire rendre un `Retained` (ou un `autorelease`) à `vues::image`. Ajouter un test qui recharge 100 fois une barre avec une image et vérifie que l'empreinte (`task_info` / `phys_footprint`) ne dépasse pas, disons, +5 Mo. C'est le seul point qui peut dégrader un Mac 8 Go en une journée.
2. **Une fonction `config::valider` appelée par `lire` (N3, N4, N6, N10, B7, B8).** Elle fusionnerait `element` et `item` au lieu de refuser le doublon, signalerait barre vide, symbole, couleur, image, intervalle, largeur et taille, avec numéro de ligne, et servirait aussi à l'éditeur. En parallèle, `afficher_erreur` devrait ajouter le badge à la barre affichée (base courante, barre d'app comprise) et pas seulement à `barres[0]`.
3. **Clavier et petits rendus (N2, N7, N8, N9).** Dans `code_du_caractere`, chercher aussi avec Maj et exclure le pavé numérique (codes 65 à 92), pour que `4` donne 21 en AZERTY comme en QWERTY ; et le prouver par un test sur la disposition « French » chargée explicitement. Remplacer `setAlignment: 2` par la constante `NSTextAlignment::Center`. Remettre image et fond à zéro quand une sortie JSON est illisible. Ne garder que la première ligne d'une sortie.

## Restauration

- `~/.config/tactile/barre.toml` restauré depuis la sauvegarde (`cmp` identique) ; seul `barre.toml` est présent dans `~/.config/tactile/`.
- `pkill -x tactile`, puis `target/release/tactile 2>/dev/null &` relancé (pid 51410, vérifié par `pgrep -x tactile`). Capture finale conforme à la barre de départ : , luminosité, volume, ⏪, ⏩, heure, ✳ Claude.
- TextEdit fermé (ouvert puis fermé par l'autotest), aucun `sleep 1000` restant, dossier temporaire `~/.cache/marc-tactile` supprimé.
- Aucun fichier du dépôt modifié hormis ce rapport et les captures de `docs/retours/0.2/captures/` (10 fichiers, tous sous 40 Ko). Rien n'a été commité.
