mod config;
mod fetch;
mod gitlab;
mod jira;
mod llm;
mod report;

use config::AppConfig;
use report::DailyReport;
use simplelog::*;
use std::fs::File;

#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String> {
    config::save_config(&config)
}

#[tauri::command]
fn load_config() -> Result<AppConfig, String> {
    config::load_config()
}

#[tauri::command]
fn fetch_daily_items(date: String) -> Result<fetch::FetchedItems, String> {
    let config = config::load_config()?;

    if config.jira.base_url.is_empty() {
        return Err("请先配置 Jira 信息".to_string());
    }
    if config.gitlab.base_url.is_empty() {
        return Err("请先配置 GitLab 信息".to_string());
    }
    if config.user_email.is_empty() {
        return Err("请先配置用户邮箱".to_string());
    }

    fetch::fetch_daily_items(&config, &date)
}

#[tauri::command]
fn generate_report(date: String) -> Result<String, String> {
    log::info!("Generating report for date: {}", date);

    let config = config::load_config()?;

    if config.jira.base_url.is_empty() {
        return Err("请先配置 Jira 信息".to_string());
    }

    if config.gitlab.base_url.is_empty() {
        return Err("请先配置 GitLab 信息".to_string());
    }

    if config.user_email.is_empty() {
        return Err("请先配置用户邮箱".to_string());
    }

    // 获取 Jira 任务
    log::info!("Fetching Jira tasks...");
    let tasks = jira::fetch_tasks(&config, &date)?;

    // 获取 GitLab 提交
    log::info!("Fetching GitLab commits...");
    let commits = gitlab::fetch_commits(&config, &date)?;

    // 生成日报
    let report = DailyReport {
        date: date.clone(),
        tasks,
        commits,
    };

    let file_path = report::generate_docx(&report)?;
    Ok(file_path)
}

fn init_logger() {
    let log_dir = config::CONFIG_DIR.lock().unwrap().clone();
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("daily-paper-generator.log");

    if let Ok(file) = File::create(&log_path) {
        let _ = WriteLogger::init(
            LevelFilter::Info,
            Config::default(),
            file,
        );
        log::info!("Logger initialized, log file: {:?}", log_path);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logger();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            save_config,
            load_config,
            fetch_daily_items,
            generate_report
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
