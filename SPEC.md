# tactile — spécification

Réécriture en Rust, native Apple Silicon, de ce que fait BetterTouchTool, en commençant par la Touch Bar. Développée en salle blanche : API Apple (publiques et privées), code open source de MTMR (MIT) comme référence, BTT seulement observé de l'extérieur et lu dans la documentation qu'il livre (`BetterTouchTool.app/Contents/Resources/*.mdx`). Aucun désassemblage de BTT.

## Objectif

Une app de barre de menus (sans icône dans le Dock), légère au repos, qui remplace la Touch Bar d'Apple par une barre décrite dans un fichier, puis gagne module par module les autres fonctions de BTT.

## Critères de réussite (vérifiables)

- Native arm64, aucune dépendance à Rosetta.
- Au repos : moins de 1 % de CPU et moins de 40 Mo de RAM, mesurés avec `top`.
- Chaque élément de la barre est vérifié par une capture `screencapture -b` et par un test du comportement (valeur relue après action).
- Un changement du fichier de config s'applique sans relancer l'app.
- Tout ce qui est écrit pour l'utilisateur est en français.

## Modules, par ordre de livraison

1. **Barre** (jalon 1) : barre système plein écran ; boutons (texte, symbole SF, couleur), sliders natifs luminosité et volume, groupes qui s'ouvrent et se referment, heure, titre en cours avec ⏪ ⏯ ⏩, widget de script (sortie d'une commande rafraîchie), ✳ Claude vocal (micro déclaré par l'app elle-même). Config `~/.config/tactile/barre.toml`.
2. **Gestes Touch Bar** : glisser à deux ou trois doigts (volume, luminosité), appui long.
3. **Raccourcis clavier** globaux vers des actions.
4. **Gestes trackpad** (framework privé MultitouchSupport).
5. **Fenêtres** : aimantation aux bords, déplacement et redimensionnement (API Accessibilité).
6. **Presse-papier** : historique.
7. **Import des presets BTT** (format JSON documenté par BTT).

Chaque module est livré testé avant que le suivant commence.

## Hors périmètre pour l'instant

Stream Deck, Notch Bar, télécommande mobile, synchronisation.

## Contraintes

- Rust stable via mise (`mise.toml`), `objc2` pour parler à AppKit.
- Une action = une fonction testable sans interface (luminosité, volume, médias, shell…).
- Commits Conventional Commits en français, un par étape validée.
