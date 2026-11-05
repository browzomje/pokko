# pokko

Download manga and comics to read offline. Available for Linux and Windows.

[Download the latest release](https://github.com/browzomje/pokko/releases/latest) · [Report a problem](https://github.com/browzomje/pokko/issues)

## Features

- **Multiple export formats**: Save series as PDF, CBZ, CBR, or image folders.
- **Granular chapter selection**: Select entire series, specific volumes, or individual chapters.
- **Calibrated progress & verification**: Two-phase live progress tracking (downloads + format building) with image verification and automatic corrupt-file repair.
- **Multi-language interface**: Full localization across 15 languages.
- **Source Lab**: Built-in 5-step visual co-pilot with a sandboxed Firefox + uBlock Origin profile to create and test custom declarative connectors without writing code.

## Installation

Download the installer for your platform from the [Releases](https://github.com/browzomje/pokko/releases/latest) page:

| Platform | Packages |
| --- | --- |
| **Linux** | `.AppImage`, `.deb`, `.rpm` |
| **Windows** | `.exe`, `.msi` |

*Note for Linux AppImage*: run `chmod +x pokko-*.AppImage` before running. Firefox is required only if you want to record custom connectors in Source Lab.

## Quick Start

1. **Catalog Sources**: Enable desired sources from the sidebar.
2. **Explore**: Search for a manga or paste a series URL.
3. **Select**: Choose volumes or specific chapters, then click **Download**.
4. **Settings**: Configure output folder (`~/Downloads/Pokko` by default), request delay, and format.
5. **Downloads**: Follow live download and packing progress, or open output folders.

## Custom Sources

Pokko uses 100% declarative JSON connectors (`adapter: "generic"`). You can create new connectors visually via **Source Lab** (`Ctrl+Shift+L` or `Ctrl+L`) or import community presets. See the [connector guide](connectors/README.md) for details.

## Build from Source

Prerequisites: Node.js 22+, stable Rust toolchain, and [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
git clone https://github.com/browzomje/pokko.git
cd pokko
npm ci
npm run tauri dev
```

Build release packages:

```sh
npm run tauri build
```

Run test suite:

```sh
npm run check
npm run check:i18n
npm test
cargo test --manifest-path src-tauri/Cargo.toml
```

## License

[MIT](LICENSE)
