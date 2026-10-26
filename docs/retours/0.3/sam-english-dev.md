# Sam, English-speaking Rust dev: Tactile 0.3 (main) review

## Context

- Who: Sam, 27, Rust developer in Austin, TX. I don't speak French. I reviewed 0.1 and 0.2 (`docs/retours/0.2/sam-english-dev.md`).
- What I looked at: `main` (from `9e559e6` when I started to `6c58054` at the end; other commits landed while I worked). The public repo `lepalph/tactile-touch-bar`, latest release `v0.2.0` (2026-10-26, `Tactile.zip`, 1.87 MB), good first issues #1 to #7, Discussions on.
- What I did: rendered the README with `gh api markdown -f mode=gfm` and looked at `docs/hero.png` and both GIFs (contact sheets of the frames). I followed CONTRIBUTING's "Your first contribution in 10 minutes" on a fresh `git clone` and timed each step. I implemented good first issue #2 in a throwaway worktree (`sam-essai`, removed afterwards, nothing pushed). I wrote a 150-line English config from the README alone and probed 10 edge cases with `--check`. I didn't run the bar or `scripts/verifier.sh`, and I left the user's Tactile.app alone.
- Scratch files are in `/private/tmp/claude-501/-Users-palphinho/03bc99cc-4742-4072-87c4-fe525a6d77ae/scratchpad/sam3/`: `sam.toml` (my config), `p1.toml` to `p10.toml` (probes), `issue2.diff`, `readme.html`.

Build and test results on `main` at `6c58054`:

| Command | Result |
|---|---|
| `CI=1 mise exec -- cargo test` | **117 passed, 0 failed, 18 ignored**, 10.6 s for the tests (52/0/2 in 0.2) |
| `cargo clippy --all-targets -- -D warnings` | **clean** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | **clean** |
| Fresh clone at `9e559e6`: cold `cargo test` | 15 s build + 10 s tests, 93/0/17 |
| Fresh clone: `cargo build --release` | 23 s |

## First impression (30 seconds)

**Do I get it? Yes, within 10 seconds.** The tagline ("A free, open-source Touch Bar for your MacBook Pro, described in a small text file or set up in a window") plus `hero.png` does the job. The hero shows a real-looking bar (esc, a drawer, Clair de Lune, 13.1 °C, battery, clock) above the English editor, with the drawer selected and four app launchers listed under it. "Highlights" is six bullets I can skim, and "A bar in ten lines" right after it is exactly what a terminal person wants to see: a TOML snippet, in English.

**Do I want to try it? Yes, and then I'd be disappointed.** Everything that sold me in the first screen (the app drawer, the weather from a data source, `launcher`, `[[source]]`, MCP, the import/export buttons in the hero) is **not in the download**. I only found that out by reading the Roadmap table ("Done (unreleased)") and then comparing the `v0.2.0` tag with `main`: `git show v0.2.0:src/config.rs` has no `lanceur` and no `Source`, and `main.rs` has no `--export`. The Highlights, the hero, both GIFs, and the "Install → latest release" steps say nothing about it. I would install 0.2.0, paste the drawer snippet from `README.md:195-213`, and get `valeur inconnue «launcher»`.

What works well:
- The hero is crisp at 1600×1267 and has descriptive alt text, and it's all English. The two-GIF table (`README.md:33-35`) shows that the editor isn't a toy.
- The badges, roadmap and "Why Tactile" sections are where I expect them. Comparing it with BetterTouchTool/MTMR at `:74-80` is honest and concrete (0.0-0.2 % CPU, 14 MB).
- `README.md:70` sends me to the good first issues, and I like it.

Small things:
- The GIF frames show the logo copied to `~/Library/Caches/tactile-demos/icones/…`. That's the recording rig's path, while the README says logos go to `~/.config/tactile/icones/` (`:134`). Someone reading closely will wonder which is right.
- Both GIFs show "— °C" and "Title · Artist". It's a placeholder bar, so it looks less alive than the hero.
- In the first frames of `demo-editeur.gif` the symbol grid is almost empty (3 glyphs) before the search runs. It reads as a loading glitch.

## First contribution walkthrough (timed)

I followed `CONTRIBUTING.md:9-35` literally, on a fresh clone.

| Step | Wall time | What happened |
|---|---|---|
| 1. `git clone` + `cd tactile-touch-bar` + `mise install` | 2 s (mise and Rust already installed) | Fine. Note that the clone folder is `tactile-touch-bar`, while `exemples/mcp.toml` hard-codes `~/code/tactile/exemples/mcp/horloge-mcp.py`. |
| 2. `CI=1 cargo test` | **failed: `cargo not found`**, then 26 s with `mise exec --` | `mise install` installs Rust but does not put `cargo` on PATH unless mise is activated in your shell. CONTRIBUTING never says `mise exec -- cargo …` or "activate mise". Once that was sorted: 15 s build, then **10 s** of tests, not "about a second" (`:19`). The slow ones are `sources::mcp::*` and `sources::http::*` (restart backoff, timeouts). |
| 3. `cargo build --release`, editor on a scratch file, `--check` | 23 s build. `--check exemples/simple.toml` → `OK : 6 éléments` | The block at `:27-31` runs the editor **in the foreground**, so the `--check` line under it only runs after you close the window. It uses `--personnaliser` while the README uses `--customize`, and `/tmp/x.toml` doesn't exist yet. The editor started and stayed up (2 windows, according to CoreGraphics). Launched from a background shell, though, it didn't come to the front, and I couldn't see it on the current Space. |
| 4. Pick a good first issue | 3 min of reading | **Excellent.** Each issue has Context / Where to look / How to test / Done when, and names functions and tests. This is the best part of the onboarding. |
| 5. Branch, test, clippy | see the issue below | Clear. |

**Time to a first contribution:** about 10 minutes of tool time is realistic *if* your shell already activates mise. For an English speaker starting from zero, I'd say **20 to 30 minutes** in total, including the issue. That's mostly reading time, not friction.

Where I got lost:
1. `cargo not found` right after `mise install` (step 2). That's the only hard stop.
2. "The tests run in about a second" is off by 10×, and the 18 `ignored` reasons are in French (`pas de session graphique sur un runner de CI`, `pilote la vraie lecture`…), so a newcomer can't tell whether a skip is expected.
3. In step 3 the two commands look sequential but block each other.

(Environment note, not a project bug: my scratch directory had a stray `Cargo.toml` in a parent folder, so I had to append `[workspace]` to the clone's `Cargo.toml` before cargo would build. I don't count it against the project.)

## Issue attempt: #2 "Unknown modifier error: list every accepted modifier"

- **Time:** a human contributor would need about 15-20 minutes: 5 to read the issue and `clavier.rs`, 5 to write the code, and 5 for the full test suite and clippy. The issue told me exactly where to go: `MODIFICATEURS` at `src/actions/clavier.rs:34`, the error at `:107`, the test at `:247`.
- **Process (TDD, as CONTRIBUTING asks):** I changed the expected string in `touche_inconnue_dit_la_forme_attendue` first and saw it fail (`left: "... cmd, shift, ctrl, alt, fn"`). Then I added `fn modificateurs_attendus()`, which groups `MODIFICATEURS` by flag, so the first name is canonical and the later ones become "(aliases)". The diff is 17 lines (`issue2.diff`).
- **Result:** `cargo test clavier` passed 7/7, the full suite 94/0/18, clippy was clean. End to end: `action = { key = "opt+a" }` now gives `ligne 1 : modificateur inconnu « opt » ; attendu : cmd (command), shift (maj), ctrl (control), alt (option), fn`.
- **Blockers:** none. The French identifiers were not a problem here: `MODIFICATEURS`, `drapeaux`, `mots` and `attendu` are guessable, and the issue spelled them out. I named my function in French to match the style. The doc comments are French but short. If I'd had to read `config::Controle` for #5, it would have been slower.
- **What the issue doesn't mention:** `README.md:229` and `README.fr.md` list modifiers as "`cmd`, `shift`, `ctrl`, `alt` (or `option`), `fn`", which leaves out `command`, `control` and `maj`. "Done when" should say whether the README changes too. Also, the message itself stays French ("modificateur inconnu", "attendu"), so an English user still needs the Troubleshooting table, and issue #7 doesn't list this message yet.
- **Contributor-friendliness score:** 8/10 for this kind of issue. Small, well-scoped, test next to the code, with fast feedback (`cargo test clavier` in 0.07 s).

## English config from the README alone

`sam.toml` (150 lines) uses only English keys: three `[[source]]` (http with `every`, http with `headers` + `trousseau:`, mcp with `command`/`tool`/`arguments`), a launcher drawer (`group` with 3 `launcher`, `display`, `long_press = "app_windows"`), a system-actions group (`mission_control`, `spotlight`, `launchpad`, `screenshot_area`, `screenshot_options`, `lock_screen`, `force_quit`, `emoji`, `show_desktop`, `screenshot`), two `text` elements with `template`, `symbol_by` and `color_by`, a button with a `source`, `battery`, `clock`, and two `[[application]]` bars (Safari with `inherit = true` and a nested group, Terminal with a `text` element).

```
$ tactile --check sam.toml
attention : ligne 39 : app « Visual Studio Code » introuvable : le lanceur affichera son nom en texte ; écris le nom du dossier .app sans l'extension, ou son bundle id
OK : 10 éléments, 2 barres d'application
```

It worked the first time, and the warning is right: VS Code isn't installed on this Mac. Two 0.2 issues are now fixed: mixing `[[element]]` and `[[item]]` works (`p4.toml` → OK), and `--check` now catches unknown symbols, bad colors, missing apps and too-short intervals. Also "1 élément" is now singular.

### README errors

| Line | Quote | Problem |
|---|---|---|
| `README.md:22`, `:31`, `:35` | hero, highlights ("A `launcher` shows an app's real icon…", "A `[[source]]` feeds live text…"), GIFs | All show features that are **not in the latest release** (v0.2.0). Only `:60-63` say "(unreleased)". Add "(unreleased, build from source)" next to them, or ship 0.3. See B1. |
| `README.md:77` | "The v0.2.0 download is a 2 MB zip." | True (1.87 MB), but next to everything else it suggests v0.2.0 is what the README describes. |
| `README.md:94` | "Download `Tactile.zip` from the latest release" | Gets you v0.2.0: no `launcher`, no `[[source]]`, no `text` element, no `--export`/`--import` (`:457-458`). |
| `README.md:229` | "Modifiers: `cmd`, `shift`, `ctrl`, `alt` (or `option`), `fn`." | Leaves out `command`, `control` and `maj`, which are accepted (`clavier.rs:34-44`). The runtime error drops `option` (issue #2). |
| `README.md:412` | "Anything that isn't JSON is read with `{texte}`." | Since the glossary maps `texte` → `text` everywhere else, an English user writes `{text}`. That isn't aliased (`sources/mod.rs:90` only produces `texte`), it shows "—" at runtime, and `--check` says OK (`p1.toml`). See B2. |
| `README.md:415` | "its `texte` / `text` becomes the template (`gabarit` / `template` already means the monochrome image there)" | Correct, but a trap. On a `text` element `template` is the format string, and on a button it's a boolean. Writing `template = "{texte}"` on a button gives `type invalide : le texte "{texte}", attendu true ou false` (`p3.toml`), which doesn't hint at the fix. Worth an example. |
| `README.md:416` | "write `trousseau:openweather`" | It's the only French token you *must* type in an all-English config; there's no `keychain:` alias. Fine if intended, but say "(French for keychain)". |
| `README.md:411` | `mcp = { commande = …, outil = …, arguments = … }` (`command`, `tool` in English) | Clear, but a missing `tool` gives `champ manquant «outil»` (`p8.toml`), the French key. Same old B11 pattern. |
| `README.md:417` | "`tactile --check` also reports … an interval under 5 s" | It does, but the warning names `toutes_les` even when I wrote `every` (`p1.toml`). |
| `README.md:420` | "[exemples/mcp.toml](exemples/mcp.toml) (the tiny demo MCP server …)" | The example is French and hard-codes `~/code/tactile/…`, while CONTRIBUTING's clone creates `tactile-touch-bar/`. The demo tool is called `heure`, which the English README never says. |
| `README.md:475-492` | Troubleshooting table | It doesn't cover the new source warnings I hit: `toutes_les est ramené à 5 secondes`, `seules les adresses https://`, `aucune [[source]] ne s'appelle`, `un gabarit sans source`, `semble contenir une clé en clair`, `écris un seul de http, commande ou mcp`, nor `modificateur inconnu` / `touche inconnue`. Issue #7 should list these. |
| `README.md:179` | Button fields | `source` isn't in the button's field list. It only shows up at `:415`. |
| `CONTRIBUTING.md:16-22` | `mise install` then `CI=1 cargo test` | `cargo` isn't on PATH unless mise is activated. Say `mise exec -- cargo test`, or "run `mise activate` first". |
| `CONTRIBUTING.md:19` | "the tests themselves run in about a second" | 10-11 s (MCP and HTTP tests). |
| `CONTRIBUTING.md:29-30` | `TACTILE_CONFIG=/tmp/x.toml target/release/tactile --personnaliser` then `--check` | The first command blocks, so the second waits until the window closes. Use `--customize` (README's spelling) and `&`, or split the two. |
| `CONTRIBUTING.md:60` | "those hardware tests show as `ignored`" | Yes, but the 18 reasons are French. |

## Bugs

Severity: **High** = blocks an English user or contributor, **Medium** = costs real time, **Low** = polish.

### B1. The landing page sells features the download doesn't have (High)
- Steps: open the GitHub page, look at the hero and Highlights, follow Install (`:94`), paste the drawer snippet from `:195-213` into `barre.toml`.
- Expected: it works, or the page says it's unreleased.
- Actual: v0.2.0 rejects `type = "launcher"` (there's no `lanceur` in `git show v0.2.0:src/config.rs`). 27+ commits on `main` since the tag carry launchers, system actions, sources, the `text` element and import/export. Only the Roadmap rows say "unreleased". On `main`, `tactile --version` prints `0.2.0`, so even a source build claims to be the release.
- This is the 0.2 B8 again, one version later.

### B2. `{text}` in a template silently shows "—" (Medium)
- Steps: `p1.toml`: a `command` source that prints plain text, a `text` element with `template = "{text}"`, then `tactile --check p1.toml`.
- Expected: the English alias works like every other one, or `--check` warns.
- Actual: `OK : 1 élément`. At runtime the value is `{"texte": …}` (`src/sources/mod.rs:90`), so `{text}` is a missing path and shows "—". The fix is one line: also insert `"text"`, or have `--check` warn on `{text}`.

### B3. Errors and warnings name French keys you never typed (Low, still open from 0.2 B11)
- `every = 2` → `toutes_les est ramené à 5 secondes`. A `mcp` without `tool` → `champ manquant «outil»`. Two sources → `écris un seul de http, commande ou mcp`. A `text` with `template` and no source → `un gabarit sans source`. Every new 0.3 message keeps the pattern.

### B4. A button's `template` string gives an unhelpful type error (Low)
- `p3.toml`: a button with `source = "w"` and `template = "{texte}"` → `ligne 5 : type invalide : le texte "{texte}", attendu true ou false`, exit 1. The message should say "on a button, put the template in `text`".

### B5. Ignored-test reasons are French (Low)
- `cargo test` output: `ignored, pas de session graphique sur un runner de CI` ×9, etc. These are the first lines of French a contributor reads, and they decide whether the contributor trusts the run.

### B6. CONTRIBUTING step 2 fails with `cargo not found` after `mise install` (Medium for newcomers)
- Steps: a fresh shell without `mise activate`, then `mise install` and `CI=1 cargo test`.
- Actual: `cargo not found`. `mise exec -- cargo test` works.

## 3 proposals

1. **Ship what the page shows, or label it.** Tag 0.3 now (the suite is 117/0, clippy and doc are clean). Until then, add one line under the hero: "Launchers, data sources and import/export are on `main`, not yet in the release: [build from source](#contributing)". Bump `Cargo.toml` to `0.3.0-dev` so `--version` doesn't lie.
2. **Make CONTRIBUTING's 10 minutes literal.** Use `mise exec -- cargo …` (or a one-line `mise activate` hint), say the tests take about 10 s, use `--customize … &` in step 3, and translate the `ignore = "…"` reasons, or at least prefix them with an English tag (`no-gui`, `no-audio`, `drives-playback`).
3. **One English pass on the new 0.3 surface.** Alias `{text}` in source values (B2), and make warnings echo the key as written: serde can't, but `controler` knows the line, so it can print both forms (`toutes_les/every`). Extend issue #7's table with the eight source and keyboard messages above, and update `README.md:229` along with issue #2.
