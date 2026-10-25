# Contribuer à Tactile

Merci de ton intérêt. Commence par [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) : la carte du code, les recettes pour ajouter une action ou un élément, et les pièges déjà rencontrés. Le pourquoi des choix est dans [docs/DECISIONS.md](docs/DECISIONS.md).

## Style

- Noms en français (fonctions, variables, types, messages) : `luminosite::regler`, pas `brightness::set`.
- Peu de commentaires : seulement le pourquoi que le code ne dit pas.
- Une action système = une fonction testable sans interface, dans `src/actions/`.
- Tout ce que lit l'utilisateur est en français.
- `cargo build` sans warning.

## Tests d'abord

Écris le test qui décrit le comportement attendu, regarde-le échouer, puis écris le code. Une action se prouve en relisant la valeur après l'avoir changée, puis en restaurant l'état d'origine (voir `src/actions/volume.rs`). `cargo test` doit passer avant toute demande de fusion.

## Changements visuels

Tout changement visible dans la barre s'accompagne d'une capture de la Touch Bar, jointe à la demande de fusion :

```
screencapture -b barre.png
```

Si l'image est entièrement noire, l'écran était en veille : réveille-le et recommence.

Deux variables aident à capturer ce qu'on ne peut pas toucher :

- `TACTILE_OUVRIR=1 target/release/tactile` présente directement la sous-barre n° 1 (premier slider ou groupe) ;
- `TACTILE_MASQUER=1 target/release/tactile` rend la barre d'Apple 2 s après le lancement.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/fr/), en français : `feat: groupe repliable`, `fix: volume relu après la veille`, `docs: …`. Un commit par étape qui marche.
