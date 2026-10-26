# Chantier 0.2 — personnalisation, ergonomie, open source

Orchestration par phases. Chaque phase est menée par un ou plusieurs sous-agents ; l'orchestrateur ne fait que cadrer, lire les rapports, trancher et pousser. Règle de matériel : **un seul processus `tactile` à la fois** (la Touch Bar est exclusive) ; tout agent qui lance la barre doit la relancer pour l'utilisateur à la fin (`target/release/tactile &`).

## Phase 1 — faux clients (parallèle, lecture seule sauf un)

Quatre personas lisent le dépôt tel qu'un nouvel utilisateur le découvre et rendent un rapport dans `docs/retours/0.1/` :

| Persona | Angle | Touche au matériel |
|---|---|---|
| Nadia, débutante | Installe depuis Tactile.zip en suivant le README ; chaque phrase floue est un bug | non |
| Marc, utilisateur avancé | Écrit des barres riches (groupes, scripts, couleurs), provoque des erreurs de config | oui, avec sauvegarde / restauration de `~/.config/tactile/barre.toml` |
| Lise, réfugiée BTT | Compare fonction par fonction avec sa barre BetterTouchTool, liste les manques | non |
| Sam, dev anglophone | Lit uniquement la partie anglaise, juge si on peut contribuer | non |

Format du rapport : contexte · ce qui a marché · bugs (étapes, attendu, obtenu, gravité) · frictions · manques · 3 propositions concrètes.

## Phase 2 — synthèse et carnet

L'orchestrateur fusionne les rapports en `docs/retours/0.1/SYNTHESE.md` : carnet ordonné (bugs > ergonomie > personnalisation), chaque item avec sa preuve attendue.

## Phase 3 — chantiers (séquentiels pour le matériel, parallèles pour le reste)

- **3a Personnalisation** : nouvelles options et actions tirées du carnet (icône + texte, couleur du texte, raccourcis clavier, lancer une app, ouvrir une URL, éléments conditionnels…), `tactile --verifier fichier.toml`, messages d'erreur guidés. Tests `cargo test` + `scripts/verifier.sh`.
- **3b Éditeur visuel** « Personnaliser ma barre » (skill `ui-ux-director` d'abord) : fenêtre native, aperçu de la barre, glisser-déposer, application en direct, FR + EN.
- **3c Open source** : README en anglais, esthétique (bannière, captures, badges, GIF), README français conservé en `README.fr.md`, gabarits d'issues, CONTRIBUTING en anglais.

## Phase 4 — seconde vague de faux clients

Les mêmes personas sur la 0.2. Corrections, `scripts/verifier.sh`, push, release `v0.2.0`.

## Phase 5 — installation chez l'utilisateur

`scripts/empaqueter.sh`, copie dans `~/Applications`, lancement, vérification au doigt par l'utilisateur.

## Journal

- 2026-10-26 : dépôt renommé `lepalph/tactile-touch-bar`, public, release `v0.1.0` en ligne. Phase 1 lancée.
