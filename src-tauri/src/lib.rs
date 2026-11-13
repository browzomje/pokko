pub mod catalog;
pub mod cli;
pub mod export;
pub mod integrity;
pub mod lab;
pub mod logger;
pub mod provider;
pub mod rar;
pub mod sources;
use futures_util::StreamExt;
use provider::{Manga, Volume};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tauri::{Emitter, Manager, State};
use tokio::sync::Mutex;

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub base_url: String,
    pub output: String,
    pub delay_ms: u64,
    pub cbz: bool,
    #[serde(default)]
    pub export_format: String,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub sources: Vec<sources::Source>,
}

impl Settings {
    pub fn new_default(output: String) -> Self {
        Self {
            base_url: String::new(),
            output,
            delay_ms: 500,
            cbz: false,
            export_format: "pdf".into(),
            user_agent: None,
            sources: Vec::new(),
        }
    }
}

pub const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36";

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Verification {
    pub state: String,
    pub verified: usize,
    pub total: usize,
    pub repaired: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Job {
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub verification: Verification,
    pub id: String,
    pub manga: Manga,
    pub volume: Volume,
    pub status: String,
    pub chapter: String,
    pub chapters_done: usize,
    pub pages_done: usize,
    pub pages_total: usize,
    pub bytes: u64,
    pub message: String,
    pub output: String,
    pub settings: Settings,
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub export_done: usize,
    #[serde(default)]
    pub export_total: usize,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Saved {
    #[serde(default)]
    pub source_catalog_version: u32,
    #[serde(default)]
    pub download_preferences_version: u32,
    pub settings: Option<Settings>,
    pub jobs: Vec<Job>,
}
// Adopt the direct-download PDF default once, then preserve user choices.
fn remove_bundled_sources(saved: &mut Saved) {
    if saved.source_catalog_version >= 7 {
        return;
    }
    if let Some(settings) = &mut saved.settings {
        settings.sources.retain(|source| {
            source.origin.as_deref() != Some("official")
                && !(source.origin.is_none()
                    && [
                        "mw",
                        "mf",
                        "mangaread",
                        "mangapill",
                        "mangatown",
                        "md",
                        "readberserk",
                        "readcomics",
                    ]
                    .contains(&source.id.as_str()))
        });
    }
    saved.source_catalog_version = 7;
}

fn adopt_download_preferences(saved: &mut Saved) {
    if saved.download_preferences_version < 1 {
        if let Some(settings) = &mut saved.settings {
            settings.export_format = "pdf".into();
            settings.cbz = false;
        }
        saved.download_preferences_version = 1;
    }
    if saved.download_preferences_version < 2 {
        if let Some(settings) = &mut saved.settings {
            settings.delay_ms = 500;
        }
        saved.download_preferences_version = 2;
    }
    if saved.download_preferences_version < 3 {
        if let Some(settings) = &mut saved.settings {
            let p = Path::new(&settings.output);
            if p.file_name().is_some_and(|name| name == "Downloads") {
                settings.output = p.join("Pokko").display().to_string();
            }
        }
        saved.download_preferences_version = 3;
    }
}

pub fn default_download_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("POKKO_DOWNLOAD_DIR") {
        PathBuf::from(dir)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join("Downloads").join("Pokko")
    } else if let Ok(profile) = std::env::var("USERPROFILE") {
        PathBuf::from(profile).join("Downloads").join("Pokko")
    } else {
        PathBuf::from("./Pokko")
    }
}

pub struct Engine {
    pub client: reqwest::Client,
    pub saved: Mutex<Saved>,
    pub controls: Arc<Mutex<HashMap<String, String>>>,
    pub network: Mutex<HashMap<String, Arc<Mutex<tokio::time::Instant>>>>,
    pub concurrency: Mutex<HashMap<String, Arc<tokio::sync::Semaphore>>>,
    #[allow(dead_code)]
    pub cookie_jar: Arc<reqwest::cookie::Jar>,
    pub active: Mutex<Option<String>>,
    pub preview_pages: Mutex<HashMap<String, Vec<String>>>,
    pub creator_cache: Mutex<HashMap<String, Vec<String>>>,
    pub progress_sink: Mutex<Option<tokio::sync::mpsc::UnboundedSender<Job>>>,
    pub file: PathBuf,
}
fn network_error(error: &reqwest::Error) -> String {
    use std::error::Error;
    let mut details = vec![error.to_string()];
    let mut cause = error.source();
    while let Some(value) = cause {
        let text = value.to_string();
        if !details.contains(&text) {
            details.push(text);
        }
        cause = value.source();
    }
    let chain = details.join(" · ");
    let lower = chain.to_lowercase();
    let hint = if error.is_timeout() {
        "Tempo di connessione scaduto. Verifica la rete e riprova."
    } else if lower.contains("certificate")
        || lower.contains("certificato")
        || lower.contains("notvalidforname")
    {
        "Certificato HTTPS non valido per il dominio. Controlla DNS, proxy e filtri di rete: il dominio potrebbe essere reindirizzato."
    } else if lower.contains("dns") || lower.contains("resolve") || lower.contains("lookup") {
        "Il dominio non viene risolto. Controlla la configurazione DNS e la disponibilità della fonte."
    } else if error.is_connect() {
        "Connessione alla fonte non riuscita. Controlla rete, DNS e proxy."
    } else {
        "Richiesta alla fonte non riuscita."
    };
    format!("Errore di rete: {hint}\nDettagli: {chain}")
}

pub fn build_http_client(
    cookie_jar: Arc<reqwest::cookie::Jar>,
) -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .cookie_provider(cookie_jar)
        .gzip(true)
        .brotli(true)
        .tcp_keepalive(Duration::from_secs(60))
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(10)
        .timeout(Duration::from_secs(45))
        .build()
}

impl Engine {
    pub fn headless(output_dir: Option<PathBuf>) -> Arc<Self> {
        let cookie_jar = Arc::new(reqwest::cookie::Jar::default());
        let output = output_dir
            .unwrap_or_else(default_download_dir)
            .display()
            .to_string();

        let client = build_http_client(cookie_jar.clone()).expect("Failed to build reqwest client");

        let app_dir = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
            .or_else(|_| std::env::var("APPDATA").map(PathBuf::from))
            .map(|p| p.join("pokko"))
            .unwrap_or_else(|_| PathBuf::from("./.pokko"));
        let queue_file = app_dir.join("queue.json");
        let mut saved: Saved = if queue_file.exists() {
            std::fs::read(&queue_file)
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                .unwrap_or_default()
        } else {
            Saved::default()
        };

        remove_bundled_sources(&mut saved);
        if saved.settings.is_none() {
            saved.settings = Some(Settings::new_default(output));
        }

        if let Some(parent) = queue_file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        Arc::new(Self {
            client,
            saved: Mutex::new(saved),
            controls: Arc::new(Mutex::new(HashMap::new())),
            network: Mutex::new(HashMap::new()),
            concurrency: Mutex::new(HashMap::new()),
            cookie_jar,
            active: Mutex::new(None),
            preview_pages: Mutex::new(HashMap::new()),
            creator_cache: Mutex::new(HashMap::new()),
            progress_sink: Mutex::new(None),
            file: queue_file,
        })
    }

    pub async fn persist(&self) -> Result<(), String> {
        let saved = self.saved.lock().await;
        let data = serde_json::to_vec_pretty(&*saved).map_err(|e| e.to_string())?;
        if let Some(parent) = self.file.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let tmp = self.file.with_extension("tmp");
        tokio::fs::write(&tmp, data)
            .await
            .map_err(|e| e.to_string())?;
        tokio::fs::rename(tmp, &self.file)
            .await
            .map_err(|e| e.to_string())
    }
    pub async fn update(&self, app: Option<&tauri::AppHandle>, id: &str, f: impl FnOnce(&mut Job)) {
        let mut saved = self.saved.lock().await;
        if let Some(j) = saved.jobs.iter_mut().find(|j| j.id == id) {
            f(j);
            if let Some(app) = app {
                let _ = app.emit("job-update", j.clone());
            }
            if let Some(sink) = self.progress_sink.lock().await.as_ref() {
                let _ = sink.send(j.clone());
            }
        }
    }
    pub async fn cancelled(&self, id: &str) {
        loop {
            if self
                .controls
                .lock()
                .await
                .get(id)
                .map(String::as_str)
                .is_some_and(|c| c == "cancel" || c == "pause")
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    pub async fn remove(&self, id: &str) -> Result<(), String> {
        self.controls
            .lock()
            .await
            .insert(id.into(), "cancel".into());
        self.saved.lock().await.jobs.retain(|j| j.id != id);
        self.persist().await
    }
    pub async fn checkpoint(&self, id: &str) -> Result<(), String> {
        match self.controls.lock().await.get(id).map(String::as_str) {
            Some("cancel") => Err("Annullato".into()),
            Some("pause") => Err("In pausa".into()),
            _ => Ok(()),
        }
    }
    pub async fn request(
        &self,
        url: &str,
        referer: &str,
        delay: u64,
        id: Option<&str>,
    ) -> Result<reqwest::Response, String> {
        let image_host = url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_owned));
        let page_host = url::Url::parse(referer)
            .ok()
            .and_then(|u| u.host_str().map(str::to_owned));
        let is_image_path = url.split('?').next().is_some_and(|p| {
            [".jpg", ".jpeg", ".png", ".webp", ".gif", ".avif"]
                .iter()
                .any(|e| p.to_lowercase().ends_with(e))
        });
        let is_external_cdn = is_image_path && image_host != page_host;

        let delay = if is_external_cdn && id.is_some_and(|id| !id.starts_with("lab-")) {
            200
        } else if is_image_path && id.is_some_and(|id| !id.starts_with("lab-")) {
            350
        } else if let Some(id) = id {
            self.saved
                .lock()
                .await
                .jobs
                .iter()
                .find(|j| j.id == id)
                .map(|j| {
                    let minimum = j
                        .settings
                        .sources
                        .iter()
                        .find(|s| s.id == j.manga.source_id)
                        .map(|s| s.min_delay_ms)
                        .unwrap_or(1000);
                    j.settings.delay_ms.max(minimum)
                })
                .unwrap_or(delay)
        } else {
            delay
        };
        let host = url::Url::parse(url)
            .map_err(|e| e.to_string())?
            .host_str()
            .ok_or("Dominio mancante")?
            .to_string();

        let max_host_concurrency = if is_external_cdn { 4 } else { 2 };
        let semaphore = self
            .concurrency
            .lock()
            .await
            .entry(host.clone())
            .or_insert_with(|| Arc::new(tokio::sync::Semaphore::new(max_host_concurrency)))
            .clone();
        let _permit = semaphore.acquire().await.map_err(|e| e.to_string())?;

        let limiter = self
            .network
            .lock()
            .await
            .entry(host)
            .or_insert_with(|| {
                Arc::new(Mutex::new(
                    tokio::time::Instant::now() - Duration::from_secs(120),
                ))
            })
            .clone();

        let base_delay = if is_external_cdn {
            delay.max(150)
        } else if is_image_path {
            delay.max(300)
        } else {
            delay.max(500)
        };
        let jitter = 0.80 + rand::random::<f64>() * 0.40;
        let actual_delay = (base_delay as f64 * jitter) as u64;

        let mut gate = loop {
            if let Some(id) = id {
                self.checkpoint(id).await?;
            }
            let gate = limiter.lock().await;
            let wait = (*gate + Duration::from_millis(actual_delay))
                .saturating_duration_since(tokio::time::Instant::now());
            if wait.is_zero() {
                break gate;
            }
            drop(gate);
            tokio::time::sleep(wait.min(Duration::from_millis(100))).await;
        };
        *gate = tokio::time::Instant::now();
        drop(gate);

        let user_agent = self
            .saved
            .lock()
            .await
            .settings
            .as_ref()
            .and_then(|s| s.user_agent.as_ref())
            .filter(|ua| !ua.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| DEFAULT_USER_AGENT.to_string());

        let accept_header = if is_image_path {
            "image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8"
        } else {
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"
        };

        let effective_referer = if !referer.trim().is_empty() {
            referer.to_string()
        } else if let Ok(parsed) = url::Url::parse(url) {
            parsed.join("/").map(|u| u.to_string()).unwrap_or_default()
        } else {
            String::new()
        };

        let mut attempt = 0;
        let max_retries = 2;
        loop {
            let mut req = self
                .client
                .get(url)
                .header(reqwest::header::USER_AGENT, &user_agent)
                .header(reqwest::header::ACCEPT, accept_header)
                .header(
                    reqwest::header::ACCEPT_LANGUAGE,
                    "it-IT,it;q=0.9,en-US;q=0.8,en;q=0.7",
                )
                .header(
                    "Sec-Ch-Ua",
                    "\"Not A(Brand\";v=\"8\", \"Chromium\";v=\"133\", \"Google Chrome\";v=\"133\"",
                )
                .header("Sec-Ch-Ua-Mobile", "?0")
                .header("Sec-Ch-Ua-Platform", "\"Linux\"")
                .header(
                    "Sec-Fetch-Dest",
                    if accept_header.starts_with("image") {
                        "image"
                    } else {
                        "document"
                    },
                )
                .header("Sec-Fetch-Mode", "navigate")
                .header("Sec-Fetch-Site", "cross-site");

            if !effective_referer.is_empty() {
                req = req.header(reqwest::header::REFERER, &effective_referer);
            }

            let send = req.send();
            let response = if let Some(id) = id {
                tokio::select! {
                    response=send => response.map_err(|e|network_error(&e)),
                    _=self.cancelled(id) => { self.checkpoint(id).await?; return Err("In pausa".into()); },
                }
            } else {
                send.await.map_err(|e| network_error(&e))
            };
            let r = response?;

            if r.status() == reqwest::StatusCode::SERVICE_UNAVAILABLE && attempt < max_retries {
                attempt += 1;
                let backoff = Duration::from_millis(1000 * attempt as u64);
                if let Some(id) = id {
                    self.checkpoint(id).await?;
                }
                tokio::time::sleep(backoff).await;
                continue;
            }

            if r.status() == reqwest::StatusCode::FORBIDDEN
                || r.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
            {
                let retry = r
                    .headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("non indicato");
                let cooldown = retry
                    .parse::<u64>()
                    .ok()
                    .or_else(|| {
                        httpdate::parse_http_date(retry)
                            .ok()
                            .and_then(|date| date.duration_since(std::time::SystemTime::now()).ok())
                            .map(|d| d.as_secs())
                    })
                    .unwrap_or(60)
                    .min(86400);
                *limiter.lock().await = tokio::time::Instant::now() + Duration::from_secs(cooldown);
                return Err(format!("Blocco HTTP {}. Coda sospesa; Retry-After: {retry}. Riprova più tardi aumentando l'intervallo.",r.status()));
            }
            return r
                .error_for_status()
                .map_err(|e| format!("Risposta del sito: {e}"));
        }
    }
}
#[tauri::command]
async fn bootstrap(engine: State<'_, Arc<Engine>>) -> Result<Saved, String> {
    let s = engine.saved.lock().await;
    Ok(Saved {
        source_catalog_version: s.source_catalog_version,
        download_preferences_version: s.download_preferences_version,
        settings: s.settings.clone(),
        jobs: s.jobs.clone(),
    })
}
#[tauri::command]
async fn save_settings(settings: Settings, engine: State<'_, Arc<Engine>>) -> Result<(), String> {
    let mut ids = std::collections::HashSet::new();
    for source in &settings.sources {
        sources::validate(source)?;
        if !ids.insert(&source.id) {
            return Err("Identificatori fonti duplicati".into());
        }
    }
    if !settings.output.trim().is_empty() && !Path::new(&settings.output).is_absolute() {
        return Err("Scegli una cartella di destinazione assoluta".into());
    }
    if !["images", "cbz", "cbr", "pdf"].contains(&output_format(&settings)) {
        return Err("Formato non valido".into());
    }
    if settings.delay_ms < 500 || settings.delay_ms > 120000 {
        return Err("Intervallo consentito: da 0,5 a 120 secondi".into());
    }
    engine.saved.lock().await.settings = Some(settings);
    engine.persist().await
}
#[derive(Serialize)]
struct SourceReport {
    id: String,
    name: String,
    count: usize,
    has_next: bool,
    error: Option<String>,
}
#[derive(Serialize)]
struct CombinedSearch {
    items: Vec<Manga>,
    has_next: bool,
    reports: Vec<SourceReport>,
}
pub fn manga_source(settings: &Settings, manga: &Manga) -> Result<sources::Source, String> {
    settings
        .sources
        .iter()
        .find(|s| s.id == manga.source_id)
        .cloned()
        .or_else(|| {
            if manga.source_id.is_empty() {
                let url = url::Url::parse(&manga.url).ok()?;
                let host = url.host_str()?;
                settings
                    .sources
                    .iter()
                    .find(|source| {
                        url::Url::parse(&source.base_url)
                            .ok()
                            .and_then(|base| base.host_str().map(|candidate| candidate == host))
                            .unwrap_or(false)
                    })
                    .cloned()
            } else {
                None
            }
        })
        .ok_or_else(|| "La fonte è stata rimossa. Aggiungila nuovamente nelle impostazioni".into())
}
#[tauri::command]
async fn search_manga(
    app: tauri::AppHandle,
    request_id: String,
    query: String,
    page: u32,
    source_ids: Vec<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<CombinedSearch, String> {
    let settings = engine
        .saved
        .lock()
        .await
        .settings
        .clone()
        .ok_or("Impostazioni non disponibili")?;
    let enabled = settings
        .sources
        .iter()
        .filter(|s| s.enabled && (source_ids.is_empty() || source_ids.contains(&s.id)))
        .cloned()
        .collect::<Vec<_>>();
    if enabled.is_empty() {
        return Err("Attiva almeno una fonte nella sezione Fonti".into());
    }
    let responses = futures_util::stream::iter(enabled.into_iter().map(|source| {
        let engine = engine.inner().clone();
        let query = query.clone();
        let delay = settings.delay_ms;
        let app = app.clone();
        let request_id = request_id.clone();
        async move {
            let result = catalog::search_source(&engine, &source, &query, page.max(1), delay, |partial| {
                let _ = app.emit("catalog-results", serde_json::json!({"request_id":request_id,"source_id":source.id,"items":partial.items,"report":{"id":source.id,"name":source.name,"count":partial.items.len(),"has_next":partial.has_next,"error":null}}));
            }).await;
            if let Err(error) = &result {
                let _ = app.emit("catalog-results", serde_json::json!({"request_id":request_id,"source_id":source.id,"items":[],"report":{"id":source.id,"name":source.name,"count":0,"has_next":false,"error":error}}));
            }
            (source, result)
        }
    }))
    .buffer_unordered(8)
    .collect::<Vec<_>>()
    .await;
    let mut items = Vec::new();
    let mut reports = Vec::new();
    let mut has_next = false;
    for (source, result) in responses {
        match result {
            Ok(r) => {
                has_next |= r.has_next;
                reports.push(SourceReport {
                    id: source.id,
                    name: source.name,
                    count: r.items.len(),
                    has_next: r.has_next,
                    error: None,
                });
                items.extend(r.items);
            }
            Err(error) => reports.push(SourceReport {
                id: source.id,
                name: source.name,
                count: 0,
                has_next: false,
                error: Some(error),
            }),
        }
    }
    items.sort_by(|a, b| {
        a.title
            .to_lowercase()
            .cmp(&b.title.to_lowercase())
            .then(a.source_name.cmp(&b.source_name))
            .then(a.language.cmp(&b.language))
    });
    reports.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(CombinedSearch {
        items,
        has_next,
        reports,
    })
}
#[tauri::command]
async fn manga_detail(
    manga: Manga,
    engine: State<'_, Arc<Engine>>,
) -> Result<provider::Detail, String> {
    let settings = engine
        .saved
        .lock()
        .await
        .settings
        .clone()
        .ok_or("Impostazioni non disponibili")?;
    let source = manga_source(&settings, &manga)?;
    sources::detail(&engine, &source, manga, settings.delay_ms).await
}
#[tauri::command]
async fn manga_cover(url: String, engine: State<'_, Arc<Engine>>) -> Result<String, String> {
    use base64::Engine as _;
    let parsed = url::Url::parse(&url).map_err(|e| e.to_string())?;
    if parsed.scheme() != "https" {
        return Err("Copertina non valida".into());
    }
    let response = engine.request(&url, &url, 500, None).await?;
    if response
        .content_length()
        .is_some_and(|n| n > 4 * 1024 * 1024)
    {
        return Err("Copertina troppo grande".into());
    }
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Copertina troppo grande".into());
    }
    let ext = image_extension(&bytes)?;
    Ok(format!(
        "data:image/{};base64,{}",
        if ext == "jpg" { "jpeg" } else { ext },
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}
pub fn output_format(settings: &Settings) -> &str {
    if settings.export_format.is_empty() {
        if settings.cbz {
            "cbz"
        } else {
            "images"
        }
    } else {
        &settings.export_format
    }
}
#[tauri::command]
async fn remove_all_jobs(
    app: tauri::AppHandle,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), String> {
    let ids = engine
        .saved
        .lock()
        .await
        .jobs
        .iter()
        .filter(|j| j.status != "completed")
        .map(|j| j.id.clone())
        .collect::<Vec<_>>();
    for id in ids {
        engine.remove(&id).await?;
        let _ = app.emit("job-removed", id);
    }
    Ok(())
}
#[tauri::command]
async fn open_job_folder(id: String, engine: State<'_, Arc<Engine>>) -> Result<(), String> {
    let saved = engine.saved.lock().await;
    let job = saved
        .jobs
        .iter()
        .find(|j| j.id == id)
        .ok_or("Download non trovato")?;
    let path = PathBuf::from(&job.output);
    if job.output.is_empty() {
        return Err("Nessun file ancora salvato".into());
    }
    let folder = if path.is_dir() {
        path
    } else {
        path.parent().ok_or("Cartella non trovata")?.to_path_buf()
    };
    open::that_detached(folder).map_err(|e| e.to_string())
}
#[tauri::command]
async fn chapter_pages(
    manga: Manga,
    chapter: provider::Chapter,
    engine: State<'_, Arc<Engine>>,
) -> Result<usize, String> {
    let settings = engine
        .saved
        .lock()
        .await
        .settings
        .clone()
        .ok_or("Impostazioni mancanti")?;
    let source = manga_source(&settings, &manga)?;
    if let Some(pages) = engine.preview_pages.lock().await.get(&chapter.url) {
        return Ok(pages.len());
    }
    let pages = sources::pages(
        &engine,
        &source,
        &chapter,
        &manga.url,
        settings.delay_ms,
        None,
    )
    .await?;
    let count = pages.len();
    let mut cache = engine.preview_pages.lock().await;
    if cache.len() >= 512 {
        cache.clear();
    }
    cache.insert(chapter.url, pages);
    Ok(count)
}
#[tauri::command]
async fn test_source_connection(
    source: sources::Source,
    engine: State<'_, Arc<Engine>>,
) -> Result<String, String> {
    sources::validate(&source)?;
    let url = &source.base_url;
    let response = engine
        .request(url, &source.base_url, source.min_delay_ms, None)
        .await?;
    if !response.status().is_success() {
        return Err(format!("La fonte risponde HTTP {}", response.status()));
    }
    Ok("Connessione HTTPS verificata. Puoi abilitare la fonte nel passo Verifica e salva.".into())
}
#[tauri::command]
async fn archive_completed(
    app: tauri::AppHandle,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), String> {
    {
        let mut saved = engine.saved.lock().await;
        for job in &mut saved.jobs {
            if job.status == "completed" {
                job.archived = true;
                let _ = app.emit("job-update", job.clone());
            }
        }
    }
    engine.persist().await
}
#[tauri::command]
async fn remove_job(
    id: String,
    app: tauri::AppHandle,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), String> {
    engine.remove(&id).await?;
    let _ = app.emit("job-removed", &id);
    Ok(())
}
#[tauri::command]
async fn enqueue(
    manga: Manga,
    volumes: Vec<Volume>,
    app: tauri::AppHandle,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), String> {
    provider::site(&manga.url)?;
    let mut saved = engine.saved.lock().await;
    let settings = saved
        .settings
        .clone()
        .ok_or("Impostazioni non disponibili")?;
    manga_source(&settings, &manga)?;
    for volume in volumes {
        if volume.chapters.is_empty() {
            continue;
        }
        for c in &volume.chapters {
            provider::site(&c.url)?;
        }
        if saved.jobs.iter().any(|j| {
            j.manga.url == manga.url
                && j.volume.title == volume.title
                && output_format(&j.settings) == output_format(&settings)
                && !["error", "cancelled"].contains(&j.status.as_str())
        }) {
            continue;
        }
        let id = format!(
            "{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            saved.jobs.len()
        );
        let job = Job {
            archived: false,
            verification: Verification::default(),
            id,
            manga: manga.clone(),
            volume,
            status: "queued".into(),
            chapter: String::new(),
            chapters_done: 0,
            pages_done: 0,
            pages_total: 0,
            bytes: 0,
            message: "In attesa".into(),
            output: String::new(),
            settings: settings.clone(),
            phase: "queued".into(),
            export_done: 0,
            export_total: 0,
        };
        let _ = app.emit("job-update", job.clone());
        saved.jobs.push(job);
    }
    drop(saved);
    engine.persist().await
}
#[tauri::command]
async fn control_job(
    id: String,
    action: String,
    app: tauri::AppHandle,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), String> {
    let status = engine
        .saved
        .lock()
        .await
        .jobs
        .iter()
        .find(|j| j.id == id)
        .map(|j| j.status.clone())
        .ok_or("Download non trovato")?;
    match action.as_str() {
        "pause" if ["queued", "running"].contains(&status.as_str()) => {
            engine
                .controls
                .lock()
                .await
                .insert(id.clone(), "pause".into());
            engine
                .update(Some(&app), &id, |j| {
                    j.status = "paused".into();
                    j.message = "In pausa".into();
                })
                .await;
        }
        "resume" if ["paused", "error", "cancelled"].contains(&status.as_str()) => {
            engine.controls.lock().await.remove(&id);
            let latest = engine.saved.lock().await.settings.clone();
            engine
                .update(Some(&app), &id, |j| {
                    j.status = "queued".into();
                    j.archived = false;
                    if let Some(settings) = &latest {
                        j.settings.delay_ms = settings.delay_ms;
                        if let Some(source) =
                            settings.sources.iter().find(|s| s.id == j.manga.source_id)
                        {
                            if let Some(existing) =
                                j.settings.sources.iter_mut().find(|s| s.id == source.id)
                            {
                                *existing = source.clone();
                            }
                        }
                    }
                    j.message = "Ripresa richiesta".into();
                })
                .await;
        }
        "cancel" => {
            engine
                .controls
                .lock()
                .await
                .insert(id.clone(), "cancel".into());
            engine.saved.lock().await.jobs.retain(|j| j.id != id);
            let _ = app.emit("job-removed", &id);
        }
        _ => return Err("Azione non disponibile per questo stato".into()),
    };
    engine.persist().await
}
fn image_extension(bytes: &[u8]) -> Result<&'static str, String> {
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Ok("jpg")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Ok("png")
    } else if bytes.starts_with(b"GIF8") {
        Ok("gif")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Ok("webp")
    } else if bytes.get(4..8) == Some(b"ftyp") && bytes.windows(4).any(|w| w == b"avif") {
        Ok("avif")
    } else {
        Err("La risposta non contiene un'immagine riconosciuta".into())
    }
}
async fn fetch_verified_page(
    engine: &Engine,
    app: Option<&tauri::AppHandle>,
    job: &Job,
    referer: &str,
    url: &str,
    minimum: u64,
) -> Result<Vec<u8>, String> {
    let response = engine
        .request(
            url,
            referer,
            job.settings.delay_ms.max(minimum),
            Some(&job.id),
        )
        .await?;
    let declared = response.content_length();
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    let mut pending_bytes = 0;
    let mut last_progress = tokio::time::Instant::now();
    loop {
        let chunk = tokio::select! {chunk=stream.next()=>chunk,_=engine.cancelled(&job.id)=>{engine.checkpoint(&job.id).await?;return Err("In pausa".into())}};
        let Some(chunk) = chunk else { break };
        let chunk = chunk.map_err(|e| e.to_string())?;
        if bytes.len() + chunk.len() > 64 * 1024 * 1024 {
            return Err("Immagine oltre 64 MB".into());
        }
        bytes.extend_from_slice(&chunk);
        pending_bytes += chunk.len() as u64;
        if last_progress.elapsed() >= Duration::from_millis(200) {
            engine
                .update(app, &job.id, |j| j.bytes += pending_bytes)
                .await;
            pending_bytes = 0;
            last_progress = tokio::time::Instant::now();
        }
    }
    engine
        .update(app, &job.id, |j| j.bytes += pending_bytes)
        .await;
    if declared.is_some_and(|size| size != bytes.len() as u64) {
        return Err("Trasferimento incompleto".into());
    }
    integrity::image(&bytes)?;
    Ok(bytes)
}
pub async fn download(
    engine: &Engine,
    app: Option<&tauri::AppHandle>,
    job: &Job,
) -> Result<(), String> {
    engine.checkpoint(&job.id).await?;
    let source = manga_source(&job.settings, &job.manga)?;
    let dir = if job.manga.source_id.is_empty() {
        PathBuf::from(&job.settings.output)
            .join(provider::safe_name(&job.manga.title))
            .join(provider::safe_name(&job.volume.title))
    } else {
        PathBuf::from(&job.settings.output)
            .join(provider::safe_name(&format!(
                "{} [{}]",
                job.manga.title,
                if job.manga.language.is_empty() {
                    "it"
                } else {
                    &job.manga.language
                }
            )))
            .join(provider::safe_name(&source.name))
            .join(provider::safe_name(&job.volume.title))
    };
    let format = output_format(&job.settings);
    let destination_dir = dir.clone();
    let dir = if format == "images" {
        dir
    } else {
        dir.parent().unwrap().join(format!(".mwr-{}", job.id))
    };
    if format != "images" {
        let destination = destination_dir.with_extension(format);
        let expected = if job.verification.total > 0 {
            Some(job.verification.total)
        } else {
            None
        };
        if let Ok(pages) = integrity::verify_export(&destination, format, expected) {
            engine
                .update(app, &job.id, |j| {
                    j.output = destination.display().to_string();
                    j.status = "completed".into();
                    j.chapters_done = j.volume.chapters.len();
                    j.bytes = 0;
                    j.verification = Verification {
                        state: "verified".into(),
                        verified: pages,
                        total: pages,
                        repaired: 0,
                    };
                    j.message = format!("File già esportato · {pages} pagine verificate");
                })
                .await;
            return Ok(());
        }
    }
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;
    engine
        .update(app, &job.id, |j| {
            j.status = "running".into();
            j.phase = "downloading".into();
            j.output = dir.display().to_string();
            j.chapters_done = 0;
            j.bytes = 0;
            j.verification = Verification {
                state: "checking".into(),
                ..Verification::default()
            };
        })
        .await;
    let mut total_pages = 0;
    for (ci, chapter) in job.volume.chapters.iter().enumerate() {
        engine.checkpoint(&job.id).await?;
        engine
            .update(app, &job.id, |j| {
                j.chapter = chapter.title.clone();
                j.pages_done = 0;
                j.pages_total = 0;
                j.message = "Estrazione delle pagine…".into();
            })
            .await;
        let chapter_dir = dir.join(format!(
            "{:04}-{}",
            ci + 1,
            provider::safe_name(&chapter.title)
        ));
        tokio::fs::create_dir_all(&chapter_dir)
            .await
            .map_err(|e| e.to_string())?;
        let cache = chapter_dir.join(".pages.json");
        let preview = engine.preview_pages.lock().await.get(&chapter.url).cloned();
        let cached = if let Some(preview) = preview {
            Some(preview)
        } else {
            tokio::fs::read(&cache)
                .await
                .ok()
                .and_then(|b| serde_json::from_slice::<(String, Vec<String>)>(&b).ok())
                .filter(|(url, pages)| {
                    url == &chapter.url
                        && !pages.is_empty()
                        && pages.iter().all(|p| provider::resolve(p, p).is_ok())
                })
                .map(|(_, pages)| pages)
        };
        let pages = if let Some(pages) = cached {
            pages
        } else {
            let pages = sources::pages(
                engine,
                &source,
                chapter,
                &job.manga.url,
                job.settings.delay_ms,
                Some(&job.id),
            )
            .await?;
            let data = serde_json::to_vec(&(chapter.url.clone(), pages.clone()))
                .map_err(|e| e.to_string())?;
            tokio::fs::write(&cache, data)
                .await
                .map_err(|e| e.to_string())?;
            pages
        };
        {
            let mut cache = engine.preview_pages.lock().await;
            if cache.len() >= 512 {
                cache.clear();
            }
            cache.insert(chapter.url.clone(), pages.clone());
        }
        if pages.is_empty() || chapter.page_count.is_some_and(|count| count != pages.len()) {
            return Err("Conteggio pagine diverso da quello annunciato dalla fonte".into());
        }
        total_pages += pages.len();
        engine
            .update(app, &job.id, |j| j.verification.total = total_pages)
            .await;
        let proof_file = chapter_dir.join(".proofs.json");
        let mut proofs: integrity::PageProofs = tokio::fs::read(&proof_file)
            .await
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let mut kept = std::collections::HashSet::new();
        engine
            .update(app, &job.id, |j| j.pages_total = pages.len())
            .await;

        let mut to_fetch = Vec::new();
        let mut existing_count = 0;

        for (pi, url) in pages.iter().enumerate() {
            engine.checkpoint(&job.id).await?;
            let stem = format!("{:04}", pi + 1);
            let mut exists = false;
            let mut damaged = false;
            for ext in ["jpg", "png", "gif", "webp", "avif"] {
                let name = format!("{stem}.{ext}");
                let path = chapter_dir.join(&name);
                if let Ok(bytes) = tokio::fs::read(&path).await {
                    let matches = proofs.get(&stem).is_none_or(|proof| {
                        proof.url == *url
                            && proof.file == name
                            && proof.size == bytes.len() as u64
                            && proof.crc32 == crc32fast::hash(&bytes)
                    });
                    if !exists && matches && integrity::image(&bytes).is_ok() {
                        proofs.insert(
                            stem.clone(),
                            integrity::PageProof {
                                url: url.clone(),
                                file: name.clone(),
                                size: bytes.len() as u64,
                                crc32: crc32fast::hash(&bytes),
                            },
                        );
                        kept.insert(name);
                        exists = true;
                    } else {
                        tokio::fs::remove_file(&path)
                            .await
                            .map_err(|e| e.to_string())?;
                        damaged = true;
                    }
                }
            }
            if exists {
                existing_count += 1;
                engine
                    .update(app, &job.id, |j| {
                        j.verification.verified += 1;
                        j.pages_done = existing_count;
                        j.message = "Pagina già presente, saltata".into();
                    })
                    .await;
            } else {
                to_fetch.push((pi, url.clone(), damaged));
            }
        }

        if !to_fetch.is_empty() {
            const SCAN_CONCURRENCY: usize = 3;
            let mut stream = futures_util::stream::iter(to_fetch)
                .map(|(pi, url, damaged)| {
                    let chapter_url = chapter.url.clone();
                    let min_delay_ms = source.min_delay_ms;
                    let chapter_dir = chapter_dir.clone();
                    let pages_total = pages.len();
                    async move {
                        let stem = format!("{:04}", pi + 1);
                        let mut last_error = String::new();
                        for attempt in 0..3 {
                            engine.checkpoint(&job.id).await?;
                            engine
                                .update(app, &job.id, |j| {
                                    j.verification.state = if damaged || attempt > 0 {
                                        "repairing"
                                    } else {
                                        "checking"
                                    }
                                    .into();
                                    j.message = format!(
                                        "Pagina {} di {} · tentativo {}/3",
                                        pi + 1,
                                        pages_total,
                                        attempt + 1
                                    );
                                })
                                .await;
                            match fetch_verified_page(
                                engine,
                                app,
                                job,
                                &chapter_url,
                                &url,
                                min_delay_ms,
                            )
                            .await
                            {
                                Ok(bytes) => {
                                    let ext = image_extension(&bytes)?;
                                    let name = format!("{stem}.{ext}");
                                    let path = chapter_dir.join(&name);
                                    let part = path.with_extension("part");
                                    tokio::fs::write(&part, &bytes)
                                        .await
                                        .map_err(|e| e.to_string())?;
                                    engine.checkpoint(&job.id).await?;
                                    tokio::fs::rename(part, path)
                                        .await
                                        .map_err(|e| e.to_string())?;
                                    let proof = integrity::PageProof {
                                        url: url.clone(),
                                        file: name.clone(),
                                        size: bytes.len() as u64,
                                        crc32: crc32fast::hash(&bytes),
                                    };
                                    return Ok((pi, stem, name, proof, damaged, attempt > 0));
                                }
                                Err(e) => {
                                    engine.checkpoint(&job.id).await?;
                                    if e.contains("HTTP 403") {
                                        return Err(e);
                                    }
                                    last_error = e;
                                    if attempt < 2 {
                                        tokio::time::sleep(Duration::from_millis(
                                            800 * (attempt + 1),
                                        ))
                                        .await;
                                    }
                                }
                            }
                        }
                        Err(format!(
                            "Pagina {} non valida dopo 3 tentativi: {last_error}",
                            pi + 1
                        ))
                    }
                })
                .buffer_unordered(SCAN_CONCURRENCY);

            while let Some(result) = stream.next().await {
                match result {
                    Ok((_pi, stem, name, proof, damaged, retried)) => {
                        proofs.insert(stem, proof);
                        kept.insert(name);
                        if damaged || retried {
                            engine
                                .update(app, &job.id, |j| j.verification.repaired += 1)
                                .await;
                        }
                        existing_count += 1;
                        let count = existing_count;
                        engine
                            .update(app, &job.id, |j| {
                                j.verification.verified += 1;
                                j.pages_done = count;
                                j.message = "Pagina salvata".into();
                            })
                            .await;

                        let part = proof_file.with_extension("json.part");
                        if let Ok(data) = serde_json::to_vec(&proofs) {
                            if tokio::fs::write(&part, data).await.is_ok() {
                                let _ = tokio::fs::rename(part, &proof_file).await;
                            }
                        }
                    }
                    Err(err) => {
                        engine.preview_pages.lock().await.remove(&chapter.url);
                        let _ = tokio::fs::remove_file(&cache).await;
                        return Err(err);
                    }
                }
            }
        }

        let part = proof_file.with_extension("json.part");
        tokio::fs::write(
            &part,
            serde_json::to_vec(&proofs).map_err(|e| e.to_string())?,
        )
        .await
        .map_err(|e| e.to_string())?;
        tokio::fs::rename(part, &proof_file)
            .await
            .map_err(|e| e.to_string())?;
        // Remove stale extra pages so the export cannot silently contain duplicates.
        for entry in std::fs::read_dir(&chapter_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().is_some_and(|e| {
                ["jpg", "png", "webp", "gif", "avif"].contains(&e.to_string_lossy().as_ref())
            }) && !kept.contains(&entry.file_name().to_string_lossy().to_string())
            {
                tokio::fs::remove_file(path)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
        engine
            .update(app, &job.id, |j| j.chapters_done = ci + 1)
            .await;
        engine.persist().await?;
    }
    engine.checkpoint(&job.id).await?;
    if format != "images" {
        engine
            .update(app, &job.id, |j| {
                j.phase = "exporting".into();
                j.export_done = 0;
                j.export_total = total_pages;
                j.message = format!(
                    "Creazione {} · {} pagine verificate",
                    format.to_uppercase(),
                    total_pages
                )
            })
            .await;
        let source = dir.clone();
        let controls = engine.controls.clone();
        let job_id = job.id.clone();
        let destination = destination_dir.with_extension(format);
        let export_format = format.to_string();
        let is_pdf = format == "pdf";
        let (progress_tx, mut progress_rx) =
            tokio::sync::mpsc::unbounded_channel::<(usize, usize)>();
        let blocking_handle =
            tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
                if export::images(&source)?.len() != total_pages {
                    return Err("Numero di immagini diverso dalle pagine attese".into());
                }
                if export_format == "cbr" {
                    rar::create(&source, &destination, || {
                        match controls.blocking_lock().get(&job_id).map(String::as_str) {
                            Some("cancel") => Err("Annullato".into()),
                            Some("pause") => Err("In pausa".into()),
                            _ => Ok(()),
                        }
                    })?;
                    rar::verify(&destination, total_pages)?;
                    integrity::save_export_proof(&destination, "cbr", total_pages)?;
                    return Ok(destination.display().to_string());
                }
                if is_pdf {
                    let controls_clone = controls.clone();
                    let job_id_clone = job_id.clone();
                    let tx = progress_tx;
                    let exported_pages = export::pdf(&source, &destination, |curr, total| {
                        match controls_clone
                            .blocking_lock()
                            .get(&job_id_clone)
                            .map(String::as_str)
                        {
                            Some("cancel") => return Err("Annullato".into()),
                            Some("pause") => return Err("In pausa".into()),
                            _ => {}
                        }
                        let _ = tx.send((curr, total));
                        Ok(())
                    })?;
                    if exported_pages != total_pages {
                        return Err("PDF incompleto".into());
                    }
                    integrity::save_export_proof(&destination, "pdf", total_pages)?;
                    return Ok(destination.display().to_string());
                }
                let temporary = destination.with_extension("cbz.part");
                let file = std::fs::File::create(&temporary).map_err(|e| e.to_string())?;
                let mut zip = zip::ZipWriter::new(std::io::BufWriter::new(file));
                let options = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored);
                let mut dirs = std::fs::read_dir(&source)
                    .map_err(|e| e.to_string())?
                    .filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|p| p.is_dir())
                    .collect::<Vec<_>>();
                dirs.sort();
                let mut files_to_write = Vec::new();
                for d in dirs {
                    let mut files = std::fs::read_dir(d)
                        .map_err(|e| e.to_string())?
                        .filter_map(Result::ok)
                        .map(|e| e.path())
                        .filter(|p| {
                            p.is_file()
                                && p.extension()
                                    .map(|x| {
                                        ["jpg", "png", "webp", "gif", "avif"]
                                            .contains(&x.to_string_lossy().as_ref())
                                    })
                                    .unwrap_or(false)
                        })
                        .collect::<Vec<_>>();
                    files.sort();
                    files_to_write.extend(files);
                }
                let total_files = files_to_write.len();
                for (idx, f) in files_to_write.iter().enumerate() {
                    match controls.blocking_lock().get(&job_id).map(String::as_str) {
                        Some("cancel") => return Err("Annullato".into()),
                        Some("pause") => return Err("In pausa".into()),
                        _ => {}
                    }
                    let _ = progress_tx.send((idx + 1, total_files));
                    zip.start_file(
                        f.strip_prefix(&source)
                            .unwrap()
                            .to_string_lossy()
                            .replace('\\', "/"),
                        options,
                    )
                    .map_err(|e| e.to_string())?;
                    zip.write_all(&std::fs::read(f).map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?;
                }
                zip.finish().map_err(|e| e.to_string())?;
                std::fs::rename(temporary, &destination).map_err(|e| e.to_string())?;
                let mut archive = zip::ZipArchive::new(
                    std::fs::File::open(&destination).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                if archive.len() != total_pages {
                    return Err("CBZ incompleto".into());
                }
                for i in 0..archive.len() {
                    use std::io::Read;
                    let mut bytes = Vec::new();
                    archive
                        .by_index(i)
                        .map_err(|e| e.to_string())?
                        .read_to_end(&mut bytes)
                        .map_err(|e| e.to_string())?;
                    integrity::image(&bytes)?;
                }
                integrity::save_export_proof(&destination, "cbz", total_pages)?;
                Ok(destination.display().to_string())
            });
        let fmt_upper = format.to_uppercase();
        while let Some((curr, total)) = progress_rx.recv().await {
            engine
                .update(app, &job.id, |j| {
                    j.phase = "exporting".into();
                    j.export_done = curr;
                    j.export_total = total;
                    j.message = format!("Creazione {} · pagina {} di {}", fmt_upper, curr, total);
                })
                .await;
        }
        let result = blocking_handle.await.map_err(|e| e.to_string())??;
        engine.checkpoint(&job.id).await?;
        engine.update(app, &job.id, |j| j.output = result).await;
        tokio::fs::remove_dir_all(&dir)
            .await
            .map_err(|e| e.to_string())?;
    }
    if format == "images" {
        for entry in std::fs::read_dir(&dir)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
        {
            let cache = entry.path().join(".pages.json");
            if cache.is_file() {
                tokio::fs::remove_file(cache)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    engine
        .update(app, &job.id, |j| {
            j.status = "completed".into();
            j.phase = "completed".into();
            j.export_done = total_pages;
            j.export_total = total_pages;
            j.verification.state = "verified".into();
            j.message = format!(
                "Completo · {total_pages}/{total_pages} pagine verificate · {} riparate",
                j.verification.repaired
            );
        })
        .await;
    Ok(())
}
fn verify_saved_job(job: &Job) -> Result<usize, String> {
    let format = output_format(&job.settings);
    if format != "images" {
        return integrity::verify_export(
            Path::new(&job.output),
            format,
            Some(job.verification.total),
        );
    }
    let mut count = 0;
    for entry in std::fs::read_dir(&job.output).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.path().is_dir() {
            continue;
        }
        let proofs: integrity::PageProofs = serde_json::from_slice(
            &std::fs::read(entry.path().join(".proofs.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        for proof in proofs.values() {
            if Path::new(&proof.file).components().count() != 1 {
                return Err("Nome pagina non valido".into());
            }
            if integrity::fingerprint(&entry.path().join(&proof.file))? != (proof.size, proof.crc32)
            {
                return Err("Pagina mancante o danneggiata".into());
            }
            count += 1;
        }
    }
    if count == 0 || count != job.verification.total {
        return Err("Pagine mancanti".into());
    }
    Ok(count)
}
#[tauri::command]
fn sources_import_file(path: String) -> Result<sources::Source, String> {
    sources::import_connector_file(&path)
}

#[tauri::command]
fn sources_import_folder(path: String) -> Result<Vec<sources::Source>, String> {
    sources::import_connectors_from_folder(&path)
}

#[tauri::command]
async fn sources_import_github(
    repo: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<sources::Source>, String> {
    sources::import_connectors_from_github(&repo, &engine.client).await
}

#[tauri::command]
fn sources_export_file(source: sources::Source, path: String) -> Result<(), String> {
    sources::check_not_archive(&path)?;
    let p = Path::new(&path);
    if p.extension().and_then(|e| e.to_str()) != Some("json") {
        return Err("È possibile esportare solo con estensione .json".into());
    }
    let data = serde_json::to_string_pretty(&source).map_err(|e| e.to_string())?;
    std::fs::write(p, data).map_err(|e| e.to_string())
}

pub fn run_app() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            logger::init(app.handle());
            logger::info("app", "Pokko avviato");
            let path = app.path().app_data_dir()?;
            std::fs::create_dir_all(&path)?;
            let file = path.join("queue.json");
            let mut saved: Saved = if file.exists() {
                serde_json::from_slice(&std::fs::read(&file)?)?
            } else {
                Saved::default()
            };
            remove_bundled_sources(&mut saved);
            saved.jobs.retain(|j| j.status != "cancelled");
            let output = app
                .path()
                .download_dir()
                .or_else(|_| app.path().home_dir().map(|p| p.join("Downloads")))?
                .display()
                .to_string();
            if let Some(settings) = &mut saved.settings {
                if settings.output.trim().is_empty() {
                    settings.output = output;
                }
            } else {
                saved.settings = Some(Settings::new_default(output));
            }
            if let Some(settings) = &mut saved.settings {
                if settings.export_format.is_empty() {
                    settings.export_format = if settings.cbz { "cbz" } else { "images" }.into();
                }
            }
            adopt_download_preferences(&mut saved);
            for job in &mut saved.jobs {
                if ["running", "queued"].contains(&job.status.as_str()) {
                    job.status = "paused".into();
                    job.message = "Sessione ripristinata. Premi Riprendi per continuare".into();
                }
            }
            let cookie_jar = Arc::new(reqwest::cookie::Jar::default());
            let engine = Arc::new(Engine {
                client: build_http_client(cookie_jar.clone())?,
                saved: Mutex::new(saved),
                controls: Arc::new(Mutex::new(HashMap::new())),
                network: Mutex::new(HashMap::new()),
                concurrency: Mutex::new(HashMap::new()),
                cookie_jar,
                active: Mutex::new(None),
                preview_pages: Mutex::new(HashMap::new()),
                creator_cache: Mutex::new(HashMap::new()),
                progress_sink: Mutex::new(None),
                file,
            });
            app.manage(engine.clone());
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let _ = engine.persist().await;
                let completed = engine
                    .saved
                    .lock()
                    .await
                    .jobs
                    .iter()
                    .filter(|j| j.status == "completed" && j.verification.state == "verified")
                    .cloned()
                    .collect::<Vec<_>>();
                for job in completed {
                    let copy = job.clone();
                    let valid =
                        tauri::async_runtime::spawn_blocking(move || verify_saved_job(&copy)).await;
                    if !matches!(valid, Ok(Ok(_))) {
                        engine
                            .update(Some(&handle), &job.id, |j| {
                                j.status = "queued".into();
                                j.archived = false;
                                j.verification.state = "repairing".into();
                                j.message =
                                    "File modificato o danneggiato: riparazione automatica in coda"
                                        .into();
                            })
                            .await;
                    }
                }
                let _ = engine.persist().await;
                loop {
                    let next = engine
                        .saved
                        .lock()
                        .await
                        .jobs
                        .iter()
                        .find(|j| j.status == "queued")
                        .cloned();
                    if let Some(job) = next {
                        *engine.active.lock().await = Some(job.id.clone());
                        let result = download(&engine, Some(&handle), &job).await;
                        if let Err(error) = result {
                            let blocked = error.starts_with("Blocco HTTP");
                            engine
                                .update(Some(&handle), &job.id, |j| {
                                    j.status = if error == "In pausa" {
                                        if j.status == "queued" {
                                            "queued".into()
                                        } else {
                                            "paused".into()
                                        }
                                    } else if error == "Annullato" {
                                        "cancelled".into()
                                    } else {
                                        j.verification.state = "failed".into();
                                        "error".into()
                                    };
                                    j.message = error.clone();
                                })
                                .await;
                            if blocked {
                                let ids = engine
                                    .saved
                                    .lock()
                                    .await
                                    .jobs
                                    .iter()
                                    .filter(|j| {
                                        j.status == "queued"
                                            && j.manga.source_id == job.manga.source_id
                                    })
                                    .map(|j| j.id.clone())
                                    .collect::<Vec<_>>();
                                for id in ids {
                                    engine
                                        .update(Some(&handle), &id, |j| {
                                            j.status = "paused".into();
                                            j.message =
                                                "Coda sospesa dopo un blocco del sito".into();
                                        })
                                        .await;
                                }
                            }
                        }
                        *engine.active.lock().await = None;
                        engine.controls.lock().await.remove(&job.id);
                        let _ = engine.persist().await;
                    } else {
                        tokio::time::sleep(Duration::from_millis(300)).await;
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            save_settings,
            search_manga,
            manga_detail,
            enqueue,
            control_job,
            remove_job,
            archive_completed,
            test_source_connection,
            remove_all_jobs,
            open_job_folder,
            chapter_pages,
            manga_cover,
            lab::validate_source,
            lab::lab_capture,
            lab::lab_samples,
            lab::lab_load,
            lab::lab_delete,
            lab::lab_import,
            lab::lab_export,
            lab::lab_extract,
            lab::lab_download,
            lab::lab_cancel,
            lab::lab_open_output,
            lab::lab_firefox_launch,
            lab::lab_firefox_navigate,
            lab::lab_firefox_get_url,
            lab::lab_firefox_inject_picker,
            lab::lab_firefox_start_picker,
            lab::lab_firefox_get_picked,
            lab::lab_firefox_close,
            lab::lab_firefox_status,
            lab::lab_firefox_get_last_nav,
            lab::lab_firefox_clear_last_nav,
            lab::lab_detect_search_results,
            lab::lab_detect_chapters,
            lab::lab_infer_search_path_from_html,
            logger::get_log_path,
            logger::read_recent_logs,
            logger::clear_logs,
            sources_import_file,
            sources_import_folder,
            sources_import_github,
            sources_export_file
        ])
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::Destroyed | tauri::WindowEvent::CloseRequested { .. } = event
            {
                crate::lab::close_firefox_companion();
            }
        })
        .run(tauri::generate_context!())
        .expect("Errore avvio applicazione");
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_html_as_image() {
        assert!(image_extension(b"<html>Blocked</html>").is_err());
        assert_eq!(image_extension(b"\xff\xd8\xffabc").unwrap(), "jpg");
    }
    pub(super) fn test_engine(output: &Path) -> Engine {
        let cookie_jar = Arc::new(reqwest::cookie::Jar::default());
        Engine {
            client: build_http_client(cookie_jar.clone()).unwrap(),
            saved: Mutex::new(Saved::default()),
            controls: Arc::new(Mutex::new(HashMap::new())),
            network: Mutex::new(HashMap::new()),
            concurrency: Mutex::new(HashMap::new()),
            cookie_jar,
            active: Mutex::new(None),
            preview_pages: Mutex::new(HashMap::new()),
            creator_cache: Mutex::new(HashMap::new()),
            progress_sink: Mutex::new(None),
            file: output.join("queue.json"),
        }
    }
    fn sample_job(path: &Path, format: &str) -> Job {
        let source = sources::Source {
            id: "example".into(),
            base_url: "https://source.test".into(),
            ..Default::default()
        };
        serde_json::from_value(serde_json::json!({
            "id": format, "manga": {"title":"Test", "url":"https://source.test/manga/1/test", "cover":""},
            "volume":{"title":"Volume", "chapters":[{"title":"Chapter", "url":"https://source.test/read/1/test"}]},
            "status":"queued", "chapter":"", "chapters_done":0, "pages_done":0, "pages_total":0, "bytes":0, "message":"", "output":"",
            "settings":{"base_url":"https://source.test", "output":path.display().to_string(), "delay_ms":500, "cbz":true, "export_format":format, "sources":[source]}
        })).unwrap()
    }
    fn seed_pages(path: &Path, job: &Job) -> PathBuf {
        let work = if output_format(&job.settings) == "images" {
            path.join("Test/Volume")
        } else {
            path.join("Test").join(format!(".mwr-{}", job.id))
        };
        let chapter = work.join("0001-Chapter");
        std::fs::create_dir_all(&chapter).unwrap();
        let urls = vec!["https://cdn.test/1.jpg", "https://cdn.test/2.png"];
        std::fs::write(
            chapter.join(".pages.json"),
            serde_json::to_vec(&(job.volume.chapters[0].url.clone(), urls)).unwrap(),
        )
        .unwrap();
        image::RgbImage::from_pixel(12, 20, image::Rgb([255, 0, 0]))
            .save(chapter.join("0001.jpg"))
            .unwrap();
        image::RgbImage::from_pixel(15, 25, image::Rgb([0, 255, 0]))
            .save(chapter.join("0002.png"))
            .unwrap();
        work
    }
    #[tokio::test]
    async fn exclusive_output_formats_and_resume() {
        let path = std::env::temp_dir().join(format!("mwr-formats-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        for format in ["images", "cbz", "cbr", "pdf"] {
            let base = path.join(format);
            std::fs::create_dir_all(&base).unwrap();
            let engine = test_engine(&base);
            let job = sample_job(&base, format);
            let work = seed_pages(&base, &job);
            engine.saved.lock().await.jobs.push(job.clone());
            download(&engine, None, &job).await.unwrap();
            let output = engine.saved.lock().await.jobs[0].output.clone();
            if format == "images" {
                assert!(work.is_dir());
                assert!(!base.join("Test/Volume.cbz").exists());
                assert!(!base.join("Test/Volume.pdf").exists());
            } else {
                assert!(!work.exists());
                assert!(!base.join("Test/Volume").exists());
                if format == "pdf" {
                    let doc = lopdf::Document::load(&output).unwrap();
                    assert_eq!(doc.get_pages().len(), 2);
                } else if format == "cbr" {
                    rar::verify(Path::new(&output), 2).unwrap();
                    // Independent interoperability check when UnRAR is installed.
                    if let Ok(result) = std::process::Command::new("unrar")
                        .args(["t", "-idq", &output])
                        .output()
                    {
                        assert!(
                            result.status.success(),
                            "{}",
                            String::from_utf8_lossy(&result.stderr)
                        );
                    }
                } else {
                    let zip = zip::ZipArchive::new(std::fs::File::open(&output).unwrap()).unwrap();
                    assert_eq!(zip.len(), 2);
                }
            }
            download(&engine, None, &job).await.unwrap();
            assert_eq!(engine.saved.lock().await.jobs[0].bytes, 0);
        }
        std::fs::remove_dir_all(path).unwrap();
    }
    #[tokio::test]
    async fn corrupt_page_is_repaired_without_fetching_healthy_pages() {
        use std::io::{Read, Write};
        let base = std::env::temp_dir().join(format!("mwr-repair-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let engine = test_engine(&base);
        let job = sample_job(&base, "images");
        let work = seed_pages(&base, &job);
        let chapter = work.join("0001-Chapter");
        std::fs::write(chapter.join("0001.jpg"), b"\xff\xd8\xfftruncated").unwrap();
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_pixel(10, 20, image::Rgb([12, 34, 56]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let png = png.into_inner();
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/1.png", server.local_addr().unwrap());
        let thread = std::thread::spawn(move || {
            for bytes in [b"\xff\xd8\xfftruncated".to_vec(), png] {
                let (mut socket, _) = server.accept().unwrap();
                let mut request = [0; 4096];
                let _ = socket.read(&mut request);
                write!(
                    socket,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    bytes.len()
                )
                .unwrap();
                socket.write_all(&bytes).unwrap();
            }
        });
        engine.preview_pages.lock().await.insert(
            job.volume.chapters[0].url.clone(),
            vec![url, "https://cdn.test/2.png".into()],
        );
        engine.saved.lock().await.jobs.push(job.clone());
        tokio::time::timeout(Duration::from_secs(10), download(&engine, None, &job))
            .await
            .unwrap()
            .unwrap();
        thread.join().unwrap();
        let saved = engine.saved.lock().await.jobs[0].clone();
        assert_eq!(saved.verification.state, "verified");
        assert_eq!(saved.verification.verified, 2);
        assert_eq!(saved.verification.repaired, 1);
        assert_eq!(verify_saved_job(&saved).unwrap(), 2);
        assert!(chapter.join("0001.png").is_file());
        assert!(!chapter.join("0001.jpg").exists());
        std::fs::write(chapter.join("0002.png"), b"damaged after completion").unwrap();
        assert!(verify_saved_job(&saved).is_err());
        std::fs::remove_dir_all(base).unwrap();
    }
    #[tokio::test]
    async fn exported_file_corruption_is_detected() {
        let base = std::env::temp_dir().join(format!("mwr-proof-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let engine = test_engine(&base);
        let job = sample_job(&base, "pdf");
        seed_pages(&base, &job);
        engine.saved.lock().await.jobs.push(job.clone());
        download(&engine, None, &job).await.unwrap();
        let finished = engine.saved.lock().await.jobs[0].clone();
        assert_eq!(verify_saved_job(&finished).unwrap(), 2);
        let mut data = std::fs::read(&finished.output).unwrap();
        data[20] ^= 1;
        std::fs::write(&finished.output, data).unwrap();
        assert!(verify_saved_job(&finished).is_err());
        std::fs::remove_dir_all(base).unwrap();
    }
    #[tokio::test]
    #[ignore = "Verifica live mangapill: ricerca, capitoli e scan"]
    async fn live_mangapill_complete_pdf() {
        let base = std::env::temp_dir().join(format!("mwr-mangapill-live-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let engine = test_engine(&base);
        let source = sources::live_test_sources()
            .into_iter()
            .find(|s| s.id == "mangapill")
            .unwrap();
        let result = sources::search(&engine, &source, "Naruto", 1, 500)
            .await
            .unwrap();
        let manga = result
            .items
            .into_iter()
            .find(|m| m.title.eq_ignore_ascii_case("Naruto"))
            .unwrap();
        let detail = sources::detail(&engine, &source, manga, 500).await.unwrap();
        assert!(!detail.volumes.is_empty());
        assert!(!detail.volumes[0].chapters.is_empty());
        let chapter = &detail.volumes[0].chapters[0];
        let pages = sources::pages(
            &engine,
            &source,
            chapter,
            &detail.manga.url,
            500,
            Some("mangapill-test"),
        )
        .await
        .unwrap();
        assert!(!pages.is_empty());
        std::fs::remove_dir_all(base).unwrap();
    }
    #[tokio::test]
    async fn paused_download_releases_queue() {
        let path = std::env::temp_dir().join(format!("mwr-pause-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        let engine = test_engine(&path);
        let first = sample_job(&path, "images");
        let second = sample_job(&path, "pdf");
        seed_pages(&path, &second);
        engine
            .saved
            .lock()
            .await
            .jobs
            .extend([first.clone(), second.clone()]);
        engine
            .controls
            .lock()
            .await
            .insert(first.id.clone(), "pause".into());
        let result =
            tokio::time::timeout(Duration::from_millis(250), download(&engine, None, &first))
                .await
                .unwrap();
        assert_eq!(result.unwrap_err(), "In pausa");
        download(&engine, None, &second).await.unwrap();
        assert_eq!(engine.saved.lock().await.jobs[1].status, "completed");
        std::fs::remove_dir_all(path).unwrap();
    }
    #[tokio::test]
    async fn cancelled_request_and_persistence() {
        let path = std::env::temp_dir().join(format!("mw-state-test-{}", std::process::id()));
        tokio::fs::create_dir_all(&path).await.unwrap();
        let engine = test_engine(&path);
        engine
            .controls
            .lock()
            .await
            .insert("test".into(), "cancel".into());
        assert_eq!(
            engine
                .request("https://www.mangaworld.mx", "", 1000, Some("test"))
                .await
                .err()
                .unwrap(),
            "Annullato"
        );
        engine.saved.lock().await.settings = Some(Settings {
            base_url: "https://www.mangaworld.mx".into(),
            output: path.display().to_string(),
            delay_ms: 3000,
            cbz: true,
            export_format: "cbz".into(),
            user_agent: None,
            sources: Vec::new(),
        });
        engine.persist().await.unwrap();
        let restored: Saved =
            serde_json::from_slice(&tokio::fs::read(&engine.file).await.unwrap()).unwrap();
        assert_eq!(restored.settings.unwrap().delay_ms, 3000);
        tokio::fs::remove_dir_all(path).await.unwrap();
    }
    #[tokio::test]
    #[ignore = "Contatta MangaWorld e scarica un capitolo in una cartella temporanea"]
    async fn live_download_cbz_and_resume() {
        let path = std::env::temp_dir().join(format!("mw-live-test-{}", std::process::id()));
        tokio::fs::create_dir_all(&path).await.unwrap();
        let engine = test_engine(&path);
        let base = "https://www.mangaworld.mx";
        let html = engine
            .request(
                &format!("{base}/archive?keyword=one%20piece"),
                base,
                1000,
                None,
            )
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let source = sources::live_test_sources()
            .into_iter()
            .find(|s| s.id == "mw")
            .unwrap();
        let manga = sources::generic_search(&html, base, &source)
            .unwrap()
            .items
            .into_iter()
            .find(|m| m.title == "One Piece")
            .unwrap();
        let html = engine
            .request(&manga.url, base, 1000, None)
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let detail = sources::generic_detail(&html, manga.clone(), &source).unwrap();
        let chapter = detail.volumes[0].chapters[0].clone();
        let job = Job {
            archived: false,
            verification: Verification::default(),
            id: "smoke".into(),
            manga,
            volume: Volume {
                title: "Volume test".into(),
                chapters: vec![chapter],
            },
            status: "queued".into(),
            chapter: String::new(),
            chapters_done: 0,
            pages_done: 0,
            pages_total: 0,
            bytes: 0,
            message: String::new(),
            output: String::new(),
            settings: Settings {
                base_url: base.into(),
                output: path.display().to_string(),
                delay_ms: 1500,
                cbz: true,
                export_format: "cbz".into(),
                user_agent: None,
                sources: vec![source],
            },
            phase: "queued".into(),
            export_done: 0,
            export_total: 0,
        };
        engine.saved.lock().await.jobs.push(job.clone());
        download(&engine, None, &job).await.unwrap();
        let first = engine.saved.lock().await.jobs[0].clone();
        assert_eq!(first.status, "completed");
        assert!(first.bytes > 0);
        assert_eq!(first.pages_done, 23);
        let archive = zip::ZipArchive::new(std::fs::File::open(&first.output).unwrap()).unwrap();
        assert_eq!(archive.len(), 23);
        drop(archive);
        download(&engine, None, &job).await.unwrap();
        let resumed = engine.saved.lock().await.jobs[0].clone();
        assert_eq!(resumed.status, "completed");
        assert_eq!(resumed.bytes, 0);
        println!("Ricerca, dettaglio, 23 scan, CBZ e ripresa senza riscaricare: OK");
        tokio::fs::remove_dir_all(path).await.unwrap();
    }
    #[tokio::test]
    async fn removal_cannot_resurrect_download() {
        let path = std::env::temp_dir().join(format!("mw-remove-test-{}", std::process::id()));
        tokio::fs::create_dir_all(&path).await.unwrap();
        let engine = test_engine(&path);
        let job = Job {
            archived: false,
            verification: Verification::default(),
            id: "delete".into(),
            manga: Manga::default(),
            volume: Volume {
                title: "Test".into(),
                chapters: vec![],
            },
            status: "running".into(),
            chapter: String::new(),
            chapters_done: 0,
            pages_done: 0,
            pages_total: 0,
            bytes: 0,
            message: String::new(),
            output: String::new(),
            settings: Settings {
                base_url: "https://www.mangaworld.mx".into(),
                output: path.display().to_string(),
                delay_ms: 1000,
                cbz: true,
                export_format: "cbz".into(),
                user_agent: None,
                sources: Vec::new(),
            },
            phase: "queued".into(),
            export_done: 0,
            export_total: 0,
        };
        engine.saved.lock().await.jobs.push(job);
        engine.remove("delete").await.unwrap();
        assert_eq!(engine.checkpoint("delete").await.unwrap_err(), "Annullato");
        engine
            .update(None, "delete", |j| j.status = "completed".into())
            .await;
        assert!(engine.saved.lock().await.jobs.is_empty());
        let restored: Saved =
            serde_json::from_slice(&tokio::fs::read(&engine.file).await.unwrap()).unwrap();
        assert!(restored.jobs.is_empty());
        tokio::fs::remove_dir_all(path).await.unwrap();
    }
    #[tokio::test]
    #[ignore = "Verifica live ricerca, capitoli e una scan per fonti generiche"]
    async fn live_multiple_sources() {
        let engine = test_engine(&std::env::temp_dir());
        for s in sources::live_test_sources()
            .into_iter()
            .filter(|s| ["mw", "mf"].contains(&s.id.as_str()))
        {
            let r = sources::search(&engine, &s, "One Piece", 1, 1000)
                .await
                .unwrap();
            assert!(!r.items.is_empty(), "{}: nessun risultato", s.name);
            let manga = r.items[0].clone();
            let detail = sources::detail(&engine, &s, manga, 1000).await.unwrap();
            let chapter = &detail.volumes[0].chapters[0];
            let pages = sources::pages(
                &engine,
                &s,
                chapter,
                &detail.manga.url,
                1000,
                Some("live-source"),
            )
            .await
            .unwrap();
            assert!(!pages.is_empty());
        }
    }
    #[test]
    fn pdf_default_is_adopted_once() {
        let mut saved: Saved = serde_json::from_str(r#"{"settings":{"base_url":"https://www.mangaworld.mx","output":"/existing/folder","delay_ms":3000,"cbz":true,"export_format":"cbz"},"jobs":[]}"#).unwrap();
        adopt_download_preferences(&mut saved);
        let settings = saved.settings.as_mut().unwrap();
        assert_eq!(settings.export_format, "pdf");
        assert_eq!(settings.output, "/existing/folder");
        assert_eq!(settings.delay_ms, 500);
        settings.delay_ms = 1500;
        settings.export_format = "images".into();
        let mut reloaded: Saved =
            serde_json::from_str(&serde_json::to_string(&saved).unwrap()).unwrap();
        adopt_download_preferences(&mut reloaded);
        assert_eq!(reloaded.settings.unwrap().export_format, "images");
    }
    #[test]
    fn source_migration_preserves_explicitly_imported_connectors() {
        let mut settings = Settings::new_default("/tmp/pokko".into());
        assert!(settings.sources.is_empty());
        for (id, origin) in [
            ("mw", Some("official")),
            ("mf", None),
            ("mangapill", Some("imported")),
            ("custom", Some("created")),
        ] {
            settings.sources.push(sources::Source {
                id: id.into(),
                origin: origin.map(str::to_owned),
                ..Default::default()
            });
        }
        let mut saved = Saved {
            settings: Some(settings),
            ..Default::default()
        };
        remove_bundled_sources(&mut saved);
        let ids = saved
            .settings
            .unwrap()
            .sources
            .into_iter()
            .map(|source| source.id)
            .collect::<Vec<_>>();
        assert_eq!(ids, ["mangapill", "custom"]);
    }

    #[test]
    fn old_preferences_remain_readable() {
        let settings: Settings = serde_json::from_str(
            r#"{"base_url":"https://www.mangaworld.mx","output":"/existing/folder","delay_ms":3000,"cbz":true}"#,
        )
        .unwrap();
        assert_eq!(settings.output, "/existing/folder");
        assert!(settings.sources.is_empty());
        let manga: Manga = serde_json::from_str(
            r#"{"title":"Old","url":"https://www.mangaworld.mx/manga/1/old","cover":""}"#,
        )
        .unwrap();
        assert_eq!(manga.language, "it");
        assert!(manga_source(&settings, &manga).is_err());
    }
    #[tokio::test]
    async fn retry_after_is_enforced() {
        use std::io::{Read, Write};
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.local_addr().unwrap());
        let thread = std::thread::spawn(move || {
            for response in ["HTTP/1.1 429 Too Many Requests\r\nRetry-After: 1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n","HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"] {let (mut stream,_)=server.accept().unwrap();let mut buf=[0u8;2048];let _=stream.read(&mut buf);stream.write_all(response.as_bytes()).unwrap();}
        });
        let engine = test_engine(&std::env::temp_dir());
        assert!(engine
            .request(&url, "", 500, None)
            .await
            .err()
            .unwrap()
            .starts_with("Blocco HTTP"));
        let start = tokio::time::Instant::now();
        engine.request(&url, "", 500, None).await.unwrap();
        assert!(start.elapsed() >= Duration::from_secs(1));
        thread.join().unwrap();
    }
    #[tokio::test]
    async fn parallel_scan_downloads_and_integrity_proofs() {
        use std::io::{Read, Write};
        let base = std::env::temp_dir().join(format!("mwr-parallel-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let engine = test_engine(&base);
        let mut job = sample_job(&base, "images");
        job.settings.delay_ms = 10;
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_pixel(10, 10, image::Rgb([1, 2, 3]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let png_bytes = png.into_inner();
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = server.local_addr().unwrap();
        let urls = vec![
            format!("http://{addr}/p1.png"),
            format!("http://{addr}/p2.png"),
            format!("http://{addr}/p3.png"),
        ];
        let bytes_clone = png_bytes.clone();
        let thread = std::thread::spawn(move || {
            for _ in 0..3 {
                let (mut socket, _) = server.accept().unwrap();
                let mut request = [0; 4096];
                let _ = socket.read(&mut request);
                write!(
                    socket,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    bytes_clone.len()
                )
                .unwrap();
                socket.write_all(&bytes_clone).unwrap();
            }
        });
        engine
            .preview_pages
            .lock()
            .await
            .insert(job.volume.chapters[0].url.clone(), urls);
        engine.saved.lock().await.jobs.push(job.clone());
        download(&engine, None, &job).await.unwrap();
        thread.join().unwrap();
        let saved = engine.saved.lock().await.jobs[0].clone();
        assert_eq!(saved.verification.verified, 3);
        assert_eq!(saved.verification.repaired, 0);
        let chapter_dir = PathBuf::from(&saved.output).join("0001-Chapter");
        assert!(chapter_dir.join("0001.png").is_file());
        assert!(chapter_dir.join("0002.png").is_file());
        assert!(chapter_dir.join("0003.png").is_file());
        assert!(chapter_dir.join(".proofs.json").is_file());
        std::fs::remove_dir_all(base).unwrap();
    }
}
