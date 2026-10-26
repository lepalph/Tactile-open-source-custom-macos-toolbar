# Retour de Nadia, débutante — Tactile 0.1.0

## Contexte

Nadia, 34 ans, graphiste. MacBook Pro 13" M1 avec Touch Bar, macOS 15. Jamais ouvert un terminal, ne sait pas ce qu'est TOML. Elle connaît les codes couleur hex (`#A259FF`) et a déjà entendu parler de SF Symbols, c'est à peu près tout.

Elle a trouvé le dépôt `lepalph/tactile-touch-bar` sur GitHub et suit uniquement `README.md`, `exemples/` et ce que l'app affiche. Objectif : installer Tactile et avoir une barre « avec mes 3 apps préférées (Figma, Photoshop, Safari), l'heure et le volume ».

Méthode : lecture du README ligne par ligne, téléchargement de la release `v0.1.0` (`Tactile.zip`, 1,07 Mo), décompression et inspection de `Tactile.app` (Info.plist, `codesign -dv`, `spctl -a -t exec`), écriture d'une `barre.toml` à partir des exemples, validation TOML (`tomllib`) et comparaison avec les clés acceptées dans `src/config.rs`. Le binaire n'a pas été lancé (Touch Bar occupée par quelqu'un d'autre) : les écrans macOS décrits ci-dessous viennent de ce que Gatekeeper dit de l'app et du comportement connu de macOS 15, pas d'un lancement réel.

Fichiers produits : `/private/tmp/claude-501/-Users-palphinho/03bc99cc-4742-4072-87c4-fe525a6d77ae/scratchpad/nadia/moi/barre.toml` (version finale) et `barre-premier-jet.toml` (mes erreurs de débutante).

## Ce qui a marché

- Le pitch (`README.md:3`) et la capture (`README.md:5`) me disent tout de suite ce que fait l'app. Le tableau de compatibilité (`README.md:13-15`) : mon M1 13" est explicitement cité, rassurant.
- La release existe, un seul fichier à télécharger, `Tactile.zip`, nommé comme dans le README (`README.md:19`). Il contient juste `Tactile.app`, rien d'autre à comprendre.
- `Tactile.app` est bien universelle (Intel + Apple Silicon, vérifié avec `lipo`), version `0.1.0` dans l'Info.plist, `LSMinimumSystemVersion` 13.0 : cohérent avec `README.md:14`.
- Le menu de la barre des menus (`README.md:33-39`) est documenté entrée par entrée, et les libellés existent bien tels quels dans le binaire (« Afficher ma barre », « Ouvrir la configuration », « Lancer au démarrage », « Quitter Tactile »).
- « Ouvrir la configuration » crée le fichier s'il n'existe pas (`README.md:37`) : je n'ai pas besoin de trouver le dossier caché `~/.config` moi-même. C'est le point qui sauve une débutante.
- `exemples/barre.toml` est très lisible : un commentaire par élément, en français, et le fichier correspond exactement à la capture `docs/barre.png`. En le copiant-collant et en changeant des mots, j'ai réussi ma barre.
- Mon fichier final (6 éléments : 3 boutons, `souple`, `slider` volume, `heure`) est du TOML valide et n'utilise que des clés et des types acceptés par `src/config.rs`.
- Le tableau des éléments (`README.md:61-70`) et des actions (`README.md:78-89`) est complet par rapport au code : toutes les actions de `src/actions/mod.rs` y sont, tous les champs de `src/config.rs` aussi.
- Les erreurs de config sont traduites en français avec un numéro de ligne (`src/config.rs:140-166`) : « ligne 4 : valeur inconnue «danser», attendu parmi … ». Bien, à condition de trouver le fichier journal (voir Bugs).

## Bugs

### B1. Sur macOS 15, « clic droit → Ouvrir » ne suffit pas : l'app est bloquée — **bloquant**

- Étapes : télécharger `Tactile.zip` depuis Safari, décompresser, glisser dans Applications, clic droit → Ouvrir (`README.md:21`).
- Attendu (selon `README.md:21` et `README.md:99`) : un bouton **Ouvrir**, et la barre change.
- Obtenu : l'app est signée « ad hoc » seulement (`codesign -dv` : `Signature=adhoc`, `TeamIdentifier=not set`) et Gatekeeper la refuse (`spctl -a -t exec Tactile.app` → `rejected`). Depuis macOS 15 Sequoia, Apple a supprimé le contournement par clic droit → Ouvrir pour les apps non notarisées : la fenêtre affichée est « Apple n'a pas pu vérifier que « Tactile » ne contient aucun logiciel malveillant… » avec seulement **Terminé** et **Placer dans la corbeille**. Aucun bouton « Ouvrir ». Il faut ensuite aller dans Réglages Système → Confidentialité et sécurité → tout en bas, « Ouvrir quand même », puis confirmer avec Touch ID / mot de passe. Rien de tout ça n'est dans le README, et la ligne de Dépannage `README.md:99` répète la même consigne inopérante. Je m'arrête là, convaincue que l'app est dangereuse ou cassée.
- Note : macOS 15 est justement la version « vérifiée » (`README.md:15`), donc le chemin documenté n'a pas pu être celui suivi par l'auteur (qui lance sans doute depuis le terminal ou une app compilée localement, non mise en quarantaine).

### B2. `masquer_control_strip` n'est pas `true` par défaut, contrairement au README — **majeur**

- Étapes : écrire sa propre `barre.toml` sans la ligne `masquer_control_strip` (je ne l'ai pas mise au premier jet, je ne comprenais pas ce qu'était le « Control Strip »).
- Attendu (`README.md:72` : « `masquer_control_strip = true` (par défaut) donne toute la largeur à ta barre ») : la barre prend toute la largeur.
- Obtenu : dans `src/config.rs:15-16`, le champ est `#[serde(default)]` sur un `bool`, donc **`false` quand il est absent** ; le test `chaque_forme_d_action` (`src/config.rs:257`) le confirme (`masquer_control_strip: false` pour un fichier sans la ligne). `exemples/claude-vocal.toml` ne la contient pas non plus. Ma barre garde donc la bande d'Apple à droite, dans un mode que le README lui-même dit « pas encore vérifié sur une vraie machine » (`README.md:72`, `README.md:137`).

### B3. Une erreur dans le fichier est totalement silencieuse — **majeur** (pour une débutante)

- Étapes : premier jet avec `couleur = #A259FF` (sans guillemets, parce que `#A259FF` est une couleur, pas un texte, pour moi). Enregistrer.
- Attendu : quelque chose me dit que c'est faux et où.
- Obtenu : d'après `README.md:57` et `src/barre/mod.rs:106` (`eprintln!`), la barre reste comme avant et le message va dans `~/Library/Logs/Tactile.log`. Aucune notification, aucune entrée de menu « voir la dernière erreur ». Le README me dit d'aller « regarder » ce fichier (`README.md:101`) sans dire comment : `~` et `Library` sont invisibles dans le Finder, je ne connais ni « Aller au dossier » ni Console. Pour moi, l'app « ne marche plus ». D'autant que `tomllib` donne « Invalid value (at line 4, column 11) » : même en trouvant le journal, « valeur invalide » ne me dit pas que ce sont les guillemets qui manquent.

### B4. Une couleur mal écrite est ignorée sans un mot — **mineur**

- Étapes : `couleur = "#FFF"` (hex court, courant chez les graphistes) ou `couleur = "violet"`.
- Attendu : une erreur, ou la couleur.
- Obtenu : `src/barre/vues.rs:18-23` renvoie `None` si ce n'est pas exactement 6 chiffres hex : le bouton reste gris, sans ligne dans le journal. `README.md:63` dit `"#RRGGBB"` mais ne dit pas que le reste est ignoré en silence.

### B5. `Tactile.app` n'a pas d'icône — **mineur**

- Étapes : glisser `Tactile.app` dans Applications (`README.md:20`).
- Attendu : une icône reconnaissable dans Applications et dans la fenêtre Gatekeeper.
- Obtenu : pas de dossier `Resources`, pas de `CFBundleIconFile` dans l'Info.plist : icône générique blanche. Dans la fenêtre « Apple n'a pas pu vérifier… », ça renforce l'impression d'app louche (B1).

### B6. La page Releases est vide — **mineur**

- Étapes : cliquer « Releases » (`README.md:19`).
- Attendu : quelques lignes (quoi de neuf, quel fichier prendre, prérequis).
- Obtenu : la release `v0.1.0` n'a aucun texte (`body: null`, publiée par `github-actions[bot]`) et liste trois fichiers : `Tactile.zip`, `Source code (zip)`, `Source code (tar.gz)`. Je ne sais pas ce qu'est un « source code » ; le README dit bien `Tactile.zip`, mais il faut y retourner pour en être sûre.

## Frictions

Phrases du README que je n'ai pas comprises ou qui m'ont laissée sans savoir quoi faire.

- `README.md:21` « L'app n'est pas notarisée par Apple » : « notarisée » ne m'évoque rien. Il manque une phrase : « Apple n'a pas inspecté cette app, macOS va donc te prévenir ; voici l'écran et quoi cliquer ».
- `README.md:26` « Pour la musique en cours : `brew install media-control` » : c'est une commande, donc un terminal. Je ne sais pas ce qu'est `brew` (Homebrew n'est même pas nommé ni lié), ni où taper ça. La section « Pourquoi media-control » (`README.md:105-107`) explique le pourquoi, jamais le comment. Idem `README.md:102`.
- `README.md:27` « la touche esc simulée (action `echap`) » : je ne sais pas pourquoi je voudrais ça. Un mot d'usage (« la barre d'Apple a la touche esc, la tienne non ; ajoute ce bouton si tu en as besoin ») suffirait.
- `README.md:43` « Le fichier `~/.config/tactile/barre.toml` s'ouvre ; chaque bloc `[[element]]` est un élément » : `~`, `.config`, `.toml`, `[[element]]` : quatre notations d'un coup. Il manque « c'est un fichier texte, tu le modifies avec TextEdit, chaque paragraphe qui commence par `[[element]]` est un bouton ». Et dans quelle app il s'ouvre : d'après `src/menu.rs:101-106`, `open` sur un `.toml` sans app associée retombe sur TextEdit. Si c'est TextEdit, il faut prévenir que les « guillemets intelligents » (Édition → Substitutions) transforment `"` en `“ ”` et cassent le fichier ; je ne peux pas le vérifier sans lancer l'app, mais c'est le piège classique.
- `README.md:54` `appui_long = { shell = "open -a 'Activity Monitor'" }` : « shell », « open -a », et le nom anglais « Activity Monitor » alors que mon Finder dit « Moniteur d'activité ». Pour mes apps, j'ai dû deviner `open -a Figma`, `open -a Safari`, et pour Photoshop le nom exact du dossier (`'Adobe Photoshop 2025'`, avec les guillemets simples parce qu'il y a des espaces, ce que la doc ne dit pas). C'est pourtant le seul moyen documenté de lancer une app, donc le cas d'usage numéro un.
- `README.md:57` « Les symboles sont ceux d'Apple (SF Symbols) : `moon.fill`, `play.fill`… » : le lien mène à une page pour développeurs. Où je trouve le nom d'un symbole ? Il faut dire « télécharge l'app SF Symbols, cherche, clic droit → Copier le nom ». Et que se passe-t-il si le nom n'existe pas ? (rien n'est dit ; d'après `src/barre/vues.rs:15` le bouton est vide, sans erreur).
- `README.md:63` « `largeur` (points) » : des points de quoi ? Il manque un repère (« la barre fait environ 1000 points de large ; un bouton standard en fait 72 »).
- `README.md:63` « `appui_long` (action, après 0,5 s) » et `README.md:64` « slider … sous-barre … ✕ la referme » : ok une fois vu, mais « slider » et « sous-barre » ne sont pas montrés sur la capture.
- `README.md:65` « `format` (format de date Apple, `"HH:mm"` par défaut) » : je voulais afficher « 19h30 » ou « lun. 26 oct. ». Aucun autre exemple, aucun lien vers les lettres possibles. J'ai écrit `format = "19h30"` au premier jet (ça passe la validation et affiche… « 19h30 » en dur, probablement).
- `README.md:67` « lancée avec `zsh -c` à chaque intervalle » : zsh ne me dit rien ; je comprends juste que `script` n'est pas pour moi.
- `README.md:70` « puis des sous-éléments `[[element.element]]` » : aucun exemple complet de `groupe` dans le README ni dans `exemples/` (il n'y en a qu'un dans les tests du code, `src/config.rs:311-321`). Sans modèle, impossible d'écrire l'indentation ni l'ordre.
- `README.md:72` « `masquer_control_strip` … garde le Control Strip d'Apple à droite » : « Control Strip » n'est jamais défini (c'est la bande de boutons à droite de la Touch Bar ; le commentaire de `exemples/barre.toml:5` le dit mieux que le README).
- `README.md:76` « soit comme un mot (`action = "suivant"`), soit comme une table (`action = { volume = 0.3 }`) » : « table » est le mot TOML, pas le mien. Résultat de mon premier jet : `action = "open -a Figma"` (un « mot »), refusé par le code (valeur inconnue).
- `README.md:81` et `README.md:83` « de 0 à 1 » : j'aurais écrit 50 pour 50 %.
- `README.md:113` « Supprime ta config et le journal : `rm -r ~/.config/tactile ~/Library/Logs/Tactile.log` » : une commande de terminal pour désinstaller, sans alternative Finder. Et si j'ai mis l'app à la corbeille avant de décocher « Lancer au démarrage » (`README.md:111`), que se passe-t-il ? Rien n'est dit.
- `README.md:38` « Lancer au démarrage » : d'après `src/menu.rs:84-86`, macOS peut ouvrir Réglages Système (éléments d'ouverture) ou afficher une notification « Élément d'arrière-plan ajouté ». Écran attendu non décrit.
- `exemples/barre.toml:1` « copie ce fichier dans `~/.config/tactile/barre.toml` » contredit le README qui dit de passer par le menu : je ne sais pas copier dans un dossier caché. Il faudrait « inutile de copier : menu → Ouvrir la configuration te donne déjà ce fichier ».
- La section anglaise (`README.md:150-167`) dit la même chose en plus court, donc avec les mêmes trous ; pas gênant pour moi.

## Manques

- Un parcours d'installation **illustré** (captures : fenêtre Gatekeeper macOS 15, Réglages → « Ouvrir quand même », icône dans la barre des menus, le menu ouvert). Aujourd'hui il n'y a qu'une capture, celle de la barre, et le centre y est vide (rien ne jouait) : on ne voit ni « musique en cours », ni un slider ouvert, ni un groupe.
- Un moyen de lancer une app sans `shell` : une action `{ app = "Figma" }` ou `{ ouvrir = "Figma" }`, avec le nom tel qu'il apparaît dans Applications, et idéalement l'icône de l'app sur le bouton. C'était mon besoin principal et il repose sur une commande de terminal cachée dans un exemple secondaire.
- Un retour d'erreur **dans l'app** : notification macOS « barre.toml, ligne 4 : … » ou entrée de menu « Dernière erreur… » qui ouvre le journal. Sans ça, `README.md:57` et `README.md:101` ne servent qu'à ceux qui savent ouvrir `~/Library/Logs`.
- Un exemple dans `exemples/` pour chaque chose que le README ne montre pas : `groupe`, `espace`, `script`, `heure` avec d'autres formats, un bouton texte+symbole+couleur. Aujourd'hui `exemples/barre.toml` n'utilise ni `texte`, ni `couleur`, ni `groupe`.
- Un mini-lexique en tête de « Personnaliser » : fichier texte, `[[element]]`, guillemets obligatoires autour des textes, `#RRGGBB`, point, symbole SF, shell, Control Strip.
- La liste des formats d'heure utiles (`HH:mm`, `HH'h'mm`, `EEE d MMM`, `HH:mm:ss`) ou un lien vers une page lisible.
- Une icône d'app, et une signature/notarisation (ou au minimum le vrai chemin macOS 15 dans le README).
- Une désinstallation sans terminal (ou une entrée de menu « Tout supprimer »).
- Une mention de ce que fait l'app au premier lancement : demande-t-elle quelque chose ? (Non, sauf Accessibilité pour `echap` et micro pour l'extra ; le dire rassure.)

## 3 propositions concrètes

1. **Réécrire l'étape 3 de l'installation pour macOS 15, avec captures.** Remplacer `README.md:21` et `README.md:99` par : « Double-clique Tactile. macOS affiche « Apple n'a pas pu vérifier… » : clique **Terminé** (pas Corbeille). Ouvre Réglages Système → Confidentialité et sécurité, descends tout en bas, clique **Ouvrir quand même**, puis confirme. C'est à faire une seule fois. Sur macOS 13 et 14 : clic droit → Ouvrir suffit. » Ajouter une icône à l'app (`scripts/empaqueter.sh`) et une description à la release (quel fichier prendre, ce qu'il y a de neuf). À terme, notariser.

2. **Rendre les erreurs visibles sans terminal.** Quand `barre.toml` est refusé (`src/barre/mod.rs:106`), afficher une notification macOS avec le message traduit (« ligne 4 : valeur invalide ») et ajouter au menu une entrée « Dernière erreur… » qui ouvre `Tactile.log` dans TextEdit. Compléter les traductions de `src/config.rs:142` avec les cas débutants (« il manque des guillemets autour de la valeur », « une action s'écrit `"suivant"` ou `{ shell = "…" }` »). Et aligner `masquer_control_strip` sur le README (`#[serde(default = "true")]` ou corriger `README.md:72`).

3. **Un exemple « mes apps » et une action `app`.** Ajouter `exemples/mes-apps.toml` : trois boutons texte + couleur qui lancent des apps, un `souple`, le slider volume, l'heure (c'est exactement ce que j'ai dû assembler seule), avec un commentaire par ligne expliquant guillemets, `#RRGGBB`, `open -a 'Nom avec espaces'`. Puis dans le code une action `{ app = "Figma" }` qui évite `shell` et, si possible, affiche l'icône de l'app. Dans le README, remplacer l'exemple « veille + Moniteur d'activité » (`README.md:45-55`) par cet exemple d'apps, plus parlant pour une non-développeuse, et pointer « où trouver le nom d'un symbole : app SF Symbols → clic droit → Copier le nom ».
