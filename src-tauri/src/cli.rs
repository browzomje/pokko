use crate::{
    catalog,
    provider::{Manga, Volume},
    sources::{self, Source},
    Engine, Job, Settings, Verification,
};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "pokko",
    author = "browzomje",
    version = env!("CARGO_PKG_VERSION"),
    about = "Download manga and comics to read offline (PDF, CBZ, CBR, images)",
    long_about = "Pokko is a fast and lightweight manga and comics downloader.\nSupports declarative connectors, verified 2-phase downloads, and multiple export formats."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Search for manga and comics across sources
    Search {
        /// Title query to search for
        query: String,
        /// Filter by source identifier (e.g. mw, mangapill, mf, mangaread, mangatown)
        #[arg(short, long)]
        source: Option<String>,
        /// Filter by language (e.g. it, en)
        #[arg(short, long)]
        language: Option<String>,
        /// Maximum number of results to display
        #[arg(short = 'n', long, default_value_t = 15)]
        limit: usize,
    },
    /// Show metadata and chapter list for a manga series URL
    Info {
        /// Manga series URL (or query)
        target: String,
        /// Explicit source ID (optional if URL matches a known source)
        #[arg(short, long)]
        source: Option<String>,
        /// Path to custom connector JSON file
        #[arg(short = 'C', long)]
        connector: Option<PathBuf>,
    },
    /// Download chapters or entire volumes
    Download {
        /// Manga series or chapter URL
        url: String,
        /// Export format: pdf (default), cbz, cbr, images
        #[arg(short, long, default_value = "pdf")]
        format: String,
        /// Output destination directory (default: ~/Downloads/Pokko)
        #[arg(short, long)]
        output: Option<String>,
        /// Chapters to download: "all", single number "5", list "1,2,3", or range "1-5"
        #[arg(short, long)]
        chapters: Option<String>,
        /// Delay in milliseconds between page requests
        #[arg(short, long)]
        delay: Option<u64>,
        /// Explicit source ID
        #[arg(short, long)]
        source: Option<String>,
        /// Path to custom connector JSON file
        #[arg(short = 'C', long)]
        connector: Option<PathBuf>,
    },
    /// Manage and inspect catalog sources
    Sources {
        #[command(subcommand)]
        action: Option<SourcesAction>,
    },
    /// Generate shell auto-completions
    Completions {
        /// Target shell
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand, Debug)]
pub enum SourcesAction {
    /// List all built-in and configured sources
    List,
    /// Test connection to a source or connector JSON file
    Test {
        /// Source ID (e.g. mw) or path to a connector .json file
        target: String,
    },
}

/// Detect whether the process was invoked from the command line with CLI commands/flags.
pub fn is_cli_invocation(args: &[String]) -> bool {
    if args.len() <= 1 {
        return false;
    }
    matches!(
        args[1].as_str(),
        "search"
            | "info"
            | "download"
            | "sources"
            | "completions"
            | "help"
            | "--help"
            | "-h"
            | "--version"
            | "-V"
    )
}

/// Synchronous entry point that sets up a multithreaded Tokio runtime.
pub fn run_cli_sync() {
    std::panic::set_hook(Box::new(|info| {
        let msg = info.to_string();
        if msg.contains("Broken pipe") || msg.contains("os error 32") {
            std::process::exit(0);
        }
        eprintln!("{info}");
    }));

    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            eprintln!("Failed to initialize Tokio runtime: {}", err);
            std::process::exit(1);
        }
    };
    if let Err(err) = rt.block_on(run_cli()) {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}

pub async fn run_cli() -> Result<(), String> {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            let _ = err.print();
            if err.use_stderr() {
                std::process::exit(1);
            } else {
                std::process::exit(0);
            }
        }
    };
    match cli.command {
        Commands::Search {
            query,
            source,
            language,
            limit,
        } => handle_search(&query, source.as_deref(), language.as_deref(), limit).await,
        Commands::Info {
            target,
            source,
            connector,
        } => handle_info(&target, source.as_deref(), connector.as_deref()).await,
        Commands::Download {
            url,
            format,
            output,
            chapters,
            delay,
            source,
            connector,
        } => {
            handle_download(
                &url,
                &format,
                output.as_deref(),
                chapters.as_deref(),
                delay,
                source.as_deref(),
                connector.as_deref(),
            )
            .await
        }
        Commands::Sources { action } => match action.unwrap_or(SourcesAction::List) {
            SourcesAction::List => handle_sources_list().await,
            SourcesAction::Test { target } => handle_sources_test(&target).await,
        },
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "pokko", &mut std::io::stdout());
            Ok(())
        }
    }
}

async fn handle_search(
    query: &str,
    source_filter: Option<&str>,
    lang_filter: Option<&str>,
    limit: usize,
) -> Result<(), String> {
    let engine = Engine::headless(None);
    let mut sources = {
        let saved = engine.saved.lock().await;
        saved
            .settings
            .as_ref()
            .map(|s| s.sources.clone())
            .unwrap_or_else(sources::defaults)
    };

    if let Some(src_id) = source_filter {
        sources.retain(|s| s.id.eq_ignore_ascii_case(src_id));
        if sources.is_empty() {
            return Err(format!(
                "Source '{}' not found. Run 'pokko sources list' to view available sources.",
                src_id
            ));
        }
    }
    if let Some(lang) = lang_filter {
        sources.retain(|s| s.language.eq_ignore_ascii_case(lang));
        if sources.is_empty() {
            return Err(format!(
                "No sources found for language '{}'. Run 'pokko sources list' to view available sources.",
                lang
            ));
        }
    }

    println!(
        "Searching for \"{}\" across {} sources…\n",
        query,
        sources.len()
    );

    let mut all_results = Vec::new();
    for source in sources {
        if !source.enabled && source_filter.is_none() {
            continue;
        }
        match catalog::search_source(&engine, &source, query, 1, source.min_delay_ms, |_| {}).await
        {
            Ok(res) => {
                for item in res.items {
                    all_results.push((source.clone(), item));
                }
            }
            Err(e) => {
                eprintln!("Warning: [{}] {}", source.name, e);
            }
        }
    }

    if all_results.is_empty() {
        println!("No results found for \"{}\".", query);
        return Ok(());
    }

    println!("Found {} results:\n", all_results.len().min(limit));
    for (i, (source, item)) in all_results.iter().take(limit).enumerate() {
        println!("{}. {} [{}]", i + 1, item.title, source.name);
        println!("   Language: {}", source.language);
        if !item.authors.is_empty() {
            println!("   Authors:  {}", item.authors.join(", "));
        }
        println!("   URL:      {}", item.url);
        println!();
    }
    Ok(())
}

/// Load a custom declarative connector from a local JSON file.
async fn load_connector(path: &Path) -> Result<Source, String> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| format!("Cannot read connector file '{}': {}", path.display(), e))?;
    serde_json::from_str::<Source>(&content)
        .map_err(|e| format!("Invalid connector schema in '{}': {}", path.display(), e))
}

fn resolve_source(
    sources: &[Source],
    url: &str,
    source_override: Option<&str>,
) -> Result<Source, String> {
    if let Some(id) = source_override {
        return sources
            .iter()
            .find(|s| s.id.eq_ignore_ascii_case(id))
            .cloned()
            .ok_or_else(|| format!("Source with ID '{}' not found", id));
    }
    let parsed_url = url::Url::parse(url).map_err(|e| format!("Invalid URL '{}': {}", url, e))?;
    let host = parsed_url.host_str().unwrap_or("").to_lowercase();
    for s in sources {
        if let Ok(base) = url::Url::parse(&s.base_url) {
            if let Some(base_host) = base.host_str() {
                if host == base_host || host.ends_with(base_host) || base_host.ends_with(&host) {
                    return Ok(s.clone());
                }
            }
        }
    }
    // Fallback default
    sources
        .iter()
        .find(|s| s.id == "mw")
        .cloned()
        .ok_or_else(|| "Could not match URL to a catalog source. Specify --source <id>.".into())
}

async fn handle_info(
    target: &str,
    source_override: Option<&str>,
    connector_file: Option<&Path>,
) -> Result<(), String> {
    let engine = Engine::headless(None);
    let all_sources = {
        let saved = engine.saved.lock().await;
        saved
            .settings
            .as_ref()
            .map(|s| s.sources.clone())
            .unwrap_or_else(sources::defaults)
    };

    let source = if let Some(conn_path) = connector_file {
        load_connector(conn_path).await?
    } else {
        resolve_source(&all_sources, target, source_override)?
    };
    let manga = Manga {
        authors: Vec::new(),
        title: target.to_string(),
        url: target.to_string(),
        cover: String::new(),
        source_id: source.id.clone(),
        source_name: source.name.clone(),
        language: source.language.clone(),
    };

    println!("Fetching series metadata from {}…\n", source.name);
    let detail = sources::detail(&engine, &source, manga, source.min_delay_ms).await?;

    println!("Title:       {}", detail.manga.title);
    println!("Source:      {} ({})", source.name, source.language);
    println!("URL:         {}", detail.manga.url);
    if !detail.manga.authors.is_empty() {
        println!("Authors:     {}", detail.manga.authors.join(", "));
    }
    if !detail.description.is_empty() {
        println!("\nDescription:\n{}", detail.description.trim());
    }

    let total_chapters: usize = detail.volumes.iter().map(|v| v.chapters.len()).sum();
    println!(
        "\nVolumes ({} total, {} chapters):",
        detail.volumes.len(),
        total_chapters
    );

    let mut global_idx = 1;
    for vol in &detail.volumes {
        println!("\n• {} ({} chapters):", vol.title, vol.chapters.len());
        for ch in &vol.chapters {
            println!("  [{:>3}] {} -> {}", global_idx, ch.title, ch.url);
            global_idx += 1;
        }
    }
    Ok(())
}

/// Filter chapters by expression ("all", "1,2,5", "1-5", single index, or title substring).
fn filter_chapters(volumes: &[Volume], spec: Option<&str>) -> Result<Vec<Volume>, String> {
    let spec = spec.unwrap_or("all").trim();
    if spec.eq_ignore_ascii_case("all") {
        return Ok(volumes.to_vec());
    }

    // Collect all chapters with global 1-based index
    let mut flat = Vec::new();
    for (vol_idx, vol) in volumes.iter().enumerate() {
        for ch in &vol.chapters {
            flat.push((vol_idx, ch.clone()));
        }
    }

    let mut selected_indices = std::collections::HashSet::new();

    for part in spec.split(',') {
        let part = part.trim();
        if let Some((start_s, end_s)) = part.split_once('-') {
            let start: usize = start_s
                .trim()
                .parse()
                .map_err(|_| format!("Invalid chapter range start: '{}'", start_s))?;
            let end: usize = end_s
                .trim()
                .parse()
                .map_err(|_| format!("Invalid chapter range end: '{}'", end_s))?;
            if start == 0 || end < start {
                return Err(format!("Invalid range: {}-{}", start, end));
            }
            for i in start..=end {
                selected_indices.insert(i);
            }
        } else if let Ok(single) = part.parse::<usize>() {
            if single == 0 {
                return Err("Chapter index must be 1 or greater".into());
            }
            selected_indices.insert(single);
        } else {
            // Match chapter title substring
            let needle = part.to_lowercase();
            for (idx, (_, ch)) in flat.iter().enumerate() {
                if ch.title.to_lowercase().contains(&needle) {
                    selected_indices.insert(idx + 1);
                }
            }
        }
    }

    if selected_indices.is_empty() {
        return Err(format!(
            "No chapters matched criteria '{}'. Total chapters available: {}",
            spec,
            flat.len()
        ));
    }

    let mut result_volumes = Vec::new();
    for (vol_idx, vol) in volumes.iter().enumerate() {
        let mut matching_chapters = Vec::new();
        for (idx, (v_idx, ch)) in flat.iter().enumerate() {
            if *v_idx == vol_idx && selected_indices.contains(&(idx + 1)) {
                matching_chapters.push(ch.clone());
            }
        }
        if !matching_chapters.is_empty() {
            result_volumes.push(Volume {
                title: vol.title.clone(),
                chapters: matching_chapters,
            });
        }
    }

    Ok(result_volumes)
}

async fn handle_download(
    url: &str,
    format: &str,
    output: Option<&str>,
    chapters_spec: Option<&str>,
    delay_override: Option<u64>,
    source_override: Option<&str>,
    connector_file: Option<&Path>,
) -> Result<(), String> {
    let valid_formats = ["pdf", "cbz", "cbr", "images"];
    let format = format.to_lowercase();
    if !valid_formats.contains(&format.as_str()) {
        return Err(format!(
            "Invalid format '{}'. Supported formats: {}",
            format,
            valid_formats.join(", ")
        ));
    }

    let out_dir = output
        .map(PathBuf::from)
        .unwrap_or_else(crate::default_download_dir);

    let engine = Engine::headless(Some(out_dir.clone()));
    let mut all_sources = {
        let saved = engine.saved.lock().await;
        saved
            .settings
            .as_ref()
            .map(|s| s.sources.clone())
            .unwrap_or_else(sources::defaults)
    };

    let source = if let Some(conn_path) = connector_file {
        let custom = load_connector(conn_path).await?;
        all_sources.retain(|s| s.id != custom.id);
        all_sources.insert(0, custom.clone());
        custom
    } else {
        resolve_source(&all_sources, url, source_override)?
    };
    let delay = delay_override.unwrap_or(source.min_delay_ms);

    println!("Resolving series metadata from {}…", source.name);
    let manga = Manga {
        authors: Vec::new(),
        title: url.to_string(),
        url: url.to_string(),
        cover: String::new(),
        source_id: source.id.clone(),
        source_name: source.name.clone(),
        language: source.language.clone(),
    };

    let detail = sources::detail(&engine, &source, manga, delay).await?;
    let filtered_volumes = filter_chapters(&detail.volumes, chapters_spec)?;
    let total_chapters: usize = filtered_volumes.iter().map(|v| v.chapters.len()).sum();

    println!("\nSeries:      {}", detail.manga.title);
    println!("Source:      {} ({})", source.name, source.language);
    println!("Format:      {}", format.to_uppercase());
    println!("Destination: {}", out_dir.display());
    println!("Selected:    {} chapters\n", total_chapters);

    let mut current_idx = 0;
    for vol in filtered_volumes {
        for chapter in vol.chapters {
            current_idx += 1;
            println!(
                "[{}/{}] Downloading: {} ({})",
                current_idx, total_chapters, chapter.title, vol.title
            );

            let single_vol = Volume {
                title: vol.title.clone(),
                chapters: vec![chapter.clone()],
            };

            let mut settings = engine
                .saved
                .lock()
                .await
                .settings
                .clone()
                .unwrap_or_else(|| Settings {
                    base_url: source.base_url.clone(),
                    output: out_dir.display().to_string(),
                    delay_ms: delay,
                    cbz: format == "cbz",
                    export_format: format.clone(),
                    user_agent: None,
                    sources: all_sources.clone(),
                });
            settings.output = out_dir.display().to_string();
            settings.export_format = format.clone();
            settings.cbz = format == "cbz";
            settings.delay_ms = delay;

            let job_id = format!("cli-{}", rand::random::<u32>());
            let job = Job {
                archived: false,
                verification: Verification::default(),
                id: job_id.clone(),
                manga: detail.manga.clone(),
                volume: single_vol,
                status: "queued".into(),
                chapter: chapter.title.clone(),
                chapters_done: 0,
                pages_done: 0,
                pages_total: 0,
                bytes: 0,
                message: String::new(),
                output: String::new(),
                settings,
                phase: String::new(),
                export_done: 0,
                export_total: 0,
            };

            {
                let mut saved = engine.saved.lock().await;
                saved.jobs.push(job.clone());
            }

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Job>();
            *engine.progress_sink.lock().await = Some(tx);
            let fmt_name = format.to_uppercase();
            let reporter = tokio::spawn(async move {
                while let Some(j) = rx.recv().await {
                    if j.phase == "downloading" && j.pages_total > 0 {
                        let pct = (j.pages_done * 100) / j.pages_total;
                        print!(
                            "\r    ↳ Scans: {}/{} ({:>2}%)   ",
                            j.pages_done, j.pages_total, pct
                        );
                        let _ = std::io::stdout().flush();
                    } else if j.phase == "exporting" && j.export_total > 0 {
                        let pct = (j.export_done * 100) / j.export_total;
                        print!(
                            "\r    ↳ Packing {}: {}/{} ({:>2}%)   ",
                            fmt_name, j.export_done, j.export_total, pct
                        );
                        let _ = std::io::stdout().flush();
                    }
                }
            });

            let download_res = crate::download(&engine, None, &job).await;
            reporter.abort();
            print!("\r                                                           \r");
            let _ = std::io::stdout().flush();

            match download_res {
                Ok(_) => {
                    let saved = engine.saved.lock().await;
                    let finished_job = saved.jobs.iter().find(|j| j.id == job_id);
                    let dest = finished_job
                        .map(|j| j.output.as_str())
                        .filter(|o| !o.is_empty())
                        .unwrap_or("[completed]");
                    println!("    ✓ Saved: {}\n", dest);
                }
                Err(e) => {
                    eprintln!("    ✗ Error: {}\n", e);
                }
            }
        }
    }

    println!("Download task complete.");
    Ok(())
}

async fn handle_sources_list() -> Result<(), String> {
    let engine = Engine::headless(None);
    let sources = {
        let saved = engine.saved.lock().await;
        saved
            .settings
            .as_ref()
            .map(|s| s.sources.clone())
            .unwrap_or_else(sources::defaults)
    };

    println!(
        "{:<12} {:<15} {:<6} {:<8} {:<8} BASE URL",
        "ID", "NAME", "LANG", "TYPE", "DELAY"
    );
    println!("{}", "-".repeat(80));
    for s in sources {
        println!(
            "{:<12} {:<15} {:<6} {:<8} {:<8} {}",
            s.id,
            s.name,
            s.language,
            s.category,
            format!("{}ms", s.min_delay_ms),
            s.base_url
        );
    }
    println!("\nUse 'pokko sources test <id>' to test a source connection.");
    Ok(())
}

async fn handle_sources_test(target: &str) -> Result<(), String> {
    let engine = Engine::headless(None);
    let source = if target.ends_with(".json") {
        load_connector(Path::new(target)).await?
    } else {
        let sources = {
            let saved = engine.saved.lock().await;
            saved
                .settings
                .as_ref()
                .map(|s| s.sources.clone())
                .unwrap_or_else(sources::defaults)
        };
        sources
            .into_iter()
            .find(|s| s.id.eq_ignore_ascii_case(target))
            .ok_or_else(|| {
                format!(
                    "Source with ID '{}' not found. Run 'pokko sources list' for options.",
                    target
                )
            })?
    };

    println!(
        "Testing connection to '{}' ({}) at {}…",
        source.name, source.id, source.base_url
    );
    let start = std::time::Instant::now();
    let resp = engine
        .request(&source.base_url, &source.base_url, 0, None)
        .await?;
    let elapsed = start.elapsed();
    let status = resp.status();

    if status.is_success() {
        println!(
            "✓ Connection successful: HTTP {} (responded in {:.2?})",
            status, elapsed
        );
    } else {
        println!("⚠ Source responded with HTTP {} in {:.2?}", status, elapsed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Chapter;

    #[test]
    fn test_is_cli_invocation() {
        assert!(!is_cli_invocation(&["pokko".into()]));
        assert!(is_cli_invocation(&["pokko".into(), "search".into()]));
        assert!(is_cli_invocation(&["pokko".into(), "info".into()]));
        assert!(is_cli_invocation(&["pokko".into(), "download".into()]));
        assert!(is_cli_invocation(&["pokko".into(), "sources".into()]));
        assert!(is_cli_invocation(&["pokko".into(), "completions".into()]));
        assert!(is_cli_invocation(&["pokko".into(), "--help".into()]));
        assert!(is_cli_invocation(&["pokko".into(), "-h".into()]));
        assert!(is_cli_invocation(&["pokko".into(), "--version".into()]));
        assert!(is_cli_invocation(&["pokko".into(), "-V".into()]));
        assert!(!is_cli_invocation(&[
            "pokko".into(),
            "--some-tauri-flag".into()
        ]));
    }

    #[test]
    fn test_completions_generation() {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        clap_complete::generate(clap_complete::Shell::Bash, &mut cmd, "pokko", &mut buf);
        let script = String::from_utf8(buf).expect("valid utf-8 shell script");
        assert!(script.contains("pokko"));
        assert!(script.contains("download"));
        assert!(script.contains("completions"));
    }

    #[test]
    fn test_filter_chapters_all() {
        let volumes = vec![Volume {
            title: "Vol 1".into(),
            chapters: vec![
                Chapter {
                    page_count: None,
                    title: "Ch 1".into(),
                    url: "https://example.com/1".into(),
                },
                Chapter {
                    page_count: None,
                    title: "Ch 2".into(),
                    url: "https://example.com/2".into(),
                },
            ],
        }];
        let res = filter_chapters(&volumes, Some("all")).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].chapters.len(), 2);
    }

    #[test]
    fn test_filter_chapters_range_and_list() {
        let volumes = vec![
            Volume {
                title: "Vol 1".into(),
                chapters: vec![
                    Chapter {
                        page_count: None,
                        title: "Ch 1".into(),
                        url: "https://example.com/1".into(),
                    },
                    Chapter {
                        page_count: None,
                        title: "Ch 2".into(),
                        url: "https://example.com/2".into(),
                    },
                ],
            },
            Volume {
                title: "Vol 2".into(),
                chapters: vec![
                    Chapter {
                        page_count: None,
                        title: "Ch 3".into(),
                        url: "https://example.com/3".into(),
                    },
                    Chapter {
                        page_count: None,
                        title: "Ch 4".into(),
                        url: "https://example.com/4".into(),
                    },
                ],
            },
        ];
        // Select chapter 1 and 3
        let res = filter_chapters(&volumes, Some("1,3")).unwrap();
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].chapters[0].title, "Ch 1");
        assert_eq!(res[1].chapters[0].title, "Ch 3");

        // Select range 2-4
        let range_res = filter_chapters(&volumes, Some("2-4")).unwrap();
        let total: usize = range_res.iter().map(|v| v.chapters.len()).sum();
        assert_eq!(total, 3);
    }

    #[test]
    fn test_resolve_source_by_url() {
        let sources = sources::defaults();
        let resolved =
            resolve_source(&sources, "https://www.mangaworld.mx/manga/test", None).unwrap();
        assert_eq!(resolved.id, "mw");

        let resolved_pill =
            resolve_source(&sources, "https://mangapill.com/manga/123", None).unwrap();
        assert_eq!(resolved_pill.id, "mangapill");

        let override_source =
            resolve_source(&sources, "https://random.com/manga/123", Some("mf")).unwrap();
        assert_eq!(override_source.id, "mf");
    }
}
