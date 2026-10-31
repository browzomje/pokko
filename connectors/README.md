# Pokko Connectors Catalog

Pokko connectors use a **100% declarative JSON** specification (`adapter: "generic"`), functioning similarly to uBlock Origin rule lists. No site-specific Rust code is required or permitted.

## Verified Official Connectors

| ID | Name | Language | Type | Features |
| --- | --- | --- | --- | --- |
| `mw` | MangaWorld | Italian (`it`) | Catalog | Volume groupings (`volume_selector`), list mode (`chapter_url_suffix`) |
| `mangapill` | MangaPill | English (`en`) | Catalog | Standard web reader, clean chapter lists |
| `mangatown` | MangaTown | English (`en`) | Catalog | Single-page paginated reader (`reader_next_selector`) |
| `mf` | MangaFreak | English (`en`) | Catalog | Multi-page list-view reader |
| `mangaread` | MangaRead | English (`en`) | Catalog | WP-Manga catalog structure |
| `readberserk` | ReadBerserk | English (`en`) | Single Series | Direct chapter table index |

> **Note**: Deprecated connectors (`mangadex`, `readcomicsonline`) have been removed in favor of generic declarative definitions.

## Connector JSON Schema

Connectors are stored as individual `.json` files in `connectors/<id>.json`.

```json
{
  "id": "example",
  "name": "Example Manga",
  "base_url": "https://example.com",
  "category": "manga",
  "language": "en",
  "adapter": "generic",
  "enabled": true,
  "min_delay_ms": 500,
  "note": "Description of source",
  "search_path": "/search?q={query}&page={page}",
  "result_selector": "a.title",
  "next_selector": "a.next",
  "chapter_selector": "ul.chapters li a",
  "volume_selector": ".volume-container",
  "volume_title_selector": "h4.vol-title",
  "chapter_title_selector": "span.chapter-name",
  "image_selector": "img.chapter-img",
  "description_selector": "div.synopsis",
  "chapter_url_suffix": "?style=list",
  "reader_next_selector": "a.next-page"
}
```

### Field Reference

| Field | Type | Description |
| --- | --- | --- |
| `id` | string | Unique connector identifier (lowercase alphanumeric). |
| `name` | string | Human-readable name. |
| `base_url` | string | Base URL (`https://...`). |
| `category` | string | Content type (`manga` or `comic`). |
| `language` | string | ISO language code (`it`, `en`, `es`, `fr`, etc.). |
| `adapter` | string | Must be `"generic"`. |
| `enabled` | boolean | Whether the source is enabled by default. |
| `min_delay_ms` | number | Minimum request delay in ms (500 to 120000; default: `500`). |
| `search_path` | string | Search URL format with `{query}` and `{page}` placeholders (empty for single-series sites). |
| `result_selector` | string | CSS selector targeting search result link tags (`<a>`). |
| `next_selector` | string | CSS selector for pagination next links in search results. |
| `chapter_selector` | string | CSS selector targeting chapter link elements (`<a>`). |
| `volume_selector` | string | *(Optional)* CSS selector for volume grouping blocks. |
| `volume_title_selector` | string | *(Optional)* CSS selector for volume title inside `volume_selector`. |
| `chapter_title_selector` | string | *(Optional)* CSS selector for chapter title text within `chapter_selector`. |
| `image_selector` | string | CSS selector targeting scan images inside the reader. |
| `description_selector` | string | *(Optional)* CSS selector for series synopsis. |
| `chapter_url_suffix` | string | *(Optional)* Query parameter or path appended to chapter URLs (e.g. `?style=list`). |
| `reader_next_selector` | string | *(Optional)* CSS selector for "next page" button in single-page readers. |

## Source Lab 5-Step Co-pilot

Create and verify connectors interactively inside Pokko (`Ctrl+L`):

1. **Site**: Enter base URL and site details.
2. **Search**: Search for a title in the integrated Firefox + uBlock Origin browser, or skip for single-series sites.
3. **Series**: Navigate to a series page; use automatic detection or click-picker to capture chapter and volume selectors.
4. **Pages**: Open a reader chapter; pick scan image selector, test reader suffix or single-page next selector.
5. **Verify**: Run field test (end-to-end test chapter download to PDF), verify search generalization, and enable connector.

## Contributing

1. Create and verify the connector using the Source Lab.
2. Export the connector JSON.
3. Place the file at `connectors/<id>.json`.
4. Submit a Pull Request following [Conventional Commits](../CONTRIBUTING.md).
