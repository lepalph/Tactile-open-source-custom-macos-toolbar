# Architecture

Ce document sert à reprendre le code : où est quoi, comment ajouter quelque chose, et les pièges déjà rencontrés. Le pourquoi des choix est dans [DECISIONS.md](DECISIONS.md).

## Le trajet d'un appui

```
main.rs
 ├─ --personnaliser → editeur::lancer_seul()   la fenêtre de réglage seule, sans Touch Bar (voir plus bas)
 ├─ config::charger()          lit ~/.config/tactile/barre.toml (ou la barre par défaut)
 ├─ config::surveiller()       fil qui relève la date du fichier toutes les 0,2 s ; s'il a changé, envoie la nouvelle barre
 │                             par un canal, puis barre::reveiller() (tic: tout de suite sur le fil principal)
 └─ barre::lancer()            app AppKit sans icône dans le Dock
     ├─ plateau                icône ✳ dans le Control Strip d'Apple
     ├─ menu                   icône de la barre des menus
     ├─ installer(config)      construit les items de la principale et d'une barre par [[application]], présente la base
     ├─ observateur NSWorkspace → appActivee:   suit l'app active (aucun sondage)
     ├─ observateur NSWorkspace → appLanceeOuQuittee:   point « ouverte » sous l'icône des lanceurs
     └─ minuteur 0,1 s → tic   applique une config rechargée, rafraîchit heure / musique / scripts

appui sur un bouton
 → TactileCible::appui (le tag du bouton = index dans Etat::commandes)
 → declencher : Commande::Systeme(action) → actions::executer
                Commande::Ouvrir(k) / Fermer → montre le contenu de la sous-barre k / de la base (barre de l'app active ou principale)
                dans l'unique barre système

changement d'app au premier plan (Safari passe devant)
 → NSWorkspaceDidActivateApplicationNotification → TactileCible::appActivee: → suivre_app : base() lit NSWorkspace.frontmostApplication (bundle id, nom affiché),
   Barre::application_pour trouve la [[application]] → Etat::applis[k], sinon la principale
 → si la barre montrée est une base (principale ou barre d'app) : montrer(barre de l'app)
   si c'est une sous-barre : rien ; son ✕ (Commande::Fermer) montrera base()
 → sous la barre d'Apple (masquer) : montrer change le contenu sans représenter ; ✳ (afficher) présente base()
```

Tout ce qui touche AppKit tourne sur le fil principal. Le travail lent (une commande `script`, `media-control`) part dans un fil à part et renvoie son texte par un canal, relu au `tic` suivant.

### Sources de données (`[[source]]`)

- **Fils** : `installer` présente la barre, *puis* `sources::demarrer` lance un fil par source (rien ne part sur le réseau avant). Le fil lit (`curl`, `zsh -c` ou `tools/call`), envoie `(index, Result<Value, String>)` par `Etat::valeurs`, puis attend `toutes_les` (≥ 5 s) sur son canal d'arrêt. Au `tic`, `rafraichir` passe la valeur à chaque `Rafraichi::Source` de cet index (`sources::rendre` : gabarit, `symbole_selon`, `couleur_selon`). Un champ absent : `—` et une ligne au journal par changement ; une erreur de source : `⚠ source`, journalisée une fois par changement par le fil lui-même.
- **Arrêt** : `Etat::sources` garde le `Sender` d'arrêt de chaque fil ; la barre remplacée les lâche (`recv_timeout` → `Disconnected`, le fil sort) et `tuer_tous` tue les groupes inscrits dans `Etat::processus` : `curl`, commande, serveur MCP (lancé en `process_group(0)` comme un script). Un fil qui lance un processus après le remplacement le voit tué aussitôt (`inscrire` rend `false`). Quitter (`terminate:`) passe par `quitter:` (observateur de `NSApplicationWillTerminateNotification`), qui tue aussi tout. Un serveur stdio lâché normalement (`Drop` de `mcp::Client`) voit d'abord son entrée fermée (arrêt propre selon la spécification), puis son groupe tué après 0,5 s.
- **HTTP** : `/usr/bin/curl` plutôt qu'un crate (aucune dépendance, même machinerie de délai et de groupe que les scripts). `-q` (pas de `~/.curlrc`), `--max-time 10`, `--max-filesize 1 Mo`, `--proto =https` (`=http` seulement vers localhost), aucune redirection suivie. URL, en-têtes et corps passent par l'entrée standard (`--config -`), jamais par les arguments visibles dans `ps`.
- **MCP** : client JSON-RPC maison (`sources/mcp.rs`, ~300 lignes avec tests) plutôt que `rmcp` (async, tokio : un runtime entier pour une requête par minute). Double époque : sonde `server/discover` avec `_meta` (2026-07-28) ; une erreur non moderne ou le silence pendant 10 s → `initialize` + `notifications/initialized` (serveurs 2025). Streamable HTTP : un POST par message (`MCP-Protocol-Version`, `Mcp-Method`, `Mcp-Name` ; `Mcp-Session-Id` en ancienne époque), réponse JSON ou SSE. Erreur de lien (processus mort, pas de réponse) → relance après 5 s, 10, 20… 300 s ; erreur d'outil → le serveur reste.
- **Secrets** : `trousseau:service` est résolu à chaque lecture par `security find-generic-password -s service -w` (dans le fil), et chaque secret lu est remplacé par `•••` dans toute erreur (`sources::masquer`). `controler` signale un en-tête qui ressemble à une clé en clair sans la répéter.

## Où est quoi

| Fichier | Rôle |
|---|---|
| `src/main.rs` | Ligne de commande (`--help`, `--version`, `--verifier`, `--exporter`, `--importer`), démarrage, journal `~/Library/Logs/Tactile.log` quand il n'y a pas de terminal. |
| `src/config.rs` | Types `Barre`, `Element`, `Reglage`, `Application` (barre par app : `application_pour`, fusion `herite` dans `elements_avec`) ; lecture TOML avec erreurs en français (`lire` : `[[item]]` fondus dans `[[element]]` dans l'ordre du fichier, première erreur dans l'ordre du fichier) ; validateur partagé `controler` (couleur, symbole, image, app, débordement de chaque barre, app en double, `[[element]]` égaré, barre vide) appelé par `--verifier`, l'éditeur et le chargement de la barre ; surveillance du fichier. Tests de la config. |
| `src/actions/mod.rs` | Enum `Action` (ce qu'on écrit dans `action = …`) et `executer` ; apps (`app_existe`, `chemin_app`, `app_ouverte`) ; raccourcis système : par l'app d'Apple quand elle existe (`open -b com.apple.exposelauncher [--args 1|2]`, Launchpad, Capture d'écran), sinon au clavier (`raccourci_systeme`). |
| `src/sources/mod.rs` | `[[source]]` : fil par source, chemins de champ, gabarits, `symbole_selon` / `couleur_selon`, trousseau et masquage des secrets, `url_permise`. `http.rs` : requêtes par `curl`. `mcp.rs` : client MCP (stdio et Streamable HTTP). Serveur de démonstration des tests : `exemples/mcp/horloge-mcp.py`. |
| `src/actions/luminosite.rs` | Luminosité de l'écran intégré (DisplayServices, privé). |
| `src/actions/volume.rs` | Volume et muet de la sortie par défaut (CoreAudio). |
| `src/actions/media.rs` | Titre en cours et commandes de lecture (via `media-control`). |
| `src/actions/clavier.rs` | Frappes simulées : `echap`, `{ touche = "cmd+shift+4" }` (type `Touche`, vérifié à la lecture de la config), `{ touche = "cmd+shift+keycode:21" }` (un code avec modificateurs), `{ keycode = n }`. Les caractères sont résolus sur la disposition clavier courante (`UCKeyTranslate`) : « a » donne le bon code en AZERTY, `\` y ajoute ⌥⇧, et un chiffre vise toujours la rangée du haut. |
| `src/barre/mod.rs` | État de la barre, classe `TactileCible` (reçoit appuis, sliders, minuteur), présentation, `afficher` / `masquer`. |
| `src/barre/elements.rs` | Transforme chaque `Element` en item de Touch Bar ; sous-barres (`depliable`) ; point des lanceurs (`marquer_lanceurs`). |
| `src/barre/rafraichi.rs` | Éléments vivants : heure, titre en cours, scripts (texte brut ou JSON `Riche`), batterie. |
| `src/barre/batterie.rs` | Lecture et analyse de `pmset -g batt`, symbole de pile selon le niveau. |
| `src/barre/vues.rs` | Outils AppKit (texte, symbole SF, image de fichier et icône d'app réduites et gardées en cache par chemin, couleur, style du titre, largeur, barre, `lacher`). |
| `src/barre/plateau.rs` | Icône ✳ du Control Strip. |
| `src/menu.rs` | Icône de la barre des menus, lancement au démarrage (`SMAppService`). |
| `src/editeur/mod.rs` | Fenêtre « Personnaliser ma barre » : NSWindow + WKWebView, pont JS ⇄ Rust, écriture et vérification du fichier. |
| `src/editeur/icones.rs` | Logos de l'utilisateur : copie dans `icones/` à côté de la config (nom nettoyé + 8 caractères du hachage FNV-1a du contenu, même contenu = même fichier), conversion SVG / PDF / icns en PNG 40 px de haut par `barre::reduire` (le rendu de la barre), aperçu en data URL, NSOpenPanel. |
| `src/editeur/partage.rs` | Exporter et importer une barre (`.toml`, ou `.zip` avec `barre.toml` + `icones/`) : lecture de l'archive en mémoire avec le crate `zip`, refus des entrées dangereuses et limites de taille, liste de ce que la barre lance (`commandes`), installation par `enregistrer` (historique), `tactile --exporter` / `--importer`. |
| `src/editeur/editeur.html` | La page de l'éditeur (HTML, CSS et JS dans un seul fichier, embarqué par `include_str!`, sans réseau). |
| `exemples/barre.toml` | Barre par défaut, embarquée dans le binaire ; préréglage « Musique » de l'éditeur. `barre.en.toml` : la même avec les alias anglais (test `alias_anglais_donnent_la_meme_barre`). |
| `exemples/simple.toml`, `exemples/developpeur.toml` | Les deux autres préréglages de l'éditeur. |
| `exemples/par-application.toml` | Barres par application commentées (testé par `aller_retour_toml`). |
| `exemples/lanceur.toml` | Lanceurs, tiroir d'apps (un `groupe` de lanceurs) et raccourcis système (testé par `aller_retour_toml`). |
| `scripts/empaqueter.sh` | `Tactile.app` universelle signée ad hoc + `Tactile.zip`. |

## Recettes

### Ajouter une action

1. `src/actions/mod.rs` : une variante dans `Action`. Son nom en `snake_case` devient le mot de la config (`BasculerMuet` → `"basculer_muet"`) ; une variante avec valeur s'écrit `{ nom = valeur }`. Ajoute son alias anglais (`#[serde(alias = "toggle_mute")]`).
2. La fonction qui fait le travail, dans un fichier de `src/actions/`, avec un test qui change l'état, le relit, puis le restaure. Une action qui lance un programme dont l'échec doit se voir passe par `journaliser` (stderr au journal, sans bloquer).
3. La branche correspondante dans `executer`.
4. Le test `chaque_forme_d_action` de `src/config.rs`, le tableau des actions et le glossaire des deux README (le test `glossaire_du_readme` exige que les glossaires de README.md et README.fr.md soient identiques et que chaque ligne soit exercée par `TOUT_FR` / `TOUT_EN`). La recette complète, en huit points, est dans CONTRIBUTING.md et CONTRIBUTING.fr.md.
5. L'éditeur (`src/editeur/editeur.html`) : une action sans valeur va dans `ACTIONS_SIMPLES` et `CLE_ACTION` ; une action avec valeur dans `actionUI`, `actionModele` et le détail de `champAction`, avec ses textes FR et EN. Une action que l'éditeur ne connaît pas s'affiche « Autre » et traverse l'enregistrement intacte (test `le_json_ne_perd_aucun_champ`). Ajoute-la à `BARRE_COMPLETE` (`src/config.rs`), qui sert aux deux tests d'aller-retour.

### Ajouter un type d'élément

1. `src/config.rs` : une variante dans `Element` (son nom en `snake_case` devient `type = "…"`), avec son alias anglais et ceux de ses champs, puis un test de lecture.
2. `src/barre/elements.rs` : la branche dans `construire` qui crée l'item, et sa ligne dans `largeur_estimee`.
3. S'il se met à jour seul : une variante dans `Rafraichi` (`src/barre/rafraichi.rs`) et sa branche dans `rafraichir`. Ne bloque jamais le fil principal : lance le travail dans un fil et renvoie le texte par `etat.envoi` (modèle : `Batterie`, qui envoie la sortie brute de `pmset` et l'analyse au retour). Pour changer aussi l'icône ou les couleurs, construis un `Riche` et passe par `appliquer`.
4. Le tableau des éléments et le glossaire du README, et une étape dans `autotest.rs` si l'effet est visible (la barre `BARRE_RICHE` des étapes 22-26 sert à ça).
5. L'éditeur : `ICONES`, `neuf`, `face` (aperçu) et le `switch` de `dessinerProprietes` dans `editeur.html`, plus la liste `f-ajout-groupe` s'il peut vivre dans un groupe ; l'élément dans `BARRE_COMPLETE`.

### Ajouter une entrée au menu

`src/menu.rs` : une méthode dans `TactileMenu` (`define_class!`), puis un `ajouter(menu, "Libellé", cible, Some(sel!(methode:)))` dans `installer`.

## L'éditeur « Personnaliser ma barre »

Une `NSWindow` dont la vue de contenu est une `WKWebView` ; la page est `src/editeur/editeur.html`, chargée par `loadHTMLString:baseURL:` (base `file:///`, pour que `localStorage` garde la langue choisie). Tout passe par deux canaux :

- **JS → Rust** : `window.webkit.messageHandlers.tactile.postMessage(JSON)`, reçu par `TactileEditeur` (`userContentController:didReceiveScriptMessage:`). Messages : `pret` (la page est chargée), `enregistrer { barre, fermer? }`, `revenir`, `prereglage { nom }`, `symboles { noms }`, `iconesApps { noms }`, `choisirApp`, `verifierApp { nom }`, `modifie { valeur }`, `fermer`, `ouvrirFichier`, `relire`, `titre { texte }`, `journal { texte }`, `capture { chemin }`, `choisirImage`, `deposerImage { nom, donnees }`, `importerImage { chemin }`, `images { chemins }`, `exporter { barre, zip? }`, `exporterVers { barre, chemin }`, `importer`, `importerBarre { chemin }`, `deposerBarre { nom, donnees }`, `confirmerImport`, `annulerImport`.
- **Rust → JS** : `evaluateJavaScript:` sur `window.tactile` : `charger(barre, aPrecedent)`, `illisible(message, extrait, aPrecedent)`, `glyphes({ nom: dataURL | null })`, `iconesApps({ app: dataURL | null })`, `enregistre(manuel, aPrecedent)`, `revenu()`, `abimeGarde(nom)`, `erreur(texte)`, `appChoisie(nom)`, `appVerifiee(nom, trouvee, suggestion)`, `demanderFermeture()`, `imageChoisie(chemin, dataURL)`, `imageRefusee(texte)`, `images({ chemin: dataURL | null })`, `exportAvertir(logos, chemin)`, `exporte(nom, logos, avis)`, `importPret({ nom, barre, commandes, avis, logos })`, `importe(manuel, aPrecedent)`, `partageRefuse({ fr, en })`.

La barre circule en JSON avec les mêmes noms que le TOML (`serde_json` sur `config::Barre`, qui dérive `Serialize`). À l'enregistrement, Rust convertit le JSON en `Barre`, écrit le TOML (`config::ecrire`, en-tête fixe + `toml::to_string_pretty`), le **relit** avec `config::lire`, puis `verifier` refuse ce que la barre ne saurait pas afficher (symbole SF inconnu, couleur hors `#RRGGBB`, intervalle < 0,1 s) et `config::controler` ce qui la rendrait inutilisable : une barre principale vide, même avec des `[[application]]` (la barre montrerait « ⚠ config vide »), avec le même message que `--verifier`. Seulement alors le fichier est écrit. La barre se recharge par la surveillance habituelle du fichier : l'éditeur ne parle jamais à `barre::`.

Rien de ce que l'utilisateur a écrit ne se perd :

- **Historique** : avant d'écrire, les copies glissent d'un cran (`.precedent` → `.precedent-2` … → `.precedent-5`, la plus ancienne disparaît) et le fichier actuel devient `barre.toml.precedent` (`VERSIONS = 5`). « Revenir » fait tourner la chaîne : `.precedent` devient le fichier, chaque copie remonte, et le fichier quitté passe en dernier ; un clic de plus remonte encore, et après la plus ancienne on retombe sur la plus récente. La version restaurée est relue avant d'être remise.
- **Copie `.manuel`** : si le fichier a des commentaires autres que l'en-tête de l'éditeur (`ecrit_a_la_main`, un `#` hors des guillemets), il est copié une fois en `barre.toml.manuel`, jamais écrasé ensuite ; le message de succès le dit.
- **Fichier illisible** : `envoyer_config` n'envoie pas de barre vide mais `illisible(message, extrait)` (la ligne en cause, sans le chemin). La page n'affiche alors que l'erreur, désactive Enregistrer et l'ajout d'éléments, et propose « Ouvrir le fichier » (`open -t`), « Relire le fichier » et « Repartir d'une barre toute faite ». Une barre toute faite demandée sur un fichier illisible le copie d'abord en `barre.toml.abime-AAAAMMJJ-HHMMSS` (`mettre_de_cote`, pas de seconde copie identique) ; le fichier n'est remplacé qu'au prochain Enregistrer.
- **Changements non enregistrés** : la page envoie `modifie { valeur }` à chaque changement d'état ; Rust le garde (`MODIFIE`) et met le point dans le bouton rouge (`setDocumentEdited:`). `windowShouldClose:` refuse alors la fermeture et appelle `demanderFermeture()` ; la page répond `fermer` ou `enregistrer { fermer: true }` (qui ne ferme que si l'écriture a réussi). Les confirmations (fermer, barre toute faite, Revenir) sont un `<dialog>` modal de la page : Échap vaut Annuler, et le pilote JS peut les cliquer. Quitter l'app par le menu ne passe pas par `windowShouldClose:`.
- **Nom d'app** : `verifierApp` répond par `chercher_app`, qui demande à `actions::app_existe` (la même fonction que `--verifier` : bundle id par `URLForApplicationWithBundleIdentifier:`, chemin existant, sinon le nom comme `open -a` par `fullPathForApplication:`, sans casse). Introuvable : `suggestion` prend, parmi les apps listées par `mdfind`, le nom le plus court qui contient la saisie, sinon le plus proche en distance d'édition ; sans index Spotlight, l'app est signalée sans suggestion.

Les logos (`image` d'un bouton) : « Choisir une image… » ouvre un NSOpenPanel côté Rust ; le fichier est copié dans `icones/`, à côté du fichier de config (`TACTILE_CONFIG` compris), et `image` pointe vers la copie, avec `~` si elle est dans le home. Un fichier glissé depuis le Finder (sur un bouton de l'aperçu ou sur la ligne « Image ») passe par le JS : la page ne voit pas son chemin, elle le lit avec `FileReader` et l'envoie en base64 (`deposerImage`), Rust l'écrit dans un dossier temporaire puis l'importe comme un fichier choisi. Tout autre dépôt de fichier est refusé par la page, sinon la WKWebView ouvrirait le fichier à sa place. `importerImage { chemin }` fait la même copie sans sélecteur : c'est ce qu'utilise un pilote `TACTILE_EDITEUR_JS`. L'aperçu affiche l'image que la barre affichera (`barre::image`, réduite à 40 px, en data URL) ; monochrome (`gabarit`), un `filter: brightness(0) invert(1)` la rend blanche d'après sa transparence, comme un gabarit AppKit. Introuvable : `null`, et la page propose d'en choisir une autre. La barre préfère l'image au symbole quand les deux sont là.

Images venues de l'extérieur : avant tout décodage, `barre::controler_image` (vues.rs) lit les dimensions sans décoder (en-tête IHDR d'un PNG, puis ImageIO `CGImageSourceCopyPropertiesAtIndex` pour chaque image du fichier) et refuse au-delà de 4096 px de côté (« image trop grande : 30000 × 30000 px, maximum 4096 ») ou une image matricielle dont ImageIO ne lit pas la taille. `reduire` l'appelle d'abord et borne le rendu d'un SVG ou d'un PDF à 4096 px de large ; tout passe par lui : bouton, `image` d'un script JSON, aperçu, copie dans `icones/`. L'import le rappelle sur chaque logo à l'ouverture de l'archive, avant l'aperçu. Une image refusée dans la barre : repli texte et une ligne au journal.

Lanceurs : la page demande l'icône de chaque `app` (`iconesApps`), Rust répond avec `barre::icone_app` (cache de la barre, sans le point « ouverte ») en data URL PNG ; la page la garde par nom. Le champ App réutilise « Choisir… » et la vérification / suggestion des actions `app`. Les actions système sont listées sous un intitulé « Système » (`ACTIONS_SYSTEME`).

Exporter et importer (`partage.rs`). L'export part de la barre affichée dans la page (enregistrée ou non) ; sans logo, un `.toml` ; avec, un `.zip` dont le `barre.toml` pointe vers `icones/<nom>` et qui ne contient que les images utilisées. Un `.toml` choisi pour une barre à logos : `exportAvertir`, la page propose le `.zip`. L'import traite le fichier comme non fiable : reconnu `.zip` à sa signature `PK`, lu en mémoire (jamais extrait sous ses propres noms) ; une entrée avec `..`, un chemin absolu (`/`, `\`, `C:`) ou un lien symbolique fait refuser toute l'archive ; seuls `barre.toml` et `icones/<image>` sont gardés (le reste est listé en avis) ; limites : 10 Mo d'archive (vérifiée avant lecture), 200 entrées, 2 Mo par image et 1 Mo de TOML, relues avec `take` puisque la taille annoncée peut mentir. Les images partent dans un dossier temporaire supprimé avec le `Paquet`. Puis `config::lire`, `config::controler` (grave = refus, le reste en avis) et `verifier`. `commandes` liste ce qui lance quelque chose (`shell`, `applescript`, `touche`, `keycode`, `app`, `url` non http(s), actions système, app d'un `lanceur`, commande d'un `script`, appui long compris, puis chaque `[[source]]` : sa commande, son serveur MCP lancé ou distant, l'adresse lue) ; la page l'affiche avec l'aperçu de la barre et donne le focus à Annuler s'il y en a. « Remplacer ma barre » : chaque logo passe par `icones::importer_en_png` (toujours redessiné en PNG, jamais copié tel quel ; un suffixe `-<8 hex>` du nom d'origine est retiré avant de rajouter le hachage du contenu), puis `enregistrer` écrit avec l'historique. Un `.toml` seul peut emporter les images relatives `icones/x.png` posées à côté de lui (un `.zip` décompressé), sous les mêmes limites. Le dépôt d'un fichier dans la page (image ou barre) est limité à 10 Mo, vérifié par le JS et par Rust avant de décoder le base64 (`icones::decoder_depot`). Le `.zip` exporté réécrit la barre par `config::ecrire` : les commentaires du fichier n'y sont pas ; un `.toml` exporté en ligne de commande recopie le fichier tel quel.

Les symboles SF n'existent pas dans le web : Rust les rend en PNG (`NSImage imageWithSystemSymbolName:` → `TIFFRepresentation` → `NSBitmapImageRep`), les envoie en data URL, et la page les affiche en `mask-image` sur `currentColor`. Un nom inconnu renvoie `null` : la page le signale et l'enregistrement le refuse (un symbole inconnu fait planter la barre, cf. SYNTHESE A1).

La page imite Réglages Système (macOS Sequoia) : barre de titre transparente fondue dans la bande du haut (`setTitlebarAppearsTransparent:`, titre masqué), barre latérale grise pour ajouter, encarts groupés (libellé à gauche, contrôle à droite), interrupteurs à la place des cases, contrôle segmenté FR / EN, police système 13 px, un seul accent bleu, aperçu noir à l'échelle 1 (1004 × 60 pt, 685 avec le Control Strip), tout faisable au clavier. Clair et sombre par des jetons CSS sur `:root` et `prefers-color-scheme` : les couleurs `-apple-system-*` de WebKit marchent, mais le libellé secondaire et le bleu système ne passent pas l'AA à 13 px, d'où des valeurs propres (`AccentColor` vaut toujours #007AFF dans WebKit). Les icônes de la palette et de l'interface sont des symboles SF demandés à Rust comme les autres ; un caractère les remplace tant qu'ils ne sont pas arrivés. Un symbole inconnu s'affiche ⚠ dans l'aperçu et la page propose le plus proche de sa liste (distance d'édition ≤ 3).

## API privées utilisées

| API | Framework | Où | Pour |
|---|---|---|---|
| `NSTouchBar presentSystemModalTouchBar:placement:systemTrayItemIdentifier:` | AppKit | `barre/mod.rs` | Afficher une barre pour tout le système, pas seulement pour une app. |
| `NSTouchBar minimizeSystemModalTouchBar:` | AppKit | `barre/mod.rs` | Rendre la barre d'Apple. |
| `NSTouchBarItem addSystemTrayItem:` | AppKit | `barre/plateau.rs` | Déclarer l'icône ✳. |
| `DFRElementSetControlStripPresenceForIdentifier` | DFRFoundation | `barre/plateau.rs` | La rendre visible dans le Control Strip. |
| `DisplayServicesGetBrightness` / `SetBrightness` | DisplayServices | `actions/luminosite.rs` | Luminosité (l'API publique ne marche pas sur Apple Silicon). |

Elles peuvent changer avec une mise à jour de macOS. Les fonctions C sont chargées par `dlopen` / `dlsym` : si l'une disparaît, Tactile écrit une erreur au lieu de planter.

## Pièges déjà rencontrés

- Les keycodes macOS sont physiques : `0` est la touche « a » en QWERTY et « q » en AZERTY. `clavier.rs` cherche le code d'un caractère avec `UCKeyTranslate` sur la disposition courante : zone principale sans modificateur, puis avec Maj, Option, Option+Maj (ajoutés à l'envoi : `cmd+\` devient ⌘⌥⇧: en AZERTY), le pavé numérique (65-92) en dernier. Seules les touches sans caractère (f1-f20, flèches, esc…) ont un code fixe.
- Choix : **un chiffre = la position de la rangée du haut** (keycodes ANSI 18-29), quelle que soit la disposition. Les raccourcis d'Apple sont liés au keycode (⌘⇧4 = touche 21 + cmd + shift) ; en AZERTY, le premier code qui donne « 4 » sans Maj était celui du pavé (86), que la capture ignore. Avec cmd ou ctrl, Tactile envoie exactement les modificateurs écrits (`cmd+4` reste ⌘4, il ne devient pas ⌘⇧4) ; sans eux, il ajoute Maj si la disposition l'exige (« 4 » seul en AZERTY). Les fonctions TIS ne supportent pas deux fils à la fois : les tests qui les appellent prennent un verrou.
- Un bouton de `script` est sans bordure (`setBordered: NO`) : une couleur de fond venue du JSON ne se voit pas sans remettre la bordure (`appliquer`).
- `setTitle:` efface la couleur d'un titre attribué : `vues::styler` est rappelé après chaque `titre` d'un script riche.
- Avec des alias serde, « attendu parmi » liste aussi les mots anglais (`«icon», «icone», …`) ; les tests comparent le début du message.

- Les popovers d'AppKit ne s'ouvrent pas dans une barre système. `NSPopoverTouchBarItem` s'affiche mais rien ne se passe au toucher. D'où les sous-barres (`depliable`) : on présente une autre barre système, avec un ✕ qui ramène la principale.
- Les présentations de barres système sont animées et asynchrones : présenter une sous-barre puis la principale les fait s'empiler ou se croiser, une fois sur deux. Tactile ne présente donc qu'une barre système (`modale()`) et change son contenu (`montrer`) pour ouvrir ou fermer une sous-barre, comme MTMR. Ce bug ne se voyait qu'en capture : l'état interne disait « principale ».
- Un élément dont le texte grandit pousse ses voisins hors de la barre : la Touch Bar retire ce qui dépasse. D'où la largeur fixe par défaut des widgets `script`.
- Les touchers de la Touch Bar sont « directs ». Un `NSPressGestureRecognizer` doit avoir `setAllowedTouchTypes: 1` ; avec 2 (indirect), AppKit lève une exception.
- objc2 vérifie les signatures à l'exécution. Un `msg_send!` qui annonce le mauvais type de retour panique (ex. `setActivationPolicy:` renvoie un `Bool`, pas `()`). La fonctionnalité `catch-all` d'objc2 transforme les exceptions Objective-C en panique lisible au lieu d'un arrêt muet.
- MediaRemote ne répond plus aux apps tierces depuis macOS 15.4 : `MRMediaRemoteSendCommand` renvoie vrai sans rien faire. On passe par `media-control`, qui doit tourner en arm64 natif sur Apple Silicon (`arch -arm64`).
- `screencapture -b` rend une image noire quand l'écran est en veille ou verrouillé, et la partie dessinée par Apple (Control Strip, barre d'une app) ne s'y voit pas non plus. Seule une barre présentée par une app réveille la Touch Bar.
- Propriété des objets AppKit (les `Id` sont des pointeurs bruts) : ce qui est reçu à +1 (`alloc`/`init`, `new`, `mutableCopy`) est relâché par `vues::lacher` dès que son propriétaire le retient. Les items vont à `barre_touch`, qui les relâche une fois mis dans la barre ; les `NSTouchBar` et formateurs appartiennent à `Etat`, dont le `Drop` les relâche au rechargement. Avant, chaque rechargement gardait l'ancienne barre (14 → 32 Mo en 20 rechargements), une image de 10 000 px gardait 400 Mo, un script JSON avec `image` prenait 12 Mo/s. Les images de fichier sont réduites à 20 pt × 2 (40 px de haut) à la lecture et gardées par chemin et gabarit, relues seulement si la date du fichier change. Mesures avant/après : `docs/retours/0.2/SYNTHESE.md`, lot R.
- `NSImage imageWithSystemSymbolName:` renvoie `nil` pour un symbole inconnu, et `buttonWithImage: nil` lève `NSInternalInconsistencyException` : Tactile mourait sur une faute de frappe et replantait à chaque relancement. `vues::symbole` renvoie `Option`, et `elements::bouton` retombe sur le nom en texte.
- Un `script` tourne dans son propre groupe de processus (`process_group(0)`) : tuer `zsh -c` seul laisse son enfant (`sleep`, `curl`…) orphelin. Le groupe est tué au-delà de `delai` et quand la barre est remplacée (`Etat::processus`, mis à `None` pour que le script qui démarre pendant le remplacement soit tué aussi). Quitter par le menu les tue aussi (`quitter:`) ; un `kill` de Tactile, non : un serveur MCP stdio s'arrête alors de lui-même à la fermeture de son entrée.
- Le `placement` d'une barre système n'est lu qu'à sa présentation : changer `masquer_control_strip` à chaud impose `minimizeSystemModalTouchBar:` avant de représenter. Une présentation en placement 0 (Control Strip visible) ne réveille pas la Touch Bar : sans toucher humain, `screencapture -b` rend une image noire.
- Les notifications macOS (`UNUserNotificationCenter`, et même `NSUserNotificationCenter`) exigent un bundle : depuis `target/release/tactile` elles plantent ou ne font rien. L'erreur de config s'affiche donc dans la barre elle-même (`afficher_erreur`).
- L'app active est **observée** (`NSWorkspaceDidActivateApplicationNotification` sur le centre de notifications de `NSWorkspace`, méthode `appActivee:` de `TactileCible`), plus sondée toutes les 0,1 s : avec une `[[application]]`, le sondage coûtait 0,5-0,7 % de CPU, l'observateur 0,1-0,2 % (comme sans). Un changement d'app pendant qu'une sous-barre est ouverte est rattrapé par son ✕ (`Commande::Fermer` relit `base()`). Écran verrouillé, `frontmostApplication` reste `com.apple.loginwindow` même après `open -a TextEdit` : l'autotest le détecte et simule l'app active (`APP_SIMULEE`, puis `suivre_app()` à la main, faute de notification), en le disant au journal ; le passage réel au premier plan ne se vérifie qu'écran déverrouillé.
- `scripts/verifier.sh` n'affiche pas les lignes du journal qui contiennent « pas » entre deux espaces (c'est son filtre des lignes `AUTOTEST n pas attendu=…`) : un message d'étape ne doit pas contenir ce mot.
- Une barre par application est construite entière à chaque rechargement, comme la principale : ses scripts tournent même quand elle n'est pas montrée, et avec `herite` les éléments repris de la principale existent deux fois (deux scripts, deux horloges).
- Une config sans élément dans la barre principale (fichier vide, `>` au lieu de `>>`) laissait la Touch Bar noire, sans esc : `installer` montre la barre par défaut (`exemples/barre.toml`, embarquée) avec le badge « ⚠ config vide » (`config_affichee`).
- Le badge « ⚠ config » est ajouté à la principale **et** à chaque barre d'application (un bouton par barre : une vue n'a qu'un parent) : on édite souvent sa config dans l'app qui a sa propre barre.
- Un avertissement `NSLayoutConstraint … exceeds internal limits` apparaît au lancement. Il est inoffensif ; sa source n'est pas encore identifiée.
- `Mission Control.app` n'est qu'un lanceur : il envoie `CoreDockSendNotification` au Dock et quitte. Son premier argument choisit la notification (lu dans son code : rien = `com.apple.expose.awake`, `1` = `com.apple.showdesktop.awake`, `2` = `com.apple.expose.front.awake`). D'où `mission_control`, `bureau` et `fenetres_app` sans touche simulée ni Accessibilité ; un second appel referme. Pareil pour Launchpad (`com.apple.launchpad.toggle`) et `Screenshot.app` (⌘⇧5, lance `screencaptureui`).
- Spotlight, ⌘⇧3, ⌘⇧4, émojis, verrouiller et forcer à quitter n'ont ni app ni API publique : ce sont les raccourcis par défaut, simulés. Sans l'autorisation Accessibilité, `CGEventPost` ne fait rien, sans erreur ; lancé depuis un terminal, c'est l'autorisation du terminal qui compte. Dictée, Ne pas déranger et luminosité du clavier n'ont pas d'action : aucun moyen public fiable (raccourci de dictée choisi par l'utilisateur, Focus sans API, touches `NX_KEYTYPE_ILLUMINATION` non vérifiables et le réglage de CoreBrightness ignoré écran verrouillé).
- Le point d'un lanceur suit `NSWorkspaceDidLaunch/DidTerminateApplicationNotification` : à la notification de fin, l'app quittée est encore dans `runningApplicationsWithBundleIdentifier:`, marquée `isTerminated` (`actions::app_ouverte` l'écarte).
- Les avis de `config::controler` (apps, images, symboles) sont calculés **après** l'envoi de la barre rechargée : avant, ils retardaient le rechargement, et le fil de surveillance relisait les images sans cache (le cache d'images est par fil).

## Mesures

`TACTILE_MESURE=1` écrit au journal une ligne `mesure : …` par passage : `installer`, « enregistrement du fichier → barre rechargée » (de la date du fichier à la fin d'`installer`), « appui → sous-barre montrée », « ✕ → base montrée », « app activée → barre de l'app montrée » (le travail de Tactile après la notification ; le dessin par la Touch Bar ne se mesure pas d'ici). Mesuré sur un MacBook Pro M1 13 pouces, macOS 15.8.1, binaire release, barre de l'utilisateur + une `[[application]]` (TextEdit, `herite`), 8 enregistrements à 1,3 s d'écart, puis autotest :

| | Cible | Avant (0.2.0) | Après |
|---|---|---|---|
| Enregistrement → barre rechargée | < 300 ms | 137 à 1025 ms (moyenne 620) | 86 à 256 ms (moyenne 172) |
| `installer` complet | | 20 à 43 ms | 20 à 48 ms (avec 4 lanceurs) |
| Appui sur un groupe → sous-barre | < 30 ms | 0,1 à 0,2 ms | 0,0 à 0,2 ms |
| ✕ → base | | 0,2 à 1,2 ms | 0,1 à 0,7 ms |
| App activée → barre de l'app | < 50 ms | 0,3 ms | 2,4 ms |
| CPU au repos (`top`, 5 × 3 s) | ≤ 0,2 % | 0,2 % | 0,1 à 0,2 % |

Le rechargement attendait le sondage d'une seconde, puis les avis de `controler`, puis le `tic` suivant (jusqu'à 0,1 s). Maintenant : sondage à 0,2 s (un `stat`, sans effet mesurable sur le CPU), barre envoyée avant les avis, et `reveiller` lance le `tic` aussitôt. `installer` reconstruit toutes les barres à chaque rechargement ; les images de fichier et les icônes d'app sont en cache (par chemin et date), les symboles SF passent par le cache d'AppKit, les sliders relisent luminosité et volume. Aucune animation n'est ajoutée : `montrer` change le contenu de la barre système sans nouvelle présentation. Écran verrouillé pendant ces mesures : l'app au premier plan était simulée (`APP_SIMULEE`), la latence entre l'activation réelle et la notification n'est pas mesurée.

## Vérifier un changement

- `scripts/verifier.sh` : la vérification complète. Lance Tactile avec `TACTILE_AUTOTEST=1`, qui actionne chaque contrôle par le même chemin qu'un doigt (`performClick:` sur le vrai bouton, message `glisse:` pour les sliders) et contrôle l'effet réel : sous-barres, luminosité et volume relus puis restaurés, barre d'Apple et retour par ✳, Claude vocal qui démarre et s'arrête, lecture/pause. Il capture la Touch Bar à chaque étape (`target/verification/`) et compare chaque capture à l'écran attendu (étapes 20-21 : l'élément rouge « ⚠ config ligne 6 » apparaît puis disparaît ; 22-25 : barre riche verte ; 26-27 : barre de TextEdit, bouton bleu, après `open -a TextEdit` ; 28-29 : badge « ⚠ config ligne 9 » sur la barre de TextEdit, puis sur la principale quand TextEdit quitte ; 30 : barre de l'utilisateur rétablie). Un écran noir (Touch Bar inactive) est signalé comme non vérifiable, jamais comme réussi : touche le trackpad pendant la vérification.
- `cargo test` : config, ligne de commande, parsing des médias, symbole SF inconnu, débordement, résolution des touches (chiffres, AZERTY chargée explicitement, `keycode:n`), image réduite et en cache, config vide, scripts (délai avec `sleep 30`, stderr, barre remplacée), et luminosité / volume réels (valeur changée, relue, restaurée). Avec `CI` défini, `build.rs` active `cfg(ci)` et les tests matériels sont marqués `ignored` (`#[cfg_attr(ci, ignore = …)]`).
- `tactile --verifier [fichier]` (alias `--check`) : lit un fichier de barre sans toucher la Touch Bar ; erreur de syntaxe avec sa ligne (code 1), puis les avis de `config::controler` (« attention : … », code 0 ; barre vide : code 1), enfin « OK : n éléments ».
- `TACTILE_CONFIG=chemin` remplace `~/.config/tactile/barre.toml` pour tout le programme (lecture, surveillance, éditeur) : indispensable pour essayer l'éditeur sans toucher à sa vraie barre, par exemple `TACTILE_CONFIG=/tmp/essai.toml target/release/tactile --personnaliser`. `--personnaliser` n'ouvre que la fenêtre : ni Touch Bar, ni icône de menu.
- `TACTILE_EDITEUR_JS=fichier.js` : script évalué dans la page de l'éditeur une fois la config chargée, enveloppé dans une IIFE (sinon ses noms entrent en conflit avec ceux de la page) ; il envoie des **chaînes** JSON : `window.webkit.messageHandlers.tactile.postMessage(JSON.stringify({ type: 'capture', chemin }))`. `capture` rend la page par WebKit (`takeSnapshotWithConfiguration:completionHandler:`) et la pose dans le cadre de la fenêtre dessiné hors écran par AppKit (`cacheDisplayInRect:toBitmapImageRep:`, coins arrondis) : ça marche écran verrouillé, sans l'autorisation d'enregistrement de l'écran. Le PNG est écrit de façon asynchrone : attendre ~0,5 s avant l'étape suivante. C'est ainsi que les captures `docs/editeur-*.png` sont faites (réduites avec `sips -Z 1600` puis `pngquant`).
- `TACTILE_APPARENCE=clair|sombre` force le thème de l'app (`NSApp setAppearance:` Aqua / DarkAqua) ; la page suit par `prefers-color-scheme`. Pour capturer les deux thèmes sans toucher au réglage du Mac.
- `TACTILE_OUVRIR=k target/release/tactile` présente directement la sous-barre `k` (1 = premier slider ou groupe) : utile pour la capturer.
- `TACTILE_MASQUER=1` réduit la barre 2 s après le lancement ; `TACTILE_RETOUR=1` la ramène à 4 s comme un toucher sur ✳.
- Après une réduction, l'icône ✳ doit être réaffichée (`plateau::montrer_icone`) : le système l'oublie quand une barre système est présentée.
- `screencapture -b barre.png` capture la Touch Bar.
- `top -l 3 -s 3 -pid $(pgrep -x tactile) -stats cpu,mem` mesure la consommation au repos (cible : moins de 1 % de CPU) ; `footprint -p $(pgrep -x tactile)` donne l'empreinte mémoire, à surveiller après des rechargements (`touch` du fichier de config).
