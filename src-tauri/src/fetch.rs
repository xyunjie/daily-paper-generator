use crate::config::AppConfig;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FetchedItems {
    pub tasks: Vec<String>,
    pub commits: Vec<String>,
}

fn simple_split(items: Vec<String>) -> Vec<String> {
    items
        .into_iter()
        .flat_map(|s| {
            s.split(|c| c == '\n' || c == ';' || c == '；' || c == '，' || c == ',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}

pub fn fetch_daily_items(config: &AppConfig, date: &str) -> Result<FetchedItems, String> {
    let tasks = crate::jira::fetch_tasks(config, date)?
        .into_iter()
        .map(|t| format!("{} - {} [{}]", t.key, t.summary, t.status))
        .collect::<Vec<_>>();

    let commits = crate::gitlab::fetch_commits(config, date)?
        .into_iter()
        .map(|c| format!("[{}] {} - {}", c.project_name, c.short_id, c.title))
        .collect::<Vec<_>>();

    let mut merged = Vec::new();
    merged.extend(tasks);
    merged.extend(commits);

    // 没有模型配置时，直接做简单拆分
    if config.model.base_url.trim().is_empty() || config.model.api_key.trim().is_empty() || config.model.model.trim().is_empty() {
        let lines = simple_split(merged);
        return Ok(FetchedItems { tasks: lines, commits: vec![] });
    }

    let input = merged.join("\n");
    let lines = crate::llm::polish_with_openai(
        &config.model.base_url,
        &config.model.api_key,
        &config.model.model,
        &input,
    )?;

    Ok(FetchedItems { tasks: lines, commits: vec![] })
}
