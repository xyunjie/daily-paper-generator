use crate::config::AppConfig;
use reqwest::blocking::Client;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct GitLabProject {
    pub id: u64,
    pub path_with_namespace: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitLabCommit {
    pub short_id: String,
    pub title: String,
    pub created_at: String,
    pub web_url: String,
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub project_name: String,
    pub short_id: String,
    pub title: String,
    pub created_at: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitLabEvent {
    pub action_name: Option<String>,
    pub project_id: Option<u64>,
    pub push_data: Option<GitLabPushData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitLabPushData {
    pub commit_title: Option<String>,
}

pub fn fetch_commits(config: &AppConfig, date: &str) -> Result<Vec<CommitInfo>, String> {
    let gitlab = &config.gitlab;

    log::info!("fetch_commits called, user_id='{}', username='{}', base_url='{}'",
        gitlab.user_id, gitlab.username, gitlab.base_url);

    if !gitlab.user_id.trim().is_empty() {
        log::info!("Using events API (user_id mode)");
        return fetch_commits_by_events(config, date);
    }

    log::info!("Using projects API (membership mode)");

    let client = Client::new();

    // 获取用户参与的项目
    let projects_url = format!(
        "{}/api/v4/projects?membership=true&per_page=100",
        gitlab.base_url.trim_end_matches('/')
    );

    log::info!("Fetching projects from: {}", projects_url);

    let projects_response = client
        .get(&projects_url)
        .header("PRIVATE-TOKEN", &gitlab.private_token)
        .header("Accept", "application/json")
        .send()
        .map_err(|e| format!("GitLab projects request failed: {}", e))?;

    log::info!("Projects response status: {}", projects_response.status());

    if !projects_response.status().is_success() {
        let status = projects_response.status();
        let body = projects_response.text().unwrap_or_default();
        return Err(format!("GitLab API error: {} - {}", status, body));
    }

    let projects: Vec<GitLabProject> = projects_response
        .json()
        .map_err(|e| format!("Failed to parse GitLab projects: {}", e))?;

    log::info!("Found {} GitLab projects", projects.len());

    let mut all_commits: Vec<CommitInfo> = Vec::new();

    // 获取每个项目的当日提交
    for project in &projects {
        let commits_url = if !gitlab.username.trim().is_empty() {
            format!(
                "{}/api/v4/projects/{}/repository/commits?since={}T00:00:00Z&until={}T23:59:59Z&author_username={}",
                gitlab.base_url.trim_end_matches('/'),
                project.id,
                date,
                date,
                urlencoding::encode(&gitlab.username)
            )
        } else {
            format!(
                "{}/api/v4/projects/{}/repository/commits?since={}T00:00:00Z&until={}T23:59:59Z&author_email={}",
                gitlab.base_url.trim_end_matches('/'),
                project.id,
                date,
                date,
                urlencoding::encode(&config.user_email)
            )
        };

        if let Ok(commits_response) = client
            .get(&commits_url)
            .header("PRIVATE-TOKEN", &gitlab.private_token)
            .header("Accept", "application/json")
            .send()
        {
            if commits_response.status().is_success() {
                if let Ok(commits) = commits_response.json::<Vec<GitLabCommit>>() {
                    for commit in commits {
                        log::info!("Found commit in {}: {}", project.path_with_namespace, commit.title);
                        all_commits.push(CommitInfo {
                            project_name: project.path_with_namespace.clone(),
                            short_id: commit.short_id,
                            title: commit.title,
                            created_at: commit.created_at,
                            url: commit.web_url,
                        });
                    }
                }
            }
        }
    }

    log::info!("Fetched {} GitLab commits", all_commits.len());
    Ok(all_commits)
}

fn fetch_commits_by_events(config: &AppConfig, date: &str) -> Result<Vec<CommitInfo>, String> {
    let gitlab = &config.gitlab;
    let client = Client::new();

    log::info!("Fetching GitLab events for user_id={}, date={}", gitlab.user_id, date);

    // 计算 before 日期（明天），after 日期（昨天）
    // GitLab API: after 不包含当天，before 包含当天
    // 所以查某一天要用 after=昨天, before=明天
    let date_obj = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date format: {}", e))?;
    let after_date = (date_obj - chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
    let before_date = (date_obj + chrono::Duration::days(1)).format("%Y-%m-%d").to_string();

    let url = format!(
        "{}/api/v4/users/{}/events?after={}&before={}&per_page=100",
        gitlab.base_url.trim_end_matches('/'),
        gitlab.user_id,
        after_date,
        before_date
    );

    log::info!("GitLab events URL: {}", url);

    let response = client
        .get(&url)
        .header("PRIVATE-TOKEN", &gitlab.private_token)
        .header("Accept", "application/json")
        .send()
        .map_err(|e| format!("GitLab events request failed: {}", e))?;

    log::info!("GitLab events response status: {}", response.status());

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("GitLab API error: {} - {}", status, body));
    }

    let events: Vec<GitLabEvent> = response
        .json()
        .map_err(|e| format!("Failed to parse GitLab events: {}", e))?;

    log::info!("GitLab events count: {}", events.len());

    let mut project_ids: Vec<u64> = events
        .iter()
        .filter(|e| e.push_data.is_some())
        .filter_map(|e| e.project_id)
        .collect();
    project_ids.sort();
    project_ids.dedup();

    log::info!("Project IDs with push data: {:?}", project_ids);

    let mut project_map = std::collections::HashMap::new();
    for project_id in &project_ids {
        let project_url = format!(
            "{}/api/v4/projects/{}",
            gitlab.base_url.trim_end_matches('/'),
            project_id
        );
        if let Ok(project_response) = client
            .get(&project_url)
            .header("PRIVATE-TOKEN", &gitlab.private_token)
            .header("Accept", "application/json")
            .send()
        {
            if project_response.status().is_success() {
                if let Ok(project) = project_response.json::<GitLabProject>() {
                    log::info!("Found project: {} -> {}", project_id, project.path_with_namespace);
                    project_map.insert(*project_id, project.path_with_namespace);
                }
            }
        }
    }

    let mut all_commits: Vec<CommitInfo> = Vec::new();
    for event in &events {
        let Some(push) = &event.push_data else { continue };
        let Some(title) = &push.commit_title else { continue };
        // 过滤掉合并相关的提交
        let title_lower = title.to_lowercase();
        if title_lower.starts_with("merge ")
            || title_lower.contains("merge branch")
            || title_lower.contains("merge pull request")
            || title_lower.contains("merge remote-tracking")
        {
            continue;
        }
        let project_name = event
            .project_id
            .as_ref()
            .and_then(|id| project_map.get(id).cloned())
            .unwrap_or_else(|| "unknown".to_string());

        log::info!("Found commit: [{}] {}", project_name, title);

        all_commits.push(CommitInfo {
            project_name,
            short_id: "".to_string(),
            title: title.clone(),
            created_at: "".to_string(),
            url: "".to_string(),
        });
    }

    log::info!("Total commits found: {}", all_commits.len());
    Ok(all_commits)
}
