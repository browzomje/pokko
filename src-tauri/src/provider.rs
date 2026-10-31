use serde::{Deserialize, Serialize};
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
    fn safe_paths() {
        assert_eq!(safe_name("../../a/b"), "_.._a_b");
        assert_eq!(safe_name(".."), "Senza nome");
        assert!(site("http://www.mangaworld.mx").is_err());
        assert!(resolve("https://www.mangaworld.mx", "file:///etc/passwd").is_err());
    }
}
