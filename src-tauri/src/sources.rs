use crate::{
    provider::{self, Chapter, Detail, Manga, SearchResult, Volume},
    Engine,
};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Source {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author_selector: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reader_next_selector: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workflow: Vec<RecordedStep>,
    pub id: String,
    pub category: String,
    pub name: String,
    pub base_url: String,
    pub language: String,
    pub adapter: String,
    pub enabled: bool,
    pub min_delay_ms: u64,
    pub note: String,
    pub search_path: String,
    pub result_selector: String,
    pub chapter_selector: String,
    pub image_selector: String,
    pub description_selector: String,
    pub next_selector: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub chapter_url_suffix: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub volume_selector: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub volume_title_selector: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub chapter_title_selector: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct RecordedStep {
    pub kind: String,
    pub url: String,
    pub selector: String,
}
impl Default for Source {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "Nuova fonte".into(),
            base_url: "https://".into(),
            category: "manga".into(),
            language: "en".into(),
            adapter: "generic".into(),
            enabled: false,
            min_delay_ms: 500,
            note: "HTML generico: verifica i selettori sul sito".into(),
            search_path: "/search?keyword={query}&page={page}".into(),
            result_selector: ".manga-title, .post-title a, .manga-name a, h3 a, .story_name a"
                .into(),
            chapter_selector: "a.chap, a.chapter-link, .wp-manga-chapter a, .chapter-list a, .row-content-chapter a"
                .into(),
            image_selector: "img.page-image, .reading-content img, .chapter-content img, #readerarea img, .read_img img, img#gohere"
                .into(),
            description_selector: "#noidungm, .description-summary, .summary__content, .manga_series_description"
                .into(),
            next_selector: "a[rel=next], .pagination .next a, a.next".into(),
            reader_next_selector: String::new(),
            author_selector: "[rel=author], .author, .author-content".into(),
            chapter_url_suffix: String::new(),
            volume_selector: String::new(),
            volume_title_selector: String::new(),
            chapter_title_selector: String::new(),
            workflow: Vec::new(),
        }
    }
}
pub fn defaults() -> Vec<Source> {
    serde_json::from_str(include_str!("../../resources/sources.json"))
        .expect("Preset fonti incorporati non validi")
}
pub fn validate(s: &Source) -> Result<(), String> {
    provider::site(&s.base_url)?;
    if !["manga", "comics"].contains(&s.category.as_str()) {
        return Err("Categoria fonte non valida".into());
    }
    if s.id.trim().is_empty() || s.name.trim().is_empty() {
        return Err("Nome e identificatore fonte obbligatori".into());
    }
    if s.language.trim().is_empty()
        || s.language.len() > 10
        || !s
            .language
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        || s.adapter != "generic"
    {
        return Err("Lingua o adattatore non valido".into());
    }
    if !(500..=120000).contains(&s.min_delay_ms) {
        return Err("Intervallo fonte consentito: 0,5–120 secondi".into());
    }
    if s.language == "both" {
        return Err("Per HTML generico scegli una lingua precisa".into());
    }
    if !s.search_path.is_empty() && !s.search_path.contains("{query}") {
        return Err("Il percorso ricerca deve contenere {query}".into());
    }
    if s.enabled
        && [&s.chapter_selector, &s.image_selector]
            .iter()
            .any(|field| field.trim().is_empty())
    {
        return Err(
            "Completa i selettori di capitoli e immagini prima di abilitare la fonte".into(),
        );
    }
    if s.enabled && !s.search_path.is_empty() && s.result_selector.trim().is_empty() {
        return Err(
            "Completa il selettore dei risultati di ricerca o disabilita il percorso di ricerca"
                .into(),
        );
    }
    for value in [
        &s.result_selector,
        &s.chapter_selector,
        &s.image_selector,
        &s.description_selector,
        &s.next_selector,
        &s.reader_next_selector,
        &s.author_selector,
        &s.volume_selector,
        &s.volume_title_selector,
        &s.chapter_title_selector,
    ] {
        if !value.is_empty() {
            css(value)?;
        }
    }
    Ok(())
}
fn css(s: &str) -> Result<Selector, String> {
    Selector::parse(s).map_err(|_| format!("Selettore CSS non valido: {s}"))
}
/// Read creator names from static schema.org metadata, never from plot text.
pub fn structured_authors(html: &str) -> Vec<String> {
    fn names(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::String(name) => out.push(name.clone()),
            Value::Array(values) => {
                for v in values {
                    names(v, out)
                }
            }
            Value::Object(map) => {
                if let Some(name) = map.get("name").and_then(Value::as_str) {
                    out.push(name.into());
                }
            }
            _ => {}
        }
    }
    fn visit(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Array(values) => {
                for v in values {
                    visit(v, out)
                }
            }
            Value::Object(map) => {
                for key in ["author", "creator", "illustrator"] {
                    if let Some(value) = map.get(key) {
                        names(value, out);
                    }
                }
                if let Some(graph) = map.get("@graph") {
                    visit(graph, out);
                }
            }
            _ => {}
        }
    }
    let doc = Html::parse_document(html);
    let mut out = Vec::new();
    for script in doc.select(&css("script[type='application/ld+json']").unwrap()) {
        if let Ok(value) = serde_json::from_str::<Value>(&script.inner_html()) {
            visit(&value, &mut out);
        }
    }
    let mut seen = HashSet::new();
    out.retain(|name| !name.trim().is_empty() && seen.insert(name.to_lowercase()));
    out
}
pub async fn creators(
    engine: &Engine,
    s: &Source,
    manga: &Manga,
    delay: u64,
) -> Result<Vec<String>, String> {
    let response = engine
        .request(&manga.url, &s.base_url, delay.max(s.min_delay_ms), None)
        .await?;
    let html = response.text().await.map_err(|e| e.to_string())?;
    let mut names = structured_authors(&html);
    let doc = Html::parse_document(&html);
    if !s.author_selector.is_empty() {
        names.extend(
            doc.select(&css(&s.author_selector)?)
                .map(text)
                .filter(|n| !n.is_empty()),
        );
    }
    if let Some(name) = doc
        .select(&css("meta[name='author']")?)
        .next()
        .and_then(|el| el.value().attr("content"))
    {
        names.push(name.into());
    }
    let mut seen = HashSet::new();
    names.retain(|n| seen.insert(n.to_lowercase()));
    Ok(names)
}
fn text(e: scraper::ElementRef<'_>) -> String {
    e.text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn decorate(mut r: SearchResult, s: &Source) -> SearchResult {
    for m in &mut r.items {
        m.source_id = s.id.clone();
        m.source_name = s.name.clone();
        m.language = s.language.clone();
    }
    r
}
pub async fn search(
    engine: &Engine,
    s: &Source,
    query: &str,
    page: u32,
    delay: u64,
) -> Result<SearchResult, String> {
    let base = provider::site(&s.base_url)?;
    let query_enc: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
    let path = s
        .search_path
        .replace("{query}", &query_enc)
        .replace("{page}", &page.to_string());
    let url = provider::resolve(&base, &path)?;
    let response = engine
        .request(&url, &base, delay.max(s.min_delay_ms), None)
        .await?;
    let mut final_url = response.url().to_string();
    let mut html = response.text().await.map_err(|e| e.to_string())?;
    if page > 1 && !s.search_path.contains("{page}") {
        if page > 100 {
            return Err("Il catalogo è limitato a 100 pagine senza modello {page}".into());
        }
        let mut visited = HashSet::from([final_url.clone()]);
        for _ in 1..page {
            let Some(next) = page_link(&html, &final_url, &s.next_selector)? else {
                return Ok(SearchResult {
                    items: Vec::new(),
                    has_next: false,
                });
            };
            if !visited.insert(next.clone()) {
                return Err(
                    "Paginazione del catalogo ciclica: controlla il selettore risultati successivi"
                        .into(),
                );
            }
            let response = engine
                .request(&next, &base, delay.max(s.min_delay_ms), None)
                .await?;
            final_url = response.url().to_string();
            html = response.text().await.map_err(|e| e.to_string())?;
        }
    }
    let result = generic_search(&html, &final_url, s)?;
    Ok(decorate(result, s))
}
pub fn generic_search(html: &str, base: &str, s: &Source) -> Result<SearchResult, String> {
    let doc = Html::parse_document(html);
    if s.search_path.trim().is_empty() && s.result_selector.trim().is_empty() {
        let title = doc
            .select(&css("h1")?)
            .next()
            .map(text)
            .filter(|t| !t.is_empty())
            .or_else(|| {
                doc.select(&css("title").ok()?)
                    .next()
                    .map(text)
                    .filter(|t| !t.is_empty())
            })
            .unwrap_or_else(|| s.name.clone());
        let cover = doc
            .select(&css("meta[property='og:image']")?)
            .next()
            .and_then(|m| m.value().attr("content"))
            .and_then(|v| provider::resolve(base, v).ok())
            .unwrap_or_default();
        return Ok(SearchResult {
            items: vec![Manga {
                title,
                url: base.to_string(),
                cover,
                authors: structured_authors(html),
                ..Manga::default()
            }],
            has_next: false,
        });
    }
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    let candidate_cover_attrs = [
        "data-src",
        "data-original",
        "data-lazy-src",
        "data-url",
        "data-cfsrc",
        "src",
    ];

    for a in doc.select(&css(&s.result_selector)?) {
        let Some(href) = a.value().attr("href") else {
            continue;
        };
        let title = a
            .value()
            .attr("title")
            .map(str::to_string)
            .unwrap_or_else(|| text(a));
        if title.is_empty() {
            continue;
        }
        let url = provider::resolve(base, href)?;
        if !seen.insert(url.clone()) {
            continue;
        }
        let image = a
            .ancestors()
            .filter_map(scraper::ElementRef::wrap)
            .take(5)
            .find_map(|el| el.select(&css("img").unwrap()).next())
            .or_else(|| a.select(&css("img").unwrap()).next());
        let cover = image
            .and_then(|i| {
                for attr in candidate_cover_attrs {
                    if let Some(v) = i.value().attr(attr) {
                        let t = v.trim();
                        if !t.is_empty() && !t.starts_with("data:image/") {
                            return Some(t);
                        }
                    }
                }
                None
            })
            .and_then(|v| provider::resolve(base, v).ok())
            .unwrap_or_default();
        let authors = if s.author_selector.is_empty() {
            Vec::new()
        } else {
            let links = css(&s.result_selector)?;
            let author = css(&s.author_selector)?;
            a.ancestors()
                .filter_map(scraper::ElementRef::wrap)
                .take(5)
                .take_while(|el| el.select(&links).count() <= 1)
                .find_map(|el| {
                    let names: Vec<String> = el
                        .select(&author)
                        .map(text)
                        .filter(|v| !v.is_empty())
                        .collect();
                    (!names.is_empty()).then_some(names)
                })
                .unwrap_or_default()
        };
        items.push(Manga {
            title,
            url,
            cover,
            authors,
            ..Manga::default()
        });
    }
    if items.is_empty() {
        let low = html.to_lowercase();
        if ![
            "no results",
            "no manga",
            "no result",
            "not found",
            "nessun risultato",
            "no comics",
        ]
        .iter()
        .any(|s| low.contains(s))
        {
            return Err(
                "Pagina non riconosciuta: controlla i selettori o la disponibilità della fonte"
                    .into(),
            );
        }
    }
    let has_next = !s.next_selector.is_empty()
        && doc
            .select(&css(&s.next_selector)?)
            .any(|a| a.value().attr("href").is_some());
    Ok(SearchResult { items, has_next })
}
pub async fn detail(
    engine: &Engine,
    s: &Source,
    manga: Manga,
    delay: u64,
) -> Result<Detail, String> {
    let response = engine
        .request(&manga.url, &s.base_url, delay.max(s.min_delay_ms), None)
        .await?;
    let html = response.text().await.map_err(|e| e.to_string())?;
    let mut detail = generic_detail(&html, manga, s)?;
    if !s.author_selector.is_empty() {
        let doc = Html::parse_document(&html);
        let authors = doc
            .select(&css(&s.author_selector)?)
            .map(text)
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>();
        if !authors.is_empty() {
            detail.manga.authors = authors;
        }
    }
    if detail.manga.authors.is_empty() {
        detail.manga.authors = structured_authors(&html);
    }
    Ok(detail)
}
pub fn generic_detail(html: &str, mut manga: Manga, s: &Source) -> Result<Detail, String> {
    let doc = Html::parse_document(html);
    manga.authors.extend(structured_authors(html));
    if !s.author_selector.is_empty() {
        manga.authors.extend(
            doc.select(&css(&s.author_selector)?)
                .map(text)
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>(),
        );
    }
    let mut author_seen = HashSet::new();
    manga
        .authors
        .retain(|name| author_seen.insert(name.to_lowercase()));

    let get_chapter_label = |el: scraper::ElementRef<'_>, url: &str| -> String {
        let mut label = if !s.chapter_title_selector.is_empty() {
            if let Ok(sel) = css(&s.chapter_title_selector) {
                el.select(&sel).next().map(text).unwrap_or_else(|| text(el))
            } else {
                text(el)
            }
        } else {
            text(el)
        };
        if label.trim().is_empty()
            || label.eq_ignore_ascii_case("read")
            || label.eq_ignore_ascii_case("leggi")
            || label.eq_ignore_ascii_case("view")
        {
            let derived = derive_chapter_title(url);
            if !derived.is_empty() {
                label = derived;
            }
        }
        label
    };

    let mut volumes = Vec::new();

    if !s.volume_selector.is_empty() {
        let vol_sel = css(&s.volume_selector)?;
        let chap_sel = css(&s.chapter_selector)?;
        let vol_title_sel = if !s.volume_title_selector.is_empty() {
            Some(css(&s.volume_title_selector)?)
        } else {
            None
        };

        for vol_el in doc.select(&vol_sel) {
            let title = if let Some(ref sel) = vol_title_sel {
                vol_el
                    .select(sel)
                    .next()
                    .map(text)
                    .unwrap_or_else(|| "Senza volume".into())
            } else {
                "Volume".into()
            };

            let mut chapters = Vec::new();
            let mut seen = HashSet::new();
            for a in vol_el.select(&chap_sel) {
                let Some(href) = a.value().attr("href") else {
                    continue;
                };
                let url = provider::resolve(&manga.url, href)?;
                if seen.insert(url.clone()) {
                    let label = get_chapter_label(a, &url);
                    chapters.push(Chapter {
                        page_count: None,
                        title: label,
                        url,
                    });
                }
            }
            if !chapters.is_empty() {
                chapters.sort_by(|a, b| number(&a.title).total_cmp(&number(&b.title)));
                volumes.push(Volume { title, chapters });
            }
        }

        if !volumes.is_empty() {
            volumes.sort_by(|a, b| number(&a.title).total_cmp(&number(&b.title)));
        }
    }

    if volumes.is_empty() {
        let mut seen = HashSet::new();
        let mut chapters = Vec::new();
        for a in doc.select(&css(&s.chapter_selector)?) {
            let Some(href) = a.value().attr("href") else {
                continue;
            };
            let url = provider::resolve(&manga.url, href)?;
            if seen.insert(url.clone()) {
                let label = get_chapter_label(a, &url);
                chapters.push(Chapter {
                    page_count: None,
                    title: label,
                    url,
                });
            }
        }
        if chapters.is_empty() {
            return Err(
                "Nessun capitolo trovato: modifica il selettore capitoli della fonte".into(),
            );
        }
        // Order numerical chapter labels, preserving original order for labels without numbers.
        chapters.sort_by(|a, b| number(&a.title).total_cmp(&number(&b.title)));
        volumes = vec![Volume {
            title: "Capitoli".into(),
            chapters,
        }];
    }

    let description = if s.description_selector.is_empty() {
        String::new()
    } else {
        doc.select(&css(&s.description_selector)?)
            .next()
            .map(text)
            .unwrap_or_default()
    };
    Ok(Detail {
        manga,
        description,
        volumes,
    })
}
fn number(s: &str) -> f64 {
    let lower = s.to_lowercase();
    let start = ["chapter", "capitolo", "ch.", "volume", "vol."]
        .iter()
        .find_map(|key| lower.find(key).map(|i| i + key.len()))
        .unwrap_or(0);
    let s = &lower[start..];
    let n = s
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .find(|p| p.chars().any(|c| c.is_ascii_digit()))
        .unwrap_or("0");
    n.parse().unwrap_or(0.0)
}

pub fn derive_chapter_title(url_str: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url_str) {
        if let Some(segments) = parsed.path_segments() {
            let filtered: Vec<&str> = segments.filter(|s| !s.is_empty()).collect();
            if let Some(last) = filtered.last() {
                let parts: Vec<String> = last
                    .replace('_', "-")
                    .split('-')
                    .filter(|p| !p.is_empty())
                    .map(|w| {
                        let mut c = w.chars();
                        match c.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                        }
                    })
                    .collect();
                if !parts.is_empty() {
                    return parts.join(" ");
                }
            }
        }
    }
    String::new()
}
pub async fn pages(
    engine: &Engine,
    s: &Source,
    chapter: &Chapter,
    referer: &str,
    delay: u64,
    id: Option<&str>,
) -> Result<Vec<String>, String> {
    let mut url = url::Url::parse(&chapter.url).map_err(|e| e.to_string())?;
    if !s.chapter_url_suffix.is_empty() {
        if s.chapter_url_suffix.starts_with('?') {
            for (k, v) in
                url::form_urlencoded::parse(s.chapter_url_suffix.trim_start_matches('?').as_bytes())
            {
                url.query_pairs_mut().append_pair(&k, &v);
            }
        } else {
            let current_path = url.path();
            url.set_path(&format!(
                "{}{}",
                current_path.trim_end_matches('/'),
                s.chapter_url_suffix
            ));
        }
    }
    let response = engine
        .request(url.as_str(), referer, delay.max(s.min_delay_ms), id)
        .await?;
    let base = response.url().to_string();
    let html = response.text().await.map_err(|e| e.to_string())?;
    let mut pages = generic_pages(&html, &base, s)?;
    let mut next = reader_next(&html, &base, s)?;
    let mut visited = HashSet::from([base.clone()]);
    while let Some(next_url) = next {
        if visited.len() >= 100 || !visited.insert(next_url.clone()) {
            return Err("Paginazione lettore ciclica o superiore a 100 pagine HTML: controlla il selettore pagina successiva".into());
        }
        let response = engine
            .request(&next_url, &base, delay.max(s.min_delay_ms), id)
            .await?;
        let final_url = response.url().to_string();
        let html = response.text().await.map_err(|e| e.to_string())?;
        pages.extend(generic_pages(&html, &final_url, s)?);
        next = reader_next(&html, &final_url, s)?;
    }
    let mut seen = HashSet::new();
    pages.retain(|u| seen.insert(u.clone()));
    Ok(pages)
}
pub fn reader_next(html: &str, base: &str, source: &Source) -> Result<Option<String>, String> {
    if source.adapter != "generic" || source.reader_next_selector.is_empty() {
        return Ok(None);
    }
    page_link(html, base, &source.reader_next_selector)
}
fn page_link(html: &str, base: &str, selector: &str) -> Result<Option<String>, String> {
    if selector.is_empty() {
        return Ok(None);
    }
    let doc = Html::parse_document(html);
    doc.select(&css(selector)?)
        .next()
        .and_then(|el| el.value().attr("href"))
        .filter(|href| !href.trim().is_empty())
        .map(|href| provider::resolve(base, href))
        .transpose()
}
pub fn generic_pages(html: &str, base: &str, s: &Source) -> Result<Vec<String>, String> {
    let doc = Html::parse_document(html);
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    let candidate_attrs = [
        "data-src",
        "data-original",
        "data-lazy-src",
        "data-url",
        "data-img",
        "data-page",
        "data-cfsrc",
        "src",
    ];

    for el in doc.select(&css(&s.image_selector)?) {
        let mut found_src = None;
        for attr in candidate_attrs {
            if let Some(val) = el.value().attr(attr) {
                let trimmed = val.trim();
                if !trimmed.is_empty() && !trimmed.starts_with("data:image/") {
                    found_src = Some(trimmed);
                    break;
                }
            }
        }
        if found_src.is_none() {
            if let Some(child_img) = el.select(&css("img").unwrap()).next() {
                for attr in candidate_attrs {
                    if let Some(val) = child_img.value().attr(attr) {
                        let trimmed = val.trim();
                        if !trimmed.is_empty() && !trimmed.starts_with("data:image/") {
                            found_src = Some(trimmed);
                            break;
                        }
                    }
                }
            }
        }
        if found_src.is_none() {
            if let Some(href) = el.value().attr("href") {
                let trimmed = href.trim();
                let lower = trimmed.to_lowercase();
                if [".jpg", ".jpeg", ".png", ".webp", ".avif"]
                    .iter()
                    .any(|ext| lower.contains(ext))
                {
                    found_src = Some(trimmed);
                }
            }
        }

        if let Some(src) = found_src {
            if let Ok(url) = provider::resolve(base, src) {
                if seen.insert(url.clone()) {
                    out.push(url);
                }
            }
        }
    }

    // Static JSON reader payloads (JSON scripts or Next.js streaming chunks)
    if out.is_empty() {
        for script in doc.select(&css("script").unwrap()) {
            let body = script.inner_html();
            if let Ok(v) = serde_json::from_str::<Value>(&body) {
                json_images(&v, false, base, &mut out, &mut seen);
            } else if body.contains("self.__next_f.push(") {
                for chunk in extract_next_f_chunks(&body) {
                    if let Ok(v) = serde_json::from_str::<Value>(&chunk) {
                        json_images(&v, false, base, &mut out, &mut seen);
                    }
                }
            }
        }
    }

    if out.is_empty() {
        Err("Nessuna scan estratta: controlla il selettore immagini. I lettori che richiedono JavaScript o verifiche possono richiedere un adattatore dedicato".into())
    } else {
        Ok(out)
    }
}
fn extract_next_f_chunks(body: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("self.__next_f.push(") {
        let after = &rest[start + "self.__next_f.push(".len()..];
        let raw = after.trim_end().trim_end_matches(';').trim_end_matches(')');
        if let Ok(Value::Array(parts)) = serde_json::from_str::<Value>(raw) {
            if let Some(Value::String(chunk)) = parts.get(1) {
                for line in chunk.lines() {
                    if let Some((_, data)) = line.split_once(':') {
                        chunks.push(data.to_string());
                    }
                }
            }
            break;
        } else if let Some(end) = after.find(");") {
            let chunk_raw = after[..end].trim().trim_end_matches(')');
            if let Ok(Value::Array(parts)) = serde_json::from_str::<Value>(chunk_raw) {
                if let Some(Value::String(chunk)) = parts.get(1) {
                    for line in chunk.lines() {
                        if let Some((_, data)) = line.split_once(':') {
                            chunks.push(data.to_string());
                        }
                    }
                }
            }
            rest = &after[end + 2..];
        } else {
            break;
        }
    }
    chunks
}
fn json_images(
    v: &Value,
    in_pages: bool,
    base: &str,
    out: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    match v {
        Value::Object(map) => {
            if in_pages {
                if let Some(u_str) = map
                    .get("url")
                    .or_else(|| map.get("src"))
                    .and_then(Value::as_str)
                {
                    if let Ok(u) = provider::resolve(base, u_str) {
                        let path = url::Url::parse(&u).unwrap().path().to_lowercase();
                        if [".jpg", ".jpeg", ".png", ".webp", ".avif"]
                            .iter()
                            .any(|e| path.ends_with(e))
                            && seen.insert(u.clone())
                        {
                            out.push(u);
                        }
                    }
                }
            }
            for (k, v) in map {
                json_images(
                    v,
                    in_pages
                        || [
                            "images",
                            "pages",
                            "chapterimages",
                            "chapter_images",
                            "scan",
                            "scans",
                        ]
                        .contains(&k.to_lowercase().as_str()),
                    base,
                    out,
                    seen,
                )
            }
        }
        Value::Array(a) => {
            for v in a {
                json_images(v, in_pages, base, out, seen)
            }
        }
        Value::String(s) if in_pages => {
            if let Ok(u) = provider::resolve(base, s) {
                let path = url::Url::parse(&u).unwrap().path().to_lowercase();
                if [".jpg", ".jpeg", ".png", ".webp", ".avif"]
                    .iter()
                    .any(|e| path.ends_with(e))
                    && seen.insert(u.clone())
                {
                    out.push(u);
                }
            }
        }
        _ => {}
    }
}

pub const REJECTED_ARCHIVE_EXTS: &[&str] = &[
    "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "cbz", "cbr", "tgz",
];

pub fn check_not_archive(path_str: &str) -> Result<(), String> {
    let lower = path_str.to_lowercase();
    for ext in REJECTED_ARCHIVE_EXTS {
        if lower.ends_with(&format!(".{ext}")) {
            return Err("I file archivio (ZIP, RAR, TAR, CBZ) non sono consentiti per motivi di sicurezza. Importa solo file connettore .json.".into());
        }
    }
    Ok(())
}

pub fn validate_connector_json(content: &str) -> Result<Source, String> {
    let lower = content.to_lowercase();
    if lower.contains("<script")
        || lower.contains("javascript:")
        || lower.contains("data:text/html")
    {
        return Err("Il connettore contiene codice non sicuro o script non consentiti".into());
    }

    let mut source: Source = serde_json::from_str(content)
        .map_err(|e| format!("Formato JSON connettore non valido: {e}"))?;

    if !source.base_url.starts_with("https://") {
        return Err("L'indirizzo della fonte deve utilizzare il protocollo sicuro HTTPS".into());
    }

    if source.id.trim().is_empty() {
        source.id = format!(
            "custom-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
    }

    // Validate CSS selector syntax if provided
    for (name, sel) in [
        ("risultati", &source.result_selector),
        ("capitoli", &source.chapter_selector),
        ("immagini", &source.image_selector),
    ] {
        if !sel.trim().is_empty() && scraper::Selector::parse(sel).is_err() {
            return Err(format!(
                "Sintassi selettore CSS non valida per {name}: {sel}"
            ));
        }
    }

    validate(&source)?;
    Ok(source)
}

pub fn import_connector_file(path_str: &str) -> Result<Source, String> {
    check_not_archive(path_str)?;
    let path = std::path::Path::new(path_str);
    if !path.is_file() {
        return Err("File connettore non trovato".into());
    }
    if path.extension().and_then(|e| e.to_str()) != Some("json") {
        return Err("È possibile importare solo file con estensione .json".into());
    }
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    validate_connector_json(&content)
}

pub fn import_connectors_from_folder(dir_str: &str) -> Result<Vec<Source>, String> {
    check_not_archive(dir_str)?;
    let dir = std::path::Path::new(dir_str);
    if !dir.is_dir() {
        return Err("Cartella connettori non trovata".into());
    }
    let mut imported = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Ok(content) = std::fs::read_to_string(&p) {
                if let Ok(source) = validate_connector_json(&content) {
                    imported.push(source);
                }
            }
        }
    }
    if imported.is_empty() {
        return Err("Nessun connettore .json valido trovato nella cartella".into());
    }
    Ok(imported)
}

pub async fn import_connectors_from_github(
    repo_or_url: &str,
    client: &reqwest::Client,
) -> Result<Vec<Source>, String> {
    check_not_archive(repo_or_url)?;
    let clean = repo_or_url
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("github.com/")
        .trim_end_matches(".git")
        .trim_end_matches('/');

    let parts: Vec<&str> = clean.split('/').collect();
    if parts.len() < 2 {
        return Err(
            "Formato repository non valido. Usa 'proprietario/repository' o l'URL GitHub.".into(),
        );
    }
    let owner = parts[0];
    let repo = parts[1];

    let api_url = format!("https://api.github.com/repos/{owner}/{repo}/contents/connectors");
    let resp = client
        .get(&api_url)
        .header(reqwest::header::USER_AGENT, "Pokko-App")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .send()
        .await;

    let mut imported = Vec::new();

    if let Ok(r) = resp {
        if r.status().is_success() {
            if let Ok(text) = r.text().await {
                if let Ok(files) = serde_json::from_str::<Vec<serde_json::Value>>(&text) {
                    for item in files {
                        if item["type"] == "file"
                            && item["name"].as_str().is_some_and(|n| n.ends_with(".json"))
                        {
                            if let Some(download_url) = item["download_url"].as_str() {
                                if let Ok(res) = client
                                    .get(download_url)
                                    .header(reqwest::header::USER_AGENT, "Pokko-App")
                                    .send()
                                    .await
                                {
                                    if let Ok(content) = res.text().await {
                                        if let Ok(source) = validate_connector_json(&content) {
                                            imported.push(source);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if imported.is_empty() {
        let raw_url =
            format!("https://raw.githubusercontent.com/{owner}/{repo}/main/resources/sources.json");
        if let Ok(res) = client
            .get(&raw_url)
            .header(reqwest::header::USER_AGENT, "Pokko-App")
            .send()
            .await
        {
            if res.status().is_success() {
                if let Ok(text) = res.text().await {
                    if let Ok(list) = serde_json::from_str::<Vec<Source>>(&text) {
                        for s in list {
                            if validate(&s).is_ok() {
                                imported.push(s);
                            }
                        }
                    }
                }
            }
        }
    }

    if imported.is_empty() {
        return Err(format!(
            "Nessun connettore trovato nel repository {owner}/{repo}. Verifica che contenga la cartella 'connectors/' con file .json."
        ));
    }

    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creators_from_live_comics_schema_are_deduplicated() {
        assert_eq!(
            structured_authors(include_str!(
                "../../tests/fixtures/readcomics-creators.html"
            )),
            vec!["Robert Kirkman", "Cory Walker"]
        );
        assert!(structured_authors("<p>Plot mentions Robert Kirkman</p>").is_empty());
        let draft = Source {
            id: "new".into(),
            base_url: "https://source.test".into(),
            search_path: String::new(),
            result_selector: String::new(),
            chapter_selector: String::new(),
            image_selector: String::new(),
            ..Source::default()
        };
        assert!(validate(&draft).is_ok());
        assert!(validate(&Source {
            enabled: true,
            ..draft
        })
        .is_err());
    }

    #[test]
    fn authors_stay_with_their_own_result_and_reader_pagination_is_separate() {
        let source = Source {
            result_selector: "a.title".into(),
            author_selector: ".author".into(),
            reader_next_selector: "a.scan-next".into(),
            ..Source::default()
        };
        let html = r#"<article><a class="title" href="/invincible">Invincible</a><span class="author">Robert Kirkman</span></article><article><a class="title" href="/other">Other</a><span class="author">Other Writer</span></article><a class="scan-next" href="?page=2">Next page</a><a class="next" href="/chapter/next">Next chapter</a>"#;
        let result = generic_search(html, "https://source.test/search", &source).unwrap();
        assert_eq!(result.items[0].authors, vec!["Robert Kirkman"]);
        assert_eq!(result.items[1].authors, vec!["Other Writer"]);
        assert_eq!(
            reader_next(html, "https://source.test/read/1", &source)
                .unwrap()
                .as_deref(),
            Some("https://source.test/read/1?page=2")
        );
        assert!(
            reader_next(html, "https://source.test/read/1", &Source::default())
                .unwrap()
                .is_none()
        );
        assert_eq!(
            page_link(html, "https://source.test/search", "a.next")
                .unwrap()
                .as_deref(),
            Some("https://source.test/chapter/next")
        );
    }

    #[test]
    fn mf_html() {
        let s = defaults().into_iter().find(|s| s.id == "mf").unwrap();
        let r = generic_search(
            include_str!("../../tests/fixtures/mf-search.html"),
            &s.base_url,
            &s,
        )
        .unwrap();
        assert!(r.items.iter().any(|m| m.title == "20th Century Boys"));
        let d = generic_detail(
            include_str!("../../tests/fixtures/mf-detail.html"),
            r.items[0].clone(),
            &s,
        )
        .unwrap();
        assert_eq!(d.volumes[0].chapters.len(), 242);
        assert!(d.volumes[0].chapters[0].title.starts_with("Chapter 1 -"));
        assert_eq!(
            generic_pages(
                include_str!("../../tests/fixtures/mf-reader.html"),
                &s.base_url,
                &s
            )
            .unwrap()
            .len(),
            36
        );
    }

    #[test]
    fn mangaread_html() {
        let source = defaults()
            .into_iter()
            .find(|s| s.id == "mangaread")
            .unwrap();
        let results = generic_search(
            include_str!("../../tests/fixtures/mangaread-search.html"),
            &source.base_url,
            &source,
        )
        .unwrap();
        let manga = results
            .items
            .into_iter()
            .find(|m| m.title == "One Piece Episode A")
            .unwrap();
        let detail = generic_detail(
            include_str!("../../tests/fixtures/mangaread-detail.html"),
            manga,
            &source,
        )
        .unwrap();
        assert_eq!(detail.volumes[0].chapters.len(), 5);
        let pages = generic_pages(
            include_str!("../../tests/fixtures/mangaread-reader.html"),
            &source.base_url,
            &source,
        )
        .unwrap();
        assert_eq!(pages.len(), 46);
        assert!(pages[0].ends_with("/2.jpeg"));
    }

    #[test]
    fn mangaworld_generic_html() {
        let s = defaults().into_iter().find(|s| s.id == "mw").unwrap();
        assert_eq!(s.adapter, "generic");
        let r = generic_search(
            include_str!("../../tests/fixtures/archive.html"),
            &s.base_url,
            &s,
        )
        .unwrap();
        assert_eq!(r.items.len(), 6);
        assert!(r
            .items
            .iter()
            .all(|m| m.cover.starts_with("https://cdn.mangaworld.")));
        assert!(r.items.iter().any(|m| m.title == "One Piece"));

        let manga = r
            .items
            .into_iter()
            .find(|m| m.title == "One Piece")
            .unwrap();
        let detail =
            generic_detail(include_str!("../../tests/fixtures/detail.html"), manga, &s).unwrap();
        assert_eq!(detail.volumes.len(), 116);
        assert_eq!(detail.volumes[0].title, "Volume 01");
        assert_eq!(detail.volumes[0].chapters[0].title, "Capitolo 00");

        let pages = generic_pages(
            include_str!("../../tests/fixtures/reader.html"),
            &s.base_url,
            &s,
        )
        .unwrap();
        assert_eq!(pages.len(), 23);
        assert!(pages[3].ends_with("4.png"));
    }

    #[test]
    fn readberserk_html() {
        let source: Source =
            serde_json::from_str(include_str!("../../connectors/readberserk.json")).unwrap();
        let manga = Manga {
            title: "Berserk".into(),
            url: "https://readberserk.com".into(),
            cover: "".into(),
            source_id: "readberserk".into(),
            source_name: "ReadBerserk".into(),
            language: "en".into(),
            authors: vec![],
        };
        let detail = generic_detail(
            include_str!("../../tests/fixtures/readberserk-detail.html"),
            manga,
            &source,
        )
        .unwrap();
        assert_eq!(detail.volumes[0].chapters.len(), 10);
        assert_eq!(detail.volumes[0].chapters[0].title, "Berserk Chapter 377");
        assert_eq!(detail.volumes[0].chapters[9].title, "Berserk Chapter 386");
        assert!(detail.volumes[0].chapters[9]
            .url
            .contains("berserk-chapter-386"));

        let pages = generic_pages(
            include_str!("../../tests/fixtures/readberserk-reader.html"),
            &source.base_url,
            &source,
        )
        .unwrap();
        assert_eq!(pages.len(), 24);
        assert!(pages[0].ends_with("/1.jpeg"));
        assert!(pages[23].ends_with("/24.jpeg"));
    }

    #[test]
    fn generic_json() {
        let s = Source::default();
        assert_eq!(
            generic_pages(
                r#"<script type="application/json">{"images":["https://cdn.test/1.jpg","https://cdn.test/2.png"]}</script>"#,
                "https://example.org",
                &s
            )
            .unwrap()
            .len(),
            2
        );
        assert!(generic_pages("<img src='ads.jpg'>", "https://example.org", &s).is_err());
    }

    #[test]
    fn next_js_streaming_payload() {
        let s = Source {
            image_selector: "img.page".into(),
            ..Source::default()
        };
        let pages = generic_pages(
            include_str!("../../tests/fixtures/readcomics-reader.html"),
            "https://readcomicsonline.lol",
            &s,
        )
        .unwrap();
        assert_eq!(pages.len(), 27);
        assert!(pages[26].ends_with("p027.webp"));
    }

    #[test]
    fn source_validation() {
        for s in defaults() {
            validate(&s).unwrap();
        }
        let s = Source {
            result_selector: "[".into(),
            ..Source::default()
        };
        assert!(validate(&s).is_err());
    }

    #[test]
    fn connector_archive_rejection_and_json_validation() {
        assert!(check_not_archive("malicious.zip").is_err());
        assert!(check_not_archive("package.cbz").is_err());
        assert!(check_not_archive("source.tar.gz").is_err());
        assert!(check_not_archive("valid.json").is_ok());

        let valid_json = r#"{
            "id": "test",
            "name": "Test Source",
            "base_url": "https://example.com",
            "category": "manga",
            "language": "en",
            "adapter": "generic",
            "min_delay_ms": 1000
        }"#;
        assert!(validate_connector_json(valid_json).is_ok());

        let unsafe_json = r#"{
            "id": "test",
            "name": "<script>alert(1)</script>",
            "base_url": "https://example.com",
            "category": "manga",
            "language": "en",
            "adapter": "generic",
            "min_delay_ms": 1000
        }"#;
        assert!(validate_connector_json(unsafe_json).is_err());

        let insecure_http = r#"{
            "id": "test",
            "name": "HTTP Source",
            "base_url": "http://example.com",
            "category": "manga",
            "language": "en",
            "adapter": "generic",
            "min_delay_ms": 1000
        }"#;
        assert!(validate_connector_json(insecure_http).is_err());
    }
}
