## [1.2.2](https://github.com/browzomje/pokko/compare/v1.2.1...v1.2.2) (2026-11-01)

### 🐛 Fixes

* ✨ transparent Firefox navigation tracking, smart selector generation, and leercapitulo flow ([29c88ea](https://github.com/browzomje/pokko/commit/29c88ea9bb64a52687f8ddd23417f0035f162169))

## [1.2.1](https://github.com/browzomje/pokko/compare/v1.2.0...v1.2.1) (2026-10-31)

### 🐛 Fixes

* ✨ harden Firefox uBlock profile, prevent auto-step jumps, remove readberserk, and polish UI/i18n ([73722fd](https://github.com/browzomje/pokko/commit/73722fdcbe8b4e1879505c2dbe3b3f8bfe7c1783))

## [1.2.0](https://github.com/browzomje/pokko/compare/v1.1.1...v1.2.0) (2026-10-31)

### ✨ Features

* ✨ declarative generic connector engine, interactive Source Lab co-pilot, FMHY connectors, and 15-language i18n ([5efd690](https://github.com/browzomje/pokko/commit/5efd6906e816cd2860ac3180822688a746fbba8b))

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

* **connectors**: 100% declarative JSON architecture (`adapter: "generic"`), supporting `chapter_url_suffix`, `volume_selector`, `volume_title_selector`, `chapter_title_selector`, and `reader_next_selector`
* **catalog**: verified official catalog (`mangaworld`, `mangapill`, `mangatown`, `mangafreak`, `mangaread`, `readberserk`); removed legacy site-specific `mangadex` and `readcomicsonline`
* **lab**: 5-step Source Lab co-pilot with integrated Firefox + uBlock Origin Marionette profile, automatic structural detection, live click-picker, and end-to-end trial PDF download
* **i18n**: 15 complete UI translations (Arabic, Chinese, Dutch, English, French, German, Hindi, Italian, Japanese, Korean, Polish, Portuguese, Russian, Spanish, Turkish)
* **network**: fast 500ms default request interval, configurable desktop User-Agent, origin Referer headers, per-domain concurrency limits, jitter, and retry backoff
* **logs**: structured JSONL file logger (`logs/pokko.log`) with rotation and live diagnostics viewer in Settings

## 1.0.0 (2026-10-08)

### ✨ Features

* ✨ launch pokko with manga and comics downloads ([dd8fbf6](https://github.com/browzomje/pokko/commit/dd8fbf6093794e3afcf090a44f657be2b2be7f58))

# Changelog

Release notes are generated automatically by semantic-release.
