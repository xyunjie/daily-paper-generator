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
    pub id: String,
    pub short_id: String,
    pub title: String,
    pub message: Option<String>,
    pub created_at: String,
    pub authored_date: Option<String>,
    pub author_name: Option<String>,
    pub web_url: String,
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub project_name: String,
    pub short_id: String,
    pub title: String,
    #[allow(dead_code)]
    pub created_at: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitLabEvent {
    #[allow(dead_code)]
    pub action_name: Option<String>,
    pub project_id: Option<u64>,
    pub push_data: Option<GitLabPushData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitLabPushData {
    pub commit_title: Option<String>,
    pub commit_to: Option<String>,
    pub commit_from: Option<String>,
    pub commit_count: Option<u32>,
}

fn is_merge_like_commit_title(title: &str) -> bool {
    let title_lower = title.trim().to_lowercase();
    title_lower.starts_with("merge ")
        || title_lower.contains("merge branch")
        || title_lower.contains("merge pull request")
        || title_lower.contains("merge remote-tracking")
}

fn normalize_commit_title(title: &str) -> String {
    title.trim().split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn fetch_commits(config: &AppConfig, date: &str) -> Result<Vec<CommitInfo>, String> {
    let gitlab = &config.gitlab;

    log::info!(
        "fetch_commits called, user_id='{}', username='{}', base_url='{}'",
        gitlab.user_id,
        gitlab.username,
        gitlab.base_url
    );

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
                        let title = normalize_commit_title(&commit.title);
                        // 过滤掉合并相关的提交（对齐 events 模式）
                        if is_merge_like_commit_title(&title) {
                            continue;
                        }

                        log::info!(
                            "Found commit in {}: {}",
                            project.path_with_namespace,
                            title
                        );
                        all_commits.push(CommitInfo {
                            project_name: project.path_with_namespace.clone(),
                            short_id: commit.short_id,
                            title,
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
    let after_date = (date_obj - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let before_date = (date_obj + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

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

    // 收集所有 push events 的信息
    struct PushInfo {
        project_id: u64,
        commit_to: String,
        commit_from: Option<String>,
        commit_count: u32,
    }

    let mut push_infos: Vec<PushInfo> = Vec::new();
    for event in &events {
        let Some(push) = &event.push_data else { continue; };
        let Some(project_id) = event.project_id else { continue; };
        let Some(commit_to) = &push.commit_to else { continue; };
        push_infos.push(PushInfo {
            project_id,
            commit_to: commit_to.clone(),
            commit_from: push.commit_from.clone(),
            commit_count: push.commit_count.unwrap_or(1),
        });
    }

    // 获取涉及的项目信息
    let mut project_ids: Vec<u64> = push_infos.iter().map(|p| p.project_id).collect();
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
                    log::info!(
                        "Found project: {} -> {}",
                        project_id,
                        project.path_with_namespace
                    );
                    project_map.insert(*project_id, project.path_with_namespace);
                }
            }
        }
    }

    // 对每个 push event 获取完整的 commit 信息
    let mut all_commits: Vec<CommitInfo> = Vec::new();
    let mut seen_shas: std::collections::HashSet<String> = std::collections::HashSet::new();

    for push in &push_infos {
        let project_name = project_map
            .get(&push.project_id)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        // 获取这个 push 的所有 commits
        let commits_url = format!(
            "{}/api/v4/projects/{}/repository/commits?ref_name={}&per_page={}",
            gitlab.base_url.trim_end_matches('/'),
            push.project_id,
            push.commit_to,
            push.commit_count
        );

        log::info!("Fetching commits for push: {}", commits_url);

        if let Ok(response) = client
            .get(&commits_url)
            .header("PRIVATE-TOKEN", &gitlab.private_token)
            .header("Accept", "application/json")
            .send()
        {
            if response.status().is_success() {
                if let Ok(commits) = response.json::<Vec<GitLabCommit>>() {
                    for commit in commits {
                        // 如果已经处理过这个 commit，跳过
                        if seen_shas.contains(&commit.id) {
                            continue;
                        }

                        // 如果 commit_from 不为 null，过滤掉 commit_from 之前的 commits（不包括 commit_from 本身）
                        if let Some(from) = &push.commit_from {
                            if commit.id == *from {
                                break;
                            }
                        }

                        // 过滤：只保留当天的提交
                        let commit_date = commit.authored_date.as_ref().or(Some(&commit.created_at));
                        if let Some(commit_datetime) = commit_date {
                            // 提取日期部分（YYYY-MM-DD）
                            let commit_date_str = &commit_datetime[..10];
                            if commit_date_str != date {
                                log::debug!("Skipping commit {} (date mismatch: {} != {})", commit.short_id, commit_date_str, date);
                                continue;
                            }
                        }

                        // 过滤：只保留指定用户的提交（如果配置了 username）
                        if !gitlab.username.trim().is_empty() {
                            if let Some(author_name) = &commit.author_name {
                                if author_name != &gitlab.username {
                                    log::debug!("Skipping commit {} (author mismatch: {} != {})", commit.short_id, author_name, gitlab.username);
                                    continue;
                                }
                            } else {
                                log::debug!("Skipping commit {} (no author_name)", commit.short_id);
                                continue;
                            }
                        }

                        let title = normalize_commit_title(&commit.title);
                        if !is_merge_like_commit_title(&title) {
                            // 使用完整的 message，如果没有则使用 title
                            let content = commit.message.as_ref().unwrap_or(&commit.title).clone();
                            let normalized_content = normalize_commit_title(&content);

                            log::info!("Found commit: [{}] {} ({})", project_name, normalized_content, commit.short_id);
                            seen_shas.insert(commit.id.clone());
                            all_commits.push(CommitInfo {
                                project_name: project_name.clone(),
                                short_id: commit.short_id,
                                title: normalized_content,
                                created_at: commit.created_at,
                                url: commit.web_url,
                            });
                        }
                    }
                }
            }
        }
    }

    log::info!("Total commits found: {}", all_commits.len());
    Ok(all_commits)
}
