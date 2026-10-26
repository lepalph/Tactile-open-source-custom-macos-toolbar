# Vision

Tactile doit devenir un outil libre qui permet de tout personnaliser sur la Touch Bar, sans devenir une usine à options. La règle : **tout doit être possible, rien ne doit être codé en dur pour un seul cas.**

## Trois couches

1. **Des briques génériques, peu nombreuses.** Bouton, curseur, choix, texte vivant (script), groupe, lanceur d'app, barre par application, barre par site ; actions : raccourci clavier, app, URL, AppleScript, commande. Une nouvelle brique n'entre que si elle débloque une famille de besoins (le curseur libre couvre le volume de n'importe quelle app, un écran externe, tout réglage pilotable), jamais un seul cas.
2. **Des modules dans n'importe quel langage.** Un widget est un programme qui parle JSON avec Tactile : il envoie quoi afficher (texte, icône, couleur, état), Tactile lui renvoie les appuis. Protocole documenté et stable, dossier `modules/`. On ajoute météo, Pomodoro, état d'un serveur… sans toucher au code de Tactile ni attendre une version.
3. **Le partage.** Une barre est un fichier (`.toml`, ou `.zip` avec ses logos) qu'on importe et exporte. Le dépôt accueille une galerie (`galerie/`) de barres et de modules de la communauté, vérifiés en CI par `tactile --verifier`, installables depuis l'éditeur.

## Principes

- Local d'abord : rien ne part sur le réseau sans action explicite ; les logos de sites sont récupérés une fois, sur le site lui-même, et gardés sur le Mac.
- Léger : moins de 20 Mo de mémoire, CPU au repos ≤ 0,2 %.
- Une barre importée est une entrée non fiable : elle est vérifiée, et ce qu'elle exécute est montré avant d'être accepté.
- L'éditeur et le fichier disent la même chose : tout ce que fait l'éditeur s'écrit dans le TOML, et le TOML se lit sans l'éditeur.
- Ouvert aux contributions : README anglais, CONTRIBUTING, gabarits d'issues, « good first issues », releases régulières.

## Priorités (décidé le 2026-10-26)

L'outil est fait d'abord pour son auteur ; l'open source sert au partage du code, pas à la course à l'adoption. Priorité à la puissance de personnalisation, à la fluidité et à la solidité. Pas d'effort dédié à la notarisation, à un site vitrine ou au marketing ; la galerie communautaire passe après les briques.

Exception voulue : la page GitHub doit donner envie à un développeur de contribuer (visuel en tête, points forts, feuille de route, « good first issues », première contribution en 10 minutes).
