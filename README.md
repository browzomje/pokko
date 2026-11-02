# pokko

Download manga and comics to read offline. Available for Linux and Windows.

[Download the latest release](https://github.com/browzomje/pokko/releases/latest) · [Report a problem](https://github.com/browzomje/pokko/issues)

## Install

Choose the installer for your system from the release page:

| System | Package |
| --- | --- |
| Linux | `.AppImage`, `.deb` or `.rpm` |
| Windows | `.exe` or `.msi` |

For AppImage, make the downloaded file executable before opening it.

Firefox is needed for guided source recording in Source Lab. Pokko uses a separate Firefox profile and installs uBlock Origin there.

## Download chapters

1. Open **Catalog Sources** and enable the sources you want to search.
2. In **Explore**, search for a title or paste a series link from an enabled source.
3. Open the series and select chapters or volumes.
4. Choose your output folder and export format in **Settings**.
5. Start the download. Follow progress and open completed files from **Downloads**.

Export formats include PDF, CBZ, CBR and image folders.

## Add a source

Open **Source Lab** from the sidebar (`Ctrl+Shift+L`). Follow the instruction above each step and use the main button at the bottom right:

1. **Site** — enter the website, source name and scan language; open the site.
2. **Search** — search in Firefox, then detect and check the results. You can also start from an open series or skip search for a single-series site.
3. **Series** — detect the chapter list and confirm a chapter to open.
4. **Pages** — detect the scans and check the preview.
5. **Verify** — try a chapter download, then save the connector. Enable it for catalog searches when ready.

If detection needs correcting, expand **Other ways to complete this step** to select an element in Firefox. Manual selectors and other settings are under **Advanced configuration**.

You can import and export JSON connectors from the source tools. See the [connector guide](connectors/README.md) for the schema and contribution instructions.

## Troubleshooting

- **A source stops working:** check its current address in Catalog Sources, then update its rules in Source Lab.
- **Firefox does not connect:** close the Source Lab companion window and use **Next – open site** again. Your usual Firefox profile is separate.
- **A download fails:** check the error in Downloads. Open **Settings → Diagnostics & Logs** for recent activity and the log file path.

When reporting a problem, include the Pokko version, operating system, source URL and relevant error message.

## Build from source

Requires Node.js 22+, stable Rust and the [Tauri system prerequisites](https://v2.tauri.app/start/prerequisites/). On Linux, install WebKitGTK 4.1 development packages. Install Firefox to use guided source recording.

```sh
git clone https://github.com/browzomje/pokko.git
cd pokko
npm ci
npm run tauri dev
```

Create installers with `npm run tauri build`.

Run checks with:

```sh
npm run check
npm run check:i18n
npm test
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

## License

[MIT](LICENSE)
