# Contributing to Tactile

🇫🇷 [Lire en français](CONTRIBUTING.fr.md)

Thanks for your interest. You can contribute in English or in French.

Identifiers and comments in the code are in French. This is a project choice ([DECISIONS.md §5](docs/DECISIONS.md), in French), so functions are called `luminosite::regler`, not `brightness::set`. If you don't speak French, write your code in English and say so in the pull request; a maintainer will rename things. The [glossary](#glossary) below covers the identifiers you'll run into first.

## Build

You need stable Rust. The repo has a `mise.toml`, so `mise install` sets it up.

```
cargo build --release      # target/release/tactile
scripts/empaqueter.sh      # universal target/Tactile.app (Apple Silicon + Intel) and target/Tactile.zip
```

`target/release/tactile --help` lists the command-line options. To try the editor without touching your real bar:

```
TACTILE_CONFIG=/tmp/try.toml target/release/tactile --personnaliser
```

## Test

Write the test that describes the behavior first, watch it fail, then write the code. An action is proven by changing a value, reading it back, then restoring the original state (see `src/actions/volume.rs`).

```
CI=1 cargo test
```

Without `CI=1`, the brightness and volume tests really change your screen brightness and output volume, then put them back. With `CI` set, `build.rs` turns on `cfg(ci)` and those hardware tests show as `ignored` (`#[cfg_attr(ci, ignore = "…")]`). CI runs `cargo build --release`, `cargo clippy --all-targets -- -D warnings`, `cargo doc --no-deps` and `cargo test` on macOS; all four must pass, with no warnings.

## Full check on a real Touch Bar

Before a pull request that touches the bar, run `scripts/verifier.sh`. It needs a MacBook Pro with a real Touch Bar and the screen on; touch the trackpad now and then so the Touch Bar stays awake. The script starts Tactile in self-test mode, presses every control the way a finger would, checks the real effect, and compares a capture of the Touch Bar at each step with the expected screen. A black capture is reported as unverifiable, never as a pass. Attach its output to your pull request.

## Visual changes

Any change you can see in the bar comes with a capture of the Touch Bar, attached to the pull request:

```
screencapture -b barre.png
```

If the image is all black, the screen was asleep: wake it and try again.

A few variables help capture what you can't touch:

- `TACTILE_OUVRIR=1 target/release/tactile` opens sub-bar number 1 (the first slider or group) right away.
- `TACTILE_MASQUER=1 target/release/tactile` switches to Apple's bar 2 s after launch.
- `TACTILE_RETOUR=1` (with the previous one) brings Tactile's bar back at 4 s, the same way a tap on ✳ does.
- `TACTILE_MESURE=1` writes to the log how long a reload, an app switch and opening or closing a sub-bar take (`mesure : …` lines); the numbers to keep are in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#mesures).

## Adding an action

An action is what goes after `action =` in the bar file. A new one touches eight places; the tests fail if you miss 4 to 8.

1. `src/actions/mod.rs`: a variant in the `Action` enum. Its `snake_case` name is the word in the file (`BasculerMuet` → `"basculer_muet"`); a variant with a value is written `{ name = value }`. Add its English alias, `#[serde(alias = "toggle_mute")]`, and `#[serde(serialize_with = "court")]` on an `f32` value (otherwise `0.3` is saved as `0.30000001192092896`).
2. The function that does the work, in a file of `src/actions/`, usable without any UI, with a test that changes the state, reads it back and restores it. A test that needs the hardware takes `#[cfg_attr(ci, ignore = "…")]`.
3. Its branch in `executer` (`src/actions/mod.rs`). An action the bar handles itself, like `barre_apple`, goes in `declencher` (`src/barre/mod.rs`) instead.
4. `src/config.rs`, test `chaque_forme_d_action`: one line with the French spelling and the expected `Action`.
5. `src/config.rs`, `TOUT_FR` and `TOUT_EN`: the action in both, French and English spelling; `alias_anglais_donnent_la_meme_barre` checks they give the same bar.
6. `src/config.rs`, `BARRE_COMPLETE`: the action once, for the round-trip tests (`aller_retour_toml`, and `le_json_ne_perd_aucun_champ` in the editor).
7. The editor, `src/editeur/editeur.html`: an action without a value goes in `ACTIONS_SIMPLES` and `CLE_ACTION`, with its label in both the `fr:` and `en:` dictionaries (leave the French label to a maintainer if you don't speak French); an action with a value goes in `actionUI`, `actionModele` and `champAction`. An action the editor doesn't know shows as "Other" and survives a save.
8. Both READMEs: the row of the actions table, and the glossary row in `README.md` (`### English aliases`) and `README.fr.md` (`### Glossaire`). `glossaire_du_readme` checks that both glossaries match and that every row appears in `TOUT_FR` / `TOUT_EN`.

If the action points at something that may be missing (an app, a file), also teach `config::controler` to warn about it: `tactile --check`, the editor and the log all go through it.

## Code style

- French names for functions, variables, types and user-facing messages.
- Few comments: only the why that the code doesn't say.
- A system action is a function you can test without any UI, in `src/actions/`.
- `cargo build` and `cargo clippy` without warnings.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/). French is preferred and English is accepted: `feat: groupe repliable` or `feat: collapsible group`, `fix: …`, `docs: …`. One commit per step that works.

## Architecture overview

1. `src/main.rs` reads the command line, then loads the bar (`config::charger`) and starts the app (`barre::lancer`).
2. `src/config.rs` defines the bar file: types `Barre`, `Element`, `Reglage`, with serde aliases for the English keys. It also checks the file (`lire` for the syntax, `controler` for what breaks in use: colors, SF Symbols, images, apps, overflow) and watches it, re-reading it when its date changes (checked every 0.2 s).
3. `src/actions/` holds the system actions (`Action` enum, `executer`): brightness, volume, media, keyboard. None of them needs a UI.
4. `src/barre/` is the AppKit layer. `mod.rs` holds the state and `TactileCible`, which receives taps, slider moves and a 0.1 s timer.
5. `elements.rs` turns each `Element` into a Touch Bar item; sliders and groups open system sub-bars.
6. `rafraichi.rs` updates live items (clock, now playing, scripts, battery). Slow work runs on a thread and comes back through a channel.
7. `plateau.rs` puts the ✳ icon in the Control Strip; `src/menu.rs` is the menu bar icon.
8. `src/editeur/` is the "Customize my bar" window: an `NSWindow` with a `WKWebView` showing `editeur.html`, and a JS ⇄ Rust bridge.
9. A tap goes `TactileCible::appui` → `declencher` → `actions::executer`, or opens or closes a sub-bar.
10. The details, the recipes to add an action or an element, the private APIs and the known pitfalls are in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) (French).

## Glossary

| French | English |
|---|---|
| `barre` | bar |
| `bouton` | button |
| `luminosite` | brightness |
| `appui` | tap, press |
| `glisse` | slide (slider moved) |
| `declencher` | trigger |
| `rafraichi` | refreshed (live items) |
| `souple` | flexible (space) |
| `heure` | clock, time |
| `en_cours` | now playing |
| `plateau` | tray (the ✳ icon in the Control Strip) |
| `vues` | views (AppKit helpers) |
| `cible` | target (Objective-C action target) |
| `montrer` | show |
| `presenter` | present |
| `masquer` | hide |
| `afficher` | display, show |
| `config` | configuration |
| `lire` | read, parse |
| `charger` | load |
