mod config;
mod fetch;
mod gitea;
mod gitlab;
mod jira;
mod llm;
mod report;
mod utils;

use config::AppConfig;
use report::WeeklyWorkItem;
use simplelog::*;
use std::fs::OpenOptions;

#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String> {
    config::save_config(&config)
}

#[tauri::command]
fn load_config() -> Result<AppConfig, String> {
    config::load_config()
}

#[tauri::command]
async fn fetch_daily_items(date: String) -> Result<fetch::FetchedItems, String> {
    let config = config::load_config()?;

    let has_jira = !config.jira.base_url.is_empty();
    let has_gitlab = !config.gitlab.base_url.is_empty();
    let has_gitea = !config.gitea.base_url.is_empty();

    if !has_jira && !has_gitlab && !has_gitea {
        return Err("请至少配置一个数据源（Jira / GitLab / Gitea）".to_string());
    }

    tauri::async_runtime::spawn_blocking(move || {
        fetch::fetch_daily_items(&config, &date)
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

#[tauri::command]
async fn polish_daily_items(date: String, items_json: String) -> Result<Vec<String>, String> {
    let config = config::load_config()?;

    #[derive(serde::Deserialize)]
    struct RawItem {
        content: String,
        source: String,
    }

    let raw: Vec<RawItem> = serde_json::from_str(&items_json)
        .map_err(|e| format!("解析数据失败: {}", e))?;

    let items: Vec<fetch::WorkItemWithSource> = raw
        .into_iter()
        .map(|i| fetch::WorkItemWithSource { content: i.content, source: i.source })
        .collect();

    tauri::async_runtime::spawn_blocking(move || {
        fetch::polish_daily_items(&config, &date, &items)
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

#[tauri::command]
fn export_week_report(
    start_date: String,
    end_date: String,
    items_json: String,
    summary: String,
    key_tasks: String,
    completion_status: String,
    _employee: String,
) -> Result<String, String> {
    #[derive(serde::Deserialize)]
    struct DayItems {
        date: String,
        contents: Vec<String>,
    }

    let day_items: Vec<DayItems> = serde_json::from_str(&items_json)
        .map_err(|e| format!("解析工作内容失败: {}", e))?;

    let weekly_items: Vec<WeeklyWorkItem> = day_items
        .into_iter()
        .map(|d| WeeklyWorkItem { date: d.date, contents: d.contents })
        .collect();

    report::generate_week_xlsx(&start_date, &end_date, &weekly_items, &summary, &key_tasks, &completion_status)?;

    let file_name = format!("周报_{}_{}.xlsx", start_date, end_date);
    let src_path = crate::config::CONFIG_DIR
        .lock()
        .unwrap()
        .join(&file_name);

    // 弹出系统保存对话框，让用户选择保存位置
    let save_path = rfd::FileDialog::new()
        .set_file_name(&file_name)
        .add_filter("Excel 文件", &["xlsx"])
        .save_file();

    match save_path {
        Some(dest) => {
            std::fs::copy(&src_path, &dest)
                .map_err(|e| format!("保存文件失败: {}", e))?;

            // 保存成功后打开文件所在文件夹并选中文件
            #[cfg(target_os = "macos")]
            std::process::Command::new("open")
                .arg("-R")
                .arg(&dest)
                .spawn()
                .ok();

            #[cfg(target_os = "windows")]
            std::process::Command::new("explorer")
                .arg("/select,")
                .arg(&dest)
                .spawn()
                .ok();

            #[cfg(target_os = "linux")]
            {
                // Linux 上打开文件所在目录
                if let Some(parent) = dest.parent() {
                    std::process::Command::new("xdg-open")
                        .arg(parent)
                        .spawn()
                        .ok();
                }
            }

            Ok(dest.to_string_lossy().to_string())
        }
        None => Err("已取消".to_string()),
    }
}

fn check_model_config(config: &AppConfig) -> Result<(), String> {
    if config.model.base_url.trim().is_empty()
        || config.model.api_key.trim().is_empty()
        || config.model.model.trim().is_empty()
    {
        return Err("请先配置模型信息（base_url / api_key / model）".to_string());
    }
    Ok(())
}

#[tauri::command]
async fn summarize_week(items_json: String) -> Result<String, String> {
    let config = config::load_config()?;
    check_model_config(&config)?;

    let items: Vec<String> = serde_json::from_str(&items_json)
        .map_err(|e| format!("解析数据失败: {}", e))?;

    if items.is_empty() {
        return Err("本周暂无工作内容".to_string());
    }

    log::info!("周总结: 共 {} 条工作内容，调用模型 {}", items.len(), config.model.model);

    tauri::async_runtime::spawn_blocking(move || {
        let result = llm::summarize_week_with_openai(
            &config.model.base_url,
            &config.model.api_key,
            &config.model.model,
            &items,
            &config.prompts.summary_system,
        );
        match &result {
            Ok(s) => log::info!("周总结完成，字数: {}", s.chars().count()),
            Err(e) => log::error!("周总结失败: {}", e),
        }
        result
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

#[tauri::command]
async fn generate_week_tasks(items_json: String) -> Result<(String, String), String> {
    let config = config::load_config()?;
    check_model_config(&config)?;

    let items: Vec<String> = serde_json::from_str(&items_json)
        .map_err(|e| format!("解析数据失败: {}", e))?;

    if items.is_empty() {
        return Err("本周暂无工作内容".to_string());
    }

    log::info!("生成重点任务: 共 {} 条工作内容，调用模型 {}", items.len(), config.model.model);

    tauri::async_runtime::spawn_blocking(move || {
        let result = llm::generate_week_tasks_with_openai(
            &config.model.base_url,
            &config.model.api_key,
            &config.model.model,
            &items,
        );
        match &result {
            Ok((kt, cs)) => log::info!("重点任务生成完成: key_tasks={} chars, completion={} chars", kt.len(), cs.len()),
            Err(e) => log::error!("重点任务生成失败: {}", e),
        }
        result
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

#[tauri::command]
fn get_log_path() -> Result<String, String> {
    let log_dir = config::CONFIG_DIR.lock().unwrap().clone();
    let log_path = log_dir.join("daily-paper-generator.log");
    Ok(log_path.to_string_lossy().to_string())
}

#[tauri::command]
fn read_log_file() -> Result<String, String> {
    let log_dir = config::CONFIG_DIR.lock().unwrap().clone();
    let log_path = log_dir.join("daily-paper-generator.log");

    if !log_path.exists() {
        return Ok("日志文件不存在".to_string());
    }

    std::fs::read_to_string(&log_path)
        .map_err(|e| format!("读取日志文件失败: {}", e))
}

fn init_logger() {
    let log_dir = config::CONFIG_DIR.lock().unwrap().clone();
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("daily-paper-generator.log");

    if let Ok(file) = OpenOptions::new().append(true).create(true).open(&log_path) {
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
        .setup(|_app| Ok(()))
        .invoke_handler(tauri::generate_handler![
            save_config,
            load_config,
            fetch_daily_items,
            polish_daily_items,
            summarize_week,
            generate_week_tasks,
            export_week_report,
            get_log_path,
            read_log_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
