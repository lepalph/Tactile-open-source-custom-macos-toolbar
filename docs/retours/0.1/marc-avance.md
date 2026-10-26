# Retour de Marc, testeur « avancé » — Tactile 0.1

## Contexte

- Marc, 41 ans, dev backend. MacBook Pro 13" M1 Touch Bar, macOS 15.8. Configure tout en fichiers texte, pousse les outils dans leurs coins.
- Build : `mise exec -- cargo build --release` depuis le dépôt (binaire `target/release/tactile`, 1,3 Mo). Lancé depuis un terminal avec stderr redirigé vers un journal, puis une fois en `env -i` pour imiter un lancement Finder.
- Lu avant de tester : README, `exemples/`, `docs/ARCHITECTURE.md`, `src/config.rs` (+ `elements.rs`, `rafraichi.rs`, `vues.rs`, `mod.rs` pour comprendre ce que je voyais).
- Méthode : modification de `~/.config/tactile/barre.toml` à chaud (c'est le vrai usage), capture Touch Bar à chaque étape (`screencapture -b`), lecture du journal. Sous-barres ouvertes avec `TACTILE_OUVRIR=k`. Je ne peux pas toucher la barre à distance : les appuis (action shell qui échoue, appui long) n'ont pas été exercés, je le dis où ça compte.
- Captures : `docs/retours/0.1/captures/marc-*.png`.
- Config sauvegardée avant tout, **restaurée à la fin**, Tactile relancé (voir dernière section).

## Ce qui a marché

- **Rechargement à chaud** : fiable, dans la seconde, sans scintillement perceptible sur les captures. Une sous-barre ouverte reste ouverte après rechargement.
- **Groupes imbriqués** : `groupe` dans `groupe` (Dev › Git ›) fonctionne, la ✕ ramène bien au niveau au-dessus (`marc-A-sousbarre-1.png`, `marc-A-sousbarre-2.png`).
- **Symbole + texte** sur un bouton (`hammer.fill` + « Build ») : rendu propre, couleur de fond respectée.
- **Scripts** : charge système, batterie, sortie multi-lignes (seule la première ligne compte), `~` expansé, repli `|| echo '?'` fonctionne. `espace`, `souple`, `largeur` font ce qu'ils disent.
- **Erreurs de config** : la barre en place survit à un TOML cassé, un `type` inconnu, un champ en trop ; le message dans le journal est précis (fichier, ligne, « attendu parmi … »). Exemples obtenus :
  - `ligne 6 : valeur inconnue «batterie», attendu parmi «bouton», «slider», …`
  - `ligne 2 : champ inconnu «largeur», attendu parmi «texte», «symbole», «element»`
  - `ligne 3 : invalid basic string, attendu «"»` (moitié anglais, mais exploitable).
- **Fichier supprimé** : la barre par défaut prend le relais, et la recréation du fichier remet la mienne. Sans message dans le journal, par contre.
- **Conso** (`top -l 3 -s 3`, 14 Mo partout) :

  | Barre | CPU |
  |---|---|
  | Par défaut (`en_cours` toutes les 2 s) | 0,0–0,2 % |
  | 2 boutons, rien de vivant | 0,0–0,1 % |
  | 1 script `cat` à 0,3 s | 0,4 % |
  | 3 scripts (`date`, `uptime`, `pmset`) à 0,3 s | 1,6 % |
  | 1 script à `intervalle = 0` (ramené à 0,1 s) | 1,3 % |

  Les `zsh -c` forkés (jusqu'à 10 par seconde) n'apparaissent pas dans le compteur du processus ; `top` ne les attrape pas non plus, trop courts. Le chiffre du README (0,5 %) est tenu.

## Bugs

### B1 — Un symbole SF inexistant plante Tactile et laisse une Touch Bar noire — **bloquant**

- Étapes : dans un `bouton`, `symbole = "symbole.qui.nexiste.pas"` (une faute de frappe suffit : `terminal.fil`). Enregistrer.
- Attendu : comme pour les autres erreurs de config, barre en place conservée + ligne dans le journal.
- Obtenu : `panicked at src/barre/elements.rs:21:26 : NSInternalInconsistencyException 'Invalid parameter not satisfying: image != nil'`. Le processus meurt. La Touch Bar est **entièrement noire** à ma capture (`marc-B6-apres-crash.png`), plus de barre d'Apple visible, plus d'esc. Et **au relancement, ça replante** tant que le fichier contient le symbole : il faut corriger le fichier à l'aveugle, sans barre.
- Même chemin de code (`elements::bouton` → `buttonWithImage:`) pour `symbole` d'un `slider` ou d'un `groupe`, donc même crash attendu.
- Depuis le Finder, l'utilisateur n'a que `~/Library/Logs/Tactile.log` avec une backtrace Rust de 30 lignes pour comprendre. Cause : `NSImage imageWithSystemSymbolName:` renvoie `nil`, non vérifié dans `vues::symbole`.

### B2 — Un texte long sans `largeur` avale toute la barre — **majeur**

- Étapes : `bouton` avec un `texte` de ~200 caractères, suivi de quatre autres éléments.
- Attendu : texte tronqué (« … ») comme le fait le `groupe`, les voisins toujours visibles.
- Obtenu : le bouton occupe toute la largeur, les quatre éléments suivants disparaissent, rien dans le journal (`marc-B2-long.png`). Le README dit « un texte plus large que l'élément est coupé » pour `script`, mais un `bouton` ou une `heure` au format long (`EEEE d MMMM yyyy 'à' HH:mm:ss zzzz`) ne le sont pas.

### B3 — Les éléments qui débordent disparaissent en silence — **majeur**

- Étapes : 15 `bouton` (puis 25) ; ou `largeur = 100000` sur un bouton suivi d'un autre.
- Attendu : au minimum une ligne dans le journal « N éléments ne tiennent pas (largeur X / 1004 pt) ».
- Obtenu : 12 boutons affichés, les 13 autres n'existent pas, aucun message (`marc-B1-25boutons.png`). Avec la largeur énorme, le bouton énorme **et** le suivant disparaissent (`marc-B3-largeurs.png`). Quand on a 1004 pt de large et un fichier de 30 blocs, deviner lequel a sauté est pénible.

### B4 — `masquer_control_strip = false` : sans effet à chaud, et bizarre au lancement — **majeur** (documenté « pas vérifié », donc le voici)

- À chaud (`true` → `false`, et inversement) : strictement rien ne change. `PLEIN_ECRAN` est bien mis à jour dans `installer`, mais le `placement` n'est relu qu'à une nouvelle présentation.
- Au lancement avec `false` : une **✕ système** apparaît tout à gauche, la zone utile rétrécit (≈ 685 pt : mon bouton esc et un bouton de fin sont tombés), et la droite reste noire sur la capture — pas de Control Strip visible (`marc-C-control-strip-lancement.png`). Je ne peux pas toucher la barre pour savoir si la ✕ ferme Tactile ou la barre ; dans le doute je l'ai retirée.
- Après un retour à `true` à chaud, la ✕ reste (`marc-B12-infini.png`).

### B5 — Un script qui ne se termine pas laisse un processus par rechargement — **majeur**

- Étapes : `commande = "sleep 1000; echo fini"`, `intervalle = 1`, puis enregistrer le fichier 3 fois (un commentaire ajouté).
- Attendu : l'ancien script est tué (ou au moins abandonné) quand sa barre est reconstruite ; un délai maximal (`intervalle`, ou un `delai`) coupe la commande.
- Obtenu : 4 paires `zsh -c` + `sleep` vivantes, toutes enfants de Tactile, plus un fil Rust bloqué par processus. L'élément reste vide pour toujours, sans indication. Sur une commande réseau qui pend (`curl` sans `--max-time`, une API qui ne répond pas), c'est la fuite assurée au fil de la journée.

### B6 — Le stderr des scripts est jeté — **majeur** (pour le diagnostic)

- Étapes : `commande = "commande_inexistante_xyz"` ou `"echo 'sur stderr' >&2"`.
- Attendu : « zsh: command not found: commande_inexistante_xyz » quelque part (journal), ou dans la case.
- Obtenu : case vide, journal vide. `Command::output()` capture stderr et on ne le lit jamais (`rafraichi.rs`). Idem pour le code de retour (`exit 1` affiche quand même « rc1 »). On écrit un script, on voit une case vide, et on n'a aucun moyen de savoir pourquoi, sauf à relancer la commande dans un terminal en espérant que l'environnement soit le même.

### B7 — `intervalle = 0` ou négatif accepté en silence — **mineur**

- Le README dit « 0,1 au minimum ». `0` et `-3` passent la validation (`config.rs` ne vérifie rien), `elements.rs` fait `.max(0.1)` sans le dire. Résultat : 10 exécutions par seconde, 1,3 % CPU, et l'utilisateur croit avoir un script « instantané ». Une erreur de config (« ligne N : intervalle doit être ≥ 0,1 ») serait cohérente avec le reste.

### B8 — Couleur invalide ignorée en silence — **mineur**

- `"rouge"`, `"#GGGGGG"`, `"#FFF"`, `"#B4552F80"` → bouton gris, rien dans le journal (`marc-B4b-intervalle0-couleurs.png`). `"B4552F"` sans `#` marche (bien). `#FFF` et le canal alpha sont des formes CSS courantes : soit les accepter, soit les refuser avec une ligne. Même remarque pour `largeur = 0` et `largeur = -50` (bouton invisible, pas de message).

### B9 — Bruit AppKit dans le journal — **mineur**

- À chaque construction d'un slider : `This NSLayoutConstraint is being configured with a constant that exceeds internal limits` (probablement `setMinimumSliderWidth: 600`). Et une sortie de script multi-lignes déclenche `Expected min height of view … got a height of 54`. Ça noie les vraies erreurs dans `Tactile.log` ; il faudrait soit corriger, soit filtrer.

### Non reproduit, à surveiller

- À mon arrivée, le Tactile lancé 13 h 49 plus tôt tournait mais la Touch Bar était noire (3 captures, écran allumé, pas d'économiseur) ; relancer l'a ramenée. Possiblement une barre perdue après une veille/réveil. Je n'ai pas pu le reproduire en une session.

## Frictions

- **Le libellé d'un `groupe` est tronqué** : largeur fixe de 72 pt dans `depliable`, donc `symbole` + `texte = "Dev"` donne « De… » et le chevron disparaît (`marc-A-riche.png`). Pas de `largeur` acceptée sur `groupe` (« champ inconnu »). Un `texte` seul de 4 lettres passe, au-delà non.
- **Où lire les erreurs sans terminal** : `Tactile.log` existe et c'est bien, mais rien ne signale qu'une erreur vient d'arriver. Après un enregistrement raté, la barre ne bouge pas et on ne sait pas si c'est « pas encore rechargé » ou « ignoré ». Un bouton rouge temporaire « config ignorée, ligne 6 » dans la barre elle-même, ou une notification macOS, réglerait ça.
- **`TACTILE_OUVRIR=k` numérote les sous-barres dans l'ordre de construction** : un groupe imbriqué prend un numéro avant son parent (k=1 = Git, k=2 = Dev). Outil de dev, mais piégeux.
- **Fichier supprimé** : passage à la barre par défaut sans ligne de journal ; un `mv` malheureux et on croit que la config est cassée.
- **Les scripts d'une sous-barre fermée tournent quand même** (tous les `Rafraichi` sont rafraîchis, visibles ou non). Pas gênant à 5 s, coûteux à 0,3 s.
- **Environnement des scripts** : `zsh -c` sans `-l`, donc le PATH est celui de Tactile. Lancé depuis le Finder, `/opt/homebrew/bin` n'y est pas : tout script qui appelle un outil brew (`jq` sort en `/usr/bin` sur macOS 15, mais `gh`, `btop`, `media-control` dans un script perso) échouera… en silence (B6). Rien ne le dit dans le README.
- **`heure` suit la locale système** (ici `en_US` → « Mon 26 Oct »). Logique, mais pas de clé `locale` pour forcer le français, et le README ne renvoie pas vers les motifs de format (`EEE`, `MMM`…).
- Le message TOML pur reste en anglais (« invalid basic string »). Détail.

## Manques (classés par valeur pour Marc)

1. **Raccourci clavier comme action** (`action = { touche = "cmd+shift+p" }` ou `{ touches = ["cmd", "k"] }`). C'est 80 % de ce que je mettrais dans une Touch Bar de dev : palette de commandes, commentaire, terminal. Aujourd'hui seul `echap` existe, et `clavier.rs` sait déjà envoyer un code touche.
2. **Barre par application** (`[[application]] bundle = "com.microsoft.VSCode"` avec ses `[[element]]`) : une barre Xcode/VS Code, une pour Safari, une par défaut. Sans ça la barre unique est un compromis permanent.
3. **Le stderr / code retour des scripts quelque part** (voir B6) et **un `delai` maximal** (B5). Sans ça, écrire un script est un jeu de devinettes.
4. **Lancer une app / ouvrir une URL** en une clé : `{ app = "Safari" }`, `{ url = "https://…" }`. Faisable aujourd'hui avec `{ shell = "open -a Safari" }`, mais c'est le genre de ligne que le README devrait montrer, ou que la config devrait offrir nativement (pas d'erreur de guillemets dans un `shell`).
5. **Batterie / Wi-Fi / niveau sonore** comme éléments natifs (type `batterie`, `volume_affiche`) ou au moins un exemple de script `pmset -g batt | grep -o '[0-9]*%'` dans `exemples/`. J'ai mis 2 min à l'écrire, un débutant n'y arrivera pas.
6. **Couleur et symbole dynamiques pour `script`** : une sortie qui pilote la couleur (`rouge` si la CI échoue). Par exemple, la première ligne = texte, une deuxième ligne optionnelle = couleur ou symbole.
7. **`largeur` sur `groupe`**, `couleur` sur `script`, et **troncature avec « … »** partout (B2).
8. **Appui long sur `script` et `groupe`** (seul `bouton` l'a).
9. **Retour haptique / état enfoncé** (bascule : `basculer_muet` qui change de symbole quand c'est coupé).
10. Rechargement par `SIGHUP` ou `tactile --recharger`, pour les configs générées par script, et un `tactile --verifier barre.toml` qui valide sans lancer (il suffirait d'exposer `config::lire`).

## 3 propositions concrètes

1. **Ne jamais planter sur une config** : dans `vues::symbole`, si `imageWithSystemSymbolName:` renvoie `nil`, journaliser « ligne N : symbole SF inconnu «x» » et retomber sur `questionmark.square.dashed` (ou le texte seul). Mieux : valider les symboles dans `config::lire` (un appel AppKit au chargement) pour que l'erreur soit traitée comme les autres, barre conservée. Et un `catch_unwind` autour de `installer` au rechargement, pour qu'une exception ObjC imprévue ignore la config au lieu de tuer la barre (B1). C'est le seul bloquant : une faute de frappe ne doit pas laisser un utilisateur sans Touch Bar ni esc.
2. **Rendre les scripts diagnosticables** : lire `stderr` et le code retour dans `rafraichi.rs` ; si la sortie est vide et le code non nul, afficher « ⚠ » dans la case et écrire `script «…» : code 127, zsh: command not found` dans le journal (une fois, pas à chaque intervalle). Ajouter `delai` (par défaut = `intervalle`, max 30 s) et tuer le processus au-delà ; tuer les enfants à la reconstruction de la barre (B5, B6, frictions PATH). Trois changements localisés dans le même fichier.
3. **Signaler ce qui ne s'affiche pas** : après construction, comparer la somme des largeurs demandées à la largeur disponible et journaliser « éléments 13–25 hors de la barre (1420 pt demandés pour 1004) » ; tronquer les `bouton`/`heure` sans `largeur` avec `lineBreakMode` + une largeur max raisonnable (B2, B3). Et faire de `masquer_control_strip` quelque chose de défini : soit appliquer le placement à chaud en re-présentant la barre, soit le documenter comme « au lancement seulement » et retirer la ✕ système (B4).

## Restauration

- `~/.config/tactile/barre.toml` restauré depuis la sauvegarde (`cmp` identique), tous les `sleep 1000` orphelins tués, `pkill -x tactile` puis `target/release/tactile 2>/dev/null &` relancé (pid 41248, vérifié par `pgrep`).
- Capture finale conforme à la barre de départ : , deux sliders, ⏪ titre (vide, rien ne joue) ⏩, heure, état Claude (vide), ✳ Claude.
- Aucun fichier du dépôt modifié hormis ce rapport et les captures dans `docs/retours/0.1/captures/`. Rien n'a été commité.
