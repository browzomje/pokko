//! Query fallback and bounded, cached creator enrichment shared by every source.
use crate::{
    provider::SearchResult,
    sources::{self, Source},
    Engine,
};
fn candidate_score(title: &str, query: &str) -> usize {
    let title = title.to_lowercase();
    query
        .split_whitespace()
        .filter(|token| title.contains(&token.to_lowercase()))
        .count()
}
pub async fn search_source(
    engine: &Engine,
    source: &Source,
    query: &str,
    page: u32,
    delay: u64,
    mut publish: impl FnMut(&SearchResult) + Send,
) -> Result<SearchResult, String> {
    let mut result = sources::search(engine, source, query, page, delay).await?;
    if page == 1 && result.items.is_empty() {
        if let Some((title, _)) = query.trim().rsplit_once(char::is_whitespace) {
            result = sources::search(engine, source, title.trim(), page, delay).await?;
        }
    }
    {
        let cache = engine.creator_cache.lock().await;
        for manga in &mut result.items {
            if manga.authors.is_empty() {
                if let Some(names) = cache.get(&manga.url) {
                    manga.authors = names.clone();
                }
            }
        }
    }
    publish(&result);
    if query.split_whitespace().count() > 1 && source.adapter != "mangadex" {
        let mut candidates = (0..result.items.len())
            .filter(|&i| {
                result.items[i].authors.is_empty()
                    && candidate_score(&result.items[i].title, query) > 0
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|&i| {
            (
                std::cmp::Reverse(candidate_score(&result.items[i].title, query)),
                result.items[i].title.len(),
            )
        });
        for index in candidates.into_iter().take(3) {
            let manga = &mut result.items[index];
            let cached = engine.creator_cache.lock().await.get(&manga.url).cloned();
            if let Some(names) = cached {
                manga.authors = names;
                publish(&result);
                continue;
            }
            match sources::creators(engine, source, manga, delay).await {
                Ok(names) => {
                    let mut cache = engine.creator_cache.lock().await;
                    if cache.len() >= 512 {
                        cache.clear();
                    }
                    cache.insert(manga.url.clone(), names.clone());
                    manga.authors = names;
                    publish(&result);
                }
                Err(error) => {
                    if error.contains("403") || error.contains("429") {
                        break;
                    }
                }
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "Live source verification"]
    async fn invincible_creators_live() {
        let dir = std::env::temp_dir().join("mwr-creators-live");
        let engine = crate::tests::test_engine(&dir);
        let source = sources::defaults()
            .into_iter()
            .find(|s| s.id == "readcomics")
            .unwrap();
        let mut updates = Vec::new();
        let result = search_source(&engine, &source, "invincible kirkman", 1, 500, |partial| {
            updates.push(
                partial
                    .items
                    .iter()
                    .find(|m| m.title == "Invincible (2003)")
                    .map(|m| m.authors.clone())
                    .unwrap_or_default(),
            );
        })
        .await
        .unwrap();
        let invincible = result
            .items
            .iter()
            .find(|m| m.title == "Invincible (2003)")
            .unwrap();
        assert!(invincible
            .authors
            .iter()
            .any(|name| name == "Robert Kirkman"));
        assert!(updates.len() >= 2);
        assert!(
            updates.first().unwrap().is_empty(),
            "Initial results must be published before metadata requests"
        );
        assert!(updates
            .last()
            .unwrap()
            .iter()
            .any(|name| name == "Robert Kirkman"));
        let cached = search_source(&engine, &source, "invincible kirkman", 1, 500, |_| {})
            .await
            .unwrap();
        assert_eq!(
            cached
                .items
                .iter()
                .find(|m| m.title == "Invincible (2003)")
                .unwrap()
                .authors,
            invincible.authors
        );
    }
}
