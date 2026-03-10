use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub static CONFIG_DIR: Lazy<Mutex<PathBuf>> = Lazy::new(|| {
    let dir = dirs_next::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("daily-paper-generator");
    Mutex::new(dir)
});

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JiraConfig {
    pub base_url: String,
    pub email: String,
    pub api_token: String,
    #[serde(default)]
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitLabConfig {
    pub base_url: String,
    pub private_token: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub jira: JiraConfig,
    pub gitlab: GitLabConfig,
    pub user_email: String,
    pub model: ModelConfig,
}

pub fn get_config_path() -> PathBuf {
    CONFIG_DIR.lock().unwrap().join("config.json")
}

pub fn ensure_config_dir() -> Result<(), String> {
    let dir = CONFIG_DIR.lock().unwrap().clone();
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    Ok(())
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    ensure_config_dir()?;
    let path = get_config_path();
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write config file: {}", e))?;
    log::info!("Config saved to {:?}", path);
    Ok(())
}

pub fn load_config() -> Result<AppConfig, String> {
    let path = get_config_path();
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    let config: AppConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))?;
    log::info!("Config loaded from {:?}", path);
    Ok(config)
}
