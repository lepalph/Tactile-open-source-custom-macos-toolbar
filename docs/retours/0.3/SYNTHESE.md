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

État du lot S : **fait** (tests d'attaque `a2b21c0` qui échouaient d'abord, correction `9677451`, fusion `6a15769` ; restes `747e8ef`).

| # | Fait | Preuve |
|---|---|---|
| S1 | Chaque en-tête d'une source est listé ; un `trousseau:` est dit en clair (« Lit ton mot de passe enregistré « x » et l’envoie à hôte ») ; envoyé hors de ce Mac : bandeau `imp-alerte` dans l'éditeur, `--importer` refuse sans `--oui` | test `le_trousseau_envoye_ailleurs_est_montre_et_signale` |
| S2 | `partage::commandes` : un seul parcours, chaque type, champ, action et source nommés sans `..` ni `_ =>` (un ajout ne compile pas sans y être classé) ; images lues hors de `icones/` listées ; titre « Ce que cette barre fait sur ton Mac » / « What this bar does on your Mac » (CLI et éditeur) | tests `tout_ce_que_la_barre_fait_est_liste`, `resume_dans_les_deux_langues`, `barre_meteo_importee_liste_sa_source_http` (barre météo faite par l'éditeur E, source http listée) |
| S3 | Sorties lues avec un plafond de 1 Mo (script, source `commande`, trousseau, curl, ligne MCP stdio) : groupe tué, « ⚠ trop long » ; une source `commande` coupée affiche aussi « ⚠ trop long » (avant : « ⚠ source ») | tests `une_sortie_geante_est_coupee_sans_remplir_la_memoire` (50 Mo, < 30 Mo de mémoire, `texte_d_erreur`), `une_ligne_geante_arrete_le_serveur` |
| S4 | Une barre importée ne garde que les images de son archive (`icones/…`) ; les autres sont retirées et dites | test `image_hors_de_la_barre_importee_retiree` |
| S5 | `capture`, `importerImage`, `importerBarre`, `exporterVers` : chemin suivi seulement avec le pilote (dossier temporaire ou dépôt), sinon le fichier rendu par NSSavePanel (`chemin_permis`) | test `chemins_venus_de_la_page` |

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

État du lot E : **fait** (`d5bd152`, `540a12d`, fusion `a432d15` ; compléments `14e3877`). Preuves : tests Rust, et le pilote `TACTILE_EDITEUR_JS` qui passe par les vrais contrôles de la page (46 assertions, toutes vertes en clair et en sombre ; scripts et captures dans le scratchpad de la session, `lotE3/fonc.js`, `lotE3/contrastes.js`).

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
| E9 (suite) | En-tête sur une ligne à la largeur par défaut : Importer, Exporter et Revenir dans un menu « ⋯ » (popover, flèches, fermé par un choix), Enregistrer visible ; fenêtre plus haute si l'écran le permet (760 à 860 pt) ; colonne Ajouter qui défile, bas estompé tant qu'il reste à voir | `14e3877` ; pilote : en-tête 102 → 64 px, boutons sur la ligne du titre, menu ouvert sous « ⋯ », ↓ passe d'Importer à Exporter, colonne 521/638 px qui défile jusqu'à « Développeur » ; captures `docs/editeur-clair.png`, `docs/editeur-sombre.png` (FR) regardées |

## Lot C — config, CLI, contribution (Sam, Lise)

| # | Problème | Source | Gravité |
|---|---|---|---|
| C1 | Le README décrit des fonctions absentes de la release téléchargeable (0.2.0), `--version` dit 0.2.0 | Sam | majeur → release 0.3.0 |
| C2 | `{text}` dans un `template` affiche « — » alors que `--check` dit OK ; `--check` doit valider les variables d'un gabarit | Sam | moyen |
| C3 | Les messages citent la clé française même quand l'utilisateur a écrit l'alias anglais ; `template` chaîne sur un bouton → « attendu true ou false » | Sam | moyen |
| C4 | Même app sous deux noms (`Finder` / `com.apple.finder`) non détectée ; message trompeur pour un bundle id absent ; lignes d'erreur d'un zip importé renvoient au fichier réécrit ; `cmd++` refusé | Lise | mineur |
| C5 | CONTRIBUTING : `cargo not found` sans `mise exec`, durée des tests, étape 3 bloquante, raisons d'`ignore` en français ; README:229 incomplet (modificateurs) | Sam | mineur |

État du lot C : **fait** (`b8ab914`, `05a6a31`, `f8e92c6`, `a507db5`, fusion `af9d4e8` ; restes `258f4c5`, version `3bf5132`).

| # | Fait | Preuve |
|---|---|---|
| C1 | Version 0.3.0 : `Cargo.toml`, CHANGELOG `[0.3.0] - 2026-10-26`, feuille de route des README en « Done (0.3.0) » | `tactile --version` → `tactile 0.3.0` ; `Info.plist` 0.3.0 ; publication par l'orchestrateur |
| C2 | `{text}` alias de `{texte}` ; `--verifier` signale une variable de gabarit impossible | test `variables_de_gabarit_controlees` |
| C3 | Messages qui citent la clé telle qu'écrite (alias anglais compris) ; `template` chaîne sur un bouton expliqué | test `messages_avec_la_cle_ecrite` |
| C4 | Même app sous deux noms (LaunchServices) ; bundle id absent sans faux conseil ; `cmd++` ; avis d'un zip importé situés par élément (« élément 3 », « application « Safari », élément 1 »), jamais par une ligne du fichier réécrit, et un avis en français garde « attention : » dans un résumé anglais | tests `meme_app_sous_deux_noms_et_bundle_id_absent`, `plus_et_moins_comme_touche`, `avis_d_un_zip_situes_par_element_dans_une_seule_langue` |
| C5 | CONTRIBUTING suivable sans mise activé, durée réelle des tests ; README avec tous les modificateurs ; les raisons d'`ignore` toutes en anglais (les 7 de `src/editeur/` comprises) | tests `commandes_du_guide_marchent_telles_quelles`, `readme_liste_les_modificateurs` ; `rg 'ignore = ' src` sans français |

## Lot F — vraie barre

Rapport complet : `fonctionnel.md` (écran verrouillé toute la session : ce qui dépend de l'affichage est compté non vérifiable).

| # | Constat | État |
|---|---|---|
| F1 | Chaque rechargement gardait un NSImage par symbole SF (16,6 → 26,6 Mo en 30 min, 90 rechargements) | **Fait** `6c58054` : cache par nom dans `vues::symbole` ; test `symbole_reutilise_d_un_rechargement_a_l_autre` ; second passage : palier vers 18 Mo dès la minute 15 |
| F2 | `scripts/verifier.sh` tuait aussi les éditeurs `--personnaliser` (`pkill -x`) | **Fait** `1ac97c1` : arrêt par chemin exact, relance de l'app de l'utilisateur |
| F3 | 0,80 % de CPU sous la charge d'endurance (script à 0,3 s + rechargements) ; un script à 0,3 s seul coûte 0,40 % | **Fait** `c57687f`, `90ea322` : mesures dans `docs/ARCHITECTURE.md` (Mesures) ; `controler` avertit au-delà de 2 scripts sous 1 s ou de 5 lancements par seconde ; test `rafraichissements_rapides_signales` |
| F4 | Autotest étendu (étapes 31-42 : sources http, commande, MCP et sa relance, logo, captures, import/export) et mode `fluidite` | **Fait** (campagne) : `AUTOTEST fin 0 échec(s)` ; rechargement médiane 119,5 ms (cible < 300), sous-barre 0,1 ms, app 0,3 ms (simulée) |
| F5 | Bouton `affichage = "icone"` sans texte titré « Button » pour VoiceOver | **Reporté** : pas dessiné, à traiter avec le volet accessibilité de la barre (hors éditeur) |
| F6 | PNG opaque en `gabarit = true` → carré blanc | **Fait** côté éditeur (E10, avertissement en monochrome) ; dans le TOML, documenté seulement |
| F7 | Barre d'Apple, Control Strip visible, vraie bascule d'app, Mission Control et bureau à l'écran | **Reporté** : non vérifiable écran verrouillé, à revoir écran ouvert |

## Validation de la 0.3.0

`cargo build --release` sans avertissement, `cargo clippy --all-targets -- -D warnings`, `cargo doc` avec `-D warnings`, `cargo test` : 154 réussis (2 ignorés) sans `CI`, 138 (18 ignorés) avec `CI=1` ; `--verifier` OK sans avis sur les 10 exemples ; `scripts/empaqueter.sh` : `x86_64 arm64`, signature valide, `Info.plist` 0.3.0 ; `scripts/verifier.sh` : `AUTOTEST fin 0 échec(s)`, 40 captures conformes, 2 non vérifiables (barre d'Apple, écran verrouillé, code 2 attendu).

## Reportés

Barre d'Apple automatique par app (`barre = "apple"`), liste d'apps (`app = [...]`), éditeur des barres par app et des sources (prévu au chantier éditeur), import BTT, essai sur Mac Intel.

## Vérification finale écran déverrouillé (2026-10-27, Tactile 0.3.0 installée)

`scripts/verifier.sh` : « AUTOTEST fin 0 échec(s) », « Tout est vérifié », code 0, aucune étape simulée (`grep -c simulé` = 0). Pour la première fois, la barre d'Apple (étapes 7-8 : barre native avec ✳ dans le Control Strip, puis ✳ ramène la barre Tactile) et la vraie bascule d'app (étape 27 : TextEdit réellement au premier plan, « com.apple.TextEdit », barre d'application présentée, sous-barre gardée) sont vues en capture. L'app de l'utilisateur est relancée après, config intacte.
