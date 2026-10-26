# Retour de Nadia, débutante : Tactile 0.3 (main, `9e559e6`)

## Contexte

Nadia, 34 ans, graphiste. MacBook Pro 13" M1 avec Touch Bar, macOS 15. Elle n'a jamais ouvert de terminal et ne sait pas ce qu'est TOML. Elle a testé la 0.1 ([rapport](../0.1/nadia-debutante.md)) et la 0.2 ([rapport](../0.2/nadia-debutante.md)). Elle lit `README.fr.md` et ce que la fenêtre affiche, rien d'autre. Son objectif cette fois : ses trois apps préférées dans un tiroir (Figma, Musique, Photos), son logo en monochrome, la météo de Lyon, un bouton « capture d'une zone », puis partager sa barre.

Méthode :

- Lecture de `README.fr.md`, puis de `src/editeur/editeur.html` pour citer les libellés.
- La barre n'a pas été lancée. Seule la fenêtre a tourné (`tactile --personnaliser`), sur des fichiers à moi (`TACTILE_CONFIG=…/scratchpad/nadia3/*.toml`, `HOME` factice), pilotée par `TACTILE_EDITEUR_JS`. Les clics passent par les vrais boutons et champs de la page, qui envoient les vrais messages à Rust. Thèmes forcés avec `TACTILE_APPARENCE=clair|sombre`.
- **Cette fois, les captures marchent** (`capture`, takeSnapshot) : j'ai regardé chaque écran comme Nadia le verrait, en clair et en sombre.
- Les fenêtres modales macOS (Choisir…, Choisir une image…, Exporter…, Importer…) n'ont pas été ouvertes : j'ai envoyé les messages équivalents (`importerImage`, `exporterVers`, `importerBarre`), qui suivent le même chemin côté Rust. Le glisser-déposer depuis le Finder n'a pas été testé.
- Piège de pilotage, à savoir pour les prochains testeurs : quand la fenêtre est cachée par une autre, WebKit ralentit puis gèle les `setTimeout` après une vingtaine de secondes, et le pilote reste bloqué en silence. Contournement : attendre par aller-retour avec Rust (message `symboles`), voir `prelude.js`.
- Scripts, journaux et captures brutes : `/private/tmp/claude-501/-Users-palphinho/03bc99cc-4742-4072-87c4-fe525a6d77ae/scratchpad/nadia3/` (`pB.js` tiroir + logo + 1er enregistrement, `pC.js` raccourci, ordre, erreur, Revenir, `pD.js` export, import, météo, anglais, `pE.js` clair/sombre, `pG.js` débordement, `pH.js` fichier abîmé).

Verdict : la fenêtre a fait un vrai bond. Elle protège enfin le travail (confirmations, fichier abîmé, historique) et elle est belle, en clair comme en sombre. Mais deux des cinq choses que je voulais faire restent impossibles ou fausses sans aide : **la météo demande d'écrire du TOML**, et **mon tiroir enregistre deux apps qui ne marcheront pas sans que la fenêtre me le dise vraiment**. En plus, j'ai enregistré une barre trop longue avec « Ta Touch Bar est à jour ».

## Anciens problèmes (0.2)

| 0.2 | État | Preuve |
|---|---|---|
| B1 Fichier abîmé : Enregistrer écrase la barre par une barre vide (majeur) | **Réglé** | Avec `couleur = #A259FF`, la fenêtre affiche « Ton fichier de barre contient une erreur », la ligne citée, « Enregistrer reste désactivé » et trois issues (`editeur.html:362-366`, `pH.log`). Le message reste un peu redondant : « un texte doit être entre guillemets, attendu un texte entre guillemets ». |
| B2 Fermer perd les modifications (majeur) | **Réglé** (lu dans le code, pas cliqué) | `demanderFermeture` propose Ne pas enregistrer / Annuler / Enregistrer (`editeur.html:587-591`), point dans le bouton rouge. |
| B3 Barre toute faite écrase sans confirmation (majeur) | **Réglé** | « Remplacer ta barre par « Musique » ? » quand il y a des modifications (`editeur.html:880-882`). |
| B4 Revenir sans confirmation ni message (majeur) | **Réglé** | Dialogue « Revenir à la version précédente ? … La version actuelle reste dans l'historique. » puis « Version précédente rétablie. Ta Touch Bar est à jour. » (`pC.log`). |
| B5 Nom d'app tapé non vérifié (majeur) | **Partiel** | « Aucune application « Figma » sur ce Mac » s'affiche sous le champ. Mais : aucune suggestion pour Figma, le nom français affiché par le Finder (« Musique ») est refusé, l'alerte devient invisible en clair dans la liste du groupe, et Enregistrer accepte quand même (voir B1, B2 ci-dessous). |
| B6 Revenir actif après le premier enregistrement (mineur) | **Réglé** | Revenir reste grisé après le premier enregistrement, s'active au deuxième (`pB.log`, `pC.log`). |
| B7 Messages dans la mauvaise langue (mineur) | **Partiel** | Les erreurs Rust sont en français (« Élément 1 : le symbole « pinceau » n'existe pas »), mais un message de réussite reste en français après passage en EN (B6 ci-dessous), et les avis de l'import parlent de « bundle id ». |
| B8 Changer de langue efface l'erreur (mineur) | **Toujours là** | FR → EN → FR : l'encadré rouge devient « Unsaved changes. » puis « Modifications non enregistrées. » (`editeur.html:1168`, `pC.log`). |
| B9 Élément fautif non montré (mineur) | **Réglé** | Le bouton fautif porte un ⚠ orange dans l'aperçu. |
| B10 Captures du README en anglais (mineur) | **Toujours là** | `README.fr.md:22` et `:35` : l'image d'accroche et les deux GIF montrent la fenêtre « (en anglais) », c'est même écrit dans le texte alternatif. Seule `docs/editeur-erreur-sombre.png` (`:434`) est en français. |
| B11 Version 0.1.0 (mineur) | **Réglé** | `Cargo.toml:3` dit 0.2.0, le README aussi (`:77`). |

Frictions 0.2 encore présentes : « Couleur » au lieu de « Couleur du fond » (`editeur.html:338`), « Afficher : automatique » (`:339`), recherche de symbole en anglais seulement (« pinceau » ne donne rien, `:349`, `:972`), « Résultat d'une commande » dans la palette (`:335`), « Commande shell » et « AppleScript » au même niveau que « Lancer une application… » (`:350`), aucun avertissement Accessibilité, un élément ajouté va toujours au bout de la barre (`:711`), Suppr sans confirmation ni ⌘Z (`:868`), pas de « Voici la barre par défaut » à l'ouverture sans fichier, `gabarit = false` sur chaque bouton du fichier écrit, désinstallation par `rm -r` (`README.fr.md:444`).

## Parcours étape par étape

### 1. Ouvrir la fenêtre sur un fichier vide, puis sans fichier

- **Elle cherche** : à comprendre où elle en est.
- **Elle trouve** : sans fichier, la barre Musique s'affiche (pomme, luminosité, volume, ⏪, Titre · Artiste, ⏩, heure) sans un mot ; l'état est vide. Avec un fichier vide : « Ta barre est vide. Ajoute un élément à gauche, ou pars d'une barre toute faite. » La fenêtre façon Réglages Système est nette, lisible, rassurante. En sombre, mêmes qualités.
- **Ce qui la gêne** : rien ne dit que la barre affichée sans fichier est « la barre par défaut, pas encore enregistrée ». Avec un fichier vide, la fenêtre dit « vide » alors que la vraie Touch Bar affiche la barre par défaut avec « ⚠ config vide » (ARCHITECTURE, pièges) : les deux ne racontent pas la même chose.

### 2. « Mes 3 apps préférées dans un tiroir »

- **Elle cherche** : « tiroir » ou « apps » dans la palette. Le README en parle (`README.fr.md:31`, « quelques-uns dans un `groupe` font un tiroir d'apps »), mais la section Personnaliser ne cite pas les lanceurs (`README.fr.md:131`).
- **Elle trouve** : « Lanceur d'app » et « Groupe ». Elle devine qu'il faut un Groupe, puis « Ajouter dans le groupe… » → menu « Choisir… » → « Lanceur d'app ». Chaque lanceur arrive réglé sur **Safari**, qu'elle doit remplacer.
- **Figma (pas installé)** : « Aucune application « Figma » sur ce Mac. Avec Choisir…, le nom est toujours juste. » Pas de suggestion (normal, rien ne ressemble), mais pas non plus de « Installe-la d'abord, ou retire ce lanceur ». Le champ Texte garde « Safari » en texte d'exemple alors que l'app est Figma (`editeur.html:1031`, calculé au premier dessin).
- **Musique** : son Mac est en français, le Finder et le Dock disent « Musique ». La fenêtre répond « Aucune application « Musique » sur ce Mac ». Il faut écrire « Music », ce que seul le README explique (`README.fr.md` « Trouver le nom d'une app » : « en anglais quand l'app en a un »). Pour une débutante, c'est incompréhensible : l'app est là, sous ses yeux.
- **Photos** : marche, avec la vraie icône. Très bien.
- **Ce qui la bloque** : de retour sur le groupe, la liste « Contenu du groupe » montre l'icône de Photos, et **rien** devant Figma et Musique en clair (l'alerte ⚠ est blanche sur blanc) ; en sombre, un petit triangle blanc qui n'alarme personne. Dans la barre, le tiroir est un carré gris sans texte, à droite de l'heure. Puis **Enregistrer** répond « Enregistré. Ta Touch Bar est à jour. » : deux lanceurs sur trois afficheront leur nom au lieu d'ouvrir une app (`tactile --verifier` le signale, la fenêtre non). Capture : `captures/nadia-groupe-alerte-invisible-clair.png` et `captures/nadia-groupe-alerte-sombre.png`.

### 3. Son logo, puis monochrome

- **Elle cherche** : « image » ou « logo ». Dans un bouton, la section « Image » avec « Choisir une image… » et l'aide « PNG, JPEG, SVG, PDF… Tu peux aussi glisser un fichier ici ». Clair.
- **Elle trouve** : l'image est copiée dans `icones/` et apparaît sur le bouton. L'interrupteur « Monochrome, comme les symboles Apple » est juste dessous, avec « Les couleurs disparaissent : la barre teinte la forme en blanc. »
- **Ce qui la gêne** : sous « Image », le chemin complet de la copie s'affiche en trois lignes (`/private/tmp/…/icones/sequoia-sunrise-thumbnail-9753c9ed.png`, `editeur.html:992`) : du charabia pour elle. Son image n'a pas de transparence (un JPEG/PNG plein, le cas courant d'un export rapide) : en monochrome, elle devient un **rectangle blanc**, sans avertissement du type « ton image n'a pas de fond transparent, le monochrome en fera un bloc blanc ». Le symbole « star.fill » reste affiché au-dessus avec toute sa grille, alors qu'il ne servira pas (« L'image s'affiche à la place du symbole »). Capture : `captures/nadia-logo-monochrome.png`.

### 4. La météo de Lyon

- **Elle cherche** : « Météo » ou « Texte » dans la palette. Rien. Le README promet « un texte vivant … (la météo …) » (`README.fr.md:31`), et l'image d'accroche montre « la météo de Paris ».
- **Elle trouve** : la seule voie est la section « Sources de données » (`README.fr.md:334`) : un bloc `[[source]]` avec une adresse `https://api.open-meteo.com/v1/forecast?latitude=48.85&longitude=2.35&…`, `gabarit`, `symbole_selon`, `couleur_selon`. Pour Lyon, il faut connaître sa latitude et sa longitude et les écrire dans l'adresse, dans le fichier. Le code le dit franchement : « pas `texte` : il lui faut une [[source]], écrite dans le fichier » (`editeur.html:473`).
- **Détour essayé** : importer `exemples/meteo.toml` (si quelqu'un le lui donne). L'aperçu montre « — °C », la liste dit « Adresse lue toutes les 600 s · Source « meteo » » avec l'adresse. Mais **« Remplacer ma barre » remplace tout** : son tiroir et son logo disparaissent (ils restent dans l'historique). Il n'existe pas d'« Ajouter à ma barre ». Ensuite, l'élément Texte ne montre que « Source : meteo », « Gabarit : {current.temperature_2m} °C » et la largeur (`editeur.html:1046-1049`) : impossible de changer la ville, ni de voir le symbole ou les couleurs selon la température. Capture : `captures/nadia-texte-meteo.png`.
- **Verdict** : **échec sans TOML. Manque majeur**, d'autant que c'est mis en avant en tête du README.

### 5. Un raccourci « capture d'une zone »

- **Elle cherche** : dans un Bouton, « Au toucher ». Elle doit d'abord descendre sous la grille de 48 symboles, l'image, les couleurs, la taille, l'affichage et la largeur : l'action est tout en bas.
- **Elle trouve** : sous « Système », « Capture d'une zone ». Bien nommé, rien à taper.
- **Ce qui la bloque** : rien ne dit que ce raccourci demande l'autorisation Accessibilité ; sans elle, le bouton ne fera rien, sans erreur (`README.fr.md`, actions : « Envoie le raccourci par défaut de macOS (Accessibilité) »). Et le nouveau bouton est ajouté **hors de la barre** : l'aperçu dépasse du cadre noir, à droite, sans un mot (voir B3). Capture : `captures/nadia-ajout-hors-barre.png`.

### 6. Réordonner

- ◀ ▶ dans l'en-tête de l'élément, quinze clics pour l'amener en tête ; glisser dans l'aperçu marche aussi. Elle a ensuite retiré « Musique en cours » pour faire de la place : Supprimer, sans confirmation ni annulation possible. L'ordre enregistré est conforme.

### 7. Erreur volontaire

- Symbole « pinceau » : ⚠ orange dans l'aperçu et sur la vignette, « Ce symbole n'existe pas. » sous le champ, puis Enregistrer : encadré rouge « Pas enregistré. Élément 1 : le symbole « pinceau » n'existe pas. » Rien n'est écrit. Très bien, en clair comme en sombre.
- Mais aucune piste : pas de « essaie paintbrush » (la recherche est en anglais, et la proposition par distance d'édition ne trouve rien). Et si elle passe en EN puis revient, l'encadré rouge disparaît (ancien B8).

### 8. Enregistrer

- « Enregistré. Ta Touch Bar est à jour. » en vert, le badge « Modifié » disparaît, Revenir s'active. Clair. Mais le même message s'affiche pour une barre qui déborde ou des apps introuvables (B1, B3).

### 9. Revenir à la version précédente

- Dialogue clair, avec « La version actuelle reste dans l'historique », puis « Version précédente rétablie. » Un second clic revient à la version d'après : c'est expliqué dans le README (`README.fr.md:137`), pas dans la fenêtre. Le libellé du bouton, très long, reste au même poids qu'Enregistrer.

### 10. Exporter puis importer

- **Exporter** : « Barre exportée dans ma-barre.zip, avec ses logos (1). » Le zip contient `barre.toml` et le logo. Parfait.
- **Importer** : un grand dialogue montre la barre reçue à sa vraie taille, « 11 éléments · 1 logo », puis « Ce qu'elle peut lancer sur ton Mac ». Nadia **lit** la liste et comprend « Ouvre l'app · Figma / Musique / Photos » et « Capture une zone (simule ⌘⇧4) ». Elle bute sur :
  - la colonne de droite en police de code, avec `capture_zone` (le nom interne, `editeur.html:622`) ;
  - « Barre principale, élément 10 › 1 » : il faut compter, et le tiroir est invisible dans l'aperçu ;
  - l'avertissement « une commande peut tout faire sur ton Mac, y compris effacer des fichiers » (`editeur.html:380`), affiché en tête même quand la liste ne contient que des apps et une capture d'écran : elle prend peur pour rien, et la prochaine fois elle ne lira plus ;
  - « À savoir » : « ligne 56 : app « Figma » introuvable : … écris le nom du dossier .app sans l'extension, ou son bundle id ». « Ligne 56 » d'un fichier qu'elle n'a jamais vu, « bundle id » : du jargon.
- Capture : `captures/nadia-import-commandes.png`.

### 11. Passer en anglais et revenir

- Tout bascule, titre de fenêtre compris, et le choix est retenu. Mais le message d'état reste dans l'ancienne langue (« Barre importée… » en français sous une fenêtre anglaise), et une erreur est remplacée par « Unsaved changes. » Capture : `captures/nadia-erreur-effacee-en.png`.

## Bugs

### B1. Un lanceur introuvable est enregistré sans prévenir, et son alerte est invisible en clair (majeur)

- Étapes : Groupe → Ajouter dans le groupe → Lanceur d'app, app « Figma » (non installée) ; idem « Musique » ; Enregistrer (`pB.js`).
- Attendu : l'enregistrement prévient, comme pour un symbole inconnu, ou au moins la liste du groupe montre clairement les lanceurs cassés.
- Obtenu : « Enregistré. Ta Touch Bar est à jour. » Dans « Contenu du groupe », l'alerte ⚠ est blanche sur fond blanc en clair (la tuile `.tuile` a `color: #fff`, `editeur.html:174`, et `.tuile.app` retire le fond, `editeur.html:208`), donc invisible ; en sombre, un triangle blanc sans couleur d'alerte. `tactile --verifier` signale les deux apps, la fenêtre non.
- Captures : `captures/nadia-groupe-alerte-invisible-clair.png`, `captures/nadia-groupe-alerte-sombre.png`.

### B2. Le nom d'app affiché par le Finder français est refusé (majeur)

- Étapes : lanceur, app « Musique » (ou « Calendrier », « Réglages Système »…).
- Attendu : la fenêtre trouve l'app que Nadia voit dans son Dock, ou propose « Music ».
- Obtenu : « Aucune application « Musique » sur ce Mac », sans suggestion. La recherche ne connaît que les noms de dossier anglais. Pour un Mac en français, toutes les apps d'Apple tombent dans ce piège.

### B3. Une barre qui déborde est enregistrée avec « Ta Touch Bar est à jour » (majeur)

- Étapes : barre par défaut + un groupe + deux boutons + une batterie, Enregistrer (`pG.js`, en sombre).
- Attendu : l'aperçu montre ce qui ne tiendra pas, et l'enregistrement prévient (« 2 éléments seront cachés »).
- Obtenu : dans l'aperçu, les derniers éléments sortent du cadre noir sans signal (et la barre de 1004 points est elle-même coupée par la fenêtre). Enregistrer répond « Enregistré. Ta Touch Bar est à jour. » ; `tactile --verifier` dit pourtant « la barre principale déborde (environ 1159 pt pour 1004) : 2 éléments sur 13 seront cachés ».
- Captures : `captures/nadia-barre-deborde-sombre.png`, `captures/nadia-ajout-hors-barre.png`.

### B4. Importer une barre remplace toute la mienne, sans moyen d'ajouter (majeur)

- Étapes : importer `exemples/meteo.toml` pour avoir la météo.
- Attendu : « Ajouter à ma barre » à côté de « Remplacer ma barre », ou au moins un avertissement visible que mon tiroir et mon logo vont disparaître de la barre.
- Obtenu : seul « Remplacer ma barre ». La phrase « Cette barre remplacera la tienne » est là, mais rien ne permet de combiner. Les barres partagées (la galerie communautaire prévue) ne pourront donc servir qu'en entier.

### B5. Changer de langue efface l'erreur affichée (mineur, ancien B8)

- Étapes : erreur à l'enregistrement, EN, puis FR.
- Obtenu : l'encadré rouge est remplacé par « Unsaved changes. » puis « Modifications non enregistrées. » (`editeur.html:1168`). Capture : `captures/nadia-erreur-effacee-en.png`.

### B6. Le message d'état ne change pas de langue (mineur)

- Étapes : importer, puis passer en EN.
- Obtenu : « Barre importée. Ta Touch Bar est à jour, et l'ancienne barre est dans l'historique. » reste en français sous une fenêtre en anglais : l'état garde le texte, pas sa clé (`editeur.html:820-826`).

### B7. Le texte d'exemple d'un lanceur reste « Safari » (mineur)

- Étapes : lanceur, remplacer Safari par Figma.
- Obtenu : le champ Texte affiche toujours « Safari » en gris (`editeur.html:1031`, calculé au dessin, pas à la saisie). Nadia croit que le bouton s'appellera Safari.

### B8. Monochrome sur une image sans transparence : un rectangle blanc, sans explication (mineur)

- Étapes : logo PNG sans fond transparent, interrupteur Monochrome.
- Obtenu : un bloc blanc sur le bouton et une vignette vide dans les réglages. L'aide parle de « la forme », sans dire qu'il faut un fond transparent. Capture : `captures/nadia-logo-monochrome.png`.

## Frictions

- `editeur.html:335` « Lanceur d'app » et « Groupe » : rien ne dit qu'un groupe de lanceurs fait un tiroir. « Tiroir d'apps » mériterait sa propre entrée dans la palette.
- `editeur.html:1057` + `:359` « Ajouter dans le groupe… » → « Choisir… » : le même mot que le bouton qui ouvre le dossier Applications. Ici, c'est « Choisir un type… » ou une liste de boutons.
- `editeur.html:672` un lanceur neuf vaut « Safari » : il devrait ouvrir directement le choix d'une app.
- `editeur.html:376` « Avec Choisir…, le nom est toujours juste. » : bon conseil, mais il ne dit pas quoi faire d'une app absente (« installe-la ou retire ce lanceur »).
- `editeur.html:992` le chemin complet du logo sous « Image » : le nom du fichier suffit.
- `editeur.html:342` « Gabarit » désigne le modèle d'un élément Texte (« {current.temperature_2m} °C »), alors que `gabarit` dans le fichier veut dire « monochrome ». Et « Chaque {chemin} devient la valeur lue par la source » est incompréhensible sans le README.
- `editeur.html:342` « Source » : un champ libre où taper un nom (« meteo ») qui doit exister ailleurs, dans le fichier. Rien ne liste les sources disponibles.
- `editeur.html:356` « Capture d'une zone », `:354` « Touche esc », `:350` « Raccourci clavier » : aucune mention de l'autorisation Accessibilité, sans laquelle rien ne se passe.
- `editeur.html:380` « Lis chaque ligne avant d'accepter : une commande peut tout faire sur ton Mac, y compris effacer des fichiers. » : à réserver aux barres qui contiennent vraiment une commande shell, un AppleScript ou un script.
- `editeur.html:622` la valeur en police de code (`capture_zone`, `Figma`) : pour une action système, la phrase de gauche suffit.
- Import, « À savoir » : « ligne 56 », « bundle id », « nom du dossier .app sans l'extension ».
- `editeur.html:332` « Revenir à la version précédente » : le libellé le plus long de la fenêtre, au même rang qu'Enregistrer. « Version précédente… » ou un menu « Historique » serait plus calme.
- Ordre des réglages d'un bouton : « Au toucher », le réglage le plus important, est le dernier, sous une grille de symboles qui prend un écran entier.
- `editeur.html:338` « Couleur » (du fond ?), `:339` « Afficher : automatique » (= les deux) : inchangés depuis la 0.2.
- En sombre, la pastille de couleur « gris de la barre » (#363636) se voit à peine sur l'encart (#323234).
- `README.fr.md:131` la liste « Ajouter » ne cite ni le lanceur ni le tiroir, alors que c'est la nouveauté phare.
- `README.fr.md:135` promet « Photoshop » → « Adobe Photoshop 2025 », mais ne dit pas que les noms français (« Musique ») ne marchent pas.
- `README.fr.md:22`, `:35` les images de présentation sont en anglais.
- `README.fr.md:332` « copie-le vers `~/.config/tactile/barre.toml` » : la fenêtre sait importer, il faudrait le dire.

## Manques

- **La météo (et tout texte vivant) dans la fenêtre.** Un élément « Météo » avec un champ « Ville » (géocodage Open-Meteo, gratuit et sans clé), unité, symbole selon le temps : c'est la promesse du README, et rien ne la tient sans TOML.
- « Ajouter à ma barre » à l'import, pour piocher dans une barre partagée.
- Un contrôle de largeur dans l'aperçu : les éléments qui ne tiendront pas, grisés ou barrés, et un avertissement à l'enregistrement.
- Les avis de `--verifier` (apps introuvables, débordement) dans la fenêtre, à l'enregistrement.
- La correspondance nom français → nom d'app (ou une liste des apps installées avec leurs icônes, dans laquelle choisir).
- Un avertissement « autorisation Accessibilité requise » à côté des raccourcis et des actions simulées, avec un bouton qui ouvre le bon panneau de Réglages Système.
- Annuler (⌘Z) dans la fenêtre.
- Une recherche de symboles en français.

## 3 propositions

1. **Que « Enregistré. Ta Touch Bar est à jour. » dise vrai.** À l'enregistrement, renvoyer à la page les avis de `config::controler` (déjà calculés pour `--verifier`) et les afficher en orange sous le message vert : « 2 éléments ne tiendront pas », « Figma n'est pas installée ». Dans l'aperçu, griser ce qui dépasse 1004 points. Dans la liste du groupe, une tuile ⚠ orange visible en clair et en sombre (`.tuile.app` : `color: var(--erreur)`). Pour les apps, chercher aussi par nom affiché (`NSFileManager displayNameAtPath:` ou `kMDItemDisplayName` dans `mdfind`), pour que « Musique » trouve Music.app.
2. **Un élément « Météo » dans la palette.** Champ « Ville » → Rust géocode par `geocoding-api.open-meteo.com` et écrit la `[[source]]` et l'élément `texte` avec les valeurs de `exemples/meteo.toml` (symboles WMO, couleurs selon la température). Plus largement, un sélecteur « Source » qui liste les sources du fichier au lieu d'un champ libre, et « Gabarit » renommé « Ce qui s'affiche ».
3. **Un tiroir d'apps en un geste, et un import qui sait ajouter.** Dans la palette, « Tiroir d'apps… » ouvre le choix d'apps (sélection multiple), crée le groupe avec un lanceur par app et un texte « Apps ». À l'import, un second bouton « Ajouter à ma barre » qui insère les éléments reçus (et leurs `[[source]]`) à la fin, et l'avertissement « une commande peut tout faire » réservé aux barres qui en contiennent une.
