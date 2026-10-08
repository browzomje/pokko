//! Local connector samples. Parsing uses the same adapters as real downloads.
use crate::{
    export, image_extension,
    provider::{self, Manga},
    sources::{self, Source},
    Engine,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use futures_util::StreamExt;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
};
use tauri::{Emitter, State};

const MAX_HTML: usize = 8 * 1024 * 1024;
const MAX_ASSET: usize = 12 * 1024 * 1024;
const MAX_SAMPLE: usize = 64 * 1024 * 1024;
const MAX_ASSETS: usize = 300;
#[derive(Clone, Serialize, Deserialize)]
pub struct Sample {
    pub id: String,
    pub source: Source,
    pub url: String,
    pub html: String,
    pub created: u64,
    pub resources: HashMap<String, String>,
    pub warnings: Vec<String>,
}
#[derive(Serialize)]
pub struct SampleInfo {
    id: String,
    url: String,
    created: u64,
    resources: usize,
}
async fn delay(engine: &Engine, source: &Source) -> u64 {
    engine
        .saved
        .lock()
        .await
        .settings
        .as_ref()
        .map(|s| s.delay_ms)
        .unwrap_or(500)
        .max(source.min_delay_ms)
}
fn id() -> String {
    format!(
        "{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}
fn folder(engine: &Engine) -> PathBuf {
    engine.file.parent().unwrap().join("connector-samples")
}
fn sample_path(engine: &Engine, id: &str) -> Result<PathBuf, String> {
    if id.is_empty() || !id.bytes().all(|c| c.is_ascii_digit()) {
        return Err("Campione non valido".into());
    }
    Ok(folder(engine).join(format!("{id}.json")))
}
async fn write_sample(engine: &Engine, sample: &Sample) -> Result<(), String> {
    tokio::fs::create_dir_all(folder(engine))
        .await
        .map_err(|e| e.to_string())?;
    let data = serde_json::to_vec(sample).map_err(|e| e.to_string())?;
    if data.len() > MAX_SAMPLE {
        return Err("Campione troppo grande (massimo 64 MB)".into());
    }
    tokio::fs::write(sample_path(engine, &sample.id)?, data)
        .await
        .map_err(|e| e.to_string())
}
async fn limited(response: reqwest::Response, max: usize) -> Result<Vec<u8>, String> {
    if response.content_length().is_some_and(|n| n > max as u64) {
        return Err("Risorsa troppo grande".into());
    }
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        if bytes.len() + chunk.len() > max {
            return Err("Risorsa troppo grande".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
fn asset_urls(html: &str, base: &str, source: &Source) -> Vec<String> {
    let doc = Html::parse_document(html);
    let mut urls = Vec::new();
    for el in doc.select(&Selector::parse("img, link[rel='stylesheet']").unwrap()) {
        let value = if el.value().name() == "link" {
            el.value().attr("href")
        } else {
            el.value()
                .attr("data-src")
                .filter(|s| !s.is_empty())
                .or(el.value().attr("data-original"))
                .or(el.value().attr("src"))
        };
        if let Some(value) = value {
            if let Ok(url) = provider::resolve(base, value) {
                urls.push(url);
            }
        }
    }
    if let Ok(pages) = reader(html, base, source) {
        urls.extend(pages);
    }
    for style in doc.select(&Selector::parse("style, [style]").unwrap()) {
        urls.extend(css_urls(&style.inner_html(), base));
        if let Some(value) = style.value().attr("style") {
            urls.extend(css_urls(value, base));
        }
    }
    urls
}
fn css_urls(css: &str, base: &str) -> Vec<String> {
    let mut urls = Vec::new();
    // Follow url(...) dependencies; imported stylesheets using this form are included.
    for part in css.split("url(").skip(1) {
        if let Some((raw, _)) = part.split_once(')') {
            let raw = raw.trim().trim_matches(['\'', '"']);
            if let Ok(url) = provider::resolve(base, raw) {
                urls.push(url);
            }
        }
    }
    urls
}
fn reader(html: &str, url: &str, source: &Source) -> Result<Vec<String>, String> {
    match source.adapter.as_str() {
        "generic" => sources::generic_pages(html, url, source),
        "mangaworld" => provider::pages(html, url),
        "readcomics" => sources::comics_pages(html),
        _ => Err(
            "MangaDex usa l’API: i campioni HTML sono disponibili per gli adattatori HTML.".into(),
        ),
    }
}
#[tauri::command]
pub fn validate_source(source: Source) -> Result<(), String> {
    sources::validate(&source)
}
#[tauri::command]
pub async fn lab_capture(
    url: String,
    source: Source,
    assets: bool,
    token: String,
    app: tauri::AppHandle,
    engine: State<'_, Arc<Engine>>,
) -> Result<Sample, String> {
    sources::validate(&source)?;
    provider::site(&url)?;
    if source.adapter == "mangadex" {
        return Err("MangaDex usa l’API, non i selettori HTML.".into());
    }
    let _ = app.emit(
        "lab-progress",
        json!({"token":token,"message":"Acquisizione HTML…"}),
    );
    let throttle = delay(&engine, &source).await;
    let response = engine
        .request(&url, &source.base_url, throttle, Some(&token))
        .await?;
    let url = response.url().to_string();
    provider::site(&url)?;
    let html = String::from_utf8(limited(response, MAX_HTML).await?)
        .map_err(|_| "HTML non UTF-8: importa una copia convertita in UTF-8")?;
    let mut sample = Sample {
        id: id(),
        source: source.clone(),
        url: url.clone(),
        html,
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        resources: HashMap::new(),
        warnings: Vec::new(),
    };
    if assets {
        let mut queue: VecDeque<_> = asset_urls(&sample.html, &url, &source).into();
        let mut seen = HashSet::new();
        let mut total = sample.html.len();
        while let Some(asset) = queue.pop_front() {
            engine.checkpoint(&token).await?;
            if !seen.insert(asset.clone()) {
                continue;
            }
            if seen.len() > MAX_ASSETS || total >= MAX_SAMPLE / 2 {
                sample.warnings.push(
                    "Limite campione raggiunto: alcune risorse non sono state salvate.".into(),
                );
                break;
            }
            let _ = app.emit(
                "lab-progress",
                json!({"token":token,"message":format!("Risorsa {} · {}",seen.len(),asset)}),
            );
            let result = async {
                let response = engine.request(&asset, &url, throttle, Some(&token)).await?;
                let final_url = response.url().to_string();
                let mime = response
                    .headers()
                    .get("content-type")
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("application/octet-stream")
                    .split(';')
                    .next()
                    .unwrap()
                    .to_string();
                let bytes = limited(response, MAX_ASSET).await?;
                Ok::<_, String>((mime, bytes, final_url))
            }
            .await;
            match result {
                Ok((mime, bytes, final_url)) => {
                    if total + bytes.len() * 2 > MAX_SAMPLE / 2 {
                        sample
                            .warnings
                            .push(format!("Risorsa esclusa per dimensione: {asset}"));
                        continue;
                    }
                    if mime == "text/css" {
                        queue.extend(css_urls(&String::from_utf8_lossy(&bytes), &final_url));
                    }
                    total += bytes.len() * 2;
                    let mime = if mime == "text/css" {
                        mime
                    } else if let Ok(ext) = image_extension(&bytes) {
                        format!("image/{}", if ext == "jpg" { "jpeg" } else { ext })
                    } else {
                        mime
                    };
                    sample.resources.insert(
                        asset,
                        format!("data:{mime};base64,{}", STANDARD.encode(bytes)),
                    );
                }
                Err(error) => {
                    if error == "Annullato" {
                        return Err(error);
                    }
                    sample.warnings.push(format!("{asset}: {error}"));
                    // A refusal must not trigger more requests to the same site.
                    if error.contains("403") || error.contains("429") {
                        break;
                    }
                }
            }
        }
    }
    write_sample(&engine, &sample).await?;
    Ok(sample)
}
#[tauri::command]
pub async fn lab_samples(
    source_id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<SampleInfo>, String> {
    let dir = folder(&engine);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = tokio::fs::read_dir(dir).await.map_err(|e| e.to_string())?;
    let mut samples = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let bytes = tokio::fs::read(entry.path())
            .await
            .map_err(|e| e.to_string())?;
        if let Ok(sample) = serde_json::from_slice::<Sample>(&bytes) {
            if sample.source.id == source_id {
                samples.push(SampleInfo {
                    id: sample.id,
                    url: sample.url,
                    created: sample.created,
                    resources: sample.resources.len(),
                });
            }
        }
    }
    samples.sort_by_key(|a| std::cmp::Reverse(a.created));
    Ok(samples)
}
#[tauri::command]
pub async fn lab_load(id: String, engine: State<'_, Arc<Engine>>) -> Result<Sample, String> {
    let bytes = tokio::fs::read(sample_path(&engine, &id)?)
        .await
        .map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn lab_delete(id: String, engine: State<'_, Arc<Engine>>) -> Result<(), String> {
    tokio::fs::remove_file(sample_path(&engine, &id)?)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn lab_import(
    path: String,
    url: String,
    source: Source,
    engine: State<'_, Arc<Engine>>,
) -> Result<Sample, String> {
    sources::validate(&source)?;
    let metadata = tokio::fs::metadata(&path)
        .await
        .map_err(|e| e.to_string())?;
    if metadata.len() > MAX_SAMPLE as u64 {
        return Err("File troppo grande".into());
    }
    let bytes = tokio::fs::read(&path).await.map_err(|e| e.to_string())?;
    let mut sample = if Path::new(&path).extension().and_then(|s| s.to_str()) == Some("json") {
        serde_json::from_slice::<Sample>(&bytes)
            .map_err(|e| format!("Campione JSON non valido: {e}"))?
    } else {
        provider::site(&url)?;
        Sample {id: id(), source: source.clone(), url, html: String::from_utf8(bytes).map_err(|e|e.to_string())?, created:0, resources:HashMap::new(), warnings:vec!["HTML importato senza risorse: acquisisci un campione con immagini per provare il PDF offline.".into()]}
    };
    provider::site(&sample.url)?;
    if sample.html.len() > MAX_HTML {
        return Err("HTML troppo grande".into());
    }
    sample.id = id();
    // Associate with the current draft without discarding the imported selectors.
    sample.source.id = source.id;
    sources::validate(&sample.source)?;
    sample.created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    write_sample(&engine, &sample).await?;
    Ok(sample)
}
#[tauri::command]
pub async fn lab_export(
    id: String,
    path: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), String> {
    let bytes = tokio::fs::read(sample_path(&engine, &id)?)
        .await
        .map_err(|e| e.to_string())?;
    tokio::fs::write(path, bytes)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn lab_extract(
    source: Source,
    html: String,
    url: String,
    kind: String,
) -> Result<Value, String> {
    sources::validate(&source)?;
    provider::site(&url)?;
    if html.len() > MAX_HTML {
        return Err("HTML troppo grande".into());
    }
    let manga = Manga {
        title: "Campione".into(),
        url: url.clone(),
        source_id: source.id.clone(),
        source_name: source.name.clone(),
        language: source.language.clone(),
        ..Manga::default()
    };
    match kind.as_str() {
        "search" => {
            let result = match source.adapter.as_str() {
                "generic" => sources::generic_search(&html, &url, &source)?,
                "mangaworld" => provider::search(&html, &url, 1)?,
                "readcomics" => sources::comics_search(&html, &url, &source)?,
                _ => return Err("Adattatore API: i selettori HTML non si applicano.".into()),
            };
            Ok(
                json!({"items":result.items.iter().map(|m|json!({"title":m.title,"url":m.url})).collect::<Vec<_>>(), "description":"", "has_next":result.has_next}),
            )
        }
        "detail" => {
            let detail = match source.adapter.as_str() {
                "generic" => sources::generic_detail(&html, manga, &source)?,
                "mangaworld" => provider::detail(&html, manga)?,
                "readcomics" => sources::comics_detail(&html, manga)?,
                _ => return Err("Adattatore API: i selettori HTML non si applicano.".into()),
            };
            Ok(
                json!({"items":detail.volumes.iter().flat_map(|v|v.chapters.iter().map(move |c|json!({"title":c.title,"url":c.url,"group":v.title}))).collect::<Vec<_>>(),"description":detail.description, "has_next":false}),
            )
        }
        "reader" => Ok(
            json!({"items":reader(&html, &url, &source)?.iter().enumerate().map(|(i,u)|json!({"title":format!("Pagina {}",i+1),"url":u})).collect::<Vec<_>>(),"description":"", "has_next":sources::reader_next(&html,&url,&source)?.is_some(), "next_url":sources::reader_next(&html,&url,&source)?}),
        ),
        _ => Err("Tipo prova non valido".into()),
    }
}
#[tauri::command]
pub async fn lab_cancel(token: String, engine: State<'_, Arc<Engine>>) -> Result<(), String> {
    engine.controls.lock().await.insert(token, "cancel".into());
    Ok(())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTest {
    source: Source,
    sample_id: String,
    html: String,
    output: String,
    offline: bool,
    token: String,
}
#[tauri::command]
pub async fn lab_download(
    request: DownloadTest,
    app: tauri::AppHandle,
    engine: State<'_, Arc<Engine>>,
) -> Result<Value, String> {
    let sample = lab_load(request.sample_id.clone(), engine.clone()).await?;
    run_download(request, sample, &engine, |message| {
        let _ = app.emit("lab-progress", message);
    })
    .await
}
async fn run_download(
    request: DownloadTest,
    mut sample: Sample,
    engine: &Engine,
    progress: impl Fn(Value),
) -> Result<Value, String> {
    let DownloadTest {
        source,
        html,
        output,
        offline,
        token,
        ..
    } = request;
    sources::validate(&source)?;
    if !Path::new(&output).is_absolute() || !Path::new(&output).is_dir() {
        return Err("Scegli una cartella di destinazione".into());
    }
    if html.len() > MAX_HTML {
        return Err("HTML troppo grande".into());
    }
    let mut pages = reader(&html, &sample.url, &source)?;
    let mut next = sources::reader_next(&html, &sample.url, &source)?;
    let mut visited = HashSet::from([sample.url.clone()]);
    while let Some(url) = next {
        engine.checkpoint(&token).await?;
        if visited.len() >= 100 || !visited.insert(url.clone()) {
            return Err("Paginazione lettore ciclica o superiore a 100 pagine HTML".into());
        }
        let mut local = None;
        if folder(engine).exists() {
            let mut entries = tokio::fs::read_dir(folder(engine))
                .await
                .map_err(|e| e.to_string())?;
            while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
                if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                if let Ok(bytes) = tokio::fs::read(entry.path()).await {
                    if let Ok(saved) = serde_json::from_slice::<Sample>(&bytes) {
                        if saved.source.id == source.id && saved.url == url {
                            local = Some(saved);
                            break;
                        }
                    }
                }
            }
        }
        let (html, base) = if let Some(saved) = local {
            let total = sample.resources.values().map(String::len).sum::<usize>()
                + saved
                    .resources
                    .iter()
                    .filter(|(url, _)| !sample.resources.contains_key(*url))
                    .map(|(_, data)| data.len())
                    .sum::<usize>();
            if total > MAX_SAMPLE * 4 / 3 {
                return Err("Le risorse della sequenza superano il limite del test (64 MB)".into());
            }
            sample.resources.extend(saved.resources);
            (saved.html, saved.url)
        } else if offline {
            return Err(format!("Manca il campione della pagina successiva: {url}. Acquisiscila prima del test offline."));
        } else {
            let response = engine
                .request(
                    &url,
                    &sample.url,
                    delay(engine, &source).await,
                    Some(&token),
                )
                .await?;
            let base = response.url().to_string();
            let html =
                String::from_utf8(limited(response, MAX_HTML).await?).map_err(|e| e.to_string())?;
            (html, base)
        };
        pages.extend(reader(&html, &base, &source)?);
        if pages.len() > 500 {
            return Err("Il test è limitato a 500 pagine".into());
        }
        next = sources::reader_next(&html, &base, &source)?;
    }
    let mut seen = HashSet::new();
    pages.retain(|url| seen.insert(url.clone()));
    if pages.len() > 500 {
        return Err("Il test è limitato a 500 pagine. Scegli un capitolo campione.".into());
    }
    if offline {
        let missing = pages
            .iter()
            .filter(|url| !sample.resources.contains_key(*url))
            .count();
        if missing > 0 {
            return Err(format!("Mancano {missing} immagini nel campione. Acquisisci la pagina con le risorse oppure disattiva il test offline."));
        }
    }
    let dir = Path::new(&output).join(format!("prova-connettore-{}", id()));
    let chapter_dir = dir.join("pagine");
    tokio::fs::create_dir_all(&chapter_dir)
        .await
        .map_err(|e| e.to_string())?;
    let mut bytes_total = 0;
    let throttle = delay(engine, &source).await;
    for (i, url) in pages.iter().enumerate() {
        engine.checkpoint(&token).await?;
        progress(json!({"token":token,"message":format!("Pagina {}/{}",i+1,pages.len())}));
        let bytes = if let Some(data) = sample.resources.get(url) {
            let (_, raw) = data.split_once(',').ok_or("Risorsa locale non valida")?;
            STANDARD.decode(raw).map_err(|e| e.to_string())?
        } else if offline {
            return Err("Immagine non disponibile offline".into());
        } else {
            limited(
                engine
                    .request(url, &sample.url, throttle, Some(&token))
                    .await?,
                MAX_ASSET,
            )
            .await?
        };
        let ext = image_extension(&bytes)?;
        bytes_total += bytes.len();
        tokio::fs::write(chapter_dir.join(format!("{:04}.{ext}", i + 1)), bytes)
            .await
            .map_err(|e| e.to_string())?;
    }
    engine.checkpoint(&token).await?;
    let destination = dir.join("campione.pdf");
    let source_dir = dir.clone();
    let dest = destination.clone();
    let controls = engine.controls.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        export::pdf(&source_dir, &dest, || {
            if controls
                .blocking_lock()
                .get(&token)
                .is_some_and(|v| v == "cancel")
            {
                Err("Annullato".into())
            } else {
                Ok(())
            }
        })
    })
    .await
    .map_err(|e| e.to_string())?;
    result?;
    let count = lopdf::Document::load(&destination)
        .map_err(|e| e.to_string())?
        .get_pages()
        .len();
    if count != pages.len() {
        return Err("Il PDF non contiene tutte le pagine attese".into());
    }
    Ok(json!({"output":destination.display().to_string(),"pages":count,"bytes":bytes_total}))
}
#[tauri::command]
pub fn lab_open_output(path: String) -> Result<(), String> {
    let path = Path::new(&path);
    if !path.is_absolute()
        || !path.is_file()
        || path.file_name().and_then(|s| s.to_str()) != Some("campione.pdf")
    {
        return Err("File di prova non valido".into());
    }
    open::that_detached(path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Source {
        Source {
            id: "fixture".into(),
            base_url: "https://source.test".into(),
            result_selector: ".result a".into(),
            chapter_selector: ".chapter a".into(),
            image_selector: "img.scan".into(),
            description_selector: ".description".into(),
            next_selector: "a.next".into(),
            ..Source::default()
        }
    }
    fn sample() -> Sample {
        Sample {
            id: id(),
            source: source(),
            url: "https://source.test/read/1".into(),
            html: "<img class='scan' src='/1.png'><img class='scan' data-src='/2.png'>".into(),
            created: 1,
            resources: HashMap::new(),
            warnings: Vec::new(),
        }
    }
    #[test]
    fn same_parser_for_all_three_workbench_stages() {
        let html = "<div class='result'><a href='/manga/1'>Title</a></div><a class='next' href='?page=2'>Next</a><div class='chapter'><a href='/read/1'>Chapter 1</a></div><p class='description'>Description</p><img class='scan' data-src='/page/1.png'>";
        let run = |kind: &str| {
            lab_extract(
                source(),
                html.into(),
                "https://source.test/list".into(),
                kind.into(),
            )
            .unwrap()
        };
        let search = run("search");
        assert_eq!(search["items"][0]["url"], "https://source.test/manga/1");
        assert_eq!(search["has_next"], true);
        let detail = run("detail");
        assert_eq!(detail["items"][0]["group"], "Capitoli");
        assert_eq!(detail["description"], "Description");
        assert_eq!(
            run("reader")["items"][0]["url"],
            "https://source.test/page/1.png"
        );
        let mut bad = source();
        bad.image_selector = "[".into();
        assert!(lab_extract(
            bad,
            html.into(),
            "https://source.test".into(),
            "reader".into()
        )
        .is_err());
    }
    #[test]
    fn dedicated_adapters_are_available_in_workbench() {
        let presets = sources::defaults();
        let mw = presets.iter().find(|s| s.adapter == "mangaworld").unwrap();
        let result = lab_extract(
            mw.clone(),
            include_str!("../../tests/fixtures/detail.html").into(),
            "https://www.mangaworld.mx/manga/1/test".into(),
            "detail".into(),
        )
        .unwrap();
        assert!(result["items"].as_array().unwrap().len() > 100);
        let comics = presets.iter().find(|s| s.adapter == "readcomics").unwrap();
        let result = lab_extract(
            comics.clone(),
            include_str!("../../tests/fixtures/readcomics-reader.html").into(),
            "https://readcomicsonline.lol/comic/test/1".into(),
            "reader".into(),
        )
        .unwrap();
        assert_eq!(result["items"].as_array().unwrap().len(), 27);
    }
    #[tokio::test]
    async fn paginated_offline_pdf_requires_saved_pages_and_rejects_cycles() {
        let dir = std::env::temp_dir().join(format!("mwr-lab-paging-{}", id()));
        std::fs::create_dir_all(&dir).unwrap();
        let engine = crate::tests::test_engine(&dir);
        let mut first = sample();
        first.source.reader_next_selector = "a.scan-next".into();
        first.html =
            "<img class='scan' src='/1.png'><a class='scan-next' href='/reader/2'>Next</a>".into();
        first.source.image_selector = "img.scan".into();
        let request = || DownloadTest {
            source: first.source.clone(),
            sample_id: first.id.clone(),
            html: first.html.clone(),
            output: dir.display().to_string(),
            offline: true,
            token: "pagination-test".into(),
        };
        let error = run_download(request(), first.clone(), &engine, |_| {})
            .await
            .unwrap_err();
        assert!(error.contains("Manca il campione"));
        let mut second = first.clone();
        second.id = id();
        second.url = "https://source.test/reader/2".into();
        second.html = "<img class='scan' src='/1.png'><img class='scan' src='/2.png'>".into();
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_pixel(12, 20, image::Rgb([255, 0, 0]))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let data = format!(
            "data:image/png;base64,{}",
            STANDARD.encode(bytes.into_inner())
        );
        first
            .resources
            .insert("https://source.test/1.png".into(), data.clone());
        second
            .resources
            .insert("https://source.test/2.png".into(), data);
        write_sample(&engine, &second).await.unwrap();
        let result = run_download(request(), first.clone(), &engine, |_| {})
            .await
            .unwrap();
        assert_eq!(result["pages"], 2);
        assert_eq!(
            lopdf::Document::load(result["output"].as_str().unwrap())
                .unwrap()
                .get_pages()
                .len(),
            2
        );
        second.html = format!(
            "<img class='scan' src='/2.png'><a class='scan-next' href='{}'>Back</a>",
            first.url
        );
        write_sample(&engine, &second).await.unwrap();
        let error = run_download(request(), first.clone(), &engine, |_| {})
            .await
            .unwrap_err();
        assert!(error.contains("ciclica"));
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[tokio::test]
    async fn persisted_sample_exports_full_pdf_offline_and_rejects_missing_images() {
        let dir = std::env::temp_dir().join(format!("mwr-lab-test-{}", id()));
        std::fs::create_dir_all(&dir).unwrap();
        let engine = crate::tests::test_engine(&dir);
        let mut sample = sample();
        let request = || DownloadTest {
            source: source(),
            sample_id: String::new(),
            html: sample.html.clone(),
            output: dir.display().to_string(),
            offline: true,
            token: "lab-test".into(),
        };
        let missing = run_download(request(), sample.clone(), &engine, |_| {})
            .await
            .unwrap_err();
        assert!(missing.contains("Mancano 2 immagini"));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_pixel(12, 20, image::Rgb([255, 0, 0]))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let data = format!(
            "data:image/png;base64,{}",
            STANDARD.encode(bytes.into_inner())
        );
        sample
            .resources
            .insert("https://source.test/1.png".into(), data.clone());
        sample
            .resources
            .insert("https://source.test/2.png".into(), data);
        write_sample(&engine, &sample).await.unwrap();
        let persisted: Sample = serde_json::from_slice(
            &std::fs::read(sample_path(&engine, &sample.id).unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(persisted.resources.len(), 2);
        let request = DownloadTest {
            source: source(),
            sample_id: sample.id.clone(),
            html: sample.html.clone(),
            output: dir.display().to_string(),
            offline: true,
            token: "lab-test".into(),
        };
        let result = run_download(request, persisted, &engine, |_| {})
            .await
            .unwrap();
        assert_eq!(result["pages"], 2);
        let pdf = lopdf::Document::load(result["output"].as_str().unwrap()).unwrap();
        assert_eq!(pdf.get_pages().len(), 2);
        assert!(sample_path(&engine, "../../queue").is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
