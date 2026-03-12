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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsConfig {
    #[serde(default = "default_polish_system")]
    pub polish_system: String,
    #[serde(default = "default_polish_few_shot")]
    pub polish_few_shot: String,
    #[serde(default = "default_summary_system")]
    pub summary_system: String,
}

fn default_polish_system() -> String {
    "你是日报助手。请将输入信息整合为可直接填日报的中文要点。\n\
硬性规则：\n\
1) 只输出要点列表，每条一行，不要标题/解释/前后缀。\n\
2) 输出条数为 3-8 条（信息确实很少时可少于 3 条，但不要胡编）。\n\
3) 每条必须是纯中文要点，尽量以动词开头（如：优化/修复/测试/联调/完善/修改/添加）。\n\
4) 严禁在输出中出现 Jira Key（如 ABC-123）、GitLab 项目名/路径（如 group/repo）、提交 hash/short_id、URL。\n\
5) 优先合并同一主题/同一任务的多条提交，避免碎片化。".to_string()
}

fn default_polish_few_shot() -> String {
    "【示例输入】\n\
【日期】2026-03-06\n\
【Jira Done 任务】\n\
key=ABC-101 | summary=导出 CSV 表头字段调整\n\
key=ABC-102 | summary=累计流量统计修复\n\
【GitLab 提交摘要】\n\
- feat: export csv header mapping\n\
- fix: traffic total calc\n\
- refactor: analytics api\n\
\n\
【示例输出】\n\
修改导出 CSV 表头字段\n\
修复累计流量统计问题\n\
优化分析与数据接口".to_string()
}

fn default_summary_system() -> String {
    "你是工作总结助手。请将本周的工作内容整合为一段精炼的周总结。\n\
硬性规则：\n\
1) 输出一段连贯的中文总结，不超过200字。\n\
2) 总结应该概括本周的主要工作方向和成果，而不是简单罗列。\n\
3) 使用专业的工作汇报语言，突出重点和价值。\n\
4) 严禁在输出中出现 Jira Key、GitLab 项目名/路径、提交 hash、URL 等技术细节。\n\
5) 如果工作内容较少，如实总结，不要编造。".to_string()
}

impl Default for PromptsConfig {
    fn default() -> Self {
        Self {
            polish_system: default_polish_system(),
            polish_few_shot: default_polish_few_shot(),
            summary_system: default_summary_system(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub jira: JiraConfig,
    pub gitlab: GitLabConfig,
    pub user_email: String,
    pub model: ModelConfig,
    #[serde(default)]
    pub prompts: PromptsConfig,
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
