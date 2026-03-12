use crate::config::AppConfig;
use reqwest::blocking::Client;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct JiraIssue {
    pub key: String,
    pub fields: JiraFields,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JiraFields {
    pub summary: String,
    pub status: JiraStatus,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JiraStatus {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JiraSearchResponse {
    pub issues: Vec<JiraIssue>,
}

#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub key: String,
    pub summary: String,
    pub status: String,
    pub url: String,
}

pub fn fetch_tasks(config: &AppConfig, date: &str) -> Result<Vec<TaskInfo>, String> {
    let jira = &config.jira;
    let client = Client::new();

    let start_time = format!("{} 00:00", date);
    let end_time = format!("{} 23:59", date);
    let jql = format!(
        "status CHANGED TO \"Done\" DURING (\"{}\", \"{}\") AND assignee = \"{}\" ORDER BY updated DESC",
        start_time,
        end_time,
        jira.username
    );

    log::info!("Jira: 开始获取任务, date={}, user={}", date, jira.username);

    let url = format!(
        "{}/rest/api/2/search?jql={}&fields=summary,status",
        jira.base_url.trim_end_matches('/'),
        urlencoding::encode(&jql)
    );

    let response = client
        .get(&url)
        .bearer_auth(&jira.api_token)
        .header("Accept", "application/json")
        .send()
        .map_err(|e| {
            log::error!("Jira: 请求失败: {}", e);
            format!("Jira request failed: {}", e)
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        log::error!("Jira: API 返回错误 {} - {}", status, body);
        return Err(format!("Jira API error: {} - {}", status, body));
    }

    let data: JiraSearchResponse = response
        .json()
        .map_err(|e| {
            log::error!("Jira: 解析响应失败: {}", e);
            format!("Failed to parse Jira response: {}", e)
        })?;

    let tasks: Vec<TaskInfo> = data
        .issues
        .into_iter()
        .map(|issue| TaskInfo {
            key: issue.key.clone(),
            summary: issue.fields.summary,
            status: issue.fields.status.name,
            url: format!("{}/browse/{}", jira.base_url.trim_end_matches('/'), issue.key),
        })
        .collect();

    log::info!("Jira: 获取到 {} 条任务", tasks.len());
    for t in &tasks {
        log::info!("  - [{}] {}", t.key, t.summary);
    }
    Ok(tasks)
}
