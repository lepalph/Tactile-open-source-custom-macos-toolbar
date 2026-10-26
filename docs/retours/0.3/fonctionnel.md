# Campagne 0.3 : volet fonctionnel sur la vraie Touch Bar

26 octobre 2026, MacBook Pro M1 13 pouces, macOS 15.8.1, binaire release de `main` (commits `1ac97c1` à `6c58054`). Tous les essais ont tourné avec `TACTILE_CONFIG` sur une copie : la vraie `~/.config/tactile/barre.toml` a été sauvegardée au début et comparée (`cmp`) à la fin, sans écart.

**Écran verrouillé toute la session** (`IOConsoleLocked` = `true` au début, à chaque minute des deux endurances, avant la matrice et à la fin). Conséquences :

- La Touch Bar s'est quand même dessinée : une barre présentée par Tactile réveille la Touch Bar, et `screencapture -b` rend son contenu (voir les captures). Ces captures donnent une indication, mais le protocole les compte comme **non vérifiables**, jamais comme réussies.
- La barre d'Apple (étapes 7-8) et le placement « Control Strip visible » (`masquer_control_strip = false`) restent noirs en capture : non vérifiables.
- `frontmostApplication` reste `loginwindow` après `open -a TextEdit` : le changement d'app est **simulé** (`APP_SIMULEE`) dans l'autotest et dans la mesure de fluidité. Le vrai passage au premier plan n'est pas prouvé.

## 1. Autotest étendu (`scripts/verifier.sh`)

Résultat final sur `main` à jour (`dd88f89` + ce rapport) : **`AUTOTEST fin 0 échec(s)`**. 40 captures conformes à l'écran attendu, 2 non vérifiables (barre d'Apple, étapes 7-8, noires comme prévu). Le script sort avec le code 2 (« des écrans n'ont pas pu être vus »), qui est le comportement documenté pour ce cas.

Nouvelles étapes (31-42), chacune avec une preuve interne :

| Étape | Ce qui est fait | Preuve interne | Capture |
|---|---|---|---|
| 31 | Serveur `python3 -m http.server` local (port 47811, JSON fixe `{"t": 21.5}`), barre des sources installée | 8 contrôles présents | barre verte en tête : ok |
| 32 | `texte` lié à une `commande` (`cat valeur.txt`), à la source `http` locale, à la source `mcp` (`exemples/mcp/horloge-mcp.py`) | « val un », « 21.5 °C », « MCP 20:21:16 » | ok |
| 33 | `valeur.txt` passe à « deux » | « val deux » relu en ≤ 5 s | ok |
| 33-34 | Serveur MCP tué (groupe de processus) | « ⚠ source » | ok ([capture](captures/fonctionnel-sources.png)) |
| 35 | Relance automatique | nouveau pid (24487 → 24974), « MCP 20:21:31 » | ok |
| 36 | Logo perso : `image` PNG + `gabarit = true` + `affichage = "icone"` | image `isTemplate`, `imagePosition` = image seule | ok |
| 36 | Captures `capture_ecran` / `capture_zone` | construction seule : la commande du bouton vaut `CaptureEcran` / `CaptureZone` ; jamais appuyées | ok |
| 36-38 | `mission_control` puis `bureau`, chacun ouvert puis refermé à l'étape suivante | `open -b` réussi 4 fois ; la commande de chaque bouton est la bonne | ok (effet à l'écran non visible : écran verrouillé) |
| 39 | `tactile --exporter` d'une barre à logo, puis `tactile --importer … --oui` dans `TACTILE_CONFIG` | `.zip` (signature `PK`), deux codes de sortie 0 | barre orange : ok ([capture](captures/fonctionnel-import.png)) |
| 40 | Rechargement à chaud de la barre importée | bouton « Importée » avec son logo, présenté | ok |
| 40-41 | Fichier d'origine réécrit, rechargé à chaud | bouton importé parti | ok |
| 42 | Barre de l'utilisateur, arrêt des serveurs | aucun enfant `horloge-mcp` ni `http.server` | ok |

Le lanceur (icône, point « ouverte ») était déjà couvert par les étapes 23-25. `verrouiller` et `forcer_quitter` n'ont été ni appuyés ni exécutés.

Une étape qui attend une source se rejoue toutes les 0,5 s (`patienter`, 15 s au plus) sans nouvelle ligne `pas`, donc sans capture en plus. L'import est sauté si `TACTILE_CONFIG` n'est pas posé.

`scripts/verifier.sh` a aussi changé :

- il faisait `pkill -x tactile`, qui tuait aussi les éditeurs `--personnaliser` ouverts. Il arrête maintenant la barre seulement : l'app installée ou `target/release/tactile`, par chemin exact et sans argument (`pgrep -f '^(…)$'`). Vérifié avec `pgrep -fl` avant et après : les éditeurs `--personnaliser` des autres agents ont survécu ;
- il relance `~/Applications/Tactile.app` si elle tournait ;
- il fixe `TACTILE_CONFIG` sur une copie dans `target/verification/` ;
- il reconnaît la barre importée (orange) et tue le serveur HTTP de l'autotest s'il a été interrompu.

## 2. Endurance, 30 minutes

Charge : `exemples/vitrine.toml` (tiroir de 4 lanceurs, musique, météo Open-Meteo, batterie, heure), un `script` JSON à `intervalle = 0.3`, un `texte` lié à une source `http` locale à `toutes_les = 5`, et un `touch` du fichier toutes les 20 s (90 rechargements). Une mesure par minute : empreinte (`vmmap --summary`, *Physical footprint*), CPU (`top` sur 2 s, et temps CPU cumulé `ps -o time`), enfants (`pgrep -P`), descripteurs (`lsof -p | wc -l`).

### Premier passage : fuite trouvée

| Minute | 1 | 6 | 11 | 16 | 21 | 26 | 29 |
|---|---|---|---|---|---|---|---|
| Empreinte | 16,6 Mo | 19,0 | 20,7 | 22,3 | 24,1 | 25,8 | 26,6 |
| Temps CPU cumulé | 0,5 s | 2,5 | 4,5 | 7,1 | 9,8 | 12,4 | 14,0 |

+10 Mo en 28 minutes, sans palier. Le CPU par minute montait aussi (0,33 s puis 0,55 s). `heap` montrait 823 `NSImage` au lieu de 60 et des `CUINamedImage` / `CUIRenditionKey` (CoreUI) toujours plus nombreux. Isolement, 3 minutes chacun :

| Essai | Empreinte | NSImage |
|---|---|---|
| vitrine, `touch` toutes les 5 s | 13,6 → 19,9 Mo | 60 → 823 |
| vitrine, sans `touch` | 13,4 → 13,9 Mo | 60 → 60 |
| 2 boutons (dont 1 symbole), 30 rechargements | 10,7 → 12,7 Mo | 23 → 55 |
| 1 lanceur seul (icône en cache), 30 rechargements | 11,3 → 12,7 Mo | 12 → 12 |

Cause : chaque rechargement redemandait chaque symbole SF (`imageWithSystemSymbolName:`), qui rend un NSImage neuf, et cet objet restait en mémoire. Correction : les symboles sont gardés par nom (`vues::symbole`, même principe que le cache des images de fichier). Voir « Bugs ».

### Second passage, après correction

| Minute | 0 | 1 | 5 | 10 | 15 | 20 | 25 | 30 |
|---|---|---|---|---|---|---|---|---|
| Empreinte | 14,1 Mo | 16,1 | 17,1 | 17,5 | 17,7 | 17,8 | 17,7 | 18,0 |
| CPU `top` | 0,5 % | 0,6 | 0,5 | 0,7 | 0,4 | 0,5 | 0,4 | 0,5 |
| Temps CPU cumulé | 0,22 s | 0,74 | 2,87 | 5,55 | 7,89 | 9,96 | 12,08 | 14,63 |
| Enfants (`pgrep -P`) | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| Descripteurs | 40 | 40 | 40 | 40 | 40 | 40 | 40 | 40 |

| Critère | Cible | Mesuré | Verdict |
|---|---|---|---|
| Mémoire | ≤ +2 Mo sur 30 min | +1,9 Mo de la minute 1 à la minute 30 (palier vers 17,8-18,0 Mo dès la minute 15) ; +3,9 Mo depuis la minute 0, prise 20 s après le lancement, pendant la montée initiale | **limite** : stable après la montée initiale, mais +3,9 Mo si on compte depuis le lancement |
| CPU moyen | ≤ 0,5 % | 14,41 s en 30 min = **0,80 %** | **échec** sous cette charge |
| Orphelins | 0 | 0 (rien sous `pgrep -P` à chaque minute ; après `kill`, aucun `zsh -c`, `curl` ni processus de l'essai rattaché à launchd) | ok |
| Descripteurs | stables | 40 du début à la fin | ok |

D'où vient le CPU (3 minutes chacun, binaire corrigé) :

| Charge seule | CPU |
|---|---|
| vitrine au repos | 0,24 % |
| un `script` à 0,3 s, seul | 0,40 % (200 lancements de `zsh` par minute) |
| vitrine + `touch` toutes les 20 s | 0,40 % |

Ce n'est pas une fuite : le temps CPU par minute reste stable (≈ 0,48 s/min) sur les 30 minutes. La charge demandée, avec un script à 0,3 s et un rechargement toutes les 20 s, dépasse simplement le budget de 0,5 %. À noter pour la doc : un script en dessous de 1 s coûte à lui seul presque tout ce budget.

## 3. Fluidité (`TACTILE_MESURE=1`)

Nouveau mode `TACTILE_AUTOTEST=fluidite` : 20 ouvertures et 20 fermetures de sous-barre par le même chemin qu'un doigt (`performClick:`), 20 allers-retours vers une barre de TextEdit, 20 `installer`, toutes les 0,3 s. Ensuite, 20 `touch` du fichier à 1,5 s d'écart. Config : copie de la barre de l'utilisateur (71 lignes) + une `[[application]]` TextEdit.

| Chemin | Essais | Médiane | Max | Cible |
|---|---|---|---|---|
| Enregistrement du fichier → barre rechargée | 20 | 119,5 ms | 220,0 ms | < 300 ms |
| `installer` complet (rechargements) | 20 | 23,8 ms | 27,6 ms | |
| `installer` complet (autotest) | 20 | 21,3 ms | 25,2 ms | |
| Appui → sous-barre montrée | 20 | 0,1 ms | 0,2 ms | < 30 ms |
| ✕ → base montrée | 20 | 0,6 ms | 1,1 ms | |
| App activée → barre de l'app montrée | 40 | 0,3 ms | 1,1 ms | < 50 ms |

Changement d'app **simulé** (écran verrouillé) : on mesure le travail de Tactile après la notification, pas la latence de la notification elle-même, ni le dessin par la Touch Bar.

## 4. Matrice

| Cas | Comment | Preuve | Capture |
|---|---|---|---|
| `masquer_control_strip` true → false à chaud | `sed` sur le fichier | rechargé ; avis « la barre principale déborde (1031 pt pour 685) : 3 éléments sur 11 cachés » (juste : la barre de l'utilisateur ne tient pas avec le Control Strip) | noire : non vérifiable (le placement 0 ne réveille pas la Touch Bar, piège documenté) |
| false → true à chaud | `sed` | barre revenue | identique à l'initiale |
| Config vide | `: > barre.toml` | « config vide : aucun élément, barre par défaut affichée » | badge rouge « ⚠ config vide » sur la barre par défaut ([capture](captures/fonctionnel-config-vide.png)) |
| Config cassée | `type = "truc"` à la ligne 6 | « config ignorée, … ligne 6 : valeur inconnue «truc» » | badge « ⚠ config ligne 6 », barre précédente gardée ([capture](captures/fonctionnel-config-cassee.png)) |
| Config rétablie | copie de la sauvegarde | badge parti | identique à l'initiale |
| Barre par application (TextEdit) | `exemples/par-application.toml`, `open -a TextEdit`, puis `quit` | premier plan resté `loginwindow` (`lsappinfo front`) : pas de bascule réelle. Couvert en simulé par l'autotest (étapes 26-29 : barre de TextEdit, ✕ qui y revient, badge sur elle, retour à la principale) | non vérifiable |
| `touche` sur 3 dispositions | test `trois_dispositions` : US, French, German chargées explicitement par TIS, sans changer la disposition de l'utilisateur ni rien envoyer | US : cmd+a → 0, cmd+z → 6, cmd+y → 16 ; AZERTY : 12, 13, 16 ; QWERTZ : 0, 16, 6 ; cmd+shift+4 → 21 partout ; « / » jamais sur le pavé | sans objet |
| Envoi réel d'une touche | `f15` (sans effet), étape 23 | `CGEventPost` sans erreur | sans objet |

## Bugs

| # | Bug | Gravité | Test qui échouait d'abord | Correction |
|---|---|---|---|---|
| F1 | Chaque rechargement gardait un NSImage par symbole SF : +21 NSImage par rechargement avec la vitrine, 16,6 → 26,6 Mo en 30 min (90 rechargements), CPU qui montait avec | majeur (un utilisateur qui édite souvent sa barre) | `symbole_reutilise_d_un_rechargement_a_l_autre` (deux appels donnaient deux objets) | `6c58054` : cache par nom dans `vues::symbole` |
| F2 | `scripts/verifier.sh` faisait `pkill -x tactile`, qui tuait aussi les éditeurs `--personnaliser` | mineur (outil de dev) | constaté avec `pgrep -fl` | `1ac97c1` : arrêt par chemin exact, sans argument |

Observations sans correction :

- 0,8 % de CPU sous la charge d'endurance (voir plus haut) : c'est le coût des processus lancés, pas une fuite. À dire dans la doc des scripts : en dessous d'une seconde, un script coûte cher.
- Après correction, il reste environ +2 Mo de montée pendant les 60 premiers rechargements, puis un palier (+0,4 Mo pour les 60 suivants). `heap` n'en explique que 300 Ko : le reste vient de l'allocateur (`MALLOC_SMALL`) et des `IOSurface` de la Touch Bar. C'est le même palier que celui noté en 0.2 (14 → 18 Mo).
- Un bouton `affichage = "icone"` sans texte a pour titre « Button » (valeur par défaut d'`NSButton`). Il n'est pas dessiné (image seule), mais VoiceOver peut le lire : à voir avec le volet accessibilité.
- Un PNG opaque avec `gabarit = true` devient un carré blanc plein. C'est normal pour un gabarit, mais pas évident pour un débutant.

## Ce qui n'a pas pu être vu

- Tout ce qui demande l'écran déverrouillé : la barre d'Apple et le retour par ✳ en capture, le placement Control Strip visible, la vraie bascule d'app (TextEdit au premier plan), l'effet de Mission Control et du bureau à l'écran.
- Les captures d'écran système (`capture_ecran`, `capture_zone`) : seulement construites, comme demandé.
- `verrouiller`, `forcer_quitter` : jamais lancés.
- Un serveur MCP distant (Streamable HTTP) : pas de serveur de test ; c'est couvert par les tests unitaires du client.
- La latence de dessin de la Touch Bar elle-même : elle ne se mesure pas depuis Tactile.

## Fichiers

- Autotest : `src/barre/autotest.rs` (étapes 31-42, mode `fluidite`) ; `scripts/verifier.sh`.
- Tests ajoutés : `logo_monochrome_est_un_gabarit` (`elements.rs`), `trois_dispositions` (`clavier.rs`), `symbole_reutilise_d_un_rechargement_a_l_autre` (`vues.rs`).
- Validation : `cargo build --release` sans avertissement, `cargo clippy --all-targets -D warnings`, `cargo doc` avec `-D warnings`, `cargo test` vert avec et sans `CI=1`.
