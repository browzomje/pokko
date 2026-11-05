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

| Platform | Packages | Standalone CLI |
| --- | --- | --- |
| **Linux** | `.AppImage`, `.deb`, `.rpm` | `pokko-cli-linux-x86_64.tar.gz` |
| **Windows** | `.exe`, `.msi` | `pokko-cli-windows-x64.zip` |

*Note for Linux AppImage*: run `chmod +x pokko-*.AppImage` before running. Firefox is required only if you want to record custom connectors in Source Lab.

## Quick Start

1. **Catalog Sources**: Enable desired sources from the sidebar.
2. **Explore**: Search for a manga or paste a series URL.
3. **Select**: Choose volumes or specific chapters, then click **Download**.
4. **Settings**: Configure output folder (`~/Downloads/Pokko` by default), request delay, and format.
5. **Downloads**: Follow live download and packing progress, or open output folders.

## Command Line Interface (CLI)

Pokko can also run entirely from the terminal (both as `pokko` with arguments and as dedicated `pokko-cli`):

```sh
# Search manga across sources
pokko search "Berserk" --source mw

# Inspect series metadata and chapters (supports local custom connector JSON)
pokko info "https://www.mangaworld.mx/manga/2278/berserk"
pokko info "https://example.com/manga/1" --connector my-source.json

# Download specific chapters or ranges (PDF, CBZ, CBR, images)
pokko download "https://www.mangaworld.mx/manga/2278/berserk" --chapters 1-5 --format cbz

# List and test catalog sources
pokko sources list
pokko sources test mw
pokko sources test my-source.json

# Shell auto-completions (bash, zsh, fish, powershell, elvish)
pokko completions bash > ~/.local/share/bash-completion/completions/pokko
```

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
