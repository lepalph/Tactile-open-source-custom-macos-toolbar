# Changelog

All notable changes to Tactile are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Animated GIFs on the bar: `image = "~/x.gif"` (also a script's `image`) animates at the file's own pace (frames reduced to 20 pt × 2, at most 60 frames and 2 MB, checked by `controler_image`; about 0.6 % CPU at 12 frames/s). The editor keeps the GIF as is in `icones/` and its preview animates.
- Startup animation (`animation_demarrage`, alias `startup_animation`, `true` by default, switch in the editor): at launch only, a brief ✳ Tactile, then the items fade and slide in from left to right within 0.8 s. Skipped with macOS "Reduce motion". Non-blocking (Core Animation).

## [0.4.0] - 2026-10-27

### Added

- Editor: a "Bars" section in the sidebar lists the main bar, every `[[application]]` and every `[[site]]`; each one is previewed and edited like the main bar (inherited items of the main bar are dimmed), with "New app bar…" and "New site bar…" (an `inherit` switch) and a confirmed delete.
- Editor: "Web link…" turns an address (or one of ten common sites) into a `{ url }` button. Tactile reads the page title and its `apple-touch-icon`, then favicon, once and directly from the site (https or `http://localhost`, 10 s, 1 MB, the image is checked like any logo, no third-party service), and keeps the icon in `icones/`. No icon found: the `globe` symbol.
- Editor: on a `touche` action, "Record a shortcut" captures the combination you type and writes it (`cmd+shift+4`); Esc cancels.
- Free-standing sliders: a `slider` (also `curseur`) can drive anything a command can set, with `set` (`regler`, `{valeur}` becomes the number, run at most every 0.1 s while dragging and once more on release) and `read` (`lire`, run when the slider opens and every `every` seconds while open), `min`, `max`, `step`. `{valeur}` is always a number formatted by Tactile. Templates in `exemples/curseurs.toml`: Spotify and Music volume (AppleScript), an external display through `m1ddc`.
- Slider styles for every slider (brightness, volume, free): `classic` (Apple's), and six drawn by Tactile that follow the finger without rebuilding the bar: `grid`, `gauge`, `dots`, `equalizer`, `stepper` (big digits, − and +, long press repeats), `dial` (compact ring). Per-level `colors` with `thresholds`, `notches`, `haptic` feedback at each notch.
- `choice` element (`choix`): a segmented picker; the tapped option replaces `{valeur}` in the action, single-quoted for the shell or double-quoted for AppleScript; optional `read` selects the current option.
- `--check` warns about `min` ≥ `max`, an impossible `step`, thresholds outside the slider or out of order, invalid or miscounted colors, notches out of 2-40, a `set` command without `{valeur}`, a choice without options or whose action ignores `{valeur}`. A slider with neither `setting` nor `set`, or with both, is refused.
- Import lists a slider's `set` and `read` commands and a choice's action and `read` command.
- Editor: a slider panel with what it controls (brightness, volume or a command, with Spotify / Music / external display templates), a gallery of the seven styles as live thumbnails drawn like the Touch Bar, notches, haptics and per-level colors; a Choice item with its options.
- A bar per website: a `[[site]]` table (`adresse` / `address`: a domain with its subdomains, optionally a path prefix, or a `*` pattern such as `*.google.com/maps*`; `herite` / `inherit` like `[[application]]`) shows its bar while the active tab of the browser in front is on that site, before the app's bar and the main bar. Safari, Google Chrome, Arc, Brave, Microsoft Edge, Vivaldi and Chromium are read every 0.5 s, only while one of them is in front and the file has a `[[site]]`, through a compiled `NSAppleScript` on a dedicated thread (1 s at most per read). Only "domain/path" is kept, in memory; the URL is never logged, written or sent. A refused Automation permission shows a "⚠ site" button that opens the right settings pane, logs once per browser, and `--verifier` explains where to allow it. `--verifier` also reports an empty address, an invalid pattern and two `[[site]]` tables for the same address. Imports list each `[[site]]`; the editor keeps them. Example: `exemples/par-site.toml` (YouTube, Gmail). The app bundle declares `NSAppleEventsUsageDescription` (French and English).

## [0.3.0] - 2026-10-26

### Added

- Data sources: a `[[source]]` reads JSON or text over HTTPS (`http`, via `/usr/bin/curl`, 10 s and 1 MB at most), from a command, or from an MCP tool (`mcp`, stdio server kept alive or Streamable HTTP URL; both the 2026-07-28 and the older `initialize` protocol eras), every `toutes_les` seconds (60 by default, 5 at least). A new `texte` element (or a button with `source`) shows it through a template (`"{current.temperature_2m} °C"`), with `symbole_selon` and `couleur_selon`. Headers can use `trousseau:service` to read a secret from the macOS keychain; secrets never reach the log. `--verifier` reports unknown or duplicate sources, non-https URLs, intervals under 5 s, templates without a source and plain-text keys. Examples: `exemples/meteo.toml` (Open-Meteo), `exemples/mcp.toml` with a tiny demo server.
- Quitting from the menu now also kills running scripts and source processes.
- Editor: a `texte` item shows in the preview (its symbol, `—` in place of the value) and its source, template and width can be edited.
- Editor: add your own logo to a button with Choose an image… or by dropping a file from the Finder; it is copied to `~/.config/tactile/icones/` (SVG, PDF and icns converted to PNG), with a monochrome switch. A button with both an image and a symbol now shows the image.
- Export and import a bar: Export… / Import… in the editor (or drop a `.toml` / `.zip` on the window) and `tactile --export <file>` / `tactile --import <file> [--yes]`. A bar with logos is exported as a `.zip` (`barre.toml` + the `icones/` it uses, paths made relative). Import shows a preview and lists everything the bar can run (shell, AppleScript, shortcuts, scripts, apps, non-web URLs, and each data source's command, MCP server and fetched URL) before **Replace my bar**; the current bar goes into the history. Untrusted archives are read in memory: entries with `..`, absolute paths or symlinks are refused, limits of 10 MB per archive, 2 MB per image, 200 files and 1 MB of TOML, and logos are redrawn as PNG.
- `lanceur` element (`launcher`): an app's real icon, a tap opens it or brings it to the front, a dot under the icon while it runs; `texte`, `affichage`, `largeur`, `appui_long`. A missing app shows its name as text and `--verifier` warns. In a `groupe`, launchers make an app drawer (`exemples/lanceur.toml`).
- System actions: `mission_control`, `fenetres_app` (`app_windows`), `bureau` (`show_desktop`), `launchpad`, `capture_options` (`screenshot_options`) through Apple's own apps; `spotlight`, `capture_ecran` (`screenshot`), `capture_zone` (`screenshot_area`), `emojis` (`emoji`), `verrouiller` (`lock_screen`), `forcer_quitter` (`force_quit`) as macOS's default shortcuts.
- Editor: add an app launcher (on its own or in a group) with the app's real icon in the preview, and pick the 11 system actions under a "System" heading. Import lists what launchers and system actions open or type.
- `TACTILE_MESURE=1` logs how long reloads, app switches and sub-bars take.

- Editor: a Text item and a Weather… panel in the sidebar: search a city (Open-Meteo geocoding, only when you click Search, through the same `curl` path as data sources), and the window adds the Open-Meteo `[[source]]` and a `texte` item with WMO weather symbols. A Sources section lists the bar's sources (name, web address or command, interval) and edits them; renaming a source updates the items that use it.
- Editor: Import offers **Add to my bar** next to Replace my bar: items, sources and app bars are merged, and a source whose name is taken is renamed along with the items that use it.
- Editor: a main menu while the window is open (Edit: Undo, Redo, Cut, Copy, Paste, Select All; File: Close ⌘W; View: Zoom In ⌘+, Zoom Out ⌘−, Actual Size ⌘0, remembered). ⌘C, ⌘V, ⌘X, ⌘A and ⌘Z now work in the fields.
- Editor: Delete/⌫ on a preview item can be undone (an "Undo" link in the status line, or ⌘Z); keyboard shortcuts are shown under the preview.
- `--check` (and the editor after saving) warns when a bar runs scripts faster than once a second (more than 2 such scripts, or more than 5 runs per second): a script every 0.3 s costs about 0.4 % CPU on its own.

### Changed

- Editor: saving still succeeds when something will work badly, but says so ("Saved, 1 warning: Figma isn't installed on this Mac"): the same checks as `--verifier`. The preview fades and strikes through the items the Touch Bar will hide, with "2 items don't fit".
- Editor: an app name as the Finder shows it in another language ("Musique") is found and the folder name ("Music") offered. Shortcut, esc and screenshot actions say they need the Accessibility permission, with a button to the right System Settings pane. A non-transparent image in monochrome gets a warning.
- Editor accessibility: focus stays on the same item after every redraw (Enter, symbol choice, Delete); preview items have full accessible names ("App launcher Safari, opens Safari, 2 of 9") and real `aria-selected` values; the symbol grid is a listbox with one tab stop and arrow keys; only short status messages are announced. Contrast meets WCAG AA in light and dark (focus ring, switches off, selected segment, field borders, placeholders, dark-mode links, the missing-app ⚠). Text is sized in rem and the layout holds at 200%.
- Editor: switching language keeps the message on screen (an error stays an error); import labels are plainer (no internal action names, no line numbers or "bundle id" in warnings, the "can delete files" warning only for bars that run a command).
- Editor: Replace (ready-made bar) and Go back put the focus on Cancel.
- Editor: the header fits on one line (Import…, Export… and Go back are in a **⋯** menu next to Save); the window opens taller when the screen allows it, and the Add column scrolls with a fade at the bottom instead of cutting an item.

- Editor: redesigned like macOS System Settings (sidebar, grouped rows, switches, light and dark), suggests the closest symbol for a typo, and `capture` now uses WebKit snapshots that work with the screen locked (`TACTILE_APPARENCE=clair|sombre` forces the theme).
- The bar follows a saved file in under 300 ms (was up to 1 s): the file is checked every 0.2 s, the new bar is applied right away, and warnings are computed after it.
- Images (button `image`, script JSON image, logos added or imported) are refused above 4096 × 4096 pixels, read from the file header before any decoding; SVG and PDF are rendered at most 4096 px wide. A tiny PNG announcing 30000 × 30000 px no longer makes the bar allocate gigabytes.
- Editor: a file dropped in the window is limited to 10 MB, with a clear message (it was unbounded on the Rust side).

### Fixed

- `{text}` works like `{texte}` for a source whose output isn't JSON (it showed "—"), and `--check` reports a template variable that can't exist (`{Text}`, `{a..b}`, `{}`).
- Errors and warnings name the key the way you wrote it (`every`, `template`, `color_by`, `command`, `[[item]]`); a missing field gives both names (`«outil» (ou «tool»)`); a string `template` on a button says to put the template in `text`.
- `--check` sees the same app written twice (`Finder` and `com.apple.finder`, resolved through LaunchServices), and a well-formed bundle id of an app that isn't installed no longer gets the "write the .app name" advice.
- Shortcuts accept `+` as the key (`cmd++`); `cmd+-` works too.
- The import summary (`--import`) is titled "What this bar does on your Mac", like the editor's list, since it also covers files read and headers sent.
- Importing a `.zip`: warnings name the item (`élément 3`, `application « Safari », élément 1`) instead of a line number nobody can see, and a warning that only exists in French keeps the French prefix (`attention :`) in an English summary.
- CONTRIBUTING: every `cargo` command works without activating mise (`mise exec -- cargo …`), the editor step runs in the background, the real test time (about 10 s) and what the `ignored` tests are; `ignore` reasons are in English. The README lists every accepted modifier (`command`, `control`, `maj` were missing).
- A ready-made bar dropped the `[[source]]` entries used by the app bars it kept, which then showed "⚠ source".

### Security

- Import now shows every header of a data source. A `trousseau:` header is spelled out ("Reads your saved password “github-token” and sends it to exfil.example.com"); when that secret goes to a host other than this Mac, the editor preview opens with a warning banner and `--import` refuses without an explicit `--yes` (and still prints the warning with it). The list of what a bar does is built by one exhaustive `match` over every item, action and source field, so a new type can't be forgotten; it also lists image files read outside `icones/`.
- A command's output (`script`, `commande` source, keychain lookup, curl) and each line from a stdio MCP server are read with a 1 MB cap while reading: past it, the process group is killed and the item (a `script`, or a `texte` bound to a `commande` source) shows "⚠ trop long" (the error is logged). `cat /dev/zero` no longer fills memory before the timeout.
- An imported bar can only use images from its own archive (`icones/…`): an absolute, `~` or other path (`/etc/passwd`, `~/.ssh/id_rsa`) is dropped, the button falls back to its text or symbol, and the preview says "image ignored: path outside the imported bar". Your own bars keep their paths.
- Editor: messages from the page that carry a path (`capture`, `importerImage`, `importerBarre`, `exporterVers`) are only followed with the `TACTILE_EDITEUR_JS` driver, inside the temporary folder or the repository; otherwise only the file NSSavePanel just returned can be written.

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

[Unreleased]: https://github.com/lepalph/tactile-touch-bar/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/lepalph/tactile-touch-bar/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/lepalph/tactile-touch-bar/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/lepalph/tactile-touch-bar/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/lepalph/tactile-touch-bar/releases/tag/v0.1.0
