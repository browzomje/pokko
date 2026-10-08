use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use url::Url;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Manga {
    #[serde(default)]
    pub authors: Vec<String>,
    pub title: String,
    pub url: String,
    pub cover: String,
    #[serde(default)]
    pub source_id: String,
    #[serde(default)]
    pub source_name: String,
    #[serde(default = "italian")]
    pub language: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chapter {
    #[serde(default)]
    pub page_count: Option<usize>,
    pub title: String,
    pub url: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Volume {
    pub title: String,
    pub chapters: Vec<Chapter>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Detail {
    pub manga: Manga,
    pub description: String,
    pub volumes: Vec<Volume>,
}
#[derive(Serialize)]
pub struct SearchResult {
    pub items: Vec<Manga>,
    pub has_next: bool,
}
fn italian() -> String {
    "it".into()
}
fn selector(s: &str) -> Selector {
    Selector::parse(s).unwrap()
}
fn text(e: scraper::ElementRef<'_>) -> String {
    e.text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn resolve(base: &str, value: &str) -> Result<String, String> {
    let u = Url::parse(base)
        .map_err(|e| e.to_string())?
        .join(value)
        .map_err(|e| e.to_string())?;
    if u.scheme() != "https"
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
    {
        return Err("È richiesto un URL HTTPS valido".into());
    }
    Ok(u.to_string())
}
pub fn site(base: &str) -> Result<String, String> {
    let u = Url::parse(base).map_err(|_| "Indirizzo del sito non valido")?;
    let host = u.host_str().unwrap_or("");
    if u.scheme() != "https"
        || host.is_empty()
        || !u.username().is_empty()
        || u.password().is_some()
    {
        return Err("Usa un dominio HTTPS valido".into());
    }
    Ok(format!("https://{host}"))
}
pub fn search(html: &str, base: &str, page: u32) -> Result<SearchResult, String> {
    let doc = Html::parse_document(html);
    if doc.select(&selector("a.manga-title")).next().is_none()
        && doc.select(&selector(".comics-grid")).next().is_none()
    {
        return Err(
            "Catalogo MangaWorld non riconosciuto: possibile blocco o struttura cambiata".into(),
        );
    }
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for a in doc.select(&selector("a.manga-title")) {
        let Some(href) = a.value().attr("href") else {
            continue;
        };
        let url = resolve(base, href)?;
        if !seen.insert(url.clone()) {
            continue;
        }
        let cover = a
            .ancestors()
            .filter_map(scraper::ElementRef::wrap)
            .find(|p| p.value().classes().any(|c| c == "entry"))
            .and_then(|p| p.select(&selector("img")).next())
            .and_then(|i| i.value().attr("data-src").or(i.value().attr("src")))
            .and_then(|s| resolve(base, s).ok())
            .unwrap_or_default();
        items.push(Manga {
            title: a
                .value()
                .attr("title")
                .map(String::from)
                .unwrap_or_else(|| text(a)),
            url,
            cover,
            ..Manga::default()
        });
    }
    let has_next = doc.select(&selector("a[href]")).any(|a| {
        a.value()
            .attr("href")
            .and_then(|h| resolve(base, h).ok())
            .and_then(|h| Url::parse(&h).ok())
            .map(|u| {
                u.query_pairs()
                    .any(|(k, v)| k == "page" && v.parse::<u32>().unwrap_or(0) > page)
            })
            .unwrap_or(false)
    });
    Ok(SearchResult { items, has_next })
}
pub fn detail(html: &str, manga: Manga) -> Result<Detail, String> {
    let doc = Html::parse_document(html);
    let mut volumes = Vec::new();
    for el in doc.select(&selector(".volume-element")) {
        let title = el
            .select(&selector(".volume-name"))
            .next()
            .map(text)
            .unwrap_or("Senza volume".into());
        let mut chapters = Vec::new();
        let mut seen = HashSet::new();
        for a in el.select(&selector("a.chap")) {
            if let Some(href) = a.value().attr("href") {
                let url = resolve(&manga.url, href)?;
                if seen.insert(url.clone()) {
                    chapters.push(Chapter {
                        page_count: None,
                        title: a
                            .select(&selector("span"))
                            .next()
                            .map(text)
                            .unwrap_or_else(|| text(a)),
                        url,
                    });
                }
            }
        }
        chapters.reverse();
        if !chapters.is_empty() {
            volumes.push(Volume { title, chapters });
        }
    }
    volumes.reverse();
    if volumes.is_empty() {
        return Err("Nessun volume trovato: il sito potrebbe aver cambiato struttura o richiesto una verifica".into());
    }
    let description = doc
        .select(&selector("#noidungm"))
        .next()
        .map(text)
        .unwrap_or_default();
    Ok(Detail {
        manga,
        description,
        volumes,
    })
}
pub fn pages(html: &str, base: &str) -> Result<Vec<String>, String> {
    let doc = Html::parse_document(html);
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for img in doc.select(&selector("img.page-image")) {
        if let Some(src) = img
            .value()
            .attr("data-src")
            .filter(|s| !s.is_empty())
            .or(img.value().attr("src"))
        {
            let url = resolve(base, src)?;
            if seen.insert(url.clone()) {
                out.push(url);
            }
        }
    }
    if out.is_empty() {
        Err("Nessuna scan trovata nel lettore. Possibile blocco o struttura cambiata".into())
    } else {
        Ok(out)
    }
}
pub fn safe_name(s: &str) -> String {
    let name: String = s
        .chars()
        .map(|c| {
            if c.is_control() || "/\\:*?\"<>|".contains(c) {
                '_'
            } else {
                c
            }
        })
        .take(120)
        .collect();
    let name = name.trim().trim_matches('.');
    if name.is_empty() {
        "Senza nome".into()
    } else {
        name.into()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_archive() {
        let r = search(
            include_str!("../../tests/fixtures/archive.html"),
            "https://www.mangaworld.mx",
            1,
        )
        .unwrap();
        assert_eq!(r.items.len(), 6);
        assert!(r
            .items
            .iter()
            .all(|m| m.cover.starts_with("https://cdn.mangaworld.")));
        assert!(r.items.iter().any(|m| m.title == "One Piece"));
    }
    #[test]
    fn real_detail() {
        let r = detail(
            include_str!("../../tests/fixtures/detail.html"),
            Manga {
                title: "One Piece".into(),
                url: "https://www.mangaworld.mx/manga/1708/one-piece".into(),
                cover: String::new(),
                ..Manga::default()
            },
        )
        .unwrap();
        assert_eq!(r.volumes.len(), 116);
        assert_eq!(r.volumes[0].title, "Volume 01");
        assert_eq!(r.volumes[0].chapters[0].title, "Capitolo 00");
    }
    #[test]
    fn real_reader() {
        let p = pages(
            include_str!("../../tests/fixtures/reader.html"),
            "https://www.mangaworld.mx",
        )
        .unwrap();
        assert_eq!(p.len(), 23);
        assert!(p[3].ends_with("4.png"));
    }
    #[test]
    fn safe_paths() {
        assert_eq!(safe_name("../../a/b"), "_.._a_b");
        assert_eq!(safe_name(".."), "Senza nome");
        assert!(site("http://www.mangaworld.mx").is_err());
        assert!(resolve("https://www.mangaworld.mx", "file:///etc/passwd").is_err());
    }
}
