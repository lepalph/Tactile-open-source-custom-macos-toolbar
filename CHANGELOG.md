# Changelog

All notable changes to Tactile are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- "Customize my bar" window (menu item, or `tactile --personnaliser`): live preview at real size, drag to reorder, symbol picker, three ready-made bars (Simple, Music, Developer), FR / EN; saves the same `barre.toml`, keeps a `.precedent` copy and can go back to it. Unknown settings (an `image`, a `keycode`) survive a round trip.
- Keyboard shortcuts as actions: `{ touche = "cmd+shift+4" }`, resolved on the current keyboard layout, and `{ keycode = n }`.
- `{ app = "Figma" }`, `{ url = "https://…" }` and `{ applescript = "…" }` actions.
- Button appearance: `image` (with `gabarit`), `couleur_texte`, `taille`, `affichage = "icone" | "texte" | "les_deux"`; `largeur` on groups.
- Rich script output: a JSON line `{"texte", "symbole", "image", "couleur", "couleur_texte"}` styles the widget.
- `batterie` element (`pmset`, every 30 s), with or without the battery symbol.
- English aliases for every key, type, setting and action (`exemples/barre.en.toml`, glossary in the README).
- Command line: `--help`, `--version`, `--verifier [file]` (checks a bar file and reports the line of an error); unknown argument exits with code 2. `TACTILE_CONFIG` overrides the configuration path.
- Config errors shown in the bar itself ("⚠ config ligne N") and a "View the log" menu item.
- App icon drawn in code (`scripts/icone.sh`), embedded in `Tactile.app`; issue and PR templates; clippy and `cargo doc` in CI; release notes with SHA-256.

### Fixed

- An unknown SF symbol no longer crashes the bar: its name is shown as text.
- Text without `largeur` is capped at 200 pt; the log says how many items overflow the bar.
- Scripts: 5 s timeout (`delai`), stderr and exit code logged, no orphan processes when the bar is reloaded.
- `masquer_control_strip` defaults to `true` as documented, and changing it reloads the bar placement.
- `cargo doc` without warnings.

## [0.1.0] - 2026-10-25

### Added

- Native Touch Bar replacement written in Rust: buttons, expandable brightness and volume sliders, now playing, clock, shell scripts, groups and spacers.
- Bar described in `~/.config/tactile/barre.toml`, reloaded within a second when saved.
- Native brightness, volume and media actions; long-press actions on buttons.
- Switch to Apple's bar and back with one touch (the ✳ icon in the Control Strip); `masquer_control_strip` option.
- Menu bar icon: show my bar, Apple's bar, open the configuration, launch at login, quit.
- `Tactile.app` universal bundle (Apple Silicon and Intel), ad hoc signed, built by `scripts/empaqueter.sh`; `scripts/verifier.sh` drives every control and checks the result.
- README, MIT license, contribution guide and CI.

[Unreleased]: https://github.com/lepalph/tactile-touch-bar/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/lepalph/tactile-touch-bar/releases/tag/v0.1.0
