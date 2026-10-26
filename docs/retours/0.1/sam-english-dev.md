# Sam, English-speaking Rust dev: can I use and contribute to Tactile?

## Context

- Who: Sam, 27, Rust developer, Austin TX. No French at all. Found `lepalph/tactile-touch-bar` on GitHub while searching for an open-source Touch Bar customizer (MTMR alternative).
- What I did: read the GitHub landing page and metadata (`gh repo view`), read the whole repo, ran `CI=1 mise exec -- cargo test`, `cargo clippy --all-targets`, `cargo doc --no-deps`. Did not run the binary or `scripts/verifier.sh` (hardware busy), did not modify any file except this report.
- Goal: (a) understand what it does from the landing page, (b) write a config, (c) decide whether to contribute a feature.
- Repo state: commit `ce6444b`, release `v0.1.0` (2026-10-25), 0 stars, MIT, 1 746 lines of Rust.

## What worked

- **Build and tests.** `CI=1 mise exec -- cargo test`: 17 tests, **15 passed, 0 failed, 2 ignored** (the two ignored ones drive real media playback). 1.6 s compile, 1.0 s run. `cargo clippy --all-targets`: **0 warnings**. `cargo doc --no-deps`: builds, **1 warning** (see Bugs). Pleasant surprise: a Rust project of this size with a clean clippy and a `mise.toml` is a good sign.
- **Code size and shape.** 1 746 lines, 6 directories, one obvious entry point (`src/main.rs`, 39 lines). Even with French names, the module layout is guessable: `actions/` (system actions), `barre/` (Touch Bar layer), `config.rs`, `menu.rs`. The `ARCHITECTURE.md` call graph (lines 7-22) is readable without French because it is mostly identifiers and arrows.
- **Config format.** Once I had the key translation line from the README (`README.md:162`), I could write a working bar in ten minutes by copying `exemples/barre.toml` and swapping values. TOML + `[[element]]` + `type = "..."` is a familiar shape (same as MTMR's JSON array of items). `#[serde(deny_unknown_fields)]` means a typo is caught, not silently ignored (`config.rs:13`, `:22`).
- **Tests as documentation.** `config::tests::chaque_forme_d_action` (`config.rs:234-263`) is the only complete list of accepted action spellings, and it is machine-readable. I learned more from it than from the English README.
- **Adding an action looks like a 4-step recipe** (`ARCHITECTURE.md`, "Recettes" → "Ajouter une action"): enum variant, function + test, `executer` branch, README table. I decoded it with the identifiers alone. A contributor can do this without French.
- **GitHub metadata is partly there.** MIT license detected by GitHub, topics set (`touch-bar`, `touchbar`, `macos`, `rust`, `macbook-pro`, `customization`, `bettertouchtool`), issues enabled, a release with a `Tactile.zip` asset, CI badge-able workflows (`ci.yml`, `release.yml`) with pinned action SHAs.
- **Rust-side hygiene.** No unsafe-without-comment surprises: `ARCHITECTURE.md` lists every private API and where it is called, and `DECISIONS.md` §2 explains why raw `msg_send!` is used. Even reading it through a translator, that section earned my trust.

## Bugs

Severity scale: **High** = blocks an English user or contributor; **Medium** = costs them real time or sends them the wrong way; **Low** = polish.

### B1. `tactile --help` / `--version` are not handled; any argument launches the app and takes over the Touch Bar

- Steps: a dev downloads the zip, runs `Tactile.app/Contents/MacOS/tactile --help` to see what it does (standard reflex for a CLI-shaped binary).
- Expected: usage text, config path, env vars (`TACTILE_OUVRIR`, `TACTILE_MASQUER`, `TACTILE_RETOUR`, `TACTILE_AUTOTEST`), exit 0.
- Actual (inferred from code, not run): `src/main.rs` never reads `std::env::args()`; the app starts, replaces the Touch Bar, and puts an icon in the menu bar. The four env vars exist only in French docs (`CONTRIBUTING.md:33-35`, `ARCHITECTURE.md` "Vérifier un changement").
- Severity: **Medium**. For a GUI-only app it is tolerable, but this binary is explicitly driven from the terminal by contributors.

### B2. English README describes menu items that do not exist as written

- Steps: read `README.md:160` ("The menu bar icon offers: show my bar, Apple's bar, open the configuration, launch at login, quit"), then open the actual menu.
- Expected: those labels.
- Actual: the menu is French (`src/menu.rs:134-145`: "Afficher ma barre", "Barre d'Apple", "Ouvrir la configuration", "Lancer au démarrage", "Quitter Tactile"). The English text is a loose translation, not the labels. `README.md:162` does give the one French label *Ouvrir la configuration*, so the reader has to guess the other four.
- Severity: **Medium**. Five items, so you get there by elimination, but "launch at login" vs "Lancer au démarrage" is not obvious to someone with zero French, and the README promises something the UI does not show.

### B3. Config error messages are *translated from English into French* before reaching the user

- Steps: write `type = "button"` (the natural English spelling) in `barre.toml`, save, open `~/Library/Logs/Tactile.log`.
- Expected: serde/toml's native message, e.g. `unknown variant `button`, expected one of `bouton`, `slider`, ...`.
- Actual: `ligne 2 : valeur inconnue «button», attendu parmi «bouton», «slider», ...`. `src/config.rs:139-166` (`fn traduire`) takes the English message serde produces and rewrites it in French, token by token (`"unknown variant" → "valeur inconnue"`, `"expected one of" → "attendu parmi"`, backticks → « »). The app spends 27 lines actively removing the only English a non-French user would have gotten for free.
- Severity: **High** for an English user: the one moment they need help (a broken config) is the one moment the app refuses to speak the language of the error source. The list of valid values is still readable (they are the TOML keys), which is the only thing that saves it.

### B4. Hardware tests report `ok` under `CI=1` instead of being skipped

- Steps: `CI=1 cargo test`.
- Expected: `actions::luminosite::tests::regler_puis_relire_et_restaurer ... ignored` (or a visible skip).
- Actual: `... ok`. `src/actions/luminosite.rs:60-62` and `src/actions/volume.rs:105` `return` early when `CI` is set, so the test passes without asserting anything. A CI green on GitHub Actions (`ci.yml` runs `cargo test` on `macos-14`, where `CI=true` is always set) therefore never exercises brightness or volume, but looks like it does.
- Severity: **Low** (correctness of the signal, not of the code). Rust-idiomatic fix: `#[cfg_attr(..)]` is awkward for env vars, but printing `skipped: no display on CI` or using a custom `#[ignore = "..."]` plus a `--include-ignored` local alias would make the skip visible.

### B5. `cargo doc` warning: broken intra-doc link

- Steps: `cargo doc --no-deps`.
- Expected: 0 warnings (CONTRIBUTING.md:11 says "`cargo build` sans warning"; a contributor extends that to doc).
- Actual: `warning: unresolved link to 'element'` at `src/config.rs:83` because the doc comment contains the literal TOML `[[element]]`, which rustdoc reads as a link. Fix: `\[\[element\]\]` or wrap in backticks.
- Severity: **Low**.

### B6. GitHub repo description is French and names a different product

- Steps: Google "open source touch bar rust", land on the repo card.
- Expected: one English line saying what it is ("Customizable Touch Bar for MacBook Pro, in Rust").
- Actual: `"Réécriture Rust native de BetterTouchTool, en commençant par la Touch Bar"` (from `gh repo view --json description`). An English visitor sees "BetterTouchTool" and "Rust" and nothing else they can parse. The repo name `tactile-touch-bar` carries the meaning; the description undoes it.
- Severity: **Medium** (first impression, zero cost to fix).

### B7. Release `v0.1.0` has an empty body

- Steps: open <https://github.com/lepalph/tactile-touch-bar/releases/tag/v0.1.0>.
- Expected: what is in it, supported Macs, the right-click-to-open warning, SHA-256 of the zip.
- Actual: title `v0.1.0`, author `github-actions[bot]`, body empty, one asset `Tactile.zip`. `release.yml` uses `softprops/action-gh-release` without `body_path` or `generate_release_notes: true`.
- Severity: **Low**, but it is the page the Install section links to (`README.md:158`), so an English user lands on nothing.

## Friction for English readers (quoted lines)

The English part is **19 lines out of 168** (`README.md:150-168`) and is a summary, not a translation. Every time it says "see X", X is French.

| Where I was stranded | Line | What it says / what is missing |
|---|---|---|
| Landing | `README.md:1-148` | The first 148 lines (title, hero, screenshot, every table) are French. The English reader scrolls past 88 % of the page to find a `## English` heading at the bottom, after `## Licence`. Nothing at the top says "English below". |
| Screenshot | `README.md:5` | The only screenshot (`docs/barre.png`) is in the French part with a French alt text; the English section has no image. |
| Install | `README.md:158` | "open it the first time with right-click → Open (the app is not notarized)". Fine, but the French has a 4-step list plus the two optional permissions (`:24-27`). The English omits the **Accessibility permission needed for `echap`**; an English user who adds an esc button gets nothing and the troubleshooting table that explains it (`:103`) is French. |
| Menu | `README.md:160` | "show my bar, Apple's bar, open the configuration, launch at login, quit" vs the real French labels (see B2). |
| Config keys | `README.md:162` | "Keys are French: `bouton` = button, `texte` = text, …" — one run-on line. It lists 11 words. Missing: `couleur` format, `largeur`, `reglage`, `commande`, `intervalle`, `format`, `masquer_control_strip`, and **every action word** (`veille`, `echap`, `suivant`, `precedent`, `lecture_pause`, `basculer_muet`, `luminosite_plus/moins`, `volume_plus/moins`, `barre_apple`). The actions table (`:78-89`) and the elements table (`:61-70`) are French only. I had to open `src/actions/mod.rs:15-30` to learn which actions exist. |
| Element fields | `README.md:63-70` | Required vs optional (`commande` obligatoire, `intervalle` obligatoire, `espace.largeur` obligatoire), defaults (`"HH:mm"`, 120 pt, 44 chars, 2 s refresh) exist only in the French table. I got them from `config.rs:23-58` by reading which fields are `Option`. |
| Error output | `README.md:162` | "Errors are written to `~/Library/Logs/Tactile.log`" — true, but they are French (B3), and the README does not warn about it. |
| Contribute | `README.md:164` | "Start with docs/ARCHITECTURE.md … and docs/DECISIONS.md" — both 100 % French. CONTRIBUTING.md is not even linked from the English section, and it says (`CONTRIBUTING.md:7`) "Noms en français (fonctions, variables, types, messages)" and (`:10`) "Tout ce que lit l'utilisateur est en français", i.e. **the project policy is that contributions must be in French**. An English contributor cannot find that rule in English and will write `fn set_brightness` and have the PR bounced. |
| Build commands | `README.md:164` | Mentions `cargo build --release` and `scripts/empaqueter.sh`. Not mentioned: `cargo test` **really changes brightness and volume** (French warning at `:127`), and the `CI=1` escape hatch is only in `ARCHITECTURE.md` (French). An English dev runs `cargo test` and watches the screen dim. |
| Env vars for screenshots | `CONTRIBUTING.md:31-35` | `TACTILE_OUVRIR`, `TACTILE_MASQUER`, `TACTILE_RETOUR` only here, French. |
| Compatibility | `README.md:156` | "Apple Silicon" without the French precision "M1 et M2, 13 pouces" (`:13`). An M3-era reader (no Touch Bar) is not the issue; an M1 Pro 14" owner is, and the English says nothing. |
| Troubleshooting, Uninstall, Known limits (full) | `README.md:95-113`, `:132-138` | Absent from English except one line (`:166`). "Ma barre a disparu" → "touch the ✳ icon" is the single most important recovery tip and it is French only. |
| Extras | `extras/claude-voix/README.md`, `exemples/claude-vocal.toml` | French, including the shell comments. The English section doesn't mention the Claude voice extra at all. |
| Commit convention | `CONTRIBUTING.md:39` | "Conventional Commits, en français: `feat: groupe repliable`". An English contributor's `feat: collapsible group` violates it without knowing. |

**Can I write a config without reading French?** Yes, barely, by copying `exemples/barre.toml` and using `config.rs` + `actions/mod.rs` as the reference. The 11-word glossary at `:162` covers element types but not actions or fields. A real user who is not a Rust dev cannot do this: they would need the actions table, and it is French.

**How big a barrier are French TOML keys?** Smaller than it looks for *reading* (the words are short, many are cognates: `volume`, `slider`, `format`, `script`, `action`, `shell`, `type`), bigger for *writing*: I cannot guess `souple` (flexible space), `en_cours` (now playing), `appui_long` (long press), `veille` (sleep), `echap` (esc), `basculer_muet` (toggle mute), `precedent` without an accent. I also mistyped `luminosité` with the accent once; `deny_unknown_fields` caught it (good) but the error was French (B3). The barrier is a **lookup table away** from being a non-issue, and the table does not exist in English.

**Error messages in French: impact?** This is the worst spot. Correct config → fine. Broken config → the log says `ligne 4 : valeur inconnue «danser», attendu parmi …`. The *valid values* are still the French keys, so an English reader can act on it, but the sentence structure (`attendu parmi`) must be guessed. Runtime errors (`"luminosité : impossible de charger DisplayServices"`, `"volume : aucune sortie audio par défaut"`, `src/actions/*.rs`) are completely opaque. Since these only go to a log file, nobody will file a good bug report from them.

**Contributor friendliness.** Identifiers: ~100 French function names (`declencher`, `depliable`, `rafraichir`, `enregistrer`, `plateau`, `souple`). For a Rust dev this costs a one-time 30-minute glossary pass, then it's fine: the code is small, typed, clippy-clean, and the structure is conventional. The actual blockers are: (1) `CONTRIBUTING.md` mandates French names and French commit messages, in French; (2) the two docs the README sends me to are French; (3) I cannot verify a UI change because `scripts/verifier.sh` and the `TACTILE_*` env vars are documented in French only. Comments are rare (55 `//` lines in 1 746) and French; that is the least of the problems.

**Minimum to make an English contributor productive:** an English `CONTRIBUTING.md` (even 30 lines) that states the naming policy up front, an English `ARCHITECTURE.md` (the "Où est quoi" table and the two recipes are 60 % of the value and are mostly identifiers already), a one-screen FR→EN identifier glossary, and `cargo test` instructions with the `CI=1` warning. Nothing else needs translating for a PR to land.

## Gaps

Compared with how MTMR, Rectangle and Maccy present themselves (from memory of their READMEs; not fetched during this session):

| Area | Tactile v0.1.0 | What the well-presented tools do |
|---|---|---|
| GitHub description | French, mentions a competitor (B6) | MTMR: "My Touchbar. My rules" + subtitle; Rectangle: "Move and resize windows on macOS with keyboard shortcuts and snap areas"; Maccy: "Lightweight clipboard manager for macOS". One English sentence, product first. |
| README language | French with English footer | All three: English, period. Localised READMEs are separate files linked at the top (Maccy's docs are English; Rectangle keeps translations in the app, not the README). |
| Hero image | One PNG of the bar (French part only) | MTMR: GIF of the bar changing; Rectangle: GIF of window snapping; Maccy: animated GIF in the first screen. A Touch Bar app lives or dies on a 3-second GIF of a finger tapping. |
| Badges | None | All three: build status, latest release, downloads, Homebrew, license, (Maccy) sponsors. Tactile already has `ci.yml` and `release.yml`, so build + release badges are free. |
| Install | Zip + right-click dance | All three: `brew install --cask <name>` first. A cask for a non-notarized app is possible (cask with `no_quarantine`-style instructions) or at least the `xattr -d com.apple.quarantine` one-liner. Rectangle also explains *why* the warning appears. |
| Config reference | Two French tables | MTMR: a full "Preset format" section with every item type, every key, every button type, and a link to community presets. For a config-driven tool, the key table *is* the product documentation. |
| Example gallery | One `exemples/barre.toml` | MTMR: `Presets/` folder with a dozen files + screenshots, plus a wiki. Tactile's `exemples/` name itself is French (`examples/` would be found by convention). |
| Release notes | Empty body | Rectangle/Maccy: bullet list per release, generated from PR titles (`generate_release_notes: true`) plus a hand-written summary. |
| Issue templates | None (`.github/` contains only `workflows/`) | All three: bug report (with macOS version, Mac model, logs) and feature request. For Tactile the bug template should ask: Mac model (Intel/M1/M2), macOS version, contents of `~/Library/Logs/Tactile.log`, the `barre.toml`. |
| `--help` | None (B1) | Not universal for GUI apps, but MTMR documents its CLI-ish flags; Rectangle documents `defaults write` toggles in a "Terminal commands" section. Tactile's `TACTILE_*` env vars deserve the same. |
| FAQ / Troubleshooting | French table | Rectangle and Maccy both have an English FAQ in the README or wiki. |
| Code of conduct / CONTRIBUTING | French | Maccy and Rectangle: English CONTRIBUTING, Rectangle has a specific "Contributing translations" section (relevant here: Tactile could accept FR and EN). |
| Code identifiers | French | All three: English identifiers. This is the norm GitHub-wide; Tactile's choice is defensible (DECISIONS.md §5) but is unusual enough that it must be announced in English on the first screen of CONTRIBUTING. |
| Security | No `SECURITY.md`; shell actions run arbitrary `zsh -c` from a user file | Fine for v0.1; note that `{ shell = ... }` plus hot reload means anyone who can write `~/.config/tactile/barre.toml` gets code execution under the user. Worth one line in the README, like MTMR's note on AppleScript. |
| Checksums / notarization | ad hoc signature, no SHA-256 | Rectangle and Maccy are notarized (paid dev account). Without it, publishing a SHA-256 in the release body is the cheap trust signal. |

Smaller gaps:

- No `CHANGELOG.md`.
- `exemples/`, `scripts/empaqueter.sh`, `scripts/verifier.sh`: French paths in a repo whose GitHub name is English. Renaming later breaks links; now is the cheapest time.
- `Cargo.toml` has no `description`, `repository`, `license`, `readme`, `keywords` fields (checked: 236 bytes). `cargo doc` and any future `cargo install` surface these in English automatically.
- `PLAN.md`, `SPEC.md`, `docs/CHANTIER-0.2.md` are internal French planning docs sitting at the top level next to the README; an English visitor cannot tell them from documentation.
- `graphify-out/` is in the working tree (gitignored, fine) but `.gitignore` is 31 bytes; make sure `target/` and `graphify-out/` stay out of the zip.

## 3 concrete proposals

### Proposal 1: Flip the README. English `README.md`, French `README.fr.md`, linked both ways at line 1

GitHub renders `README.md`. The audience that finds `lepalph/tactile-touch-bar` through search is overwhelmingly English-reading; French readers are the existing audience and will follow one link. Proposed structures (section order matters: install → see it → configure → fix it → contribute):

**`README.md` (English, canonical)**

```
# Tactile
> 🇫🇷 Version française : README.fr.md

[badges: CI · latest release · downloads · macOS 13+ · MIT]

One-line pitch. Hero GIF (bar switching, slider opening, tap on ✳ to return to Apple's bar).
"At rest: 0.5 % CPU, 13 MB. Rust, universal binary, MIT, no account, no telemetry."

## Compatibility            (table: Mac models incl. "M1/M2 13-inch", macOS 13+, what was actually tested)
## Install                  (1. download 2. drag 3. right-click → Open, with *why*; optional: media-control, Accessibility for esc)
## The menu                 (table with the exact labels as they appear; if the UI stays French, show "Afficher ma barre (Show my bar)")
## Your first bar           (the 7-line "sleep button" example, then "save, the bar updates within a second")
## Configuration reference
   ### Top-level keys        (masquer_control_strip)
   ### Element types         (full table: type · fields · required/default · what it shows)
   ### Actions               (full table, both spellings: word and table form)
   ### Key glossary FR → EN  (one table, every key and action word, see Proposal 2)
   ### Error messages        (where the log is, what "ligne N : …" means, the 6 message shapes with English meaning)
## Examples                 (link to examples/ with a screenshot per file)
## Troubleshooting          (the 5-row table, English)
## Uninstall
## Known limitations
## Contributing             (3 lines: naming policy stated plainly, link to CONTRIBUTING.md, build/test commands incl. CI=1 warning)
## Credits & license        (MTMR, media-control, MIT)
```

**`README.fr.md` (French)**: exactly today's `README.md:1-146`, with line 1 gaining `> 🇬🇧 English: README.md` and the bottom `## English` block removed. Zero new writing; the French content already exists and is better than the English summary. Keep both files structurally identical (same headings, same order) so a diff between them is reviewable, and add a one-line CI check that both have the same `##` headings.

Same treatment, smaller: `CONTRIBUTING.md` in English with the naming/commit policy in the first paragraph, `CONTRIBUTING.fr.md` as today's file. `docs/ARCHITECTURE.md` in English (it is 60 % identifiers and tables already; the French original moves to `ARCHITECTURE.fr.md`). `DECISIONS.md` can stay French longer; it is the least needed to land a PR, but add an English §5 so the key-language decision is visible to the people it affects.

Also, in the same PR: GitHub description → "Customizable Touch Bar for MacBook Pro, written in Rust. Buttons, sliders, now playing, clock, your own commands." Release body for v0.1.0 edited by hand (2 lines + SHA-256). `exemples/` → `examples/`. `Cargo.toml` gets `description`, `repository`, `license = "MIT"`, `readme`, `keywords`.

### Proposal 2: Keep French keys canonical, add English aliases via serde, and ship the glossary table

Three options were on the table:

- *English aliases on top of French canonical.* Cheapest, non-breaking, and `serde` does it for free: `#[serde(alias = "button")]` on `Element::Bouton`, `#[serde(alias = "long_press")]` on `appui_long`, `#[serde(alias = "sleep")]` on `Action::Veille`, etc. One attribute per key, roughly 35 lines total. Errors still list the canonical French names, so the glossary table is still needed, but an English user can *write* a config in English and it just works. Existing French configs untouched.
- *English canonical with French aliases.* Cleaner for the GitHub audience, but it contradicts `DECISIONS.md` §5 and `CONTRIBUTING.md:7`, breaks `exemples/barre.toml` embedded in the binary, every screenshot, every doc, and the error messages' "expected one of" list. Not a v0.2 change.
- *A documented key table only.* Necessary but not sufficient: the user still has to type `souple` and `en_cours`.

**Recommendation: aliases (first option) plus the table, in that order of priority.** Concretely:

1. `src/config.rs`: `#[serde(alias = "...")]` on each `Element` variant, each field, each `Reglage` variant; `src/actions/mod.rs`: same on each `Action` variant. Suggested English spellings: `button`, `clock`, `now_playing`, `space`, `flex`, `group`, `text`, `symbol`, `color`, `width`, `long_press`, `setting`, `brightness`, `command`, `interval`, `hide_control_strip`, `brightness_up/down`, `volume_up/down`, `toggle_mute`, `play_pause`, `next`, `previous`, `sleep`, `esc`, `apple_bar`.
2. One test: the existing `chaque_forme_d_action` loop gets a second column with the English spelling, and a new `examples/bar.en.toml` (English keys) must parse to the same `Barre` as `exemples/barre.toml`. That test *is* the glossary's proof of completeness.
3. The README "Key glossary" table is generated from, or at least checked against, that test, so it cannot drift.
4. Error messages: stop translating serde's output (`fn traduire`, `config.rs:139-166`). Either leave serde's English as is (the French-speaking user still sees the French key names in the list, which is what matters) or make the language follow `LANG`/`AppleLanguages`. Deleting `traduire` is minus 27 lines and fixes B3; the three tests that assert French strings (`type_inconnu`, `action_inconnue`, `champ_mal_orthographie`) change their expected prefix. If keeping French is a hard rule, at minimum append the untranslated message after the French one so both audiences can grep.

Cost: an afternoon. Risk: none for existing users. Side effect: the "French-only project" policy becomes "French-first, English-friendly", which is the honest state of a public repo.

### Proposal 3: Make the English contributor path self-contained in one PR-sized change

The goal is that a Rust dev with zero French can clone, build, test, change, verify, and open a PR that will not be bounced for policy reasons. Everything below is small:

1. **`CONTRIBUTING.md` in English, first screen** = the rules that will get a PR rejected: "Identifiers, comments and user-facing strings are French (see DECISIONS.md §5). If you don't speak French, write English and say so in the PR; maintainers will rename. Commit messages: Conventional Commits, French preferred, English accepted." Then the test-first rule, `cargo test` with the explicit warning that it drives real brightness/volume and that `CI=1` skips those, `scripts/verifier.sh` with the "screen on, touch the trackpad" caveat, and the screenshot command.
2. **A `--help`** in `src/main.rs`: if `args().nth(1)` is `--help`/`-h`/`--version`, print 15 lines (config path, log path, the four `TACTILE_*` env vars, link) and exit before `journal_si_sortie_perdue()`. Ten lines of code, no dependency (`std` only; no `clap` for two flags). Fixes B1 and documents the env vars in the binary itself, which is language-independent.
3. **`docs/GLOSSARY.md`** (or a section in ARCHITECTURE.md): FR → EN for the ~100 identifiers and the 10 module/file names. Generated once with `grep -rhoE 'fn [a-z_]+' src/ | sort -u`, translated by hand. A contributor reads it once; it costs the maintainer an hour.
4. **Issue templates** (`.github/ISSUE_TEMPLATE/bug_report.yml`, `feature_request.yml`) in English: Mac model, macOS version, Tactile version, `barre.toml`, last 20 lines of `~/Library/Logs/Tactile.log`. Templates that collect the log are the only way French runtime error strings become triageable from English bug reports.
5. **Release notes**: `generate_release_notes: true` in `release.yml` and `sha256sum target/Tactile.zip` appended to the body. Fixes B7.
6. **Fix the two signals**: rustdoc link (B5, one backtick pair), and make the `CI` skip visible (B4): `eprintln!("skipped: hardware test, CI set"); return;` is enough, or convert to `#[ignore]` and run `cargo test -- --include-ignored` locally. Add `cargo clippy --all-targets -- -D warnings` and `cargo doc --no-deps` to `ci.yml` so the current clean state is enforced; it is clean today, so this costs nothing.

**Would I contribute?** Yes, with Proposal 1 and item 1 of Proposal 3 in place. The feature I would pick: English aliases (Proposal 2) because it is self-contained in two files, has an existing test to extend, and unblocks every other English user. Without an English CONTRIBUTING, I would not open the PR, because I cannot tell whether French identifiers are a rule or a habit, and `CONTRIBUTING.md:7` says it is a rule in a language I cannot read.

---

Not checked: the actual Touch Bar behaviour, `scripts/verifier.sh`, `Tactile.zip` contents, and whether `--help` really launches the app (inferred from `src/main.rs` having no argv handling). MTMR/Rectangle/Maccy comparisons are from memory of their READMEs, not re-fetched.
