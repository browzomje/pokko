use crate::{
    provider::{self, Chapter, Detail, Manga, SearchResult, Volume},
    Engine,
};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Source {
    pub author_selector: String,
    pub reader_next_selector: String,
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
}
#[derive(Clone, Serialize, Deserialize)]
pub struct RecordedStep {
    pub kind: String,
    pub url: String,
    pub selector: String,
}
impl Default for Source {
    fn default() -> Self {
        Self { author_selector:"[rel=author], .author, .author-content".into(), reader_next_selector:String::new(), workflow:Vec::new(), category:"manga".into(), id:String::new(),name:"Nuova fonte".into(),base_url:"https://".into(),language:"en".into(),adapter:"generic".into(),enabled:false,min_delay_ms:1000,note:"HTML generico: verifica i selettori sul sito".into(),search_path:"/search?keyword={query}&page={page}".into(),result_selector:".manga-title, .post-title a, .manga-name a, h3 a, .story_name a".into(),chapter_selector:"a.chap, a.chapter-link, .wp-manga-chapter a, .chapter-list a, .row-content-chapter a".into(),image_selector:"img.page-image, .reading-content img, .chapter-content img, #readerarea img, .read_img img, img#gohere".into(),description_selector:"#noidungm, .description-summary, .summary__content, .manga_series_description".into(),next_selector:"a[rel=next], .pagination .next a, a.next".into() }
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
    if !["it", "en", "both"].contains(&s.language.as_str())
        || !["generic", "mangaworld", "mangadex", "readcomics"].contains(&s.adapter.as_str())
    {
        return Err("Lingua o adattatore non valido".into());
    }
    if !(500..=120000).contains(&s.min_delay_ms) {
        return Err("Intervallo fonte consentito: 0,5–120 secondi".into());
    }
    if s.adapter == "generic" {
        if s.language == "both" {
            return Err("Per HTML generico scegli una lingua precisa: italiano o inglese".into());
        }
        if (s.enabled || !s.search_path.is_empty()) && !s.search_path.contains("{query}") {
            return Err("Il percorso ricerca deve contenere {query}".into());
        }
        if s.enabled
            && [&s.result_selector, &s.chapter_selector, &s.image_selector]
                .iter()
                .any(|field| field.trim().is_empty())
        {
            return Err(
                "Completa i selettori di ricerca, capitoli e immagini prima di abilitare la fonte"
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
        ] {
            if !value.is_empty() {
                css(value)?;
            }
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
    if s.adapter == "mangadex" {
        return Ok(manga.authors.clone());
    }
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
    if s.adapter == "mangadex" {
        return md_search(engine, s, query, page, delay).await;
    }
    let base = provider::site(&s.base_url)?;
    let query: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
    let path = if s.adapter == "mangaworld" {
        format!("/archive?keyword={query}&page={page}")
    } else {
        s.search_path
            .replace("{query}", &query)
            .replace("{page}", &page.to_string())
    };
    let url = provider::resolve(&base, &path)?;
    let response = engine
        .request(&url, &base, delay.max(s.min_delay_ms), None)
        .await?;
    let mut final_url = response.url().to_string();
    let mut html = response.text().await.map_err(|e| e.to_string())?;
    if s.adapter == "generic" && page > 1 && !s.search_path.contains("{page}") {
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
    let result = if s.adapter == "readcomics" {
        comics_search(&html, &final_url, s)?
    } else if s.adapter == "mangaworld" {
        provider::search(&html, &final_url, page)?
    } else {
        generic_search(&html, &final_url, s)?
    };
    Ok(decorate(result, s))
}
// Decode the site's static React payload as JSON, without executing scripts.
fn comics_payloads(html: &str) -> Vec<Value> {
    let doc = Html::parse_document(html);
    let mut result = Vec::new();
    let mut chunks = String::new();
    for script in doc.select(&css("script").unwrap()) {
        let body = script.inner_html();
        if let Some(start) = body.find("self.__next_f.push(") {
            let raw = body[start + "self.__next_f.push(".len()..]
                .trim_end_matches(';')
                .trim_end_matches(')');
            if let Ok(Value::Array(parts)) = serde_json::from_str::<Value>(raw) {
                if let Some(Value::String(chunk)) = parts.get(1) {
                    chunks.push_str(chunk);
                }
            }
        }
    }
    for line in chunks.lines() {
        if let Some((_, data)) = line.split_once(':') {
            if let Ok(value) = serde_json::from_str::<Value>(data) {
                result.push(value);
            }
        }
    }
    result
}
fn find_array<'a>(value: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    match value {
        Value::Object(map) => map
            .get(key)
            .and_then(Value::as_array)
            .or_else(|| map.values().find_map(|v| find_array(v, key))),
        Value::Array(array) => array.iter().find_map(|v| find_array(v, key)),
        _ => None,
    }
}
pub fn comics_search(html: &str, base: &str, _source: &Source) -> Result<SearchResult, String> {
    let doc = Html::parse_document(html);
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for link in doc.select(&css("a[data-smartlink][href^='/comic/']")?) {
        let Some(href) = link.value().attr("href") else {
            continue;
        };
        let title = link
            .select(&css("h2, h3")?)
            .next()
            .map(text)
            .unwrap_or_else(|| text(link));
        let url = provider::resolve(base, href)?;
        if title.is_empty() || !seen.insert(url.clone()) {
            continue;
        }
        let cover = link
            .select(&css("img")?)
            .next()
            .and_then(|i| i.value().attr("src"))
            .and_then(|v| provider::resolve(base, v).ok())
            .unwrap_or_default();
        items.push(Manga {
            title,
            url,
            cover,
            ..Manga::default()
        });
    }
    if items.is_empty()
        && !html.to_lowercase().contains("no results")
        && !html.contains("No comics")
    {
        return Err(
            "Risultati comics non riconosciuti: la fonte potrebbe aver cambiato struttura".into(),
        );
    }
    let has_next = doc
        .select(&css("a[href]")?)
        .any(|a| text(a).to_lowercase() == "next");
    Ok(SearchResult { items, has_next })
}
pub fn comics_detail(html: &str, mut manga: Manga) -> Result<Detail, String> {
    let doc = Html::parse_document(html);
    if let Some(heading) = doc.select(&css("h1")?).next() {
        manga.title = text(heading);
    }
    manga.authors = doc
        .select(&css(
            "a[href*='/author/'], a[href*='/writer/'], [itemprop='author'], .author-content a",
        )?)
        .map(text)
        .filter(|name| !name.is_empty())
        .collect();
    if let Some(meta) = doc
        .select(&css("meta[name='author']")?)
        .next()
        .and_then(|el| el.value().attr("content"))
    {
        if !meta.trim().is_empty() {
            manga.authors.push(meta.into());
        }
    }
    manga.authors.extend(structured_authors(html));
    let mut unique = HashSet::new();
    manga
        .authors
        .retain(|name| unique.insert(name.to_lowercase()));
    let payloads = comics_payloads(html);
    let issues = payloads
        .iter()
        .find_map(|v| find_array(v, "issues"))
        .ok_or("Elenco albi non trovato nella fonte comics")?;
    let mut volumes = Vec::new();
    let mut seen = HashSet::new();
    for issue in issues {
        let title = issue["title"].as_str().ok_or("Titolo albo mancante")?;
        let slug = issue["slug"]
            .as_str()
            .ok_or("Identificatore albo mancante")?;
        if !seen.insert(slug) {
            continue;
        }
        let url = format!("{}/{}", manga.url.trim_end_matches('/'), slug);
        provider::site(&url)?;
        let label = title
            .split_once("_TPB ")
            .map(|(_, tail)| format!("Volume {}", tail))
            .unwrap_or_else(|| title.to_string());
        volumes.push(Volume {
            title: label,
            chapters: vec![Chapter {
                title: title.into(),
                url,
                page_count: None,
            }],
        });
    }
    if volumes.is_empty() {
        return Err("Nessun albo scaricabile trovato".into());
    }
    volumes.sort_by(|a, b| {
        let group = |v: &Volume| if v.title.starts_with("Volume ") { 0 } else { 1 };
        group(a)
            .cmp(&group(b))
            .then_with(|| comics_number(&a.title).cmp(&comics_number(&b.title)))
    });
    let doc = Html::parse_document(html);
    let description = doc
        .select(&css("meta[name=description]")?)
        .next()
        .and_then(|e| e.value().attr("content"))
        .unwrap_or("")
        .to_string();
    Ok(Detail {
        manga,
        description,
        volumes,
    })
}
fn comics_number(title: &str) -> u64 {
    let tail = title
        .split_once("Volume ")
        .or_else(|| title.rsplit_once('#'))
        .map(|(_, tail)| tail)
        .unwrap_or(title);
    tail.split(|c: char| !c.is_ascii_digit())
        .find(|s| !s.is_empty())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}
pub fn comics_pages(html: &str) -> Result<Vec<String>, String> {
    let payloads = comics_payloads(html);
    let raw = payloads
        .iter()
        .find_map(|v| find_array(v, "pages"))
        .ok_or("Elenco completo delle pagine comics non disponibile")?;
    let mut pages = Vec::new();
    for page in raw {
        let number = page["pageNumber"]
            .as_u64()
            .ok_or("Numero pagina comics mancante")?;
        let url = page["url"].as_str().ok_or("URL pagina comics mancante")?;
        provider::site(url)?;
        let parsed = url::Url::parse(url).map_err(|e| e.to_string())?;
        if parsed.host_str() != Some("cdn.readcomicsonline.lol")
            || !parsed.path().starts_with("/pages/")
        {
            return Err("Indirizzo immagine comics non riconosciuto".into());
        }
        pages.push((number, url.to_string()));
    }
    pages.sort_by_key(|(n, _)| *n);
    let mut seen = HashSet::new();
    if pages.is_empty()
        || pages
            .iter()
            .enumerate()
            .any(|(i, (n, url))| *n != i as u64 + 1 || !seen.insert(url.clone()))
    {
        return Err(
            "La fonte comics segnala pagine mancanti o duplicate: download interrotto".into(),
        );
    }
    Ok(pages.into_iter().map(|(_, url)| url).collect())
}
pub fn generic_search(html: &str, base: &str, s: &Source) -> Result<SearchResult, String> {
    let doc = Html::parse_document(html);
    let mut seen = HashSet::new();
    let mut items = Vec::new();
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
            .find_map(|el| el.select(&css("img").unwrap()).next());
        let cover = image
            .and_then(|i| {
                i.value()
                    .attr("data-src")
                    .filter(|v| !v.is_empty())
                    .or(i.value().attr("src"))
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
    if s.adapter == "mangadex" {
        return md_detail(engine, s, manga, delay).await;
    }
    let response = engine
        .request(&manga.url, &s.base_url, delay.max(s.min_delay_ms), None)
        .await?;
    let html = response.text().await.map_err(|e| e.to_string())?;
    let mut detail = if s.adapter == "readcomics" {
        comics_detail(&html, manga)?
    } else if s.adapter == "mangaworld" {
        provider::detail(&html, manga)?
    } else {
        generic_detail(&html, manga, s)?
    };
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
    let mut seen = HashSet::new();
    let mut chapters = Vec::new();
    for a in doc.select(&css(&s.chapter_selector)?) {
        let Some(href) = a.value().attr("href") else {
            continue;
        };
        let url = provider::resolve(&manga.url, href)?;
        if seen.insert(url.clone()) {
            chapters.push(Chapter {
                page_count: None,
                title: text(a),
                url,
            });
        }
    }
    if chapters.is_empty() {
        return Err("Nessun capitolo trovato: modifica il selettore capitoli della fonte".into());
    }
    // Order numerical chapter labels, preserving original order for labels without numbers.
    chapters.sort_by(|a, b| number(&a.title).total_cmp(&number(&b.title)));
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
        volumes: vec![Volume {
            title: "Capitoli".into(),
            chapters,
        }],
    })
}
fn number(s: &str) -> f64 {
    let lower = s.to_lowercase();
    let start = ["chapter", "capitolo", "ch."]
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
pub async fn pages(
    engine: &Engine,
    s: &Source,
    chapter: &Chapter,
    referer: &str,
    delay: u64,
    id: Option<&str>,
) -> Result<Vec<String>, String> {
    if s.adapter == "mangadex" {
        let cid = identifier(&chapter.url, "chapter")?;
        let url = format!("https://api.mangadex.org/at-home/server/{cid}");
        let v = json(engine, &url, delay.max(s.min_delay_ms), id).await?;
        return md_pages(&v);
    }
    let mut url = url::Url::parse(&chapter.url).map_err(|e| e.to_string())?;
    if s.adapter == "mangaworld" {
        url.query_pairs_mut().append_pair("style", "list");
    }
    let response = engine
        .request(url.as_str(), referer, delay.max(s.min_delay_ms), id)
        .await?;
    let base = response.url().to_string();
    let html = response.text().await.map_err(|e| e.to_string())?;
    if s.adapter == "readcomics" {
        comics_pages(&html)
    } else if s.adapter == "mangaworld" {
        provider::pages(&html, &base)
    } else {
        let mut pages = generic_pages(&html, &base, s)?;
        let mut next = reader_next(&html, &base, s)?;
        let mut visited = HashSet::from([base.clone()]);
        while let Some(url) = next {
            if visited.len() >= 100 || !visited.insert(url.clone()) {
                return Err("Paginazione lettore ciclica o superiore a 100 pagine HTML: controlla il selettore pagina successiva".into());
            }
            let response = engine
                .request(&url, &base, delay.max(s.min_delay_ms), id)
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
    for img in doc.select(&css(&s.image_selector)?) {
        if let Some(src) = img
            .value()
            .attr("data-src")
            .filter(|v| !v.is_empty())
            .or(img.value().attr("data-original"))
            .or(img.value().attr("src"))
        {
            if let Ok(url) = provider::resolve(base, src.trim()) {
                if seen.insert(url.clone()) {
                    out.push(url);
                }
            }
        }
    }
    // Static JSON reader payloads are supported; scripts are never executed.
    if out.is_empty() {
        for script in doc.select(&css(
            "script[type='application/json'], script#__NEXT_DATA__",
        )?) {
            if let Ok(v) = serde_json::from_str::<Value>(&script.inner_html()) {
                json_images(&v, false, base, &mut out, &mut seen);
            }
        }
    }
    if out.is_empty() {
        Err("Nessuna scan estratta: controlla il selettore immagini. I lettori che richiedono JavaScript o verifiche possono richiedere un adattatore dedicato".into())
    } else {
        Ok(out)
    }
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
            for (k, v) in map {
                json_images(
                    v,
                    in_pages || ["images", "pages", "chapterImages"].contains(&k.as_str()),
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
async fn json(engine: &Engine, url: &str, delay: u64, id: Option<&str>) -> Result<Value, String> {
    let text = engine
        .request(url, "https://mangadex.org", delay, id)
        .await?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    let value: Value =
        serde_json::from_str(&text).map_err(|_| "La fonte non ha restituito JSON valido")?;
    if value.get("result").and_then(Value::as_str) == Some("error") {
        return Err(format!(
            "Errore API MangaDex: {}",
            value.get("errors").unwrap_or(&Value::Null)
        ));
    }
    Ok(value)
}
fn identifier(url: &str, kind: &str) -> Result<String, String> {
    let u = url::Url::parse(url).map_err(|e| e.to_string())?;
    let parts = u
        .path_segments()
        .ok_or("URL MangaDex non valido")?
        .collect::<Vec<_>>();
    let id = parts
        .iter()
        .position(|p| *p == kind)
        .and_then(|i| parts.get(i + 1))
        .ok_or("Identificatore MangaDex mancante")?;
    if id.len() != 36 || !id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return Err("Identificatore MangaDex non valido".into());
    }
    Ok(id.to_string())
}
async fn md_search(
    engine: &Engine,
    s: &Source,
    query: &str,
    page: u32,
    delay: u64,
) -> Result<SearchResult, String> {
    let mut u = url::Url::parse("https://api.mangadex.org/manga").unwrap();
    u.query_pairs_mut()
        .append_pair("title", query)
        .append_pair("limit", "24")
        .append_pair("offset", &((page.saturating_sub(1)) * 24).to_string())
        .append_pair("includes[]", "cover_art")
        .append_pair("includes[]", "author")
        .append_pair("includes[]", "artist")
        .append_pair("contentRating[]", "safe")
        .append_pair("contentRating[]", "suggestive")
        .append_pair("availableTranslatedLanguage[]", "it")
        .append_pair("availableTranslatedLanguage[]", "en")
        .append_pair("order[relevance]", "desc");
    let v = json(engine, u.as_str(), delay.max(s.min_delay_ms), None).await?;
    let mut items = Vec::new();
    for m in v["data"].as_array().ok_or("Catalogo MangaDex non valido")? {
        let id = m["id"].as_str().unwrap_or_default();
        let titles = &m["attributes"]["title"];
        let title = titles
            .get("en")
            .or_else(|| titles.get("ja-ro"))
            .or_else(|| titles.as_object().and_then(|o| o.values().next()))
            .and_then(Value::as_str)
            .unwrap_or("Manga")
            .to_string();
        let filename = m["relationships"]
            .as_array()
            .and_then(|r| r.iter().find(|r| r["type"] == "cover_art"))
            .and_then(|r| r["attributes"]["fileName"].as_str());
        let cover = filename
            .map(|f| format!("https://uploads.mangadex.org/covers/{id}/{f}.256.jpg"))
            .unwrap_or_default();
        let languages = m["attributes"]["availableTranslatedLanguages"].as_array();
        for lang in ["it", "en"] {
            if s.language != "both" && s.language != lang {
                continue;
            }
            if !languages
                .map(|a| a.iter().any(|l| l == lang))
                .unwrap_or(false)
            {
                continue;
            }
            items.push(Manga {
                title: title.clone(),
                url: format!("https://mangadex.org/title/{id}?lang={lang}"),
                cover: cover.clone(),
                source_id: s.id.clone(),
                source_name: s.name.clone(),
                language: lang.into(),
                authors: m["relationships"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|r| r["type"] == "author" || r["type"] == "artist")
                    .filter_map(|r| r["attributes"]["name"].as_str().map(str::to_string))
                    .collect(),
            });
        }
    }
    let total = v["total"].as_u64().unwrap_or(0);
    Ok(SearchResult {
        items,
        has_next: (page as u64) * 24 < total,
    })
}
async fn md_detail(
    engine: &Engine,
    s: &Source,
    manga: Manga,
    delay: u64,
) -> Result<Detail, String> {
    let mid = identifier(&manga.url, "title")?;
    let lang = if manga.language == "it" { "it" } else { "en" };
    let v = json(
        engine,
        &format!("https://api.mangadex.org/manga/{mid}"),
        delay.max(s.min_delay_ms),
        None,
    )
    .await?;
    let description = v["data"]["attributes"]["description"]
        .get(lang)
        .or_else(|| v["data"]["attributes"]["description"].get("en"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let mut offset = 0;
    let mut groups: HashMap<String, Vec<Chapter>> = HashMap::new();
    let mut seen = HashSet::new();
    loop {
        let mut u = url::Url::parse(&format!("https://api.mangadex.org/manga/{mid}/feed")).unwrap();
        u.query_pairs_mut()
            .append_pair("translatedLanguage[]", lang)
            .append_pair("limit", "500")
            .append_pair("offset", &offset.to_string())
            .append_pair("order[volume]", "asc")
            .append_pair("order[chapter]", "asc")
            .append_pair("order[publishAt]", "desc")
            .append_pair("includeExternalUrl", "0");
        let feed = json(engine, u.as_str(), delay.max(s.min_delay_ms), None).await?;
        let data = feed["data"]
            .as_array()
            .ok_or("Elenco capitoli MangaDex non valido")?;
        for c in data {
            let a = &c["attributes"];
            if a["pages"].as_u64().unwrap_or(0) == 0 || a["translatedLanguage"] != lang {
                continue;
            }
            let vol = a["volume"].as_str().filter(|s| !s.is_empty()).unwrap_or("");
            let ch = a["chapter"].as_str().unwrap_or("");
            let key = if ch.is_empty() {
                c["id"].to_string()
            } else {
                format!("{vol}:{ch}")
            };
            if !seen.insert(key) {
                continue;
            }
            let title = a["title"].as_str().unwrap_or("");
            let label = if ch.is_empty() {
                title.to_string()
            } else if title.is_empty() {
                format!("Capitolo {ch}")
            } else {
                format!("Capitolo {ch} · {title}")
            };
            groups.entry(vol.into()).or_default().push(Chapter {
                page_count: a["pages"].as_u64().filter(|n| *n > 0).map(|n| n as usize),
                title: label,
                url: format!(
                    "https://mangadex.org/chapter/{}",
                    c["id"].as_str().unwrap_or_default()
                ),
            });
        }
        offset += data.len();
        if data.is_empty() || offset >= feed["total"].as_u64().unwrap_or(0) as usize {
            break;
        }
        if offset >= 10000 {
            return Err(
                "Limite API MangaDex raggiunto: l’elenco non è stato troncato silenziosamente"
                    .into(),
            );
        }
    }
    let mut entries = groups.into_iter().collect::<Vec<_>>();
    entries.sort_by(|a, b| number(&a.0).total_cmp(&number(&b.0)));
    let volumes = entries
        .into_iter()
        .map(|(v, chapters)| Volume {
            title: if v.is_empty() {
                "Capitoli senza volume".into()
            } else {
                format!("Volume {v}")
            },
            chapters,
        })
        .collect::<Vec<_>>();
    if volumes.is_empty() {
        return Err(format!("Nessun capitolo scaricabile in {lang}: il catalogo può includere traduzioni rimosse o collegamenti esterni"));
    }
    Ok(Detail {
        manga,
        description,
        volumes,
    })
}
fn md_pages(v: &Value) -> Result<Vec<String>, String> {
    let base = v["baseUrl"]
        .as_str()
        .ok_or("Server immagini MangaDex mancante")?;
    let hash = v["chapter"]["hash"]
        .as_str()
        .ok_or("Hash capitolo mancante")?;
    let data = v["chapter"]["data"]
        .as_array()
        .ok_or("Pagine MangaDex mancanti")?;
    let pages = data
        .iter()
        .filter_map(Value::as_str)
        .map(|f| provider::resolve(base, &format!("/data/{hash}/{f}")))
        .collect::<Result<Vec<_>, _>>()?;
    if pages.is_empty() {
        Err("Il capitolo non contiene immagini disponibili".into())
    } else {
        Ok(pages)
    }
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
    fn comics_collections_and_complete_page_payload() {
        let source = defaults()
            .into_iter()
            .find(|s| s.id == "readcomics")
            .unwrap();
        let search = comics_search(
            include_str!("../../tests/fixtures/readcomics-search.html"),
            &source.base_url,
            &source,
        )
        .unwrap();
        assert_eq!(search.items.len(), 1);
        assert_eq!(search.items[0].title, "Invincible (2003)");
        let detail = comics_detail(
            include_str!("../../tests/fixtures/readcomics-detail.html"),
            search.items[0].clone(),
        )
        .unwrap();
        assert_eq!(detail.volumes.len(), 170);
        assert!(detail.volumes[0].title.starts_with("Volume 1 -"));
        assert!(detail
            .volumes
            .iter()
            .any(|v| v.title.starts_with("Volume 25 -")));
        let pages =
            comics_pages(include_str!("../../tests/fixtures/readcomics-reader.html")).unwrap();
        assert_eq!(pages.len(), 27);
        assert!(pages[26].ends_with("p027.webp"));
        let broken = include_str!("../../tests/fixtures/readcomics-reader.html")
            .replace("pageNumber\\\":2,", "pageNumber\\\":3,");
        assert!(comics_pages(&broken).is_err());
    }
    #[test]
    fn generic_json() {
        let s = Source::default();
        assert_eq!(generic_pages(r#"<script type="application/json">{"images":["https://cdn.test/1.jpg","https://cdn.test/2.png"]}</script>"#,"https://example.org",&s).unwrap().len(),2);
        assert!(generic_pages("<img src='ads.jpg'>", "https://example.org", &s).is_err());
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
    fn md_payload() {
        let v = serde_json::json!({"baseUrl":"https://uploads.test","chapter":{"hash":"abc","data":["1.jpg","2.png"]}});
        assert_eq!(
            md_pages(&v).unwrap()[0],
            "https://uploads.test/data/abc/1.jpg"
        );
    }
}
