## [1.1.1](https://github.com/browzomje/pokko/compare/v1.1.0...v1.1.1) (2026-10-31)

### 🐛 Fixes

* polish lab wizard layout, sidebar settings alignment, dynamic dropdowns and i18n audit ([097f13a](https://github.com/browzomje/pokko/commit/097f13a0a53bdc301cd009542cd42c9fd0d3cbbb))

## [1.1.0](https://github.com/browzomje/pokko/compare/v1.0.0...v1.1.0) (2026-10-31)

### ✨ Features

* ✨ add Firefox Marionette Source Lab, i18n, structured logs, and connector sharing ([b5d74a6](https://github.com/browzomje/pokko/commit/b5d74a68c9b8259a0b57deefcd57ad63d90e08f9))

### 🐛 Fixes

* ✨ harden Firefox lab startup and verify ReadBerserk connector ([8f306d6](https://github.com/browzomje/pokko/commit/8f306d6b76795a1cb176572822015a9622744dbe))
* render UI by moving i18n runes to a svelte module and resync lockfile ([8c49aff](https://github.com/browzomje/pokko/commit/8c49aff09ee05b8f1fed0afe64e300e89f95da7f))

## Unreleased

### ✨ Features

* **lab**: side-by-side Source Lab with automated Firefox + uBlock Origin Marionette profile and heuristic chapter detection
* **connectors**: official `connectors/` directory and safe import/export from JSON files, folders, and GitHub repositories (blocking archive formats)
* **logs**: structured JSONL file logger (`logs/pokko.log`) with rotation and live diagnostics viewer in Settings
* **i18n**: full Italian and English UI localization plus searchable world-language selector
* **network**: configurable desktop User-Agent, origin Referer headers, per-domain concurrency limit, jitter, and retry backoff
* **ui**: streamlined headers, 2-column Settings layout, unified Workspace sidebar navigation, and Wayland desktop icon alignment

## 1.0.0 (2026-10-08)

### ✨ Features

* ✨ launch pokko with manga and comics downloads ([dd8fbf6](https://github.com/browzomje/pokko/commit/dd8fbf6093794e3afcf090a44f657be2b2be7f58))

# Changelog

Release notes are generated automatically by semantic-release.
