use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    sync::Mutex,
};
use tauri::{AppHandle, Manager};

static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
const MAX_LOG_SIZE: u64 = 5 * 1024 * 1024; // 5 MB

pub fn init(app: &AppHandle) {
    if let Ok(dir) = app
        .path()
        .app_data_dir()
        .or_else(|_| app.path().app_config_dir())
    {
        let logs_dir = dir.join("logs");
        let _ = fs::create_dir_all(&logs_dir);
        let log_file = logs_dir.join("pokko.log");
        if let Ok(mut guard) = LOG_PATH.lock() {
            *guard = Some(log_file);
        }
    }
}

pub fn log(level: &str, target: &str, message: &str, data: Option<serde_json::Value>) {
    let path = match LOG_PATH.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => None,
    };
    let path = match path {
        Some(p) => p,
        None => return,
    };

    // Check size and rotate if necessary
    if let Ok(meta) = fs::metadata(&path) {
        if meta.len() > MAX_LOG_SIZE {
            let backup = path.with_extension("log.old");
            let _ = fs::rename(&path, backup);
        }
    }

    let timestamp = httpdate::fmt_http_date(std::time::SystemTime::now());
    let entry = serde_json::json!({
        "timestamp": timestamp,
        "level": level,
        "target": target,
        "message": message,
        "data": data
    });

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(file, "{}", entry);
    }
}

pub fn info(target: &str, message: &str) {
    log("INFO", target, message, None);
}

pub fn info_data(target: &str, message: &str, data: serde_json::Value) {
    log("INFO", target, message, Some(data));
}

pub fn warn(target: &str, message: &str) {
    log("WARN", target, message, None);
}

pub fn error(target: &str, message: &str) {
    log("ERROR", target, message, None);
}

pub fn error_data(target: &str, message: &str, data: serde_json::Value) {
    log("ERROR", target, message, Some(data));
}

#[tauri::command]
pub fn get_log_path(app: AppHandle) -> Result<String, String> {
    if let Ok(guard) = LOG_PATH.lock() {
        if let Some(ref path) = *guard {
            return Ok(path.display().to_string());
        }
    }
    let dir = app
        .path()
        .app_data_dir()
        .or_else(|_| app.path().app_config_dir())
        .map_err(|e| e.to_string())?;
    Ok(dir.join("logs").join("pokko.log").display().to_string())
}

#[tauri::command]
pub fn read_recent_logs(lines: usize, app: AppHandle) -> Result<Vec<String>, String> {
    let path = PathBuf::from(get_log_path(app)?);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(&path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let all_lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
    let count = lines.clamp(1, 500);
    let start = all_lines.len().saturating_sub(count);
    Ok(all_lines[start..].to_vec())
}

#[tauri::command]
pub fn clear_logs(app: AppHandle) -> Result<(), String> {
    let path = PathBuf::from(get_log_path(app)?);
    if path.exists() {
        fs::write(&path, "").map_err(|e| e.to_string())?;
    }
    Ok(())
}
