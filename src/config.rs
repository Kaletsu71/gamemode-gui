use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write as IoWrite;
use std::path::PathBuf;

#[allow(dead_code)]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub steam_launch_applied: bool,
    #[serde(default)]
    pub last_check: HashMap<String, String>,
}

pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
}

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| home_dir().join(".config"))
        .join("gamemode-manager")
}

fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| home_dir().join(".local/share"))
        .join("gamemode-manager")
}

fn ensure_dirs() {
    let _ = std::fs::create_dir_all(config_dir());
    let _ = std::fs::create_dir_all(data_dir());
}

pub fn log_entry(msg: &str) {
    ensure_dirs();
    let ts = Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{ts}] {msg}\n");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(data_dir().join("app.log"))
    {
        let _ = f.write_all(line.as_bytes());
    }
}

#[allow(dead_code)]
pub fn load_config() -> AppConfig {
    ensure_dirs();
    let path = config_dir().join("config.json");
    if let Ok(text) = std::fs::read_to_string(&path) {
        serde_json::from_str(&text).unwrap_or_default()
    } else {
        AppConfig::default()
    }
}

#[allow(dead_code)]
pub fn save_config(cfg: &AppConfig) {
    ensure_dirs();
    if let Ok(text) = serde_json::to_string_pretty(cfg) {
        let _ = std::fs::write(config_dir().join("config.json"), text);
    }
}
