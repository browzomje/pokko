# Contributing to pokko

Thank you for contributing to pokko! This document outlines the project's commit conventions and release workflow.

## Commit Message Guidelines

pokko uses [Conventional Commits](https://www.conventionalcommits.org/) to automate semantic versioning and changelog generation via `semantic-release`.

### Format

```text
<type>(<scope>): <subject>
```

- `<scope>` is optional (e.g. `(lab)`, `(sources)`, `(network)`).
- `<subject>` is a concise, imperative description in lowercase (e.g. `add source template pre-filling`).

### Rules

1. **No emojis in commit messages or release notes**: Commit messages must use plain text prefixes only (e.g. `feat: ...`, `fix: ...`). Do not use emojis in commit subjects, commit bodies, or release notes.
2. **Imperative mood**: Use "add", "fix", "update", not "added", "fixed", "updates".
3. **No trailing period**: Keep the summary line clean and concise.

### Commit Types and Release Impact

| Type | Release | Description | Example |
| --- | --- | --- | --- |
| `feat` | Minor (`1.x.0`) | New features or functionality | `feat: support folder import for connectors` |
| `fix` | Patch (`1.0.x`) | Bug fixes | `fix: resolve blank screen caused by uncompiled runes` |
| `perf` | Patch (`1.0.x`) | Performance improvements | `perf: reduce memory usage during pdf generation` |
| `refactor` | Patch (`1.0.x`) | Code refactoring without behavior changes | `refactor: extract i18n logic into svelte module` |
| `build(release)` | Patch (`1.0.x`) | Packaging or dependency bumps that trigger a release | `build(release): synchronize lockfile and dependencies` |
| `docs` | None | Documentation changes | `docs: update connector creation guide` |
| `chore` | None | Maintenance tasks, config tweaks | `chore: update gitignore rules` |
| `test` | None | Adding or updating tests | `test: add offline fixture for readberserk` |
| `ci` | None | CI/CD workflow updates | `ci: improve build matrix caching` |

### Breaking Changes

Breaking changes trigger a **Major** release (`2.0.0`). Indicate a breaking change by either:
- Appending an exclamation mark after the type: `feat!: change connector schema`
- Adding a `BREAKING CHANGE:` footer in the commit message body:
  ```text
  feat: restructure source configuration schema

  BREAKING CHANGE: connectors now require https base_url and explicit adapter fields.
  ```

## Connector Guidelines

- Connectors reside in `connectors/<id>.json`.
- All connectors must use `adapter: "generic"` (100% declarative JSON; site-specific Rust code is not accepted).
- Connectors must be tested via the 5-step Source Lab (`Ctrl+L`) co-pilot, including the test PDF download, prior to submitting a PR.
- Commit scope: `feat(connectors): add <name> connector`.

## Development and Verification

Before submitting changes, ensure all checks pass:

```sh
npm run check
npm run check:i18n
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```
