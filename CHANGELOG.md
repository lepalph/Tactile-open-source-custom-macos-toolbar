# Changelog

All notable changes to Tactile are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

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
