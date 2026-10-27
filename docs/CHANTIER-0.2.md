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
- 2026-10-26 (suite) : phase 1 et synthèse commitées (327803f). Phase 3 : solidité (388f274..2acec72), personnalisation (1d9c02c..9504e7f), éditeur visuel + icône/GitHub/CI fusionnés (ab6252a, 8627c78, 23a394c), tout poussé, CI verte (clippy et doc en -D warnings). En cours au moment de l'arrêt (quota) : 3a-3 barre par application (agent sur `main`, commits à venir, vérifier `git log`), 3c-2 README anglais + README.fr.md + CONTRIBUTING EN (branche `worktree-agent-a21675239ab23715a`, à fusionner : si 3a-3 a ajouté une section au README français, la traduire dans README.md). Reste ensuite : phase 4 (seconde vague des 4 personas sur la 0.2, corrections, verifier.sh), tag `v0.2.0`, phase 5 (empaqueter, copier dans ~/Applications, lancer, « Lancer au démarrage » par le menu). Nettoyage : `git worktree prune` après fusion des branches worktree-agent-*.
- 2026-10-26 (fin) : seconde vague de personas (docs/retours/0.2/), lots R, C et E corrigés et fusionnés, CI verte, release `v0.2.0` publiée (SHA-256 ea066052…), installée dans `~/Applications/Tactile.app` depuis le zip publié et lancée. Reste à prouver écran déverrouillé, au doigt : changement d'app (barre par application), ⌘⇧4 sur AZERTY, fenêtre de l'éditeur (confirmations, suggestion d'app), barre d'Apple et retour ✳. « Lancer au démarrage » : à activer par l'utilisateur dans le menu de Tactile. Reportés : `barre = "apple"`, `app = [liste]`, import BTT, Mac Intel, erreurs en anglais.
- 2026-10-26 (demande de l'utilisateur) : **au début de la prochaine phase** (vers 18 h 30), lancer d'abord une campagne de tests de toute l'app : tests unitaires, tests fonctionnels, et personas IA qui testent l'app de bout en bout. En cours avant cette phase : moteur (lanceur, raccourcis système, fluidité), logos personnalisés ; en attente : import/export, éditeur des barres par app, liens avec logo, barre par site.
- 2026-10-27 : campagne de tests 0.3 terminée (docs/retours/0.3/), lots S/C/E fusionnés, release `v0.3.0` publiée et installée depuis le zip publié (SHA-256 3cc382f9…). Vérification complète écran déverrouillé : tout vérifié, code 0, barre d'Apple et vraie bascule d'app vues pour la première fois. Suite prévue : styles de curseurs + curseur libre + choix, barre par site, éditeur des barres par app, liens avec logo de site, enregistreur de raccourci, Tactile serveur MCP (à proposer).
