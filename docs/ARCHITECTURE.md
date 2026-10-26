# Architecture

Ce document sert à reprendre le code : où est quoi, comment ajouter quelque chose, et les pièges déjà rencontrés. Le pourquoi des choix est dans [DECISIONS.md](DECISIONS.md).

## Le trajet d'un appui

```
main.rs
 ├─ --personnaliser → editeur::lancer_seul()   la fenêtre de réglage seule, sans Touch Bar (voir plus bas)
 ├─ config::charger()          lit ~/.config/tactile/barre.toml (ou la barre par défaut)
 ├─ config::surveiller()       fil qui relit le fichier quand sa date change, envoie la nouvelle barre par un canal
 └─ barre::lancer()            app AppKit sans icône dans le Dock
     ├─ plateau                icône ✳ dans le Control Strip d'Apple
     ├─ menu                   icône de la barre des menus
     ├─ installer(config)      construit les items de la principale et d'une barre par [[application]], présente la base
     └─ minuteur 0,1 s → tic   applique une config rechargée, suit l'app active, rafraîchit heure / musique / scripts

appui sur un bouton
 → TactileCible::appui (le tag du bouton = index dans Etat::commandes)
 → declencher : Commande::Systeme(action) → actions::executer
                Commande::Ouvrir(k) / Fermer → montre le contenu de la sous-barre k / de la base (barre de l'app active ou principale)
                dans l'unique barre système

changement d'app au premier plan (Safari passe devant)
 → tic → suivre_app : base() lit NSWorkspace.frontmostApplication (bundle id, nom affiché),
   Barre::application_pour trouve la [[application]] → Etat::applis[k], sinon la principale
 → si la barre montrée est une base (principale ou barre d'app) : montrer(barre de l'app)
   si c'est une sous-barre : rien ; son ✕ (Commande::Fermer) montrera base()
 → sous la barre d'Apple (masquer) : montrer change le contenu sans représenter ; ✳ (afficher) présente base()
```

Tout ce qui touche AppKit tourne sur le fil principal. Le travail lent (une commande `script`, `media-control`) part dans un fil à part et renvoie son texte par un canal, relu au `tic` suivant.

## Où est quoi

| Fichier | Rôle |
|---|---|
| `src/main.rs` | Ligne de commande (`--help`, `--version`, `--verifier`), démarrage, journal `~/Library/Logs/Tactile.log` quand il n'y a pas de terminal. |
| `src/config.rs` | Types `Barre`, `Element`, `Reglage`, `Application` (barre par app : `application_pour`, fusion `herite` dans `elements_avec`) ; lecture TOML avec erreurs en français ; surveillance du fichier. Tests de la config. |
| `src/actions/mod.rs` | Enum `Action` (ce qu'on écrit dans `action = …`) et `executer`. |
| `src/actions/luminosite.rs` | Luminosité de l'écran intégré (DisplayServices, privé). |
| `src/actions/volume.rs` | Volume et muet de la sortie par défaut (CoreAudio). |
| `src/actions/media.rs` | Titre en cours et commandes de lecture (via `media-control`). |
| `src/actions/clavier.rs` | Frappes simulées : `echap`, `{ touche = "cmd+shift+4" }` (type `Touche`, vérifié à la lecture de la config), `{ keycode = n }`. Les caractères sont résolus sur la disposition clavier courante (`UCKeyTranslate`) : « a » donne le bon code en AZERTY. |
| `src/barre/mod.rs` | État de la barre, classe `TactileCible` (reçoit appuis, sliders, minuteur), présentation, `afficher` / `masquer`. |
| `src/barre/elements.rs` | Transforme chaque `Element` en item de Touch Bar ; sous-barres (`depliable`). |
| `src/barre/rafraichi.rs` | Éléments vivants : heure, titre en cours, scripts (texte brut ou JSON `Riche`), batterie. |
| `src/barre/batterie.rs` | Lecture et analyse de `pmset -g batt`, symbole de pile selon le niveau. |
| `src/barre/vues.rs` | Outils AppKit sans état (texte, symbole SF, image de fichier, couleur, style du titre, largeur, barre). |
| `src/barre/plateau.rs` | Icône ✳ du Control Strip. |
| `src/menu.rs` | Icône de la barre des menus, lancement au démarrage (`SMAppService`). |
| `src/editeur/mod.rs` | Fenêtre « Personnaliser ma barre » : NSWindow + WKWebView, pont JS ⇄ Rust, écriture et vérification du fichier. |
| `src/editeur/editeur.html` | La page de l'éditeur (HTML, CSS et JS dans un seul fichier, embarqué par `include_str!`, sans réseau). |
| `exemples/barre.toml` | Barre par défaut, embarquée dans le binaire ; préréglage « Musique » de l'éditeur. `barre.en.toml` : la même avec les alias anglais (test `alias_anglais_donnent_la_meme_barre`). |
| `exemples/simple.toml`, `exemples/developpeur.toml` | Les deux autres préréglages de l'éditeur. |
| `exemples/par-application.toml` | Barres par application commentées (testé par `aller_retour_toml`). |
| `scripts/empaqueter.sh` | `Tactile.app` universelle signée ad hoc + `Tactile.zip`. |

## Recettes

### Ajouter une action

1. `src/actions/mod.rs` : une variante dans `Action`. Son nom en `snake_case` devient le mot de la config (`BasculerMuet` → `"basculer_muet"`) ; une variante avec valeur s'écrit `{ nom = valeur }`. Ajoute son alias anglais (`#[serde(alias = "toggle_mute")]`).
2. La fonction qui fait le travail, dans un fichier de `src/actions/`, avec un test qui change l'état, le relit, puis le restaure. Une action qui lance un programme dont l'échec doit se voir passe par `journaliser` (stderr au journal, sans bloquer).
3. La branche correspondante dans `executer`.
4. Le test `chaque_forme_d_action` de `src/config.rs`, le tableau des actions et le glossaire du README (le test `glossaire_du_readme` exige que chaque ligne du glossaire soit exercée par `TOUT_FR` / `TOUT_EN`).
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

- **JS → Rust** : `window.webkit.messageHandlers.tactile.postMessage(JSON)`, reçu par `TactileEditeur` (`userContentController:didReceiveScriptMessage:`). Messages : `pret` (la page est chargée), `enregistrer { barre, fermer? }`, `revenir`, `prereglage { nom }`, `symboles { noms }`, `choisirApp`, `verifierApp { nom }`, `modifie { valeur }`, `fermer`, `ouvrirFichier`, `relire`, `titre { texte }`, `journal { texte }`, `capture { chemin }`.
- **Rust → JS** : `evaluateJavaScript:` sur `window.tactile` : `charger(barre, aPrecedent)`, `illisible(message, extrait, aPrecedent)`, `glyphes({ nom: dataURL | null })`, `enregistre(manuel, aPrecedent)`, `revenu()`, `abimeGarde(nom)`, `erreur(texte)`, `appChoisie(nom)`, `appVerifiee(nom, trouvee, suggestion)`, `demanderFermeture()`.

La barre circule en JSON avec les mêmes noms que le TOML (`serde_json` sur `config::Barre`, qui dérive `Serialize`). À l'enregistrement, Rust convertit le JSON en `Barre`, refuse une barre sans élément ni `[[application]]` (elle éteindrait la Touch Bar), écrit le TOML (`config::ecrire`, en-tête fixe + `toml::to_string_pretty`), le **relit** avec `config::lire`, puis `verifier` refuse ce que la barre ne saurait pas afficher (symbole SF inconnu, couleur hors `#RRGGBB`, intervalle < 0,1 s). Seulement alors le fichier est écrit. La barre se recharge par la surveillance habituelle du fichier : l'éditeur ne parle jamais à `barre::`.

Rien de ce que l'utilisateur a écrit ne se perd :

- **Historique** : avant d'écrire, les copies glissent d'un cran (`.precedent` → `.precedent-2` … → `.precedent-5`, la plus ancienne disparaît) et le fichier actuel devient `barre.toml.precedent` (`VERSIONS = 5`). « Revenir » fait tourner la chaîne : `.precedent` devient le fichier, chaque copie remonte, et le fichier quitté passe en dernier ; un clic de plus remonte encore, et après la plus ancienne on retombe sur la plus récente. La version restaurée est relue avant d'être remise.
- **Copie `.manuel`** : si le fichier a des commentaires autres que l'en-tête de l'éditeur (`ecrit_a_la_main`, un `#` hors des guillemets), il est copié une fois en `barre.toml.manuel`, jamais écrasé ensuite ; le message de succès le dit.
- **Fichier illisible** : `envoyer_config` n'envoie pas de barre vide mais `illisible(message, extrait)` (la ligne en cause, sans le chemin). La page n'affiche alors que l'erreur, désactive Enregistrer et l'ajout d'éléments, et propose « Ouvrir le fichier » (`open -t`), « Relire le fichier » et « Repartir d'une barre toute faite ». Une barre toute faite demandée sur un fichier illisible le copie d'abord en `barre.toml.abime-AAAAMMJJ-HHMMSS` (`mettre_de_cote`, pas de seconde copie identique) ; le fichier n'est remplacé qu'au prochain Enregistrer.
- **Changements non enregistrés** : la page envoie `modifie { valeur }` à chaque changement d'état ; Rust le garde (`MODIFIE`) et met le point dans le bouton rouge (`setDocumentEdited:`). `windowShouldClose:` refuse alors la fermeture et appelle `demanderFermeture()` ; la page répond `fermer` ou `enregistrer { fermer: true }` (qui ne ferme que si l'écriture a réussi). Les confirmations (fermer, barre toute faite, Revenir) sont un `<dialog>` modal de la page : Échap vaut Annuler, et le pilote JS peut les cliquer. Quitter l'app par le menu ne passe pas par `windowShouldClose:`.
- **Nom d'app** : `verifierApp` répond par `chercher_app` : chemin existant, bundle id (`NSWorkspace URLForApplicationWithBundleIdentifier:`), sinon `mdfind` sur `kMDItemDisplayName` (sans casse). Introuvable : `suggestion` prend le nom installé le plus court qui contient la saisie, sinon le plus proche en distance d'édition. Sans index Spotlight, rien n'est signalé.

Les symboles SF n'existent pas dans le web : Rust les rend en PNG (`NSImage imageWithSystemSymbolName:` → `TIFFRepresentation` → `NSBitmapImageRep`), les envoie en data URL, et la page les affiche en `mask-image` sur `currentColor`. Un nom inconnu renvoie `null` : la page le signale et l'enregistrement le refuse (un symbole inconnu fait planter la barre, cf. SYNTHESE A1).

La page suit le régime « Opérer » du skill ui-ux-director : police système, neutres macOS, un seul accent (`AccentColor`), aperçu à l'échelle 1 (1004 × 60 pt, 685 avec le Control Strip), cibles ≥ 44 px, tout faisable au clavier, clair et sombre selon le système.

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

- Les keycodes macOS sont physiques : `0` est la touche « a » en QWERTY et « q » en AZERTY. `clavier.rs` cherche le code d'un caractère avec `UCKeyTranslate` sur la disposition courante ; seules les touches sans caractère (f1-f20, flèches, esc…) ont un code fixe.
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
- Au rechargement de la config, les anciens objets AppKit ne sont pas libérés. C'est assumé : un rechargement est rare et l'app reste sous 20 Mo.
- `NSImage imageWithSystemSymbolName:` renvoie `nil` pour un symbole inconnu, et `buttonWithImage: nil` lève `NSInternalInconsistencyException` : Tactile mourait sur une faute de frappe et replantait à chaque relancement. `vues::symbole` renvoie `Option`, et `elements::bouton` retombe sur le nom en texte.
- Un `script` tourne dans son propre groupe de processus (`process_group(0)`) : tuer `zsh -c` seul laisse son enfant (`sleep`, `curl`…) orphelin. Le groupe est tué au-delà de `delai` et quand la barre est remplacée (`Etat::processus`, mis à `None` pour que le script qui démarre pendant le remplacement soit tué aussi). Quitter Tactile pendant un script laisse encore ce script finir seul.
- Le `placement` d'une barre système n'est lu qu'à sa présentation : changer `masquer_control_strip` à chaud impose `minimizeSystemModalTouchBar:` avant de représenter. Une présentation en placement 0 (Control Strip visible) ne réveille pas la Touch Bar : sans toucher humain, `screencapture -b` rend une image noire.
- Les notifications macOS (`UNUserNotificationCenter`, et même `NSUserNotificationCenter`) exigent un bundle : depuis `target/release/tactile` elles plantent ou ne font rien. L'erreur de config s'affiche donc dans la barre elle-même (`afficher_erreur`).
- L'app active est **sondée** au `tic` (`frontmostApplication`), pas observée par `NSWorkspaceDidActivateApplicationNotification` : même coût (rien quand la config n'a pas d'`[[application]]`), et le sondage rattrape tout seul le changement d'app survenu pendant qu'une sous-barre était ouverte. Écran verrouillé, `frontmostApplication` reste `com.apple.loginwindow` même après `open -a TextEdit` : l'autotest le détecte et simule l'app active (`APP_SIMULEE`), en le disant au journal ; le passage réel au premier plan ne se vérifie qu'écran déverrouillé.
- `scripts/verifier.sh` n'affiche pas les lignes du journal qui contiennent « pas » entre deux espaces (c'est son filtre des lignes `AUTOTEST n pas attendu=…`) : un message d'étape ne doit pas contenir ce mot.
- Une barre par application est construite entière à chaque rechargement, comme la principale : ses scripts tournent même quand elle n'est pas montrée, et avec `herite` les éléments repris de la principale existent deux fois (deux scripts, deux horloges).
- Un avertissement `NSLayoutConstraint … exceeds internal limits` apparaît au lancement. Il est inoffensif ; sa source n'est pas encore identifiée.

## Vérifier un changement

- `scripts/verifier.sh` : la vérification complète. Lance Tactile avec `TACTILE_AUTOTEST=1`, qui actionne chaque contrôle par le même chemin qu'un doigt (`performClick:` sur le vrai bouton, message `glisse:` pour les sliders) et contrôle l'effet réel : sous-barres, luminosité et volume relus puis restaurés, barre d'Apple et retour par ✳, Claude vocal qui démarre et s'arrête, lecture/pause. Il capture la Touch Bar à chaque étape (`target/verification/`) et compare chaque capture à l'écran attendu (étapes 20-21 : l'élément rouge « ⚠ config ligne 6 » apparaît puis disparaît ; 22-25 : barre riche verte ; 26-29 : barre de TextEdit, bouton bleu, après `open -a TextEdit`, puis retour à la principale quand TextEdit quitte). Un écran noir (Touch Bar inactive) est signalé comme non vérifiable, jamais comme réussi : touche le trackpad pendant la vérification.
- `cargo test` : config, ligne de commande, parsing des médias, symbole SF inconnu, débordement, scripts (délai avec `sleep 30`, stderr, barre remplacée), et luminosité / volume réels (valeur changée, relue, restaurée). Avec `CI` défini, les tests matériels sortent tout de suite.
- `tactile --verifier [fichier]` : lit un fichier de barre et dit « OK : n éléments » ou l'erreur avec sa ligne, sans toucher la Touch Bar.
- `TACTILE_CONFIG=chemin` remplace `~/.config/tactile/barre.toml` pour tout le programme (lecture, surveillance, éditeur) : indispensable pour essayer l'éditeur sans toucher à sa vraie barre, par exemple `TACTILE_CONFIG=/tmp/essai.toml target/release/tactile --personnaliser`. `--personnaliser` n'ouvre que la fenêtre : ni Touch Bar, ni icône de menu.
- `TACTILE_EDITEUR_JS=fichier.js` : script évalué dans la page de l'éditeur une fois la config chargée ; avec le message `capture { chemin }` (PNG de la fenêtre, pris par l'app elle-même : `screencapture` demanderait l'autorisation d'enregistrement de l'écran), c'est ainsi que les captures `docs/editeur-*.png` sont faites. Écran verrouillé = capture blanche.
- `TACTILE_OUVRIR=k target/release/tactile` présente directement la sous-barre `k` (1 = premier slider ou groupe) : utile pour la capturer.
- `TACTILE_MASQUER=1` réduit la barre 2 s après le lancement ; `TACTILE_RETOUR=1` la ramène à 4 s comme un toucher sur ✳.
- Après une réduction, l'icône ✳ doit être réaffichée (`plateau::montrer_icone`) : le système l'oublie quand une barre système est présentée.
- `screencapture -b barre.png` capture la Touch Bar.
- `top -l 3 -s 3 -pid $(pgrep -x tactile) -stats cpu,mem` mesure la consommation au repos (cible : moins de 1 % de CPU).
