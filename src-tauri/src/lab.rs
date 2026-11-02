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
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tauri::{Emitter, Manager, State};

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
    sources::generic_pages(html, url, source)
}
#[tauri::command]
pub fn validate_source(source: Source) -> Result<(), String> {
    sources::validate(&source)
}
fn strip_proto(s: &str) -> &str {
    s.trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.")
}

fn lab_urls_match(u1: &str, u2: &str) -> bool {
    let s1 = u1.trim().trim_end_matches('/');
    let s2 = u2.trim().trim_end_matches('/');
    if s1.eq_ignore_ascii_case(s2) {
        return true;
    }
    strip_proto(s1).eq_ignore_ascii_case(strip_proto(s2))
}

fn same_domain(u1: &str, u2: &str) -> bool {
    let d1 = url::Url::parse(u1)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_lowercase()));
    let d2 = url::Url::parse(u2)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_lowercase()));
    match (d1, d2) {
        (Some(a), Some(b)) => {
            let s1 = a.trim_start_matches("www.");
            let s2 = b.trim_start_matches("www.");
            s1 == s2
        }
        _ => false,
    }
}

async fn fetch_via_engine(
    url: &str,
    source: &Source,
    throttle: u64,
    token: Option<&str>,
    engine: &Engine,
) -> Result<(String, String), String> {
    let response = engine
        .request(url, &source.base_url, throttle, token)
        .await?;
    let final_url = response.url().to_string();
    provider::site(&final_url)?;
    let raw = String::from_utf8(limited(response, MAX_HTML).await?)
        .map_err(|_| "HTML non UTF-8: importa una copia convertita in UTF-8")?;
    Ok((final_url, raw))
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
    if source.adapter == "mangadex" {
        return Err("MangaDex usa l’API, non i selettori HTML.".into());
    }
    let _ = app.emit(
        "lab-progress",
        json!({"token":token,"message":"Acquisizione HTML…"}),
    );

    let ff_target = url.clone();
    let base_url = source.base_url.clone();
    let ff_page = tokio::task::spawn_blocking(move || {
        if let Ok(ff_url) = lab_firefox_get_url() {
            if ff_url.starts_with("http") {
                let is_match = ff_target.trim().is_empty()
                    || lab_urls_match(&ff_url, &ff_target)
                    || same_domain(&ff_url, &ff_target)
                    || same_domain(&ff_url, &base_url);
                if is_match {
                    if let Ok(resp) = marionette_exec("WebDriver:GetPageSource", json!({})) {
                        if let Some(src) = resp["value"].as_str().filter(|s| !s.trim().is_empty()) {
                            return Some((ff_url, src.to_string()));
                        }
                    }
                }
            }
        }
        None
    })
    .await
    .unwrap_or(None);

    let (url, raw_html) = if let Some((ff_url, src)) = ff_page {
        (ff_url, src)
    } else {
        provider::site(&url)?;
        let throttle = delay(&engine, &source).await;
        fetch_via_engine(&url, &source, throttle, Some(&token), &engine).await?
    };
    let html = sanitize_html_adblock(&raw_html);
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
        let mut seen: HashSet<String> = HashSet::new();
        let mut total = sample.html.len();
        let asset_limit = 3.min(MAX_ASSETS);
        let throttle = delay(&engine, &source).await;
        while let Some(asset) = queue.pop_front() {
            if seen.len() >= asset_limit || total >= MAX_SAMPLE / 2 {
                break;
            }
            if !seen.insert(asset.clone()) {
                continue;
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
            let result = sources::generic_search(&html, &url, &source)?;
            Ok(
                json!({"items":result.items.iter().map(|m|json!({"title":m.title,"url":m.url})).collect::<Vec<_>>(), "description":"", "has_next":result.has_next}),
            )
        }
        "detail" => {
            let detail = sources::generic_detail(&html, manga, &source)?;
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
    mut request: DownloadTest,
    app: tauri::AppHandle,
    engine: State<'_, Arc<Engine>>,
) -> Result<Value, String> {
    if request.output.trim().is_empty() {
        request.output = app
            .path()
            .download_dir()
            .or_else(|_| app.path().home_dir().map(|p| p.join("Downloads")))
            .unwrap_or_else(|_| std::env::temp_dir())
            .display()
            .to_string();
    }
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
    let site_slug = provider::safe_name(if source.name.trim().is_empty() {
        "site"
    } else {
        &source.name
    })
    .to_lowercase()
    .replace(' ', "_")
    .chars()
    .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
    .collect::<String>();
    let site_slug = if site_slug.is_empty() {
        "site".to_string()
    } else {
        site_slug
    };
    let secs_today = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() % 86400)
        .unwrap_or(0);
    let hh = secs_today / 3600;
    let mm = (secs_today % 3600) / 60;
    let ss = secs_today % 60;
    let pdf_filename = format!("{site_slug}_test_download_{hh:02}_{mm:02}_{ss:02}.pdf");
    let destination = dir.join(pdf_filename);
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

static FIREFOX_PROCESS: std::sync::Mutex<Option<std::process::Child>> = std::sync::Mutex::new(None);
// Serialize commands and retain one session instead of racing NewSession calls.
static MARIONETTE_CONNECTION: std::sync::Mutex<Option<std::net::TcpStream>> =
    std::sync::Mutex::new(None);
static FIREFOX_LIFECYCLE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static MARIONETTE_PORT: std::sync::Mutex<u16> = std::sync::Mutex::new(2828);

fn firefox_profile_dir() -> PathBuf {
    std::env::temp_dir().join("pokko-firefox-profile")
}

fn clean_profile_locks(profile_dir: &Path) {
    for lock in [".parentlock", "parent.lock", "lock"] {
        let p = profile_dir.join(lock);
        if p.exists() {
            let _ = fs::remove_file(p);
        }
    }
}

pub fn is_firefox_installed() -> bool {
    std::process::Command::new("firefox")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn get_available_port() -> u16 {
    if std::net::TcpListener::bind("127.0.0.1:2828").is_ok() {
        2828
    } else {
        std::net::TcpListener::bind("127.0.0.1:0")
            .and_then(|l| l.local_addr())
            .map(|a| a.port())
            .unwrap_or(2829)
    }
}

pub async fn ensure_firefox_profile_with_ublock(port: u16) -> Result<PathBuf, String> {
    let profile_dir = firefox_profile_dir();
    let ext_dir = profile_dir.join("extensions");
    fs::create_dir_all(&ext_dir).map_err(|e| e.to_string())?;

    // Download official uBlock Origin XPI if missing
    let xpi_path = ext_dir.join("uBlock0@raymondhill.net.xpi");
    if !xpi_path.exists() || fs::metadata(&xpi_path).map(|m| m.len()).unwrap_or(0) < 1000 {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| e.to_string())?;
        match client
            .get("https://addons.mozilla.org/firefox/downloads/latest/ublock-origin/latest.xpi")
            .header(reqwest::header::USER_AGENT, "Mozilla/5.0")
            .send()
            .await
        {
            Ok(res) if res.status().is_success() => {
                if let Ok(bytes) = res.bytes().await {
                    let _ = fs::write(&xpi_path, bytes);
                    crate::logger::info("lab", "uBlock Origin scaricato e salvato nel profilo");
                }
            }
            Err(e) => {
                crate::logger::warn(
                    "lab",
                    &format!("Download uBlock Origin non riuscito ({e}), avvio senza estensione"),
                );
            }
            _ => {}
        }
    }

    // Write user.js preferences
    let user_js = profile_dir.join("user.js");
    let prefs = format!(
        r#"
user_pref("marionette.port", {port});
user_pref("marionette.enabled", true);
user_pref("extensions.autoDisableScopes", 0);
user_pref("extensions.enabledScopes", 15);
user_pref("xpinstall.signatures.required", true);
user_pref("browser.startup.homepage_override.mstone", "ignore");
user_pref("startup.homepage_welcome_url", "");
user_pref("startup.homepage_welcome_url.additional", "");
user_pref("browser.shell.checkDefaultBrowser", false);
user_pref("datareporting.policy.dataSubmissionEnabled", false);
user_pref("toolkit.telemetry.enabled", false);
user_pref("browser.rights.3.shown", true);
user_pref("extensions.pocket.enabled", false);
user_pref("browser.newtabpage.enabled", false);
user_pref("app.update.auto", false);
user_pref("app.update.enabled", false);

// Strict popup & redirect blocking
user_pref("dom.disable_open_during_load", true);
user_pref("dom.popup_maximum", 0);
user_pref("dom.popup_allowed_events", "");
user_pref("privacy.popups.showBrowserMessage", false);

// Disable history & bookmarks to avoid places.sqlite conflict banner while keeping WebExtensions active
user_pref("places.history.enabled", false);
user_pref("browser.formfill.enable", false);
user_pref("signon.rememberSignons", false);
user_pref("network.cookie.lifetimePolicy", 2);
user_pref("browser.sessionstore.resume_from_crash", false);
"#
    );
    fs::write(user_js, prefs).map_err(|e| format!("Impossibile configurare Firefox: {e}"))?;

    Ok(profile_dir)
}

fn marionette_connect_with_retry(
    retries: usize,
    wait_ms: u64,
) -> Result<std::net::TcpStream, String> {
    let port = MARIONETTE_PORT.lock().map(|p| *p).unwrap_or(2828);
    let addr = format!("127.0.0.1:{port}");
    for _ in 0..retries {
        if let Ok(stream) = std::net::TcpStream::connect_timeout(
            &addr.parse().unwrap(),
            std::time::Duration::from_millis(100),
        ) {
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(30)));
            let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(30)));
            return Ok(stream);
        }
        std::thread::sleep(std::time::Duration::from_millis(wait_ms));
    }
    Err(format!(
        "Connessione a Marionette non riuscita (porta {port})"
    ))
}

fn marionette_connect() -> Result<std::net::TcpStream, String> {
    marionette_connect_with_retry(10, 100)
}

fn marionette_read_packet(stream: &mut std::net::TcpStream) -> Result<Value, String> {
    use std::io::Read;
    let mut header = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        stream.read_exact(&mut byte).map_err(|e| e.to_string())?;
        if byte[0] == b':' {
            break;
        }
        if !byte[0].is_ascii_digit() || header.len() >= 10 {
            return Err("Intestazione Marionette non valida".into());
        }
        header.push(byte[0]);
    }
    let length: usize = String::from_utf8(header)
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;

    if length > 64 * 1024 * 1024 {
        return Err("Risposta Marionette troppo grande".into());
    }
    let mut body = vec![0u8; length];
    stream.read_exact(&mut body).map_err(|e| e.to_string())?;
    serde_json::from_slice(&body).map_err(|e| e.to_string())
}

fn marionette_send_cmd(
    stream: &mut std::net::TcpStream,
    id: u64,
    name: &str,
    params: Value,
) -> Result<Value, String> {
    use std::io::Write;
    let payload = json!([0, id, name, params]).to_string();
    let wire = format!("{}:{}", payload.len(), payload);
    stream
        .write_all(wire.as_bytes())
        .map_err(|e| e.to_string())?;
    marionette_response(marionette_read_packet(stream)?, id)
}

fn marionette_response(response: Value, id: u64) -> Result<Value, String> {
    let arr = response
        .as_array()
        .filter(|a| a.len() == 4)
        .ok_or("Risposta Marionette non valida")?;
    if arr[0] != 1 || arr[1] != id {
        return Err("Risposta Marionette fuori sequenza".into());
    }
    if !arr[2].is_null() {
        return Err(format!(
            "Firefox: {} ({})",
            arr[2]["message"].as_str().unwrap_or("Comando non riuscito"),
            arr[2]["error"].as_str().unwrap_or("unknown error")
        ));
    }
    Ok(arr[3].clone())
}

fn marionette_session(stream: &mut std::net::TcpStream) -> Result<(), String> {
    let greeting = marionette_read_packet(stream)?;
    if greeting["applicationType"] != "gecko" || greeting["marionetteProtocol"] != 3 {
        return Err("Il servizio sulla porta selezionata non è Marionette".into());
    }
    marionette_send_cmd(
        stream,
        1,
        "WebDriver:NewSession",
        json!({
            "capabilities": {"alwaysMatch": {"timeouts": {"pageLoad": 20000, "script": 10000, "implicit": 0}}}
        }),
    )?;
    Ok(())
}

fn reset_marionette_connection() {
    if let Ok(mut connection) = MARIONETTE_CONNECTION.lock() {
        *connection = None;
    }
}

pub fn marionette_exec(name: &str, params: Value) -> Result<Value, String> {
    let mut connection = MARIONETTE_CONNECTION.lock().map_err(|e| e.to_string())?;
    if connection.is_none() {
        let mut stream = marionette_connect()?;
        marionette_session(&mut stream)?;
        *connection = Some(stream);
    }
    let result = marionette_send_cmd(connection.as_mut().unwrap(), 2, name, params);
    // Never replay navigation, scripts or downloads after an ambiguous transport failure.
    // A subsequent user action will create a fresh connection.
    if result.is_err() || name == "Marionette:Quit" {
        *connection = None;
    }
    result
}

#[tauri::command]
pub async fn lab_firefox_launch(url: String) -> Result<bool, String> {
    let _lifecycle = FIREFOX_LIFECYCLE.lock().await;
    provider::site(&url)?;
    if !is_firefox_installed() {
        return Err("Firefox non è installato sul sistema. Puoi comunque incollare l'URL manualmente per continuare nel laboratorio.".into());
    }

    let profile_dir = firefox_profile_dir();

    // Check if an existing Firefox instance is running
    if let Ok(mut guard) = FIREFOX_PROCESS.lock() {
        if let Some(ref mut child) = *guard {
            if child.try_wait().ok().flatten().is_none() {
                // If it is responding to Marionette, navigate cleanly!
                if marionette_exec("WebDriver:Navigate", json!({"url": &url})).is_ok() {
                    let _ = lab_firefox_inject_picker();
                    return Ok(true);
                }
            }
        }
        // If not running or not responding via Marionette, kill the old process
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    // Forget the old session before selecting a new port. Never contact an unowned port.
    reset_marionette_connection();
    std::thread::sleep(std::time::Duration::from_millis(200));
    clean_profile_locks(&profile_dir);

    let port = get_available_port();
    if let Ok(mut p) = MARIONETTE_PORT.lock() {
        *p = port;
    }

    let profile_dir = ensure_firefox_profile_with_ublock(port).await?;

    // Spawn Firefox process starting on about:blank with --no-remote for complete profile isolation
    let child = std::process::Command::new("firefox")
        .arg("--no-remote")
        .arg("--marionette")
        .arg("--profile")
        .arg(&profile_dir)
        .arg("--new-instance")
        .arg("about:blank")
        .spawn()
        .map_err(|e| format!("Errore avvio Firefox: {e}"))?;

    if let Ok(mut guard) = FIREFOX_PROCESS.lock() {
        *guard = Some(child);
    }

    // Connect Marionette, install uBlock Origin XPI, wait ~1800ms for filter lists, and navigate
    let url_clone = url.clone();
    let profile_dir_clone = profile_dir.clone();
    tokio::task::spawn_blocking(move || {
        let mut connection = MARIONETTE_CONNECTION.lock().map_err(|e| e.to_string())?;
        let mut stream = marionette_connect_with_retry(100, 200)?;
        marionette_session(&mut stream)?;
        *connection = Some(stream);
        drop(connection);

        let xpi_path = profile_dir_clone
            .join("extensions")
            .join("uBlock0@raymondhill.net.xpi");
        let ublock_installed = xpi_path.exists()
            && marionette_exec(
                "Addon:Install",
                json!({"path": xpi_path.display().to_string(), "temporary": false}),
            )
            .is_ok();

        // Wait ~1800ms so uBlock Origin compiles its built-in filter lists (assets.json, EasyList, etc.)
        if ublock_installed {
            std::thread::sleep(std::time::Duration::from_millis(1800));
        }

        marionette_exec("WebDriver:Navigate", json!({"url": url_clone}))?;

        // On initial launch with uBlock, refresh after 800ms so all initial requests pass through 100% compiled filters
        if ublock_installed {
            std::thread::sleep(std::time::Duration::from_millis(800));
            let _ = marionette_exec("WebDriver:Refresh", json!({}));
        }

        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())??;

    // Inject picker & cosmetic/popup shield immediately
    let _ = lab_firefox_inject_picker();

    crate::logger::info(
        "lab",
        "Firefox avviato con uBlock Origin attivo e shield DOM iniettato",
    );
    Ok(true)
}

#[tauri::command]
pub async fn lab_firefox_navigate(url: String) -> Result<bool, String> {
    provider::site(&url)?;
    tokio::task::spawn_blocking(move || {
        marionette_exec("WebDriver:Navigate", json!({"url": url})).map(|_| true)
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = lab_firefox_inject_picker();
    Ok(true)
}

#[tauri::command]
pub fn lab_firefox_get_url() -> Result<String, String> {
    let resp = marionette_exec("WebDriver:GetCurrentURL", json!({}))?;
    if let Some(val) = resp["value"].as_str() {
        return Ok(val.to_string());
    }
    Err("URL non disponibile".into())
}

#[tauri::command]
pub fn lab_firefox_inject_picker() -> Result<(), String> {
    let script = r##"
(function() {
  // 1. Popup & Window Shield
  try {
    window.open = function() { return null; };
    Object.defineProperty(window, 'open', {
      value: function() { return null; },
      writable: false,
      configurable: false
    });
  } catch(e) {
    try { window.open = function() { return null; }; } catch(e2) {}
  }

  // 2. Cosmetic Residual Ad Shield (strictly ad-network iframes & ins.adsbygoogle, NEVER touch page divs/modals)
  function cleanResidualAds() {
    try {
      var adSelectors = [
        'iframe[src*="doubleclick"]',
        'iframe[src*="googlesyndication"]',
        'iframe[src*="adnxs"]',
        'iframe[src*="popads"]',
        'iframe[src*="trafficjunky"]',
        'iframe[src*="exoclick"]',
        'iframe[src*="juicyads"]',
        'iframe[src*="a-ads"]',
        'ins.adsbygoogle'
      ];
      var nodes = document.querySelectorAll(adSelectors.join(','));
      for (var i = 0; i < nodes.length; i++) {
        var el = nodes[i];
        if (el.tagName === 'BODY' || el.tagName === 'HTML' || el.id === '__pokko_picker') continue;
        el.remove();
      }
    } catch(e) {}
  }

  cleanResidualAds();
  try {
    if (!window.__pokko_ad_observer) {
      window.__pokko_ad_observer = new MutationObserver(function() {
        cleanResidualAds();
      });
      window.__pokko_ad_observer.observe(document.body || document.documentElement, { childList: true, subtree: true });
    }
  } catch(e) {}

  // Helper functions for class filtering and smart selectors
  function isJunkClass(c) {
    if (!c || typeof c !== 'string') return true;
    var s = c.toLowerCase();
    return s.startsWith('dracula') ||
           s.startsWith('darkreader') ||
           s.startsWith('__pokko') ||
           s.startsWith('pokko') ||
           s.startsWith('stylus') ||
           s.startsWith('adguard') ||
           s.startsWith('ublock') ||
           s.startsWith('ext-') ||
           s.includes('processed') ||
           s.includes('transit');
  }

  var UTILITY_CLASSES = [
    'btn', 'btn-sm', 'btn-primary', 'btn-secondary', 'btn-default', 'btn-outline-light', 'btn-outline-secondary',
    'd-block', 'd-flex', 'd-inline', 'd-inline-block', 'd-none',
    'flex', 'flex-row', 'flex-column', 'flex-grow-1', 'justify-content-center', 'align-items-center',
    'row', 'col', 'container', 'nav-link', 'active', 'show', 'fade',
    'text-muted', 'text-center', 'text-left', 'text-right', 'small', 'badge',
    'mt-1', 'mt-2', 'mt-3', 'mb-1', 'mb-2', 'mb-3', 'ms-1', 'me-1', 'p-1', 'p-2', 'py-1', 'py-2', 'px-1', 'px-2',
    'is-pending', 'is-current', 'is-active', 'is-open', 'is-hidden', 'loading', 'loaded'
  ];

  function cleanClassList(el) {
    if (!el || !el.classList) return [];
    return Array.from(el.classList).filter(function(c) {
      return !isJunkClass(c) && !UTILITY_CLASSES.includes(c.toLowerCase());
    });
  }

  function suggestSelector(el) {
    if (!el) return '';
    var tag = el.tagName.toLowerCase();

    // 1. Image elements (Step 4 Reader)
    if (tag === 'img') {
      var imgClasses = cleanClassList(el);
      var specificImgClass = imgClasses.find(function(c) {
        var lc = c.toLowerCase();
        return lc.includes('page') || lc.includes('scan') || lc.includes('read') || lc.includes('chapter');
      });
      if (specificImgClass) return 'img.' + CSS.escape(specificImgClass);

      var cur = el.parentElement;
      for (var level = 0; level < 4 && cur && cur.tagName !== 'BODY'; level++) {
        if (cur.id && !isJunkClass(cur.id)) {
          return '#' + CSS.escape(cur.id) + ' img';
        }
        var pClasses = cleanClassList(cur);
        var readerParentClass = pClasses.find(function(c) {
          var lc = c.toLowerCase();
          return lc.includes('page') || lc.includes('reader') || lc.includes('reading') || lc.includes('content') || lc.includes('scan') || lc.includes('lc-pages');
        });
        if (readerParentClass) {
          return '.' + CSS.escape(readerParentClass) + ' img';
        }
        if (pClasses.length > 0) {
          return '.' + CSS.escape(pClasses[0]) + ' img';
        }
        cur = cur.parentElement;
      }
      return 'img[data-src], img[src]';
    }

    // 2. Link / Chapter elements (Step 2 Search, Step 3 Series)
    var a = tag === 'a' ? el : el.closest('a');
    if (a) {
      var href = a.getAttribute('href') || '';
      var cleanClasses = cleanClassList(a);

      // Check if inside a table (very common for chapters, e.g. readberserk)
      var table = a.closest('table');
      if (table) {
        var tableClasses = cleanClassList(table);
        if (tableClasses.includes('table')) {
          return 'table.table tbody tr td a';
        }
        return 'table tr td a';
      }

      // Check for specific chapter/manga/title classes on link itself
      var specificClass = cleanClasses.find(function(c) {
        var lc = c.toLowerCase();
        return lc.includes('chap') || lc.includes('manga') || lc.includes('episode') ||
               lc.includes('title') || lc.includes('name') || lc.includes('item') || lc.includes('card');
      });
      if (specificClass) {
        return 'a.' + CSS.escape(specificClass);
      }

      // Check parents for distinct container with id or class
      var curParent = a.parentElement;
      for (var pLevel = 0; pLevel < 4 && curParent && curParent.tagName !== 'BODY'; pLevel++) {
        if (curParent.id && !isJunkClass(curParent.id) && !UTILITY_CLASSES.includes(curParent.id.toLowerCase())) {
          return '#' + CSS.escape(curParent.id) + ' a';
        }
        var pClasses = cleanClassList(curParent);
        var goodPClass = pClasses.find(function(c) {
          var lc = c.toLowerCase();
          return lc.includes('chapter') || lc.includes('manga') || lc.includes('card') ||
                 lc.includes('result') || lc.includes('search') || lc.includes('list') || lc.includes('item');
        });
        if (goodPClass) {
          if (cleanClasses.length > 0) {
            return '.' + CSS.escape(goodPClass) + ' a.' + CSS.escape(cleanClasses[0]);
          }
          return '.' + CSS.escape(goodPClass) + ' a';
        }
        curParent = curParent.parentElement;
      }

      // Check href patterns
      if (href.indexOf('/chapter/') !== -1) return 'a[href*="/chapter/"]';
      if (href.indexOf('/chapter-') !== -1) return 'a[href*="/chapter-"]';
      if (href.indexOf('/leer/') !== -1) return 'a[href*="/leer/"]';
      if (href.indexOf('/read/') !== -1) return 'a[href*="/read/"]';
      if (href.indexOf('/manga/') !== -1) return 'a[href*="/manga/"]';
      if (href.indexOf('/comic/') !== -1) return 'a[href*="/comic/"]';
      if (href.indexOf('/series/') !== -1) return 'a[href*="/series/"]';
      if (href.indexOf('/title/') !== -1) return 'a[href*="/title/"]';
      if (href.indexOf('ch-') !== -1) return 'a[href*="ch-"]';
      if (href.indexOf('-chapter-') !== -1) return 'a[href*="-chapter-"]';

      if (cleanClasses.length > 0) {
        return 'a.' + cleanClasses.map(function(c) { return CSS.escape(c); }).join('.');
      }

      // Ascend to find any parent container
      var parent = a.parentElement;
      if (parent && parent.tagName !== 'BODY') {
        var parentTag = parent.tagName.toLowerCase();
        var parentClasses = cleanClassList(parent);
        if (parentClasses.length > 0) {
          return '.' + CSS.escape(parentClasses[0]) + ' a';
        }
        if (parentTag === 'li') return 'li a';
        if (parentTag === 'article') return 'article a';
      }

      // NEVER return bare 'a'!
      return 'main a[href], .content a[href], article a[href], a[href]:not([href^="#"]):not([href="/"])';
    }

    // 3. Fallback for other elements
    if (el.id && !isJunkClass(el.id)) return '#' + CSS.escape(el.id);
    var classes = cleanClassList(el);
    if (classes.length > 0) {
      return tag + '.' + classes.map(function(c) { return CSS.escape(c); }).join('.');
    }
    return tag;
  }

  // 3. Transparent / Passive Navigation Tracker (NEVER prevents default navigation!)
  if (!window.__pokko_nav_installed) {
    window.__pokko_nav_installed = true;

    function recordNavClick(e) {
      try {
        var a = e.target.closest('a');
        if (!a) return;
        var href = a.getAttribute('href') || a.href || '';
        if (!href || href.startsWith('#') || href.startsWith('javascript:')) return;

        var sel = suggestSelector(a);
        var text = (a.textContent || '').trim().replace(/\s+/g, ' ');
        var inputVal = '';
        var searchInputs = document.querySelectorAll('input[type="search"], input[type="text"], input[name*="q"], input[name*="search"]');
        for (var i = 0; i < searchInputs.length; i++) {
          if (searchInputs[i].value && searchInputs[i].value.trim()) {
            inputVal = searchInputs[i].value.trim();
            break;
          }
        }

        var navData = {
          from_url: window.location.href,
          from_html: document.documentElement.outerHTML,
          clicked_selector: sel,
          clicked_href: href,
          clicked_text: text,
          search_input_value: inputVal,
          timestamp: Date.now()
        };

        try {
          sessionStorage.setItem('__pokko_last_nav', JSON.stringify(navData));
        } catch(err) {}
        window.__pokko_last_nav = navData;
      } catch(err) {}
    }

    document.addEventListener('mousedown', recordNavClick, true);
    document.addEventListener('click', recordNavClick, true);
  }

  // 4. Dedicated Selection Mode (Only active when explicitly triggered by user)
  if (!window.__pokko_picker_installed) {
    window.__pokko_picker_installed = true;
    window.__pokko_picked = null;

    var activeEl = null;
    document.addEventListener('mouseover', function(e) {
      if (!window.__pokko_picker_active) return;
      if (activeEl) activeEl.style.outline = '';
      activeEl = e.target;
      if (activeEl) {
        activeEl.style.outline = '3px solid #eab308';
        activeEl.style.cursor = 'crosshair';
      }
    }, true);

    document.addEventListener('mouseout', function(e) {
      if (e.target && e.target === activeEl) {
        e.target.style.outline = '';
        e.target.style.cursor = '';
      }
    }, true);

    document.addEventListener('click', function(e) {
      if (!window.__pokko_picker_active) return;
      // In selection mode, ALWAYS prevent navigation and stop propagation!
      e.preventDefault();
      e.stopPropagation();
      e.stopImmediatePropagation();

      var target = e.target;
      var tag = target ? target.tagName.toLowerCase() : '';
      var el = target;
      if (tag === 'img' || target.closest('img')) {
        el = tag === 'img' ? target : target.closest('img');
      } else if (target.closest('a')) {
        el = target.closest('a');
      }

      var sel = suggestSelector(el);
      var text = (el.textContent || '').trim().replace(/\s+/g, ' ');
      var rawHref = el.getAttribute('href') || (el.closest('a') ? el.closest('a').getAttribute('href') : '') || '';
      var fullHref = '';
      if (rawHref && !rawHref.startsWith('#') && !rawHref.startsWith('javascript:')) {
        try {
          fullHref = new URL(rawHref, window.location.href).href;
        } catch(err) {
          fullHref = rawHref;
        }
      }

      var pickedData = {
        selector: sel,
        text: text,
        href: fullHref,
        tag: (el.tagName || '').toLowerCase()
      };

      window.__pokko_picked = JSON.stringify(pickedData);
      window.__pokko_picker_active = false;

      var banner = document.getElementById('__pokko_picker_banner');
      if (banner) banner.remove();

      if (activeEl) {
        activeEl.style.outline = '3px solid #22c55e';
        setTimeout(function() {
          if (activeEl) {
            activeEl.style.outline = '';
            activeEl.style.cursor = '';
          }
        }, 600);
      }
    }, true);
  }
})();
"##;
    marionette_exec(
        "WebDriver:ExecuteScript",
        json!({"script": script, "args": []}),
    )?;
    Ok(())
}

#[tauri::command]
pub fn lab_firefox_start_picker() -> Result<(), String> {
    let script = r#"
window.__pokko_picker_active = true;
var banner = document.getElementById('__pokko_picker_banner');
if (!banner && document.body) {
  banner = document.createElement('div');
  banner.id = '__pokko_picker_banner';
  banner.setAttribute('style', 'position:fixed;top:0;left:0;right:0;height:34px;background:#eab308;color:#000;font-family:system-ui,-apple-system,sans-serif;font-weight:700;font-size:12px;display:flex;align-items:center;justify-content:center;z-index:2147483647;box-shadow:0 2px 10px rgba(0,0,0,0.35);pointer-events:none;');
  banner.textContent = '🎯 Modalità Selezione: Clicca sull\'elemento desiderato (la navigazione è in pausa per questo click)';
  document.body.appendChild(banner);
}
"#;
    let _ = marionette_exec(
        "WebDriver:ExecuteScript",
        json!({"script": script, "args": []}),
    );
    lab_firefox_inject_picker()
}

#[tauri::command]
pub fn lab_firefox_get_picked() -> Result<Option<serde_json::Value>, String> {
    let script = "var sel = window.__pokko_picked; window.__pokko_picked = null; return sel;";
    let resp = marionette_exec(
        "WebDriver:ExecuteScript",
        json!({"script": script, "args": []}),
    )?;
    if let Some(val) = resp["value"].as_str() {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(val) {
            return Ok(Some(parsed));
        }
        return Ok(Some(json!({ "selector": val })));
    }
    if resp["value"].is_object() {
        return Ok(Some(resp["value"].clone()));
    }
    Ok(None)
}

#[tauri::command]
pub fn lab_firefox_get_last_nav() -> Result<Option<serde_json::Value>, String> {
    let script = r#"
var d = null;
try {
  var raw = sessionStorage.getItem('__pokko_last_nav');
  if (raw) d = JSON.parse(raw);
} catch(e) {}
if (!d && window.__pokko_last_nav) d = window.__pokko_last_nav;
return d ? JSON.stringify(d) : null;
"#;
    let resp = marionette_exec(
        "WebDriver:ExecuteScript",
        json!({"script": script, "args": []}),
    )?;
    if let Some(val) = resp["value"].as_str() {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(val) {
            return Ok(Some(parsed));
        }
    }
    Ok(None)
}

#[tauri::command]
pub fn lab_firefox_clear_last_nav() -> Result<bool, String> {
    let script = r#"
try { sessionStorage.removeItem('__pokko_last_nav'); } catch(e) {}
window.__pokko_last_nav = null;
return true;
"#;
    let _ = marionette_exec(
        "WebDriver:ExecuteScript",
        json!({"script": script, "args": []}),
    );
    Ok(true)
}

pub fn close_firefox_companion() {
    let owned = FIREFOX_PROCESS
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false);
    if owned {
        let _ = marionette_exec("Marionette:Quit", json!({}));
    }
    reset_marionette_connection();
    if let Ok(mut guard) = FIREFOX_PROCESS.lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(150));
    let profile_dir = firefox_profile_dir();
    clean_profile_locks(&profile_dir);
    crate::logger::info("lab", "Firefox companion chiuso e lock di profilo rimossi");
}

#[tauri::command]
pub fn lab_firefox_close() -> Result<(), String> {
    close_firefox_companion();
    Ok(())
}

#[tauri::command]
pub fn lab_firefox_status() -> Result<serde_json::Value, String> {
    let installed = is_firefox_installed();
    let mut running = false;
    if let Ok(mut guard) = FIREFOX_PROCESS.lock() {
        if let Some(ref mut child) = *guard {
            running = child.try_wait().ok().flatten().is_none();
        }
    }
    let ublock = firefox_profile_dir()
        .join("extensions")
        .join("uBlock0@raymondhill.net.xpi")
        .exists();

    Ok(json!({
        "installed": installed,
        "running": running,
        "ublock_installed": ublock
    }))
}

/// Heuristic search results detection: automatically identifies manga search results on a page
#[tauri::command]
pub fn lab_detect_search_results(html: String, _url: String) -> Result<serde_json::Value, String> {
    let doc = scraper::Html::parse_document(&html);

    let candidate_selectors = [
        ".lc-card .lc-card-name",
        "a.lc-card-name",
        ".lc-search-results a",
        "a.lc-sr-item",
        ".lc-sr-item",
        ".lc-card a",
        ".manga-list a",
        ".search-results a",
        ".media-heading a",
        ".post-title a",
        ".entry-title a",
        ".item-summary a",
        "a.item-title",
        "a.manga-title",
        "a.series-title",
        "a.comic-title",
        "a[href*=\"/manga/\"]",
        "a[href*=\"/comic/\"]",
        "a[href*=\"/series/\"]",
        "a[href*=\"/title/\"]",
        "a[href*=\"/book/\"]",
        ".book-item a",
        ".story_name a",
        ".list-manga a",
    ];

    for sel_str in candidate_selectors {
        if let Ok(sel) = scraper::Selector::parse(sel_str) {
            let mut sample_titles = Vec::new();
            let mut count = 0;
            let mut seen_urls = std::collections::HashSet::new();

            for el in doc.select(&sel) {
                let Some(href) = el.value().attr("href") else {
                    continue;
                };
                if crate::sources::is_system_or_nav_link(&el, href) {
                    continue;
                }
                if !seen_urls.insert(href.to_string()) {
                    continue;
                }
                let title = el
                    .value()
                    .attr("title")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| el.text().collect::<Vec<_>>().join(" ").trim().to_string());
                if title.is_empty() {
                    continue;
                }
                count += 1;
                if sample_titles.len() < 10 {
                    sample_titles.push(title);
                }
            }

            if count >= 1 {
                crate::logger::info(
                    "lab",
                    &format!(
                        "Rilevati risultati di ricerca con selettore '{}': {} elementi",
                        sel_str, count
                    ),
                );
                return Ok(json!({
                    "found": true,
                    "selector": sel_str,
                    "count": count,
                    "sample_titles": sample_titles
                }));
            }
        }
    }

    Ok(json!({
        "found": false,
        "selector": "",
        "count": 0,
        "sample_titles": []
    }))
}

/// Heuristic chapter detection: automatically identifies chapter lists on a page
#[tauri::command]
pub fn lab_detect_chapters(html: String, _url: String) -> Result<serde_json::Value, String> {
    let doc = scraper::Html::parse_document(&html);

    let candidate_selectors = [
        "a.lc-chapter-row",
        "#chapterList a",
        "a[href*=\"/leer/\"]",
        "table.table tbody tr td a",
        "table tr td a",
        "table.table a",
        "a.chap",
        "a.chapter-link",
        "a.chapter-row",
        ".wp-manga-chapter a",
        ".chapter-list a",
        "ul.chapters a",
        ".chapter-name a",
        ".chapters-list a",
        ".chapter-item a",
        ".chapters a",
        ".chapter-links a",
        ".ch-list a",
        ".listing-chapters_wrap a",
        "a.chapter-title",
        "a[href*=\"/chapter/\"]",
        "a[href*=\"/chapter-\"]",
        "a[href*=\"/read/\"]",
        "a[href*=\"/ch-\"]",
        "a[href*=\"-chapter-\"]",
        "li.volume a",
    ];

    for sel_str in candidate_selectors {
        if let Ok(sel) = scraper::Selector::parse(sel_str) {
            let matches: Vec<_> = doc
                .select(&sel)
                .filter(|el| {
                    el.value()
                        .attr("href")
                        .map(|h| !crate::sources::is_system_or_nav_link(el, h))
                        .unwrap_or(false)
                })
                .collect();
            if matches.len() >= 2 {
                let sample_titles: Vec<String> = matches
                    .iter()
                    .take(10)
                    .map(|el| {
                        let t = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
                        if t.is_empty()
                            || t.eq_ignore_ascii_case("read")
                            || t.eq_ignore_ascii_case("leggi")
                            || t.eq_ignore_ascii_case("view")
                        {
                            let mut cur = el.parent();
                            while let Some(node) = cur {
                                if let Some(elem) = scraper::ElementRef::wrap(node) {
                                    if elem.value().name() == "tr" {
                                        if let Ok(td_sel) = scraper::Selector::parse("td") {
                                            if let Some(td) = elem.select(&td_sel).next() {
                                                let td_text = td
                                                    .text()
                                                    .collect::<Vec<_>>()
                                                    .join(" ")
                                                    .trim()
                                                    .to_string();
                                                if !td_text.is_empty()
                                                    && !td_text.eq_ignore_ascii_case("read")
                                                    && !td_text.eq_ignore_ascii_case("leggi")
                                                {
                                                    return td_text;
                                                }
                                            }
                                        }
                                    }
                                }
                                cur = node.parent();
                            }
                            if let Some(href) = el.value().attr("href") {
                                let derived = crate::sources::derive_chapter_title(href);
                                if !derived.is_empty() {
                                    return derived;
                                }
                            }
                        }
                        t
                    })
                    .filter(|s| !s.is_empty())
                    .collect();

                crate::logger::info(
                    "lab",
                    &format!(
                        "Rilevati capitoli con selettore '{}': {} elementi",
                        sel_str,
                        matches.len()
                    ),
                );

                return Ok(json!({
                    "found": true,
                    "selector": sel_str,
                    "count": matches.len(),
                    "sample_titles": sample_titles
                }));
            }
        }
    }

    // Broad fallback heuristic: check distinct href patterns
    let a_sel = scraper::Selector::parse("a").unwrap();
    let patterns = [
        ("a[href*=\"/chapter/\"]", "/chapter/"),
        ("a[href*=\"/chapter-\"]", "/chapter-"),
        ("a[href*=\"/leer/\"]", "/leer/"),
        ("a[href*=\"/read/\"]", "/read/"),
        ("a[href*=\"-chapter-\"]", "-chapter-"),
        ("a[href*=\"/ch-\"]", "/ch-"),
        ("a[href*=\"/ch/\"]", "/ch/"),
    ];

    for (sel_name, pattern) in patterns {
        let mut sample_titles = Vec::new();
        let mut count = 0;
        for el in doc.select(&a_sel) {
            if let Some(href) = el.value().attr("href") {
                if crate::sources::is_system_or_nav_link(&el, href) {
                    continue;
                }
                if href.to_lowercase().contains(pattern) {
                    count += 1;
                    if sample_titles.len() < 10 {
                        let mut t = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
                        if t.is_empty()
                            || t.eq_ignore_ascii_case("read")
                            || t.eq_ignore_ascii_case("leggi")
                            || t.eq_ignore_ascii_case("view")
                        {
                            t = crate::sources::derive_chapter_title(href);
                        }
                        sample_titles.push(t);
                    }
                }
            }
        }
        if count >= 2 {
            return Ok(json!({
                "found": true,
                "selector": sel_name,
                "count": count,
                "sample_titles": sample_titles
            }));
        }
    }

    Ok(json!({
        "found": false,
        "selector": "",
        "count": 0,
        "sample_titles": []
    }))
}

/// Infers the search_path (and optionally suggested result_selector) directly from HTML forms or search inputs.
pub fn infer_search_path_from_html_form(
    html: &str,
    _base_url: &str,
) -> Option<(String, Option<String>)> {
    let doc = scraper::Html::parse_document(html);

    // 1. Scan for <form> with GET (or unspecified) method containing search inputs
    if let Ok(form_sel) = scraper::Selector::parse("form") {
        if let Ok(input_sel) = scraper::Selector::parse("input") {
            for form in doc.select(&form_sel) {
                let method = form.value().attr("method").unwrap_or("get").to_lowercase();
                if method == "post" {
                    continue;
                }

                let mut search_param: Option<String> = None;
                for inp in form.select(&input_sel) {
                    let itype = inp.value().attr("type").unwrap_or("text").to_lowercase();
                    let name = inp.value().attr("name").unwrap_or("").to_string();
                    let name_lower = name.to_lowercase();
                    let id_class = format!(
                        "{} {}",
                        inp.value().attr("id").unwrap_or(""),
                        inp.value().attr("class").unwrap_or("")
                    )
                    .to_lowercase();

                    let is_search_field = itype == "search"
                        || [
                            "q", "s", "search", "keyword", "term", "query", "key", "title", "text",
                            "name", "k",
                        ]
                        .iter()
                        .any(|&k| {
                            name_lower == k
                                || name_lower.contains(&format!("_{k}"))
                                || name_lower.contains(&format!("{k}_"))
                        })
                        || id_class.contains("search")
                        || id_class.contains("buscar");

                    if is_search_field {
                        search_param = Some(if name.trim().is_empty() {
                            "q".to_string()
                        } else {
                            name
                        });
                        break;
                    }
                }

                if let Some(param) = search_param {
                    let raw_action = form.value().attr("action").unwrap_or("/").trim();
                    let action = if raw_action.is_empty() {
                        "/".to_string()
                    } else if raw_action.starts_with("http://")
                        || raw_action.starts_with("https://")
                    {
                        if let Ok(parsed) = url::Url::parse(raw_action) {
                            let mut p = parsed.path().to_string();
                            if let Some(q) = parsed.query() {
                                p = format!("{p}?{q}");
                            }
                            p
                        } else {
                            raw_action.to_string()
                        }
                    } else if !raw_action.starts_with('/') {
                        format!("/{raw_action}")
                    } else {
                        raw_action.to_string()
                    };

                    let search_path = if action.contains('?') {
                        format!("{action}&{param}={{query}}")
                    } else {
                        format!("{action}?{param}={{query}}")
                    };

                    // Check for suggested result selectors in the page
                    let mut suggested_sel = None;
                    let candidate_selectors = [
                        "a.lc-card-name, a.lc-sr-item",
                        "a.lc-card-name",
                        "a.lc-sr-item",
                        ".manga_search_item h3 a",
                        ".post-title a",
                        "div.my-3 a.mb-2",
                        ".manga_pic_list li .title a",
                        "a.manga-title",
                    ];
                    for c_sel in candidate_selectors {
                        if let Ok(s) = scraper::Selector::parse(c_sel) {
                            if doc.select(&s).next().is_some() {
                                suggested_sel = Some(c_sel.to_string());
                                break;
                            }
                        }
                    }

                    return Some((search_path, suggested_sel));
                }
            }
        }
    }

    // 2. Scan for standalone <input type="search"> or search input without a <form>
    if let Ok(input_sel) = scraper::Selector::parse(
        "input[type='search'], input[data-search-input], input[name='q'], input[name='s'], input[name='search'], input[class*='search'], input[id*='search']",
    ) {
        if let Some(inp) = doc.select(&input_sel).next() {
            let param = inp.value().attr("name").unwrap_or("q");
            let param_clean = if param.trim().is_empty() {
                "q"
            } else {
                param
            };

            // Check if input has data-url or data-action or data-search-url
            for attr in ["data-action", "data-url", "data-search-url", "action"] {
                if let Some(val) = inp.value().attr(attr) {
                    if !val.trim().is_empty() {
                        let sep = if val.contains('?') { "&" } else { "?" };
                        let path = if val.starts_with('/') {
                            val.to_string()
                        } else {
                            format!("/{val}")
                        };
                        return Some((
                            format!("{path}{sep}{param_clean}={{query}}"),
                            Some("a.lc-card-name, a.lc-sr-item".into()),
                        ));
                    }
                }
            }

            // Check navbar / links for catalog or search links (e.g. /manga/, /catalogo/, /search/)
            if let Ok(a_sel) = scraper::Selector::parse("a[href]") {
                for a in doc.select(&a_sel) {
                    if let Some(href) = a.value().attr("href") {
                        let href_lower = href.to_lowercase();
                        if href_lower == "/manga/"
                            || href_lower == "/manga"
                            || href_lower == "/catalogo/"
                            || href_lower == "/catalog/"
                            || href_lower == "/series/"
                        {
                            let clean_href = if href.ends_with('/') {
                                href.to_string()
                            } else {
                                format!("{href}/")
                            };
                            let path = if clean_href.starts_with('/') {
                                clean_href
                            } else {
                                format!("/{clean_href}")
                            };
                            return Some((
                                format!("{path}?{param_clean}={{query}}"),
                                Some("a.lc-card-name, a.lc-sr-item".into()),
                            ));
                        }
                        if href_lower.contains("/search") || href_lower.contains("/buscar") {
                            let path = if href.starts_with('/') {
                                href.to_string()
                            } else {
                                format!("/{href}")
                            };
                            let sep = if path.contains('?') { "&" } else { "?" };
                            return Some((format!("{path}{sep}{param_clean}={{query}}"), None));
                        }
                    }
                }
            }

            // Check inline script for search endpoints
            if html.contains("/manga/?")
                || html.contains("/search/?q=")
                || html.contains("data-live-search")
            {
                return Some((
                    "/manga/?q={query}".to_string(),
                    Some("a.lc-card-name, a.lc-sr-item".into()),
                ));
            }

            // Generic fallback
            return Some(("/?s={query}".to_string(), None));
        }
    }

    None
}

/// Tauri command to infer search_path and result_selector from HTML
#[tauri::command]
pub fn lab_infer_search_path_from_html(
    html: String,
    base_url: String,
) -> Result<serde_json::Value, String> {
    if let Some((search_path, selector)) = infer_search_path_from_html_form(&html, &base_url) {
        Ok(json!({
            "found": true,
            "search_path": search_path,
            "result_selector": selector
        }))
    } else {
        Ok(json!({
            "found": false,
            "search_path": "",
            "result_selector": null
        }))
    }
}

/// Clean HTML adblock sanitizer for acquired pages
pub fn sanitize_html_adblock(raw: &str) -> String {
    // Strip scripts, iframes, objects, embeds, and intrusive ad tags
    let without_scripts = regex::Regex::new(r"(?is)<script\b[^>]*>.*?</script>")
        .unwrap()
        .replace_all(raw, "");
    let without_iframes = regex::Regex::new(r"(?is)<iframe\b[^>]*>.*?</iframe>")
        .unwrap()
        .replace_all(&without_scripts, "");
    let without_embeds = regex::Regex::new(r"(?is)<(?:object|embed)\b[^>]*>.*?</(?:object|embed)>")
        .unwrap()
        .replace_all(&without_iframes, "");
    let without_popups = regex::Regex::new(r#"(?i)on\w+\s*=\s*["'][^"']*window\.open[^"']*["']"#)
        .unwrap()
        .replace_all(&without_embeds, "");
    // Strip junk classes injected by browser extensions/themes (dracula, darkreader, pokko picker, etc.)
    let class_cleaner = regex::Regex::new(
        r#"(?i)\b(dracula-[a-z0-9_-]+|dracula-processed|darkreader[a-z0-9_-]*|__pokko[a-z0-9_-]*|stylus[a-z0-9_-]*)\b"#,
    )
    .unwrap();
    let cleaned = class_cleaner.replace_all(&without_popups, "");
    cleaned.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires a local Firefox installation; runs in an isolated headless profile"]
    fn marionette_real_firefox_session() {
        struct Browser {
            child: std::process::Child,
            profile: PathBuf,
        }
        impl Drop for Browser {
            fn drop(&mut self) {
                let _ = self.child.kill();
                let _ = self.child.wait();
                let _ = fs::remove_dir_all(&self.profile);
            }
        }
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let profile = std::env::temp_dir().join(format!("pokko-marionette-test-{}", id()));
        fs::create_dir_all(&profile).unwrap();
        fs::write(profile.join("user.js"), format!("user_pref(\"marionette.port\", {});\nuser_pref(\"browser.shell.checkDefaultBrowser\", false);", address.port())).unwrap();
        drop(listener);
        let child = std::process::Command::new("firefox")
            .args([
                "--headless",
                "--no-remote",
                "--new-instance",
                "--marionette",
                "--profile",
            ])
            .arg(&profile)
            .arg("about:blank")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let mut browser = Browser { child, profile };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(25);
        let mut stream = loop {
            if let Ok(stream) = std::net::TcpStream::connect_timeout(
                &address,
                std::time::Duration::from_millis(100),
            ) {
                break stream;
            }
            assert!(
                browser.child.try_wait().unwrap().is_none(),
                "Firefox exited before Marionette was ready"
            );
            assert!(
                std::time::Instant::now() < deadline,
                "Marionette startup timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(100));
        };
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        marionette_session(&mut stream).unwrap();
        for _ in 0..5 {
            marionette_send_cmd(
                &mut stream,
                2,
                "WebDriver:Navigate",
                json!({"url": "about:blank"}),
            )
            .unwrap();
            let url =
                marionette_send_cmd(&mut stream, 2, "WebDriver:GetCurrentURL", json!({})).unwrap();
            assert_eq!(url["value"], "about:blank");
            let html =
                marionette_send_cmd(&mut stream, 2, "WebDriver:GetPageSource", json!({})).unwrap();
            assert!(html["value"].as_str().unwrap().contains("<html"));
        }
    }

    #[test]
    fn marionette_errors_are_not_successful_results() {
        let error = marionette_response(
            json!([1, 2, {"error": "invalid session id", "message": "Session closed"}, null]),
            2,
        )
        .unwrap_err();
        assert!(error.contains("Session closed"));
        assert!(marionette_response(json!([1, 3, null, {}]), 2).is_err());
        assert!(marionette_response(json!([0, 2, null, {}]), 2).is_err());
        assert!(marionette_response(json!({}), 2).is_err());
        assert_eq!(
            marionette_response(json!([1, 2, null, {"value": "ok"}]), 2).unwrap(),
            json!({"value": "ok"})
        );
    }

    #[test]
    fn marionette_reuses_session_and_reads_fragmented_unicode_packets() {
        use std::io::Write;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let send = |stream: &mut std::net::TcpStream, value: Value| {
                let body = value.to_string();
                let packet = format!("{}:{}", body.len(), body);
                for part in packet.as_bytes().chunks(3) {
                    stream.write_all(part).unwrap();
                }
            };
            send(
                &mut stream,
                json!({"applicationType": "gecko", "marionetteProtocol": 3}),
            );
            let session = marionette_read_packet(&mut stream).unwrap();
            assert_eq!(session[2], "WebDriver:NewSession");
            send(&mut stream, json!([1, 1, null, {"sessionId": "test"}]));
            for command in ["WebDriver:GetCurrentURL", "WebDriver:GetPageSource"] {
                let request = marionette_read_packet(&mut stream).unwrap();
                assert_eq!(request[2], command);
                send(
                    &mut stream,
                    json!([1, 2, null, {"value": "Capitolo 日本語"}]),
                );
            }
        });
        let mut stream = std::net::TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        marionette_session(&mut stream).unwrap();
        for command in ["WebDriver:GetCurrentURL", "WebDriver:GetPageSource"] {
            let result = marionette_send_cmd(&mut stream, 2, command, json!({})).unwrap();
            assert_eq!(result["value"], "Capitolo 日本語");
        }
        server.join().unwrap();
    }

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
        let mw = presets.iter().find(|s| s.id == "mw").unwrap();
        let result = lab_extract(
            mw.clone(),
            include_str!("../../tests/fixtures/detail.html").into(),
            "https://www.mangaworld.mx/manga/1/test".into(),
            "detail".into(),
        )
        .unwrap();
        assert!(result["items"].as_array().unwrap().len() > 100);
        let mf = presets.iter().find(|s| s.id == "mf").unwrap();
        let result = lab_extract(
            mf.clone(),
            include_str!("../../tests/fixtures/mf-reader.html").into(),
            "https://ww3.mangafreak.me/Read1_Test_1".into(),
            "reader".into(),
        )
        .unwrap();
        assert_eq!(result["items"].as_array().unwrap().len(), 36);
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

    #[test]
    fn detect_chapters_readberserk_and_strip_junk_classes() {
        let raw_detail = include_str!("../../tests/fixtures/readberserk-detail.html");
        // Test dirty class cleaning
        let dirty_html = r#"<div class="row"><a class="btn btn-sm btn-primary mr-2 dracula-style-all dracula-style-link dracula-processed darkreader darkreader--sync" href="/chapter/1">Read</a></div>"#;
        let cleaned = sanitize_html_adblock(dirty_html);
        assert!(!cleaned.contains("dracula-style-all"));
        assert!(!cleaned.contains("dracula-processed"));
        assert!(!cleaned.contains("darkreader"));
        assert!(cleaned.contains("btn-primary"));

        // Test chapter detection on real readberserk detail fixture
        let detected =
            lab_detect_chapters(raw_detail.to_string(), "https://readberserk.com".into()).unwrap();
        println!("DETECTED: {:#?}", detected);
        assert_eq!(detected["found"], true);
        assert!(detected["count"].as_u64().unwrap() >= 10);
        let sel = detected["selector"].as_str().unwrap();
        assert!(sel == "table.table tbody tr td a" || sel.contains("chapter"));
        let sample_titles = detected["sample_titles"].as_array().unwrap();
        assert!(!sample_titles.is_empty());
        assert!(sample_titles[0].as_str().unwrap().contains("Berserk"));
    }

    #[test]
    fn detect_search_results_and_chapters_leercapitulo() {
        let search_html = r##"
        <nav><a href="/">Inicio</a><a href="/contact/">Contacto</a></nav>
        <a class="lc-skip" href="#contenido">Saltar al contenido</a>
        <div class="row g-3">
            <article class="lc-card">
                <a class="lc-card-name d-block" href="/manga/psvkfbmjgo/one-piece/">One Piece</a>
                <a class="lc-chapter-link d-block mt-1" href="/leer/psvkfbmjgo/one-piece/1197/">Capitulo 1197</a>
            </article>
            <article class="lc-card">
                <a class="lc-card-name d-block" href="/manga/jdoggn1qix/one-piece-catala/">One Piece (Català)</a>
            </article>
        </div>
        <footer><a href="/privacy-policy/">DMCA</a></footer>
        "##;
        let detected_search = lab_detect_search_results(
            search_html.to_string(),
            "https://www.leercapitulo.co".into(),
        )
        .unwrap();
        assert_eq!(detected_search["found"], true);
        assert!(detected_search["count"].as_u64().unwrap() >= 2);
        assert!(detected_search["selector"]
            .as_str()
            .unwrap()
            .contains("lc-card-name"));

        let chapter_html = r##"
        <nav><a href="/">Inicio</a></nav>
        <a class="lc-skip" href="#contenido">Saltar al contenido</a>
        <div id="chapterList">
            <a class="lc-chapter-row" href="/leer/psvkfbmjgo/one-piece/1197/"><span class="n">Capitulo 1197</span></a>
            <a class="lc-chapter-row" href="/leer/psvkfbmjgo/one-piece/1196/"><span class="n">Capitulo 1196</span></a>
        </div>
        <footer><a href="/privacy-policy/">DMCA</a></footer>
        "##;
        let detected_chap = lab_detect_chapters(
            chapter_html.to_string(),
            "https://www.leercapitulo.co/manga/psvkfbmjgo/one-piece/".into(),
        )
        .unwrap();
        assert_eq!(detected_chap["found"], true);
        assert_eq!(detected_chap["count"], 2);
        assert!(
            detected_chap["selector"]
                .as_str()
                .unwrap()
                .contains("lc-chapter-row")
                || detected_chap["selector"]
                    .as_str()
                    .unwrap()
                    .contains("chapterList")
        );
    }

    #[test]
    fn test_infer_search_path_from_html_form() {
        // 1. Explicit GET form (like leercapitulo /manga/ page)
        let form_html = r##"
        <form class="lc-panel p-3 mb-4" method="get" action="/manga/" data-filter-form>
            <label class="form-label small mb-1" for="f-q">Buscar</label>
            <input type="search" class="form-control" id="f-q" name="q">
            <button type="submit">Buscar</button>
        </form>
        <a class="lc-card-name" href="/manga/psvkfbmjgo/one-piece/">One Piece</a>
        "##;
        let inferred =
            infer_search_path_from_html_form(form_html, "https://www.leercapitulo.co").unwrap();
        assert_eq!(inferred.0, "/manga/?q={query}");
        assert_eq!(inferred.1, Some("a.lc-card-name, a.lc-sr-item".to_string()));

        // 2. Standalone live-search input with catalog link (like leercapitulo homepage)
        let live_search_html = r##"
        <nav class="navbar navbar-expand-lg lc-navbar sticky-top">
            <ul class="navbar-nav">
                <li class="nav-item"><a class="nav-link" href="/manga/">Catalogo</a></li>
            </ul>
            <div class="lc-search" data-live-search>
                <input type="search" class="form-control" placeholder="Buscar manga..." data-search-input>
            </div>
        </nav>
        "##;
        let inferred_live =
            infer_search_path_from_html_form(live_search_html, "https://www.leercapitulo.co")
                .unwrap();
        assert_eq!(inferred_live.0, "/manga/?q={query}");
    }
}
