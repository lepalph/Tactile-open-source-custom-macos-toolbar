# Synthèse de la campagne de tests 0.3

Demandée par l'utilisateur : tests unitaires, tests fonctionnels sur la vraie barre, personas IA. Rapports dans ce dossier : `couverture.md` (unitaires), `fonctionnel.md` (vraie barre), `nadia-debutante.md`, `lise-btt.md`, `sam-english-dev.md`, `karim-securite.md` (sécurité), `ines-accessibilite.md` (accessibilité). L'écran du Mac est resté verrouillé presque toute la campagne : ce qui dépend de l'affichage est à revoir écran ouvert.

Déjà corrigé pendant la campagne (branche des tests unitaires, intégrée) : intervalle `1e300` qui faisait planter la barre, `nan`/`inf` acceptés, `http://localhost:80@example.org/` envoyé en clair à un tiers, action d'un élément `texte` absente de l'aperçu d'import, deux `barre.toml` dans un zip, compte de logos faux, arrondis de volume/luminosité et de nombres passés par la page.

## Lot S — sécurité (avant toute release)

| # | Problème | Source | Gravité |
|---|---|---|---|
| S1 | L'aperçu d'import ne montre pas les `entetes` d'une source : un `trousseau:` peut envoyer un secret de l'utilisateur à une URL tierce toutes les 5 s | Karim | haute |
| S2 | Revue complète de ce que l'aperçu d'import doit montrer : tout ce qui exécute, lit un secret, lit un fichier ou appelle le réseau, pour **chaque** type d'élément et de source (une seule fonction exhaustive, `match` sans `_ =>`), confirmation renforcée si trousseau + URL non locale | Karim, Lise, unitaires | haute |
| S3 | Sortie d'une commande (script, source `commande`, MCP stdio) lue sans plafond : `cat /dev/zero` sature la mémoire avant le délai | Karim | moyenne |
| S4 | `image` d'une barre importée peut pointer n'importe quel fichier (`/etc/passwd`, `~/.ssh/…`) | Karim | basse |
| S5 | Messages JS `capture`, `importerImage`, `importerBarre` : chemin libre sans garde | Karim | basse |

## Lot E — éditeur (Nadia, Lise, Inès)

| # | Problème | Source | Gravité |
|---|---|---|---|
| E1 | Pas de menu Édition : ⌘C / ⌘V / ⌘X / ⌘A / ⌘Z probablement inopérants dans les champs | Inès | majeur |
| E2 | Météo impossible sans TOML : pas d'élément Texte/Météo dans la palette, pas de réglage de source ni de ville ; import qui remplace tout (pas d'« Ajouter à ma barre ») | Nadia | majeur |
| E3 | Lanceur d'app absente (Figma) ou nom localisé (« Musique ») enregistré comme réussi ; alerte ⚠ blanche sur blanc en clair | Nadia | majeur |
| E4 | Barre qui déborde enregistrée comme réussie ; l'aperçu ne montre pas ce qui sera caché | Nadia | majeur |
| E5 | Barre toute faite : efface les `[[source]]` mais garde les `[[application]]` qui les citent | Lise | moyen |
| E6 | Focus perdu après Entrée, choix de symbole, Supprimer ; ⌫ supprime sans confirmation ni annulation ; raccourcis de déplacement non dits | Inès | majeur |
| E7 | VoiceOver : aperçu « Bouton, Bouton… » sans nom ; `aria-selected="null"` ; grille de symboles (aria-pressed sur role=option, 48 arrêts de tabulation) ; `aria-live` qui relit tout | Inès | majeur |
| E8 | Contrastes : anneau de focus 1,69:1, interrupteur éteint 1,17:1, segment choisi 1,26:1, bordures 1,45:1, placeholder 3,26:1, lien accent sombre 4,46:1 | Inès | majeur |
| E9 | Zoom : pas de ⌘+ / ⌘−, à 200 % Enregistrer sort de l'écran | Inès | moyen |
| E10 | Monochrome sur une image sans transparence → bloc blanc sans avertissement ; capture d'écran sans mention de l'autorisation Accessibilité ; changer de langue efface l'erreur affichée ; libellés d'import (`capture_zone`, « bundle id », « effacer des fichiers ») | Nadia | mineur |

## Lot C — config, CLI, contribution (Sam, Lise)

| # | Problème | Source | Gravité |
|---|---|---|---|
| C1 | Le README décrit des fonctions absentes de la release téléchargeable (0.2.0), `--version` dit 0.2.0 | Sam | majeur → release 0.3.0 |
| C2 | `{text}` dans un `template` affiche « — » alors que `--check` dit OK ; `--check` doit valider les variables d'un gabarit | Sam | moyen |
| C3 | Les messages citent la clé française même quand l'utilisateur a écrit l'alias anglais ; `template` chaîne sur un bouton → « attendu true ou false » | Sam | moyen |
| C4 | Même app sous deux noms (`Finder` / `com.apple.finder`) non détectée ; message trompeur pour un bundle id absent ; lignes d'erreur d'un zip importé renvoient au fichier réécrit ; `cmd++` refusé | Lise | mineur |
| C5 | CONTRIBUTING : `cargo not found` sans `mise exec`, durée des tests, étape 3 bloquante, raisons d'`ignore` en français ; README:229 incomplet (modificateurs) | Sam | mineur |

## Lot F — vraie barre

Voir `fonctionnel.md` (à compléter à l'arrivée du rapport de la vague B).

## Reportés

Barre d'Apple automatique par app (`barre = "apple"`), liste d'apps (`app = [...]`), éditeur des barres par app et des sources (prévu au chantier éditeur), import BTT, essai sur Mac Intel.
