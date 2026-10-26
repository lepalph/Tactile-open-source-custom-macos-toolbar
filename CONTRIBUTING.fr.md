# Contribuer à Tactile

🇬🇧 [Read in English](CONTRIBUTING.md)

Merci de ton intérêt. Commence par [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) : la carte du code, les recettes pour ajouter une action ou un élément, et les pièges déjà rencontrés. Le pourquoi des choix est dans [docs/DECISIONS.md](docs/DECISIONS.md).

## Ta première contribution en 10 minutes

1. Clone le dépôt et installe Rust. Le dépôt fixe Rust stable dans `mise.toml` ; `rustup` marche aussi.

   ```
   git clone https://github.com/lepalph/tactile-touch-bar.git
   cd tactile-touch-bar
   mise install
   ```

2. Lance les tests. `CI=1` saute ceux qui changent vraiment la luminosité et le volume. La première compilation prend une minute environ, les tests eux-mêmes à peu près une seconde.

   ```
   CI=1 cargo test
   ```

3. Compile et ouvre l'éditeur sur un fichier d'essai. Il ne prend pas la Touch Bar, et ta vraie barre ne bouge pas.

   ```
   cargo build --release
   TACTILE_CONFIG=/tmp/x.toml target/release/tactile --personnaliser
   target/release/tactile --verifier exemples/simple.toml
   ```

4. Choisis une [good first issue](https://github.com/lepalph/tactile-touch-bar/labels/good%20first%20issue) et laisse un commentaire pour que personne d'autre ne la prenne. Chacune donne les fichiers à lire, comment tester le changement et quand il est terminé.

5. Fais le changement sur une branche. Lance `CI=1 cargo test` et `cargo clippy --all-targets -- -D warnings`, puis ouvre une demande de fusion ; le gabarit demande le reste. Une question a sa place dans l'issue elle-même, en français ou en anglais.

## Style

- Noms en français (fonctions, variables, types, messages) : `luminosite::regler`, pas `brightness::set`.
- Peu de commentaires : seulement le pourquoi que le code ne dit pas.
- Une action système = une fonction testable sans interface, dans `src/actions/`.
- Tout ce que lit l'utilisateur est en français.
- `cargo build` sans warning.

## Tests d'abord

Écris le test qui décrit le comportement attendu, regarde-le échouer, puis écris le code. Une action se prouve en relisant la valeur après l'avoir changée, puis en restaurant l'état d'origine (voir `src/actions/volume.rs`). `cargo test` doit passer avant toute demande de fusion.

`CI=1 cargo test` : avec `CI` posée, `build.rs` active `cfg(ci)` et les tests qui touchent le matériel (luminosité, volume, batterie, rendu des symboles) sortent `ignored` (`#[cfg_attr(ci, ignore = "…")]`) ; sans, ils changent vraiment la luminosité et le volume, puis les restaurent.

## Ajouter une action

Une action, c'est ce qu'on écrit après `action =`. Une nouvelle touche huit endroits ; les tests échouent si on oublie les points 4 à 8.

1. `src/actions/mod.rs` : une variante dans l'enum `Action`. Son nom en `snake_case` devient le mot du fichier (`BasculerMuet` → `"basculer_muet"`) ; une variante avec valeur s'écrit `{ nom = valeur }`. Ajoute son alias anglais, `#[serde(alias = "toggle_mute")]`, et `#[serde(serialize_with = "court")]` sur une valeur `f32` (sinon `0.3` s'enregistre `0.30000001192092896`).
2. La fonction qui fait le travail, dans un fichier de `src/actions/`, utilisable sans interface, avec un test qui change l'état, le relit et le restaure. Un test qui a besoin du matériel prend `#[cfg_attr(ci, ignore = "…")]`.
3. Sa branche dans `executer` (`src/actions/mod.rs`). Une action gérée par la barre elle-même, comme `barre_apple`, va plutôt dans `declencher` (`src/barre/mod.rs`).
4. `src/config.rs`, test `chaque_forme_d_action` : une ligne avec l'écriture française et l'`Action` attendue.
5. `src/config.rs`, `TOUT_FR` et `TOUT_EN` : l'action dans les deux, en français et en anglais ; `alias_anglais_donnent_la_meme_barre` vérifie qu'elles donnent la même barre.
6. `src/config.rs`, `BARRE_COMPLETE` : l'action une fois, pour les tests d'aller-retour (`aller_retour_toml`, et `le_json_ne_perd_aucun_champ` dans l'éditeur).
7. L'éditeur, `src/editeur/editeur.html` : une action sans valeur va dans `ACTIONS_SIMPLES` et `CLE_ACTION`, avec son libellé dans les dictionnaires `fr:` et `en:` ; une action avec valeur dans `actionUI`, `actionModele` et `champAction`. Une action que l'éditeur ne connaît pas s'affiche « Autre » et traverse l'enregistrement intacte.
8. Les deux README : la ligne du tableau des actions, et celle du glossaire dans `README.md` (`### English aliases`) et `README.fr.md` (`### Glossaire`). `glossaire_du_readme` vérifie que les deux glossaires sont identiques et que chaque ligne figure dans `TOUT_FR` / `TOUT_EN`.

Si l'action désigne quelque chose qui peut manquer (une app, un fichier), apprends aussi à `config::controler` à le signaler : `tactile --verifier`, l'éditeur et le journal passent tous par lui.

## Vérification complète

Avant toute demande de fusion qui touche la barre, lance `scripts/verifier.sh`, écran allumé, en touchant le trackpad de temps en temps. Il actionne chaque contrôle comme un doigt, vérifie l'effet réel et compare chaque capture de la Touch Bar à l'écran attendu. Joins la sortie à ta demande.

## Changements visuels

Tout changement visible dans la barre s'accompagne d'une capture de la Touch Bar, jointe à la demande de fusion :

```
screencapture -b barre.png
```

Si l'image est entièrement noire, l'écran était en veille : réveille-le et recommence.

Deux variables aident à capturer ce qu'on ne peut pas toucher :

- `TACTILE_OUVRIR=1 target/release/tactile` présente directement la sous-barre n° 1 (premier slider ou groupe) ;
- `TACTILE_MASQUER=1 target/release/tactile` rend la barre d'Apple 2 s après le lancement ;
- `TACTILE_RETOUR=1` (avec la précédente) ramène la barre de Tactile à 4 s, par le même chemin qu'un toucher sur ✳ ;
- `TACTILE_MESURE=1` écrit au journal la durée d'un rechargement, d'un changement d'app et de l'ouverture ou la fermeture d'une sous-barre (lignes `mesure : …`) ; les chiffres à tenir sont dans [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#mesures).

## Commits

[Conventional Commits](https://www.conventionalcommits.org/fr/), en français : `feat: groupe repliable`, `fix: volume relu après la veille`, `docs: …`. Un commit par étape qui marche.
