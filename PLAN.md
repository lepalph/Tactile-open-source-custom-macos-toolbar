# tactile — plan

Statut : `[x]` fait et prouvé, `[ ]` à faire. Chaque case cochée porte sa preuve.

## Jalon 0 — faisabilité

- [x] Barre système affichée depuis Rust via `NSTouchBar presentSystemModalTouchBar:placement:systemTrayItemIdentifier:` — preuve : capture `screencapture -b` du bouton « ✳ tactile » en pleine largeur (2026-10-25).

## Jalon 1 — la barre

- [x] Squelette : `barre` (AppKit), `config` (TOML), `actions` — preuve : `cargo build` sans warning.
- [x] Actions système testées : luminosité (DisplayServices), volume et muet (CoreAudio), médias (via media-control, MediaRemote étant bloqué depuis macOS 15.4), esc, veille, shell — preuve : `cargo test`, valeurs relues puis restaurées.
- [x] Config TOML lue et rechargée à chaud — preuve : 8 tests config ; capture « ✳ Rechargé » 2 s après modification du fichier.
- [x] Éléments : bouton, slider luminosité, slider volume (sous-barres système avec ✕), groupe, heure, titre en cours, widget script, espace, espace souple — preuve : captures `screencapture -b` de la barre et des deux sous-barres.
- [x] Barre plein écran (placement 1) sans Control Strip, une seule touche esc — preuve : capture.
- [ ] ✳ Claude vocal intégré (micro déclaré dans `Info.plist` de Tactile.app).
- [ ] Empaquetage `Tactile.app` signé ad hoc + lancement au démarrage.
- [x] Mesure repos : 0,5 % CPU, 13 Mo (release, `top`), binaire 1,2 Mo.
- [ ] Comparaison élément par élément avec la barre BTT (captures côte à côte).

## Jalons suivants

Voir l'ordre des modules dans `SPEC.md`.
