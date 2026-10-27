# Règles pour les agents qui travaillent sur Tactile

Chaque prompt d'agent renvoie à ce fichier au lieu de les répéter.

- Rust : `mise exec -- cargo …`. Lire d'abord docs/ARCHITECTURE.md et docs/VISION.md.
- Validation avant de conclure : `cargo build --release` 0 warning, `cargo clippy --all-targets -- -D warnings`, `RUSTDOCFLAGS=-D warnings cargo doc --no-deps`, `cargo test` vert avec et sans `CI=1`, `tactile --verifier` OK sur exemples/.
- Bug : test qui échoue d'abord, puis correction de la cause.
- Touch Bar (un seul processus à la fois) : `pgrep -fl tactile` avant ; arrêter l'app de l'utilisateur par `pkill -f 'Applications/Tactile.app/Contents/MacOS/tactile$'` ; essais avec `TACTILE_CONFIG` (jamais ~/.config/tactile) ; à la fin `open ~/Applications/Tactile.app` et preuve `ps -o comm= -p $(pgrep -f 'Applications/Tactile.app/Contents/MacOS/tactile$')`. Ne jamais tuer un `--personnaliser` d'un autre agent.
- Écran : `ioreg -n Root -d1 -a | plutil -extract IOConsoleLocked raw -` ; verrouillé = capture « non vérifiable », jamais « réussie ».
- Éditeur seul : `HOME` factice + `TACTILE_CONFIG` + `TACTILE_EDITEUR_JS` (messages en chaînes JSON, IIFE) + `TACTILE_APPARENCE`, captures par `capture` (takeSnapshot), arrêt par PID.
- Sécurité : tout ce qu'une barre exécute, lit ou appelle passe par `partage::commandes` (match exhaustif, sans `_ =>`).
- Docs : README.md et README.fr.md (glossaires identiques), ARCHITECTURE, CHANGELOG Unreleased en anglais.
- Commits : Conventional Commits en français, terminés par :
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01F5zcax6QngTrp6o3SE8jCv
- Ne pas pousser (l'orchestrateur pousse). Aucun secret. Rapport final : 5 à 8 lignes, preuves et non vérifié.
