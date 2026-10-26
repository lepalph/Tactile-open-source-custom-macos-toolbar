# Retour de Nadia, débutante : Tactile 0.2 (main)

## Contexte

Nadia, 34 ans, graphiste. MacBook Pro 13" M1 avec Touch Bar, macOS 15. Elle n'a jamais ouvert de terminal et ne sait pas ce qu'est TOML. Elle avait testé la 0.1 ([rapport](../0.1/nadia-debutante.md)) et s'était arrêtée sur Gatekeeper. Cette fois elle lit `README.fr.md` (sa langue) et ce que l'app affiche. Son objectif ne change pas : une barre avec « mes 3 apps préférées (Figma, Photoshop, Spotify), le volume et l'heure », mais **sans écrire de TOML**, avec la nouvelle fenêtre « Personnaliser ma barre ».

Méthode :

- Lecture de `README.fr.md` ligne par ligne, puis du code de la fenêtre (`src/editeur/editeur.html`, `src/editeur/mod.rs`).
- La barre n'a pas été lancée : la Touch Bar était occupée par un autre test. Seule la fenêtre a été ouverte (`tactile --personnaliser`), sur un fichier à moi (`TACTILE_CONFIG=…/scratchpad/nadia2/barre.toml`), pilotée par script (`TACTILE_EDITEUR_JS`). Les clics passent par les vrais boutons et champs de la page, qui envoient les vrais messages à Rust. J'ai relevé à chaque étape l'état de la page et le contenu du fichier écrit.
- **L'écran était verrouillé** : la capture de la fenêtre (`capture`) est sortie toute blanche. Je juge donc l'aspect à partir d'`editeur.html` et des captures du dépôt (`docs/editeur-musique.png`, `docs/editeur-erreur.png`). Le glisser-déposer, le sélecteur de couleur macOS et la fenêtre « Choisir… » (`NSOpenPanel`, modale) n'ont pas été manipulés à la main.
- Scripts et journaux : `/private/tmp/claude-501/-Users-palphinho/03bc99cc-4742-4072-87c4-fe525a6d77ae/scratchpad/nadia2/` (`p1.js` = parcours complet, `p2.js` = fichier abîmé, `p3.js` = Revenir sans version précédente, `*.log` = résultats).

Verdict Gatekeeper : **oui, cette fois je passerais** sur Sequoia en suivant `README.fr.md:51` (double-clic, **Terminé**, Réglages Système → Confidentialité et sécurité → **Ouvrir quand même**, Touch ID). C'est exactement l'écran que macOS 15 affiche. Deux réserves : il n'y a toujours aucune capture de ces écrans, et le bouton « Ouvrir quand même » n'apparaît dans Réglages que pendant environ une heure après la tentative bloquée, ce que le README ne dit pas.

## Anciens problèmes (0.1)

| 0.1 | État | Preuve |
|---|---|---|
| B1 Gatekeeper : « clic droit → Ouvrir » inopérant sur macOS 15 (bloquant) | **Réglé** (dans la doc ; l'app reste signée ad hoc et non notarisée) | `README.fr.md:50-53` donne le vrai chemin Sequoia, celui de macOS 13-14 et l'option Terminal ; `README.fr.md:270` le répète dans Dépannage ; `README.fr.md:56` explique pourquoi l'app n'est pas notarisée. Les notes de release le disent aussi (`.github/workflows/release.yml:26-37`), mais **en anglais seulement**. `codesign -dv` : `Signature=adhoc`. |
| B2 `masquer_control_strip` à `false` quand la ligne est absente (majeur) | **Réglé** | `src/config.rs:16` `#[serde(default = "vrai")]`, testé `src/config.rs:783` (`lire("")` donne `true`). `README.fr.md:118` dit « `true` par défaut ». Dans la fenêtre, la case « Cacher la bande de boutons d'Apple, à droite » est cochée par défaut (`editeur.html:255`) et le fichier écrit contient `masquer_control_strip = true`. |
| B3 Erreur de fichier totalement silencieuse (majeur) | **Réglé dans la fenêtre, partiel pour le fichier écrit à la main** | Dans la barre : un élément rouge « ⚠ config ligne N » (`src/barre/mod.rs:257-290`, `README.fr.md:110`). Dans le menu : « Voir le journal » (`src/menu.rs:152`). Dans la fenêtre, rien n'est écrit si quelque chose cloche, et le message est en français : « Pas enregistré : élément 2 : le symbole « pinceau » n'existe pas ». Reste partiel : l'erreur TOML type de la débutante (`couleur = #A259FF` sans guillemets) sort en anglais brut, « ligne 6 : string values must be quoted, attendu literal string », et ouvrir la fenêtre sur ce fichier mène au bug B1 ci-dessous. |
| B4 Couleur mal écrite ignorée sans un mot (mineur) | **Réglé dans la fenêtre, toujours là dans le fichier** | Avec la fenêtre, on ne peut plus mal écrire une couleur (sélecteur macOS), et `verifier` refuse tout ce qui n'est pas `#RRGGBB` (`src/editeur/mod.rs:183-185`). Mais un fichier avec `couleur = "#FFF"` et `symbole = "pinceau"` passe `tactile --verifier` : « OK : 1 éléments ». |
| B5 Pas d'icône (mineur) | **Réglé** | `assets/Tactile.icns` est copié par `scripts/empaqueter.sh`, et l'Info.plist contient `CFBundleIconFile`. |
| B6 Page Releases vide (mineur) | **Réglé**, en anglais | `release.yml:26-37` : installation, prérequis, SHA-256. |
| Pas de moyen de lancer une app sans `shell` (manque) | **Réglé** | Action `{ app = "Figma" }` (`README.fr.md:152`), et dans la fenêtre « Lancer une application… » avec un bouton **Choisir…**. |

## Ce qui a marché

- **Je n'ai pas écrit une ligne de TOML.** Ma barre est faite entièrement dans la fenêtre : trois boutons Figma / Photoshop / Spotify, chacun avec texte, symbole, couleur de fond et action « Lancer une application… », puis un espace souple, un curseur de volume et l'heure. Le fichier écrit est propre, commenté en français en tête, et relu avant écriture (`p1.log`, phase P3).
- Les libellés de la palette sont dans ma langue : « Bouton », « Curseur », « Heure », « Musique en cours », « Espace souple ». La case du Control Strip dit enfin ce que c'est : « Cacher la bande de boutons d'Apple, à droite » (`editeur.html:178`).
- Les retours d'état sont clairs quand tout va bien : « Modifications non enregistrées. » puis « Enregistré. Ta Touch Bar est à jour. », en vert (`editeur.html:197`).
- Les trois barres toutes faites se chargent avec « Barre chargée, pas encore enregistrée. » : on voit que rien n'est encore écrit.
- Couleur : un vrai sélecteur de couleur et une case « gris de la barre » pour revenir à la couleur d'origine (`editeur.html:512`). Pour une graphiste, c'est le bon outil.
- Symbole : une grille d'icônes cliquables, et un symbole inexistant est signalé en rouge (« Ce symbole n'existe pas. ») avant même d'enregistrer (`editeur.html:502`).
- Ordre : ◀ ▶ dans l'en-tête de l'élément, glisser dans l'aperçu, ⌥ + flèches au clavier. Avec ◀ ◀, Spotify est passé en tête, et ce qui a été enregistré est conforme.
- Erreur volontaire : le symbole « pinceau » est refusé, rien n'est écrit, et la phrase est en français, encadrée en rouge.
- Langue : FR / EN bascule tout de suite, y compris le titre de la fenêtre, et le choix est retenu (`localStorage`).
- `Largeur (points)` a son repère : « la barre fait 1004 points » (`editeur.html:534`). C'était une friction de la 0.1.
- L'aide du format de l'heure donne `HH'h'mm` (`editeur.html:186`) : mon « 19h30 » est enfin possible.

## Bugs

### B1. Fenêtre ouverte sur un fichier abîmé : « Enregistrer » remplace ma barre par une barre vide (majeur)

- Étapes : `barre.toml` contient `couleur = #A259FF` (sans guillemets, mon erreur de la 0.1). J'ouvre « Personnaliser ma barre » pour réparer, puis je clique **Enregistrer** (`p2.js`).
- Attendu : la fenêtre me montre ma barre, ou au moins la ligne fautive, et m'aide à la corriger.
- Obtenu : l'aperçu dit « Ta barre est vide », et l'état affiche « **Pas enregistré :** /private/tmp/…/barre.toml, ligne 6 : string values must be quoted, attendu literal string ». Trois défauts dans cette seule ligne : « Pas enregistré » alors que je n'ai rien fait (le préfixe d'erreur est réutilisé pour une erreur de chargement, `src/editeur/mod.rs:122-126` et `editeur.html:289`), le chemin complet et l'anglais. Ensuite, **Enregistrer** répond « Enregistré. Ta Touch Bar est à jour. » et écrit `element = []` : ma Touch Bar devient vide. Ma barre n'existe plus que dans `barre.toml.precedent`, et « Revenir » remet… le fichier abîmé, donc la même erreur.

### B2. Fermer la fenêtre fait perdre les modifications sans prévenir (majeur)

- Étapes : modifier la barre, puis fermer la fenêtre (bouton rouge) sans cliquer Enregistrer.
- Attendu : « Enregistrer les modifications ? », comme dans toute app Mac.
- Obtenu (lu dans le code, non cliqué : écran verrouillé) : `windowWillClose:` ne regarde pas l'état « modifié » (`src/editeur/mod.rs:45-53`), et la page n'a ni `beforeunload` ni `confirm`. Tout est perdu en silence. Pour une débutante qui a passé dix minutes à régler ses couleurs, c'est rédhibitoire.

### B3. Essayer une barre toute faite efface ma barre en cours, sans retour possible (majeur)

- Étapes : ma barre est réglée (enregistrée ou non). Je clique « Simple » pour voir, puis « Développeur », puis « Musique » (`p1.log`, P1-P1c).
- Attendu : un aperçu, ou un moyen de revenir à ma barre.
- Obtenu : chaque clic remplace aussitôt la barre affichée, sans confirmation (`editeur.html:449-453`). Il n'y a pas de bouton « Annuler les changements ». « Revenir à la version précédente » ne recharge pas ma barre actuelle : il ramène l'**avant-dernier** enregistrement. Seule issue : fermer sans enregistrer et rouvrir, ce que rien ne m'indique (et la fermeture ne prévient pas, voir B2). « Essayer les barres toutes faites » devient donc risqué.

### B4. « Revenir à la version précédente » : sans confirmation, sans message, et mes modifications non enregistrées disparaissent (majeur)

- Étapes : je modifie la barre sans enregistrer, puis je clique « Revenir à la version précédente » (P6).
- Attendu : une confirmation, puis « Version précédente rétablie ».
- Obtenu : aucune confirmation (`editeur.html:600`). Mes modifications sont remplacées, l'état devient **vide** (pas de message du tout, `editeur.html:281-282`), la sélection disparaît, et ma Touch Bar change aussitôt puisque le fichier est réécrit (`src/editeur/mod.rs:80-82`). Le bouton reste actif, et un second clic revient à la version d'après (`revenir` échange les deux fichiers, `src/editeur/mod.rs:161`), ce que le libellé ne laisse pas deviner.

### B5. Un nom d'app tapé à la main n'est pas vérifié : le bouton ne fera rien (majeur)

- Étapes : « Lancer une application… », je tape « Photoshop » dans le champ (le texte d'exemple est « Safari », `editeur.html:483`), puis j'enregistre.
- Attendu : comme pour un symbole inconnu, un avertissement en rouge si l'app n'existe pas sous ce nom.
- Obtenu : enregistré sans remarque (`app = "Photoshop"`). Or le README dit lui-même qu'il faut le nom exact du dossier `.app` (`README.fr.md:160`), chez moi « Adobe Photoshop 2025 ». Au toucher, rien ne se passe, et l'erreur ne va qu'au journal (`README.fr.md:277`). Avec **Choisir…** le nom serait juste, mais rien ne dit que c'est le chemin sûr. (Non vérifié sur une vraie Touch Bar.)

### B6. « Revenir » est actif après le tout premier enregistrement et répond « Pas enregistré » (mineur)

- Étapes : aucun fichier au départ ; Enregistrer, puis Revenir (`p3.js`).
- Obtenu : le bouton s'active dès le premier enregistrement (`editeur.html:288` met `aPrecedent = true` sans savoir si une copie existe), et le clic affiche « Pas enregistré : pas de version précédente ».

### B7. Messages d'erreur dans la mauvaise langue (mineur)

- En anglais : « Not saved: élément 1 : le symbole « pinceau » n'existe pas » (P7). Seul le préfixe est traduit.
- En français : « barre illisible : modificateur inconnu « commande » ; attendu : cmd, shift, ctrl, alt, fn » (P5b). « barre illisible » fait peur pour une simple faute dans un raccourci, et le message ne dit pas quel élément est en cause.
- Erreur TOML : « string values must be quoted, attendu literal string » (B1).

### B8. Changer de langue efface le message d'erreur (mineur)

- Étapes : une erreur s'affiche, je passe en EN puis je reviens en FR (P8).
- Obtenu : l'encadré rouge est remplacé par « Modifications non enregistrées. » (`editeur.html:609`). L'erreur n'est plus visible tant que je ne réenregistre pas.

### B9. L'élément fautif n'est pas montré dans l'aperçu (mineur)

- « élément 2 : le symbole… » : il faut compter les éléments à la main, espaces souples compris. La barre ne surligne pas l'élément en faute, et la sélection ne s'y place pas (P5 : `sel` reste où il était, aucune classe d'erreur dans l'aperçu).

### B10. Les captures du README français montrent la fenêtre en anglais, avec une erreur (mineur)

- `README.fr.md:24` (« avec la barre Musique ») et `README.fr.md:279` affichent « Customize my bar », « Not saved… », « This symbol does not exist ». La première capture, celle qui présente l'éditeur, est même incohérente : le champ dit `apple.logo`, l'aperçu `truc.machin`, et le texte du bouton est « Sleep ». Ma toute première image de la fenêtre est donc une erreur, en anglais.

### B11. La « 0.2 » se présente encore comme 0.1.0 (mineur)

- `Cargo.toml:3` `version = "0.1.0"` (donc `--version`, Info.plist et zip), et `README.fr.md:31` dit « La version 0.1.0 se télécharge en un zip de 1 Mo ».

## Frictions

- `editeur.html:176` « Revenir à la version précédente » : à côté d'Enregistrer, même taille ; je ne sais pas si cela annule mes modifications en cours ou le dernier enregistrement (c'est le second). « Annuler le dernier enregistrement » serait plus juste.
- `editeur.html:182` « Couleur » : du fond ou du texte ? Le champ suivant s'appelle « Couleur du texte » ; il faudrait « Couleur du fond ».
- `editeur.html:183` « Afficher : automatique » : que fait « automatique » ? (C'est « les deux ».)
- `editeur.html:190` « Chercher un symbole (en anglais : play, moon, sun…) » : honnête, mais je tape « pinceau » et j'obtiens une grille vide, sans « aucun résultat » ni « essaie brush » (`editeur.html:504`). La grille ne montre que 48 symboles (`editeur.html:496`) sans dire qu'il y en a d'autres.
- `editeur.html:179` « Résultat d'une commande » dans la palette : un clic donne `date +%d/%m`, « Lancée avec zsh » (`editeur.html:187`), « Tuée après (secondes) » (`editeur.html:184`). Rien n'indique que c'est pour les développeurs.
- `editeur.html:191` « Commande shell », « AppleScript », « Autre (réglé dans le fichier) » dans la liste « Au toucher » : du jargon, au même niveau que « Lancer une application… ».
- `editeur.html:195-196` « Touche esc », « Raccourci clavier » : rien ne dit qu'il faut autoriser Tactile dans Accessibilité (`README.fr.md:61`). Sans cela, le bouton ne fait rien, en silence.
- `editeur.html:179` « Musique en cours », et les actions « Lecture / pause », « Morceau suivant » : ils exigent `brew install media-control` (`README.fr.md:60`), une commande de terminal. La barre « Musique » proposée par défaut aura donc un titre vide et des ⏪ ⏩ inertes, et la fenêtre ne le dit pas.
- `editeur.html:326` Un élément ajouté va toujours en fin de barre, après l'heure : pour un 4ᵉ bouton d'app, il faut cliquer ◀ plusieurs fois ou glisser.
- `editeur.html:291-294` Après **Choisir…**, le nom de l'app ne remplit pas le texte du bouton, et le bouton garde l'étoile par défaut (`editeur.html:310`), pas l'icône de l'app.
- `editeur.html:440` Suppr dans l'aperçu supprime sans confirmation, et il n'y a pas d'annulation (⌘Z).
- Fenêtre de 1160 × 720 (`src/editeur/mod.rs:278`) : sur mon 13", les « Barres toutes faites » sont sous la ligne de flottaison, en bas de la palette (sur `docs/editeur-musique.png`, seul « Simple » dépasse). Je pourrais ne jamais les voir.
- `editeur.html:379` Le curseur de volume apparaît comme un bouton avec « › » : rien ne dit qu'il ouvre une sous-barre.
- À l'ouverture sans fichier, la fenêtre montre la barre « Musique » sans un mot (état vide, `p0.log`) : « Voici la barre par défaut » rassurerait.
- Aucun bouton « Vider la barre » : pour repartir de zéro, 9 clics sur « Supprimer » (P2).
- `README.fr.md:90` et `README.fr.md:262` `tactile --personnaliser`, `TACTILE_CONFIG=…` : du terminal dans la section qui m'est destinée. Ça me va en option, à condition que ce soit présenté comme tel.
- `README.fr.md:222` « Pour partir de l'un d'eux, copie-le vers `~/.config/tactile/barre.toml` » : la fenêtre fait mieux (barres toutes faites), il faudrait le dire. `exemples/barre.toml:1` dit toujours « copie ce fichier dans ~/.config/… ».
- `README.fr.md:289` La désinstallation passe toujours par `rm -r` (inchangé depuis la 0.1).
- Le fichier écrit par la fenêtre contient `gabarit = false` sur chaque bouton : une clé inutile qui intrigue quand on ouvre le fichier.

## Manques

- Des captures du parcours Gatekeeper sur macOS 15 (alerte, puis Réglages → Ouvrir quand même), et la mention que ce bouton n'apparaît qu'un temps après le premier essai.
- Notes de release en français, ou au moins un lien vers `README.fr.md#installation`.
- Une protection des modifications : dialogue à la fermeture, confirmation avant une barre toute faite et avant Revenir, ⌘Z.
- Un mode « réparer » quand le fichier est abîmé : la ligne fautive, une phrase en français (« il manque des guillemets autour de `#A259FF` »), un bouton « Ouvrir le fichier », et Enregistrer bloqué tant que la barre affichée n'est pas la mienne.
- La vérification des noms d'app, et l'icône de l'app sur le bouton.
- Des avertissements dans la fenêtre pour ce qui demande une autorisation ou un outil : Accessibilité (raccourcis, esc), `media-control` (musique).
- Une recherche de symboles en français, ou des mots-clés (pinceau → paintbrush).
- Une désinstallation sans terminal.

## 3 propositions concrètes

1. **Ne jamais perdre la barre de l'utilisatrice.** À la fermeture avec des modifications, proposer « Enregistrer / Ne pas enregistrer / Annuler » (`windowShouldClose:` dans `src/editeur/mod.rs`, avec l'état `modifie` envoyé par la page). Ajouter un bouton « Annuler les changements » qui recharge le fichier actuel (`envoyer_config(config::charger())`), et demander confirmation avant une barre toute faite et avant Revenir. Après Revenir, afficher « Version précédente rétablie. Ta Touch Bar est à jour. ». N'activer Revenir que si `barre.toml.precedent` existe (Rust le sait : renvoyer ce booléen dans `enregistre()`).
2. **Un vrai écran « ton fichier contient une erreur ».** Quand `config::charger()` échoue à l'ouverture, ne pas afficher une barre vide avec « Pas enregistré » : afficher « Ton fichier de barre contient une erreur à la ligne 6 » avec la ligne citée, une explication en français pour les cas courants (guillemets manquants, `#FFF`, action écrite comme une commande), et deux boutons : « Ouvrir le fichier pour corriger » et « Repartir de la dernière version qui marchait » (`.precedent`). Désactiver Enregistrer dans cet état. Au passage, traduire les messages Rust selon la langue de la fenêtre, et ne plus effacer l'erreur au changement de langue.
3. **« Ajouter une app » en un geste.** Dans la palette, un bouton « Application… » qui ouvre directement **Choisir…**, puis crée un bouton dont le texte est le nom de l'app, l'image son icône (`image` existe déjà) et l'action `{ app = … }`. Pour un nom tapé à la main, le vérifier côté Rust (`NSWorkspace URLForApplicationWithBundleIdentifier:` ou une recherche dans /Applications) et le signaler en rouge, comme un symbole inconnu. C'est le premier besoin d'une débutante, et aujourd'hui il faut quatre réglages pour y arriver (Bouton, Texte, « Lancer une application… », Choisir…).
