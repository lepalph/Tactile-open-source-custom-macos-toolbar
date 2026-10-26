# Architecture

Ce document sert à reprendre le code : où est quoi, comment ajouter quelque chose, et les pièges déjà rencontrés. Le pourquoi des choix est dans [DECISIONS.md](DECISIONS.md).

## Le trajet d'un appui

```
main.rs
 ├─ config::charger()          lit ~/.config/tactile/barre.toml (ou la barre par défaut)
 ├─ config::surveiller()       fil qui relit le fichier quand sa date change, envoie la nouvelle barre par un canal
 └─ barre::lancer()            app AppKit sans icône dans le Dock
     ├─ plateau                icône ✳ dans le Control Strip d'Apple
     ├─ menu                   icône de la barre des menus
     ├─ installer(config)      construit les items, présente la barre principale
     └─ minuteur 0,1 s → tic   applique une config rechargée, rafraîchit heure / musique / scripts

appui sur un bouton
 → TactileCible::appui (le tag du bouton = index dans Etat::commandes)
 → declencher : Commande::Systeme(action) → actions::executer
                Commande::Ouvrir(k) / Fermer → montre le contenu de la sous-barre k / de la principale
                dans l'unique barre système
```

Tout ce qui touche AppKit tourne sur le fil principal. Le travail lent (une commande `script`, `media-control`) part dans un fil à part et renvoie son texte par un canal, relu au `tic` suivant.

## Où est quoi

| Fichier | Rôle |
|---|---|
| `src/main.rs` | Ligne de commande (`--help`, `--version`, `--verifier`), démarrage, journal `~/Library/Logs/Tactile.log` quand il n'y a pas de terminal. |
| `src/config.rs` | Types `Barre`, `Element`, `Reglage` ; lecture TOML avec erreurs en français ; surveillance du fichier. Tests de la config. |
| `src/actions/mod.rs` | Enum `Action` (ce qu'on écrit dans `action = …`) et `executer`. |
| `src/actions/luminosite.rs` | Luminosité de l'écran intégré (DisplayServices, privé). |
| `src/actions/volume.rs` | Volume et muet de la sortie par défaut (CoreAudio). |
| `src/actions/media.rs` | Titre en cours et commandes de lecture (via `media-control`). |
| `src/actions/clavier.rs` | Frappes simulées (esc). |
| `src/barre/mod.rs` | État de la barre, classe `TactileCible` (reçoit appuis, sliders, minuteur), présentation, `afficher` / `masquer`. |
| `src/barre/elements.rs` | Transforme chaque `Element` en item de Touch Bar ; sous-barres (`depliable`). |
| `src/barre/rafraichi.rs` | Éléments vivants : heure, titre en cours, scripts. |
| `src/barre/vues.rs` | Outils AppKit sans état (texte, symbole SF, couleur, largeur, barre). |
| `src/barre/plateau.rs` | Icône ✳ du Control Strip. |
| `src/menu.rs` | Icône de la barre des menus, lancement au démarrage (`SMAppService`). |
| `exemples/barre.toml` | Barre par défaut, embarquée dans le binaire. |
| `scripts/empaqueter.sh` | `Tactile.app` universelle signée ad hoc + `Tactile.zip`. |

## Recettes

### Ajouter une action

1. `src/actions/mod.rs` : une variante dans `Action`. Son nom en `snake_case` devient le mot de la config (`BasculerMuet` → `"basculer_muet"`) ; une variante avec valeur s'écrit `{ nom = valeur }`.
2. La fonction qui fait le travail, dans un fichier de `src/actions/`, avec un test qui change l'état, le relit, puis le restaure.
3. La branche correspondante dans `executer`.
4. Le test `chaque_forme_d_action` de `src/config.rs`, et le tableau des actions du README.

### Ajouter un type d'élément

1. `src/config.rs` : une variante dans `Element` (son nom en `snake_case` devient `type = "…"`), puis un test de lecture.
2. `src/barre/elements.rs` : la branche dans `construire` qui crée l'item.
3. S'il se met à jour seul : une variante dans `Rafraichi` (`src/barre/rafraichi.rs`) et sa branche dans `rafraichir`. Ne bloque jamais le fil principal : lance le travail dans un fil et renvoie le texte par `etat.envoi`.
4. Le tableau des éléments du README.

### Ajouter une entrée au menu

`src/menu.rs` : une méthode dans `TactileMenu` (`define_class!`), puis un `ajouter(menu, "Libellé", cible, Some(sel!(methode:)))` dans `installer`.

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
- Un avertissement `NSLayoutConstraint … exceeds internal limits` apparaît au lancement. Il est inoffensif ; sa source n'est pas encore identifiée.

## Vérifier un changement

- `scripts/verifier.sh` : la vérification complète. Lance Tactile avec `TACTILE_AUTOTEST=1`, qui actionne chaque contrôle par le même chemin qu'un doigt (`performClick:` sur le vrai bouton, message `glisse:` pour les sliders) et contrôle l'effet réel : sous-barres, luminosité et volume relus puis restaurés, barre d'Apple et retour par ✳, Claude vocal qui démarre et s'arrête, lecture/pause. Il capture la Touch Bar à chaque étape (`target/verification/`) et compare chaque capture à l'écran attendu (étapes 20-21 : l'élément rouge « ⚠ config ligne 6 » apparaît puis disparaît ; 22-25 : barre riche verte). Un écran noir (Touch Bar inactive) est signalé comme non vérifiable, jamais comme réussi : touche le trackpad pendant la vérification.
- `cargo test` : config, ligne de commande, parsing des médias, symbole SF inconnu, débordement, scripts (délai avec `sleep 30`, stderr, barre remplacée), et luminosité / volume réels (valeur changée, relue, restaurée). Avec `CI` défini, les tests matériels sortent tout de suite.
- `tactile --verifier [fichier]` : lit un fichier de barre et dit « OK : n éléments » ou l'erreur avec sa ligne, sans toucher la Touch Bar.
- `TACTILE_OUVRIR=k target/release/tactile` présente directement la sous-barre `k` (1 = premier slider ou groupe) : utile pour la capturer.
- `TACTILE_MASQUER=1` réduit la barre 2 s après le lancement ; `TACTILE_RETOUR=1` la ramène à 4 s comme un toucher sur ✳.
- Après une réduction, l'icône ✳ doit être réaffichée (`plateau::montrer_icone`) : le système l'oublie quand une barre système est présentée.
- `screencapture -b barre.png` capture la Touch Bar.
- `top -l 3 -s 3 -pid $(pgrep -x tactile) -stats cpu,mem` mesure la consommation au repos (cible : moins de 1 % de CPU).
