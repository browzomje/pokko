# <img src="public/pokko.svg" alt="pokko" height="42" align="absmiddle"> pokko

[![Build](https://github.com/browzomje/pokko/actions/workflows/build.yml/badge.svg)](https://github.com/browzomje/pokko/actions/workflows/build.yml)
[![Release](https://img.shields.io/github/v/release/browzomje/pokko)](https://github.com/browzomje/pokko/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Desktop manga and comic downloader built with Rust, Tauri 2, and Svelte 5.

## Features

- **100% Declarative JSON Connectors**: Universal `adapter: "generic"` architecture inspired by uBlock Origin filter rules — zero hardcoded site-specific Rust code. Supports `chapter_url_suffix`, `volume_selector`, `volume_title_selector`, `chapter_title_selector`, and `reader_next_selector`.
- **Verified Official Catalog**: Preconfigured, tested connectors for Italian and English sources (`mangaworld`, `mangapill`, `mangatown`, `mangafreak`, `mangaread`).
- **5-Step Source Lab Co-pilot**: Guided connector builder with isolated Firefox + uBlock Origin profile, automatic structural detection, live click-picker, and end-to-end test PDF download.
- **Multilingual UI (15 Languages)**: Complete interface localization in English, Italian, Spanish, French, German, Portuguese, Russian, Japanese, Chinese, Korean, Arabic, Hindi, Dutch, Polish, and Turkish.
- **Fast & Resilient Networking**: Default 500ms request interval, desktop User-Agent, automatic `Referer` headers, per-domain concurrency limits, jitter, and `Retry-After` backoff.
- **Ordered PDF & Archive Exports**: Download chapters and volumes as ordered PDF, CBZ, CBR, or raw images.
- **Safe Connector Sharing**: Import and export `.json` connectors from files, folders, or GitHub repositories (`connectors/<id>.json`). Archives (`.zip`, `.tar`, `.rar`) are strictly rejected.
- **Structured Diagnostic Logs**: JSONL activity and selector logs at `<app_data_dir>/logs/pokko.log`.

## Installation

Download the latest package from [Releases](https://github.com/browzomje/pokko/releases/latest):

| Platform | Package |
| --- | --- |
| Linux | `.AppImage`, `.deb`, `.rpm` |
| Windows (x86_64) | `.exe`, `.msi` |

## Build from Source

Requires Node.js 20+, Rust (stable), and WebKitGTK 4.1 on Linux (`firefox` recommended for the live Source Lab browser).

```sh
git clone https://github.com/browzomje/pokko.git
cd pokko
npm install
npm run tauri dev
```

Build a release binary:

```sh
npm run tauri build
```

## Connectors (`connectors/`)

Pokko uses a 100% declarative JSON architecture (`adapter: "generic"`): no site-specific Rust code is required.

Verified official connectors in [`connectors/`](connectors/):
- `mangaworld` (`mw`): Italian catalog with volumes and chapter lists
- `mangapill` (`mangapill`): English manga releases
- `mangatown` (`mangatown`): English catalog with single-page paginated reader
- `mangafreak` (`mf`): English manga catalog
- `mangaread` (`mangaread`): English manga catalog

### Source Lab Co-pilot (`Ctrl+L`)
Build or update connectors using the 5-step guided wizard:
1. **Site**: Base URL and general metadata
2. **Search**: Search endpoint and card selector (or skip for single-series sites)
3. **Series**: Chapter and volume grouping selectors (`volume_selector`, `volume_title_selector`, `chapter_title_selector`)
4. **Pages**: Scan image selector, list-mode query suffix (`chapter_url_suffix`), or single-page reader next link (`reader_next_selector`)
5. **Verify**: Field test with end-to-end test chapter PDF download and activation

See [`connectors/README.md`](connectors/README.md) for complete schema documentation and contribution guidelines.

## Diagnostics & Logs

Runtime logs (HTTP requests, CSS selector match counts, Marionette events, and PDF assembly) are written to:

```text
<app_data_dir>/logs/pokko.log
```

You can view recent entries, copy the file path, or clear logs from **Settings → Diagnostics & Logs**.

## License

[MIT](LICENSE) · Copyright © 2026 browzomje
