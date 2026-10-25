# Contribuer à Tactile

Merci de ton intérêt. Quelques règles pour que le code reste homogène.

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

## Commits

[Conventional Commits](https://www.conventionalcommits.org/fr/), en français : `feat: groupe repliable`, `fix: volume relu après la veille`, `docs: …`. Un commit par étape qui marche.
