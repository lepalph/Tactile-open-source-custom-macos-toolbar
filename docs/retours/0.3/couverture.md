# Couverture des tests unitaires (0.3)

Mesurée avec `cargo-llvm-cov` 0.9.1 (`llvm-tools-preview`), lignes couvertes par fichier.

```
CI=1 cargo llvm-cov --summary-only     # comme la CI : tests matériels ignorés
cargo llvm-cov --summary-only          # sur ce Mac : luminosité et volume réels changés puis restaurés
```

« Avant » : `main` à `b8fe8e6` (94 tests sous `CI=1`, 18 ignorés ; 110 sans `CI`). « Après » : la branche de la campagne
(116 sous `CI=1`, 18 ignorés ; 132 sans `CI`). `essais.rs` est le générateur des tests par propriétés, compté dans
le total mais compilé seulement pour les tests (sans lui, le total après est de 62,0 % sous `CI=1`).

| Fichier | Lignes | Avant `CI=1` | Avant matériel | Après `CI=1` | Après matériel |
|---|---:|---:|---:|---:|---:|
| actions/clavier.rs | 244 | 64,8 % | 89,9 % | 66,8 % | 90,2 % |
| actions/luminosite.rs | 33 | 0,0 % | 87,9 % | 0,0 % | 87,9 % |
| actions/media.rs | 86 | 44,2 % | 44,2 % | 44,2 % | 44,2 % |
| actions/mod.rs | 142 | 48,1 % | 63,9 % | 51,4 % | 66,2 % |
| actions/volume.rs | 55 | 0,0 % | 89,1 % | 0,0 % | 89,1 % |
| barre/autotest.rs | 415 | 0,0 % | 0,0 % | 0,0 % | 0,0 % |
| barre/batterie.rs | 48 | 77,1 % | 100 % | 77,1 % | 100 % |
| barre/elements.rs | 356 | 45,6 % | 45,6 % | 48,6 % | 48,6 % |
| barre/mod.rs | 294 | 13,3 % | 13,3 % | 13,3 % | 13,3 % |
| barre/plateau.rs | 18 | 0,0 % | 0,0 % | 0,0 % | 0,0 % |
| barre/rafraichi.rs | 266 | 46,2 % | 46,2 % | 46,2 % | 46,2 % |
| barre/vues.rs | 287 | 52,3 % | 72,1 % | 52,3 % | 72,1 % |
| config.rs | 896 | 98,3 % | 98,3 % | 98,0 % | 98,0 % |
| editeur/icones.rs | 273 | 56,3 % | 87,9 % | 61,5 % | 90,5 % |
| editeur/mod.rs | 751 | 43,5 % | 50,2 % | 48,1 % | 54,3 % |
| editeur/partage.rs | 937 | 68,4 % | 82,0 % | 84,9 % | 91,7 % |
| essais.rs (tests) | 156 | | | 96,8 % | 96,8 % |
| main.rs | 164 | 47,0 % | 47,0 % | 47,0 % | 47,0 % |
| menu.rs | 90 | 0,0 % | 0,0 % | 0,0 % | 0,0 % |
| sources/http.rs | 168 | 99,3 % | 99,3 % | 98,2 % | 98,2 % |
| sources/mcp.rs | 265 | 93,6 % | 93,6 % | 93,6 % | 93,6 % |
| sources/mod.rs | 324 | 91,4 % | 91,4 % | 92,9 % | 92,9 % |
| **Total** | 6268 | **56,6 %** | **64,7 %** | **62,9 %** | **69,5 %** |

## Ce qui reste à 0-15 % et pourquoi

Les trous ont été comblés par risque, pas par pourcentage. Ce qui reste bas est de l'AppKit qu'un test unitaire ne
prouve pas mieux que `scripts/verifier.sh` sur une vraie Touch Bar :

- `barre/autotest.rs` (0 %) : c'est le pilote de `--autotest`, lancé par `scripts/verifier.sh` ; il n'a de sens que sur la barre affichée.
- `menu.rs`, `barre/plateau.rs` (0 %) : icône de la barre des menus et du Control Strip, sélecteurs Objective-C.
- `barre/mod.rs` (13 %) : présentation de la barre (API privée), `tic`, appuis. Couvert par l'autotest.
- `actions/luminosite.rs`, `actions/volume.rs` (0 % sous `CI=1`) : tests matériels, 88-89 % sur un Mac.
- `editeur/mod.rs` (48 %) : `recevoir`, le pont JS → Rust, appelle la `WKWebView` à chaque branche ; la logique
  derrière (`enregistrer`, `revenir`, `verifier`, `mettre_de_cote`, `suggestion`, import et export) est testée.
- `main.rs` (47 %) : les branches qui lancent l'app ; la lecture des arguments est testée.

## Tests par propriétés

Sans dépendance : `src/essais.rs` tire ses cas d'un xorshift à graine fixe (un échec se rejoue à l'identique, la graine
est dans le message). `proptest` aurait apporté la réduction des cas, mais aussi une dépendance et sa compilation pour
des générateurs qui tiennent en 200 lignes.

| Propriété | Cas |
|---|---:|
| TOML ⇄ `Barre` ⇄ JSON à l'identique, texte stable (chaque type d'élément, action, source, textes piégeux) | 400 barres |
| `[[item]]` et `[[element]]` mélangés au hasard : même barre | 300 barres |
| `lire` et `controler` sur des TOML tronqués ou mutés : jamais de panique, ligne d'erreur dans le fichier | 2 640 fichiers |
| Archives tronquées ou mutées : jamais de panique, rien d'écrit hors du dossier temporaire | 600 archives |
| Noms d'entrée composés (`..`, `/`, `\`, `C:`) : acceptés seulement s'ils restent dans l'archive | 300 noms |
| Aucune commande d'une barre générée ne manque à la liste montrée avant l'import | 300 barres |
| `enregistrer` écrit exactement la barre (ancien fichier en `.precedent`) ou refuse sans rien toucher | 150 barres |
| Raccourcis générés relus à l'identique ; textes quelconques lus ou refusés sans panique | 500 + 2 000 |
| Chemins de champ d'une valeur JSON générée ; gabarits aux accolades orphelines | 300 valeurs |
| Sortie de `curl -i` tronquée ou mutée : jamais de panique | 2 000 sorties |

## Bugs trouvés et corrigés

Chacun a son test, écrit avant la correction, et son commit `fix:`.

1. Un curseur réglé à 0,6100102 était enregistré 0,61 (`court` arrondissait au millième).
2. `serde_json` relisait un nombre de la page à un ulp près (508.27859447953637 → 508.2785944795364) : `float_roundtrip`.
3. `nan` et `inf` passaient `lire` : la page de l'éditeur les recevait en `null` et ne pouvait plus enregistrer.
4. `intervalle = 1e300` (ou `delai`) faisait paniquer la barre à sa construction : une barre importée suffisait.
5. Le crate `zip` ne garde qu'une entrée par nom : deux `barre.toml` passaient, le refus prévu ne se déclenchait jamais.
6. L'import comptait un logo par bouton qui l'affiche (« 4 logos » pour une archive qui en avait deux).
7. L'action d'un élément `texte` n'apparaissait pas dans la liste de ce qu'une barre importée peut lancer.
8. `http://localhost:80@example.org/` passait `url_permise` : curl l'envoyait en clair à example.org.

## Durée

`CI=1 cargo test` : 10,6 s, dont 10 s pour les tests de sources qui attendent un vrai serveur MCP et un délai HTTP
(déjà là avant). Les tests par propriétés ajoutent moins d'une seconde ; aucun n'est marqué `#[ignore]`.
