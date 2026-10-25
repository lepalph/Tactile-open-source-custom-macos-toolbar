# tactile — plan

Statut : `[x]` fait et prouvé, `[ ]` à faire. Chaque case cochée porte sa preuve.

## Jalon 0 — faisabilité

- [x] Barre système affichée depuis Rust via `NSTouchBar presentSystemModalTouchBar:placement:systemTrayItemIdentifier:` — preuve : capture `screencapture -b` du bouton « ✳ tactile » en pleine largeur (2026-10-25).

## Jalon 1 — la barre

- [ ] Squelette : `app` (boucle AppKit), `barre` (construction et présentation), `config` (TOML), `actions`, `widgets`.
- [ ] Actions système testées : luminosité (DisplayServices), volume et muet (CoreAudio), médias (lecture, suivant, précédent), mise en veille, commande shell.
- [ ] Config TOML lue et rechargée à chaud.
- [ ] Éléments : bouton, slider luminosité, slider volume, groupe, heure, titre en cours, widget script.
- [ ] Masquage du Control Strip et d'un esc en double.
- [ ] ✳ Claude vocal intégré (micro déclaré dans `Info.plist` de Tactile.app).
- [ ] Empaquetage `Tactile.app` signé ad hoc + lancement au démarrage.
- [ ] Mesure repos : < 1 % CPU, < 40 Mo.
- [ ] Comparaison élément par élément avec la barre BTT (captures côte à côte).

## Jalons suivants

Voir l'ordre des modules dans `SPEC.md`.
