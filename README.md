# <img src="public/pokko.svg" alt="pokko" height="42" align="absmiddle"> pokko

[![Build](https://github.com/browzomje/pokko/actions/workflows/build.yml/badge.svg)](https://github.com/browzomje/pokko/actions/workflows/build.yml)
[![Release](https://img.shields.io/github/v/release/browzomje/pokko)](https://github.com/browzomje/pokko/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Desktop manga and comic downloader built with Rust, Tauri 2, and Svelte 5.

## Features

- **Ordered PDF & Archive Exports**: Download chapters and volumes as ordered PDF, CBZ, CBR, or raw images.
- **Official & Custom Connectors**: Built-in verified sources plus a guided **Source Lab** powered by an isolated Firefox + uBlock Origin profile (via Marionette) and built-in ad-stripping inspection.
- **Safe Connector Sharing**: Import and export `.json` connectors from individual files, local directories, or GitHub repositories (`connectors/<id>.json`). Archive files (`.zip`, `.tar`, `.rar`, etc.) are strictly rejected.
- **Structured Diagnostic Logs**: Full JSONL activity and selector logs saved to `<app_data_dir>/logs/pokko.log` for debugging and external tool inspection.
- **Multilingual UI & Catalog**: English and Italian interface localization, plus a searchable world-language selector for connectors.
- **Resilient Networking**: Configurable desktop User-Agent, automatic `Referer` headers, per-domain concurrency limits, jittered rate-limiting, and `Retry-After` backoff.

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

Official connector definitions live in [`connectors/`](connectors/) as standalone `.json` files:

- **Create**: Open **Source Lab** (`Ctrl+L`), enter a site URL, and follow the side-by-side recording steps (supports both multi-series catalogs and single-series sites).
- **Export / Contribute**: Click **Export Connector** to save `<id>.json` and place it under `connectors/<id>.json` in a Pull Request.
- **Import**: In **Catalog Sources** (`Ctrl+F`), import a `.json` file, scan a local folder, or fetch directly from any GitHub repository (`owner/repo`).

## Diagnostics & Logs

Runtime logs (HTTP requests, CSS selector match counts, Marionette events, and PDF assembly) are written to:

```text
<app_data_dir>/logs/pokko.log
```

You can view recent entries, copy the file path, or clear logs from **Settings → Diagnostics & Logs**.

## License

[MIT](LICENSE) · Copyright © 2026 browzomje
