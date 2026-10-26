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

État du lot E : **fait**. Preuves : tests Rust, et le pilote `TACTILE_EDITEUR_JS` qui passe par les vrais contrôles de la page (46 assertions, toutes vertes en clair et en sombre ; scripts et captures dans le scratchpad de la session, `lotE3/fonc.js`, `lotE3/contrastes.js`).

| # | Fait | Preuve |
|---|---|---|
| E1 | Menu principal quand la fenêtre est ouverte : Édition (Annuler, Rétablir, Couper, Copier, Coller, Tout sélectionner, sélecteurs standards vers le premier répondeur), Fichier › Fermer ⌘W, Présentation › Agrandir ⌘+ / Réduire ⌘− / Taille réelle ⌘0 | test `menu_principal_complet` (actions, raccourcis, cibles, réponse du pont). Pas essayé au vrai clavier : écran verrouillé |
| E2 | Texte et Météo… dans la palette ; ville cherchée par géocodage Open-Meteo (seulement au clic, `curl` des sources, autre fil) ; source + texte WMO d'après `exemples/meteo.toml` ; section Sources (nom, type, adresse ou commande, intervalle) ; import « Ajouter à ma barre » (fusion, sources renommées) | tests `meteo_d_une_ville`, `fusion_des_elements_sources_et_applications` ; pilote : Lyon trouvée, source « Météo Lyon » et texte créés, renommage suivi, `exemples/meteo.toml` ajouté (10 éléments, sources `Lyon`, `meteo`). Capture `docs/editeur-meteo-sombre.png` |
| E3 | Avis de `controler` après l'enregistrement (« Enregistré, 1 avertissement : l’app Figma n’est pas installée sur ce Mac. ») ; « Musique » → « Music » par les noms traduits des bundles ; ⚠ orange et « pas installée » dans le groupe | tests `avis_lisibles`, `app_installee_ou_non` (Musique, calendrier, Réglages Système) ; pilote ; ⚠ du groupe 1:1 → 5,57:1 en clair |
| E4 | L'aperçu estompe et barre ce qui sera caché, « 1 élément ne tient pas : la Touch Bar le cachera » ; même avis après l'enregistrement | pilote (estimation Rust `barre::debordement` par `mesurer`) |
| E5 | Barre toute faite : les `[[source]]` citées par les barres d'application gardées restent | test `barre_toute_faite_garde_les_sources_des_applications` |
| E6 | Focus rendu après Entrée, flèches, choix de symbole, Supprimer (voisin) ; ⌫ annulable (« Annuler » ou ⌘Z) ; aide clavier sous l'aperçu | pilote : `activeElement` vérifié après chaque geste |
| E7 | Noms accessibles (« Lanceur d’app Safari, ouvre Safari, 2 sur 5 »), `aria-selected` vrai/faux, `aria-orientation`, grille en listbox à tabindex itinérant, plus d'`aria-live` sur le panneau, message identique non réécrit, erreur en `role=alert` | pilote |
| E8 | Contrastes AA | mesures avant → après (clair ; sombre) : anneau de focus 1,69 → 5,02 ; 2,11 → 6,16 · interrupteur éteint 1,33 → 4,51 ; 1,28 → 3,90 · bord de champ 1,45 → 3,98 ; 1,56 → 3,42 · segment choisi 1,26 → 3,15 ; 1,92 → 4,48 · placeholder 3,26 → 5,07 ; 3,45 → 5,31 · lien accent sombre 4,46 → 5,35 · ⚠ app absente 1,00 → 5,57 ; blanc → orange 7,44 |
| E9 | Texte en `rem`, zoom de la page (`pageZoom`, gardé), en-tête qui passe à la ligne, aperçu réduit à la largeur | pilote à `zoom: 2` : Enregistrer dans la fenêtre, aucun défilement horizontal ; capture regardée |
| E10 | Image opaque en monochrome signalée ; « Demande l’autorisation Accessibilité » + « Ouvrir les réglages » sur les frappes simulées ; la langue garde le message (l'erreur reste une erreur) ; import sans `capture_zone`, sans numéro de ligne ni « bundle id », « effacer des fichiers » seulement si la barre lance une commande | test `code_execute_seulement_si_commande` ; pilote |

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
