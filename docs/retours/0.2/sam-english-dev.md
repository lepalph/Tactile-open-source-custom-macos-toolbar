# Sam, English-speaking Rust dev: Tactile 0.2 re-review

## Context

- Who: Sam, 27, Rust developer in Austin, TX. No French. I reviewed 0.1 (`docs/retours/0.1/sam-english-dev.md`) and came back for 0.2.
- Repo state: `main` at `f6b9cdb` (same as `origin/main`), `Cargo.toml` still says `version = "0.1.0"`, latest GitHub release is still `v0.1.0` (2026-10-25). 0.2 is the `[Unreleased]` section of `CHANGELOG.md`.
- What I did: read `README.md` (English, 388 lines), `CONTRIBUTING.md`, the issue/PR templates and the GitHub metadata (`gh repo view`, `gh api`). I built the release binary and ran `--help`, `--version` and `--verifier` on 20 hand-written files. I ran tests, clippy and doc, and rendered the README through `gh api markdown` (gfm). I did not run the bar itself, the editor or `scripts/verifier.sh`, and I modified no repo file except this report.
- Scratch files: `/private/tmp/claude-501/-Users-palphinho/03bc99cc-4742-4072-87c4-fe525a6d77ae/scratchpad/sam2/` (`sam.toml` = my full config, `e*.toml` = error cases, `readme.html` = rendered README).

Build/test results:

| Command | Result |
|---|---|
| `mise exec -- cargo build --release -q` | OK |
| `CI=1 mise exec -- cargo test` | **52 passed, 0 failed, 2 ignored** (was 15/0/2 in 0.1) |
| `cargo clippy --all-targets -- -D warnings` | **clean** |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps` | **clean** |

## 0.1 blockers: status

| 0.1 item | Status | Proof |
|---|---|---|
| B1 no `--help` / `--version`, any arg launches the app | **Fixed on main, not released** | `src/main.rs:31-41` parses args. `--help` exits 0, `--check` exits 2 with the help text. The help is **French** (`main.rs:9-18`). `git merge-base --is-ancestor` shows none of this is in `v0.1.0`, the build people actually download. |
| B2 README menu labels don't match the UI | **Fixed** | `README.md:71-79` gives the real French labels with an English gloss. They match `src/menu.rs:146-160` exactly. One label is still undocumented: "Lancer au démarrage (depuis Tactile.app seulement)" (`menu.rs:156`). |
| B3 serde's English errors are rewritten into French | **Partial** | `fn traduire` is still there (`src/config.rs:281`). `README.md:322-333` now has a FR→EN table, which helps, but the table misses about half of what I actually got (see Bugs B12). |
| B4 hardware tests report `ok` under `CI=1` | **Still there** | `actions::luminosite::tests::regler_puis_relire_et_restaurer ... ok` and `actions::volume::tests::volume_et_muet_restaures ... ok`. `luminosite.rs:60` and `volume.rs:105` still `return` silently. CONTRIBUTING.md:32 now says "those hardware tests return early", so the behavior is at least documented. |
| B5 `cargo doc` warning | **Fixed** | `RUSTDOCFLAGS=-D warnings cargo doc --no-deps` is clean and enforced in `ci.yml` (`env: RUSTDOCFLAGS`). |
| B6 French description that names another product | **Fixed** | Description: "Customizable Touch Bar for MacBook Pro, written in Rust: buttons, sliders, now playing, your own commands. Free, MIT." The homepage is set to `releases/latest`. |
| B7 empty release body | **Partial** | `release.yml:19-37` now adds `generate_release_notes: true` and a SHA-256 line, so future releases will be fine. `v0.1.0` still has `body: null`. |
| English README is a 19-line footer | **Fixed** | `README.md` is now the English canonical version, `README.fr.md` is the French one, and they link to each other. |
| CONTRIBUTING French-only; French names are a hidden rule | **Fixed** | `CONTRIBUTING.md:5-7` says, in English on the first screen: "write your code in English and say so in the pull request; a maintainer will rename things". English commits are accepted (`:63`). It has a 20-word glossary (`:78-101`). |
| Issue templates | **Fixed** | `bug.yml` asks for macOS version, Mac model, Tactile version, config and log. `feature.yml` and a PR template also exist. All are in English. |
| clippy/doc not enforced in CI | **Fixed** | `ci.yml` runs build, clippy `-D warnings`, doc and test. |
| `Cargo.toml` metadata | **Still there** | The file has no `description`, `repository`, `license` or `keywords`, and the version is still `0.1.0` (so `tactile --version` prints `tactile 0.1.0` on the 0.2 branch). |
| `exemples/` and `scripts/empaqueter.sh` French paths | **Still there** | Only a cosmetic problem now that the README links them. |

## What worked

- **Writing a full config from the English README alone worked on the first try.** `sam.toml` (178 lines) uses English keys only. It covers every element type (`button`, `slider` × 2, `clock`, `now_playing`, `script`, `battery`, `space`, `flexible`, `group` with 7 children), 20 different actions (`apple_bar`, `esc`, `brightness_up/down`, `{ brightness = 0.5 }`, `volume_up/down`, `toggle_mute`, `{ volume = 0.3 }`, `previous`, `play_pause`, `next`, `{ app }`, `{ key }`, `{ keycode }`, `{ url }`, `{ applescript }`, `sleep`, `{ shell }` as `long_press`), every button field (`image`, `template`, `color`, `text_color`, `size`, `display`), and two `[[application]]` bars: Safari with `inherit = true` and a nested group, and Terminal by display name. `tactile --verifier sam.toml` printed `OK : 19 éléments` and exited 0. Huge improvement over 0.1, where I had to read `config.rs`.
- **The README's aliases match the code.** I checked every `#[serde(alias)]` in `src/config.rs`, `src/actions/mod.rs` and `src/barre/rafraichi.rs` against the "English aliases" table at `README.md:166-216`. Every alias in the code is in the table and every table row is accepted. The English and French glossaries are identical (`diff` of the two tables is empty).
- **Every README snippet and every `exemples/*.toml` passes `--verifier`**: the sleep button (`:98-106`), the full English example (`:224-276`, identical to `exemples/barre.en.toml`) and the per-app snippet (`:284-293`).
- **`--verifier` is a great tool for a terminal user.** It reports the path and line, exits 1 on error and doesn't touch the bar. Unknown values list every accepted spelling, French and English together, so I could fix `btn` and `colour` without the glossary.
- **CONTRIBUTING.md is what I asked for in 0.1.** It states the language policy first, says English PRs are fine, gives build/test commands with the `CI=1` warning, explains the `TACTILE_*` variables in English and has a 10-step architecture overview. I would no longer worry about a policy bounce.
- **The README renders cleanly on GitHub.** `gh api repos/.../readme` returns the same blob as local (`fccbcdc`). The gfm render has 8 tables, all with consistent column counts (2, 2, 3, 2, 2, 2, 2, 2), no stray `|` or `**` outside code, all 8 images resolve to tracked files (icon, 3 PNGs, 4 shields badges via camo), and all 19 relative links point to files that exist on `main`. The six in-page anchors (`#install`, `#english-aliases`, `#finding-an-apps-name`, `#troubleshooting`, `#why-media-control`, `#claude-voice-extra`) match GitHub's slugs, as does `docs/ARCHITECTURE.md#api-privées-utilisées`.
- **Honest about what's still French.** `README.md:69`, `:164`, `:316`, `:362` say up front that the menu, errors and `--help` are French, so I was never surprised.

## Bugs

Severity: **High** = blocks an English user or contributor; **Medium** = costs real time or points the wrong way; **Low** = polish.

### B8. The README documents 0.2, but every install link ships 0.1.0

- Steps: follow `README.md:48` ("Download `Tactile.zip` from the latest release"), then paste the English example from `:224-276` into `~/.config/tactile/barre.toml`.
- Expected: it works, or the README says these features are unreleased.
- Actual: the latest release is `v0.1.0`, which has none of this. English aliases arrived in commit `6866c9f` and the arg parsing after it, and neither is an ancestor of `v0.1.0`. That build rejects `[[item]]` and `type = "button"`, has no `--verifier`/`--help` (and per 0.1's `main.rs`, any argument launches the bar), no editor, no `battery`, no `[[application]]`. `README.md:31` even says "The v0.1.0 download is a 1 MB zip", and the issue template placeholder is `0.1.0`. `Cargo.toml` is still `0.1.0`, so a source build also reports `tactile 0.1.0`.
- Severity: **High**. The README is now great, but for the binary people actually download, it is wrong.

### B9. "You can mix both in one file" fails for `[[element]]` + `[[item]]`

- Steps: a file with one `[[element]]` and one `[[item]]` at the top level (`mix.toml`), then `tactile --verifier mix.toml`.
- Expected: per `README.md:114` ("you can mix both in one file") and `:164` ("Both forms can be mixed in one file"), OK.
- Actual: `mix.toml, ligne 1 : duplicate field «element»`, exit 1. That's because `rename = "element", alias = "item"` (`config.rs:18`, also `:33` and the `Groupe` children) makes serde treat both arrays as the same field. Mixing *field* names across items works, but mixing the table headers doesn't. This is easy to hit: copy a block from a French example into an English file. The message is the one untranslated serde message I saw, it points at line 1, and the troubleshooting table doesn't list it.
- Severity: **Medium**.

### B10. `--verifier` says OK to files the editor refuses

- Steps: `symbol = "not.a.real.symbol"`, `color = "red"` (`e7_symbol.toml`).
- Expected: the README says colors are `"#RRGGBB"` (`:126`), the editor refuses "a symbol that doesn't exist, a malformed color" (`:88`), and `:339` suggests `tactile --verifier` to diagnose "My change doesn't show up".
- Actual: `OK : 1 éléments`, exit 0. Colors are only parsed at render time (`src/barre/vues.rs:67`, which silently returns `None` for `red`). Symbols and images are only checked at render time, where they go to the log. The CLI check and the GUI check disagree.
- Severity: **Medium**. A terminal-first user trusts `OK` and then hunts in the log.

### B11. Missing-field errors name the French key even when you wrote English

- Steps: `type = "script"`, `command = "date"`, no interval (`e3_missing.toml`).
- Actual: `ligne 1 : champ manquant «intervalle»`. I never typed `intervalle`, so I need two lookups: "champ manquant" in the troubleshooting table and `intervalle` in the glossary. The same happens with `e6`: "ajoute au moins un [[application.element]]" when I wrote `[[application.item]]`.
- Severity: **Low** (the glossary resolves it), but it is the first error a new English user of `script` will see.

### B12. Error messages are half-translated, and the README table covers about half of them

I triggered 12 errors. Here is what `README.md:324-333` lets me decode:

| What I got | Decoded by the README table? |
|---|---|
| `ligne 5 : valeur inconnue «btn», attendu parmi «bouton», «button», …` | Yes. The list of valid values is the most helpful part. |
| `ligne 1 : champ inconnu «colour», attendu parmi «text», «texte», …` | Yes. |
| `ligne 1 : champ manquant «intervalle»` | Yes, plus a glossary lookup (B11). |
| `ligne 1 : duplicate field «element»` | No row. English, but it contradicts the README (B9). |
| `ligne 1 : type invalide: string "half", attendu f32` | Partly. "invalid type" is in the table, but `f32` is Rust jargon a non-dev won't get ("a number"). |
| `ligne 2 : invalid basic string, attendu «"»` | No. It's franglais: `traduire` replaced "expected" inside a toml parser message. |
| `ligne 1 : modificateur inconnu « hyper » ; attendu : cmd, shift, ctrl, alt, fn` | No row. "modificateur" is guessable. The list omits `option`, which `clavier.rs:34` and `README.md:150` both accept. |
| `touche inconnue « … » ; attendu : …` (`clavier.rs:79`) | No row. |
| `[[application]] « Safari » sans élément : ajoute au moins un [[application.element]]` | No row. `README.md:297` describes the rule, but the sentence is opaque. |
| `impossible de lire …/doesnotexist.toml : No such file or directory` | No row. French + English in one line. |
| `OK : 1 éléments` | `:314` glosses "OK : n éléments". Also a plural bug. |

Severity: **Medium**. It's much better than 0.1 (the README is honest and has a table), but the `traduire` pass now produces mixed-language strings that are harder to search than either pure language.

### B13. CLI flags are French-only, and the help is French

- Steps: as an English user I type `tactile --check barre.toml`, then `--verify`, `--customize` and `--edit`.
- Actual: all four exit 2 with `tactile : argument inconnu « --check »` followed by the French help. `--help` output:
  ```
  Usage :
    tactile --personnaliser        ouvre la fenêtre « Personnaliser ma barre », sans la Touch Bar
    tactile --verifier [fichier]   vérifie un fichier de barre sans toucher la Touch Bar
  ```
  Is it understandable? About 70%: "vérifie", "version", "Usage", "Variables" and the paths are cognates. "lance la barre", "sans toucher", "affiche cette aide" and "Journal" are not, and I would not have guessed that `--personnaliser` opens a GUI editor without the README.
- Severity: **Medium** for the Rust-dev audience, which lives in the terminal. The README documents it (`:316`), so I wasn't surprised, but `--check`/`--customize` aliases would cost two match arms in `commande()` (`main.rs:31`).

### B14. The English glossary isn't tested; the test reads the French README

- `config::tests::glossaire_du_readme` (`config.rs:719-736`) runs `include_str!("../README.fr.md")` and splits on `### Glossaire`. The English README's "English aliases" table (`README.md:162-216`) is identical today, but nothing keeps it so. `src/config.rs:65` says the README glossary is "vérifié par le test", and for the canonical README that's not true. For a contributor (see the PR section), an action added to `README.md` but not `README.fr.md` fails a test whose message is French.
- Severity: **Low** (correct today, drift risk).

### B15. `--verifier` summary ignores app bars

- `readme-app.toml` (the snippet at `README.md:284-293`) gives `OK : 0 éléments`. The file is valid, but "0 items" on a file I just wrote reads like a failure. `exemples/par-application.toml` says `OK : 1 éléments` even though it holds several app bars. `main.rs:73` counts only `barre.elements`.
- Severity: **Low**.

### B4 (still open). Hardware tests are silent `ok` under `CI=1`

Unchanged since 0.1. See the status table above.

## English README errors

| Line | Quote | Problem |
|---|---|---|
| `README.md:31` | "The v0.1.0 download is a 1 MB zip." | True, but the whole README describes features v0.1.0 doesn't have (B8). Either cut 0.2 or add an "Unreleased: build from source" banner. |
| `README.md:48` | "Download `Tactile.zip` from the [latest release]" | The latest release is v0.1.0 (B8). |
| `README.md:114` | "Every key, type, setting and action also has an English alias, and you can mix both in one file." | Not true for `[[element]]` + `[[item]]` in the same table (B9). |
| `README.md:164` | "Both forms can be mixed in one file." | Same as above. |
| `README.md:110` | "To check a file without touching the bar: `tactile --verifier`" | Doesn't check colors, symbols or images (B10). Should say "checks the syntax and the keys". |
| `README.md:339` | "`tactile --verifier` checks the file from a terminal." | Same. Offered as the fix for "My change doesn't show up", but it misses the render-time errors that cause exactly that. |
| `README.md:126` | "`couleur` / `color` (background, `"#RRGGBB"`)" | Correct, but nothing enforces it at load time. `"red"` passes and is silently ignored (B10). |
| `README.md:150` | "Modifiers: `cmd`, `shift`, `ctrl`, `alt` (or `option`), `fn`." | Incomplete compared with `clavier.rs:27-37`: `command` and `control` are also accepted (and French `maj`). Keys `return`, `escape`, `backspace` and `forwarddelete` are accepted (`clavier.rs:40-53`) but not listed, and `forwarddelete` is the only way to get forward delete by name. The runtime error message (`clavier.rs:84`) lists `cmd, shift, ctrl, alt, fn` and drops `option`. |
| `README.md:131` | "`batterie` / `battery` → `symbole` / `symbol` (`true` by default …)" | Not wrong, but `symbol` is a string on `button`/`slider`/`group` and a bool on `battery`. Writing `symbol = "battery.100"` gives `type invalide … attendu a boolean`. Worth one word: "(a boolean here)". |
| `README.md:133` | "two `souple` center what's between them" | French name in the English sentence; `:296` says `flexible`. Use `flexible` in the English README. |
| `README.md:297` | "An `[[application]]` without `app` or without items is rejected, with its line number." | True. The message tells you to add `[[application.element]]` (B11). |
| `README.md:314` | "Prints `OK : n éléments` (*n items*)" | Accurate. It counts only main-bar items, not `[[application]]` bars (B15). |
| `README.md:322-333` | The "In the log" table | It lacks `duplicate field`, `modificateur inconnu`, `touche inconnue`, `sans élément : ajoute au moins un`, `impossible de lire`, and the `attendu f32` / `attendu a boolean` shapes (B12). |
| `README.md:38` | `\| \| \|` (empty table header) | Renders as an empty header row on GitHub. Cosmetic: use `\| \| \|` → `\| What \| Details \|` or a list. |
| `README.md:278` | "The other examples in exemples/ are in French: simple.toml and developpeur.toml" | `par-application.toml` and `claude-vocal.toml` are French too (comments and keys). Say "all other examples". |
| `README.md:360` | "listed in [ARCHITECTURE.md](docs/ARCHITECTURE.md#api-privées-utilisées)" | The link works, but it's a French page with no "(in French)" note, unlike `:364` and `:379`. |

## Contributing: would I open a PR now?

**Time-box: adding a new action (say `lock_screen`) from CONTRIBUTING.md alone.** It took me about 10 minutes to locate every touchpoint, and only because I fell back to grepping an existing action (`veille` / `sleep`) across the repo.

- **Found from English docs alone (about 3 minutes):** CONTRIBUTING.md:58 ("A system action is a function you can test without any UI, in `src/actions/`") and :69/:75 (`Action` enum, `executer`, tap → `declencher` → `actions::executer`) gave me the three Rust spots: a variant in `src/actions/mod.rs:15` with `#[serde(alias = "lock_screen")]`, a function in `src/actions/`, and a branch in `executer` (`mod.rs:62`). The `alias` pattern is obvious from the enum.
- **Where I got lost:** CONTRIBUTING.md:76 says "the recipes to add an action … are in docs/ARCHITECTURE.md (French)". That recipe (`ARCHITECTURE.md:61-67`) lists five more touchpoints that the English docs never mention:
  1. `config::tests::chaque_forme_d_action` (`config.rs:433`), plus the `TOUT_FR` / `TOUT_EN` constants (`:478`, `:592`), which must parse to the same bar.
  2. The glossary row, **in `README.fr.md`**, because `glossaire_du_readme` reads only the French file (B14). Editing only the English README makes a test fail with « … » n'est pas dans TOUT_EN.
  3. Both README action tables (EN and FR).
  4. `src/editeur/editeur.html`: `ACTIONS_SIMPLES` (`:249`), `CLE_ACTION` (`:250`), and a label in **both** the `fr:` (`:174`) and `en:` (`:200`) dictionaries. I can write the French label only with a translator.
  5. `BARRE_COMPLETE` (`config.rs`), used by the round-trip tests.
- **Verdict:** yes, I'd open the PR now. The policy is clear and English is welcome, so the earlier blocker is gone. But I would likely miss the editor and the French README glossary on the first push and learn about them from a failing test with a French message. A 6-line English checklist in CONTRIBUTING.md would fix that (Proposal 3).
- The PR template says "`cargo test` passes, `cargo build` has no warning". It should say `CI=1 cargo test` (without it my screen dims) and `cargo clippy --all-targets -- -D warnings` plus `cargo doc`, which CI enforces.

## Gaps

- **No 0.2 release.** That's the single biggest gap (B8). Everything below matters less until the download matches the README.
- **No English error output at all.** Even an opt-in (`TACTILE_LANG=en`, or `LANG` not starting with `fr`) that skips `traduire` would give English users serde's native, searchable messages. The editor already follows the Mac's language (`README.md:83`), so the CLI and log are the odd ones out.
- **No English CLI flags** (B13).
- **`--verifier` validates structure, not content** (B10). It could reuse whatever the editor uses to reject bad symbols and colors.
- **`Cargo.toml` metadata** is missing (description, repository, license, keywords, readme), and the version wasn't bumped.
- **No GIF.** The hero is still a static PNG plus an editor screenshot. For a Touch Bar app, a 3-second GIF of a tap opening the brightness slider sells it better than anything else.
- **Brew cask:** still none. The `xattr` one-liner at `README.md:53` is a reasonable stand-in.
- **`docs/ARCHITECTURE.md` and `DECISIONS.md` are French.** That's acceptable now that CONTRIBUTING has the overview, except for the action/element recipes (see Proposal 3).

## 3 concrete proposals

### 1. Ship v0.2.0 now, or label the README as unreleased

Bump `Cargo.toml` to `0.2.0`, move `[Unreleased]` to `[0.2.0]` in the CHANGELOG and tag it. `release.yml` already writes the notes and the SHA-256. If 0.2 isn't ready, add one line under the badges: "This README describes the upcoming 0.2. The current release (0.1.0) only supports French keys; see README at tag v0.1.0." Fix `README.md:31` either way. While in `Cargo.toml`, add `description`, `repository`, `license = "MIT"` and `keywords`. This fixes B8, and B1 for real users.

### 2. Make the English path honest in three small code changes

- `src/config.rs`: stop rewriting serde's message when the user isn't French. The smallest version is `if !lang_fr() { return message.to_string() }` at the top of `traduire`, with `lang_fr()` reading `LANG` / `TACTILE_LANG`. That removes the franglais (`invalid basic string, attendu`), and English users get searchable messages. The three tests that assert French prefixes keep passing under a French `LANG`.
- `src/config.rs:18` (and `:33`, and `Groupe.elements`): either make `[[element]]` + `[[item]]` coexist (deserialize both into two `Vec`s and concatenate them in `lire`), or keep the restriction and change `README.md:114` and `:164` to "mix English and French keys freely, but use either `[[element]]` or `[[item]]` throughout a file". The README change costs 2 lines; add a troubleshooting row for `duplicate field «element»` too.
- `src/main.rs:34-38`: accept `--check` as `--verifier` and `--customize` as `--personnaliser`, and print the English help when `LANG` isn't French (or always print both columns). Add the two cases to `chaque_argument`.

### 3. An English "Add an action" checklist in CONTRIBUTING.md, and test the English glossary

Paste the five-step recipe from `ARCHITECTURE.md:61-67` into CONTRIBUTING.md in English, as a checklist with the exact file and symbol names: `Action` + alias, function + test in `src/actions/`, `executer`, `chaque_forme_d_action` + `TOUT_FR`/`TOUT_EN` + `BARRE_COMPLETE`, both READMEs' tables and glossaries, `editeur.html` (`ACTIONS_SIMPLES`, `CLE_ACTION`, `fr`/`en` labels; "leave the French label to the maintainer if you don't speak French"). Do the same for "Add an element type". Then extend `glossaire_du_readme` to also read `README.md`'s `### English aliases` table, so the canonical README can't drift (B14). Fix the PR template to say `CI=1 cargo test` and clippy `-D warnings`. With that, an English contributor's first push would pass CI.

---

Not checked: runtime behavior on the Touch Bar, the editor window (`--personnaliser` opens a GUI; I skipped it), `scripts/verifier.sh`, and the zip's contents. B8's claim about the v0.1.0 binary's behavior with English keys is inferred from git ancestry, not from running the release build. The CI run for `f6b9cdb` was still in progress when I checked; the two previous runs on `main` succeeded.
