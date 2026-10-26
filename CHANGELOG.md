# Changelog

All notable changes to Tactile are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Editor: add your own logo to a button with Choose an image… or by dropping a file from the Finder; it is copied to `~/.config/tactile/icones/` (SVG, PDF and icns converted to PNG), with a monochrome switch. A button with both an image and a symbol now shows the image.
- Export and import a bar: Export… / Import… in the editor (or drop a `.toml` / `.zip` on the window) and `tactile --export <file>` / `tactile --import <file> [--yes]`. A bar with logos is exported as a `.zip` (`barre.toml` + the `icones/` it uses, paths made relative). Import shows a preview and lists everything the bar can run (shell, AppleScript, shortcuts, scripts, apps, non-web URLs) before **Replace my bar**; the current bar goes into the history. Untrusted archives are read in memory: entries with `..`, absolute paths or symlinks are refused, limits of 10 MB per archive, 2 MB per image, 200 files and 1 MB of TOML, and logos are redrawn as PNG.
- `lanceur` element (`launcher`): an app's real icon, a tap opens it or brings it to the front, a dot under the icon while it runs; `texte`, `affichage`, `largeur`, `appui_long`. A missing app shows its name as text and `--verifier` warns. In a `groupe`, launchers make an app drawer (`exemples/lanceur.toml`).
- System actions: `mission_control`, `fenetres_app` (`app_windows`), `bureau` (`show_desktop`), `launchpad`, `capture_options` (`screenshot_options`) through Apple's own apps; `spotlight`, `capture_ecran` (`screenshot`), `capture_zone` (`screenshot_area`), `emojis` (`emoji`), `verrouiller` (`lock_screen`), `forcer_quitter` (`force_quit`) as macOS's default shortcuts.
- Editor: add an app launcher (on its own or in a group) with the app's real icon in the preview, and pick the 11 system actions under a "System" heading. Import lists what launchers and system actions open or type.
- `TACTILE_MESURE=1` logs how long reloads, app switches and sub-bars take.

### Changed

- Editor: redesigned like macOS System Settings (sidebar, grouped rows, switches, light and dark), suggests the closest symbol for a typo, and `capture` now uses WebKit snapshots that work with the screen locked (`TACTILE_APPARENCE=clair|sombre` forces the theme).
- The bar follows a saved file in under 300 ms (was up to 1 s): the file is checked every 0.2 s, the new bar is applied right away, and warnings are computed after it.
- Editor: a file dropped in the window is limited to 10 MB, with a clear message (it was unbounded on the Rust side).

## [0.2.0] - 2026-10-26

### Added

- "Customize my bar" window (menu item, or `tactile --personnaliser`): live preview at real size, drag to reorder, symbol picker, three ready-made bars (Simple, Music, Developer), FR / EN; saves the same `barre.toml`, keeps a `.precedent` copy and can go back to it. Unknown settings (an `image`, a `keycode`) survive a round trip.
- Keyboard shortcuts as actions: `{ touche = "cmd+shift+4" }`, resolved on the current keyboard layout, and `{ keycode = n }`.
- `{ app = "Figma" }`, `{ url = "https://…" }` and `{ applescript = "…" }` actions.
- Button appearance: `image` (with `gabarit`), `couleur_texte`, `taille`, `affichage = "icone" | "texte" | "les_deux"`; `largeur` on groups.
- Rich script output: a JSON line `{"texte", "symbole", "image", "couleur", "couleur_texte"}` styles the widget.
- `batterie` element (`pmset`, every 30 s), with or without the battery symbol.
- Per-app bars: an `[[application]]` table (`app` = bundle id or display name, `herite` to keep the main bar's edges) shows its own items while that app is frontmost.
- English aliases for every key, type, setting and action (`exemples/barre.en.toml`, glossary in the README).
- Command line: `--help`, `--version`, `--verifier [file]` (checks a bar file and reports the line of an error); unknown argument exits with code 2. `TACTILE_CONFIG` overrides the configuration path.
- Config errors shown in the bar itself ("⚠ config ligne N") and a "View the log" menu item.
- App icon drawn in code (`scripts/icone.sh`), embedded in `Tactile.app`; issue and PR templates; clippy and `cargo doc` in CI; release notes with SHA-256.
- `--verifier` checks what breaks in use, with one validator shared by the editor and the bar's log: unknown SF Symbol, color not `#RRGGBB`, missing image or app, overflow of the main bar and of each app bar, the same app twice; warnings start with `attention :` (exit 0), an empty bar is an error (exit 1).
- `--verifier` warns when an `[[element]]` comes after an `[[application]]` (it goes to the main bar), with its line.
- `--help` follows the system language (English unless French); `--check` and `--customize` are English aliases of `--verifier` and `--personnaliser`.
- Editor: the last 5 saved versions are kept (`barre.toml.precedent`, `.precedent-2` … `.precedent-5`) and "Go back" steps back one per click without losing any; the first save over a hand-commented file keeps a `barre.toml.manuel` copy, never overwritten.
- Editor: an app name typed by hand is checked, with the closest installed app offered in one click.

### Fixed

- An unknown SF symbol no longer crashes the bar: its name is shown as text.
- Text without `largeur` is capped at 200 pt; the log says how many items overflow the bar.
- Scripts: 5 s timeout (`delai`), stderr and exit code logged, no orphan processes when the bar is reloaded.
- `masquer_control_strip` defaults to `true` as documented, and changing it reloads the bar placement.
- `cargo doc` without warnings.
- AZERTY: digits target the top row by position (`cmd+shift+4` sends key 21, never the keypad); characters that need Shift or Option (`\`, `[`) get it added; `{ touche = "cmd+shift+keycode:21" }` gives modifiers to a key code.
- Memory leaks: a reload frees the previous bar, file images are downscaled to 20 pt (2×) and cached by path and date (a 10,000 px PNG went from 790 MB to 20 MB; a JSON script with `image` no longer grows).
- The "⚠ config" badge also shows on per-app bars.
- An empty config file (or one without items) shows the default bar with a "⚠ config vide" badge instead of a black Touch Bar.
- The frontmost app is observed (`NSWorkspaceDidActivateApplicationNotification`) instead of polled every 0.1 s: idle CPU with an `[[application]]` drops from ~0.6 % to ~0.15 %.
- The overflow warning also counts short-text buttons (at least 72 pt each).
- `[[element]]` and `[[item]]` (also as `application.` tables and group children) can be mixed in one file and keep the file order, instead of failing with "duplicate field".
- An error inside an `[[application]]` (or a group in it) reports its own line, and errors come in file order.
- TOML and serde error messages are fully translated into French; "1 élément", "2 éléments".
- Version 0.2.0 with crate metadata; hardware tests show as `ignored` under `CI`; the glossary test reads both READMEs; CONTRIBUTING lists the eight places to touch when adding an action; `{ app = "com.example.App" }` opens a bundle id with `open -b`.
- Editor: an unreadable bar file no longer opens as an empty bar that Save would write over. The window shows the faulty line, turns Save off, and offers to open the file or start from a ready-made bar (the broken file is first copied to `barre.toml.abime-YYYYMMDD-HHMMSS`). Saving a bar whose main bar has no item is refused, with the same message as `--verifier`.
- Editor: unsaved changes are flagged ("Edited", dot in the close button) and confirmed before closing the window, loading a ready-made bar or going back; going back now says it worked.

## [0.1.0] - 2026-10-25

### Added

- Native Touch Bar replacement written in Rust: buttons, expandable brightness and volume sliders, now playing, clock, shell scripts, groups and spacers.
- Bar described in `~/.config/tactile/barre.toml`, reloaded within a second when saved.
- Native brightness, volume and media actions; long-press actions on buttons.
- Switch to Apple's bar and back with one touch (the ✳ icon in the Control Strip); `masquer_control_strip` option.
- Menu bar icon: show my bar, Apple's bar, open the configuration, launch at login, quit.
- `Tactile.app` universal bundle (Apple Silicon and Intel), ad hoc signed, built by `scripts/empaqueter.sh`; `scripts/verifier.sh` drives every control and checks the result.
- README, MIT license, contribution guide and CI.

[Unreleased]: https://github.com/lepalph/tactile-touch-bar/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/lepalph/tactile-touch-bar/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/lepalph/tactile-touch-bar/releases/tag/v0.1.0
