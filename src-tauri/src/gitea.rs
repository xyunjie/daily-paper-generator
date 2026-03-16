use crate::config::AppConfig;
use crate::gitlab::CommitInfo;
use reqwest::blocking::Client;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct GiteaRepo {
    full_name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GiteaCommit {
    sha: String,
    commit: GiteaCommitDetail,
    html_url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GiteaCommitDetail {
    message: String,
    author: GiteaAuthor,
}

#[derive(Debug, Clone, Deserialize)]
struct GiteaAuthor {
    name: String,
    email: String,
    date: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GiteaRepoSearchResult {
    data: Vec<GiteaRepo>,
}

#[derive(Debug, Clone, Deserialize)]
struct GiteaBranch {
    name: String,
}

fn is_merge_like(title: &str) -> bool {
    let lower = title.trim().to_lowercase();
    lower.starts_with("merge ")
        || lower.contains("merge branch")
        || lower.contains("merge pull request")
        || lower.contains("merge remote-tracking")
}

pub fn fetch_commits(config: &AppConfig, date: &str) -> Result<Vec<CommitInfo>, String> {
    let gitea = &config.gitea;

    if gitea.base_url.trim().is_empty() || gitea.token.trim().is_empty() {
        return Ok(Vec::new());
    }

    log::info!(
        "Gitea fetch_commits: base_url='{}', username='{}', date='{}'",
        gitea.base_url,
        gitea.username,
        date
    );

    let client = Client::new();
    let base = gitea.base_url.trim_end_matches('/');

    // Fetch repos the user has access to
    let mut all_repos: Vec<GiteaRepo> = Vec::new();
    let mut page = 1u32;
    loop {
        let url = format!(
            "{}/api/v1/repos/search?limit=50&page={}",
            base, page
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("token {}", gitea.token))
            .header("Accept", "application/json")
            .send()
            .map_err(|e| format!("Gitea repos request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!("Gitea API error: {} - {}", status, body));
        }

        let result: GiteaRepoSearchResult = response
            .json()
            .map_err(|e| format!("Failed to parse Gitea repos: {}", e))?;

        if result.data.is_empty() {
            break;
        }

        all_repos.extend(result.data);

        if all_repos.len() >= 200 {
            break;
        }
        page += 1;
    }

    log::info!("Gitea: found {} repos", all_repos.len());

    let target_date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date format: {}", e))?;

    let mut all_commits: Vec<CommitInfo> = Vec::new();
    let mut seen_shas: std::collections::HashSet<String> = std::collections::HashSet::new();

    for repo in &all_repos {
        // 获取该仓库所有分支
        let branches = fetch_branches(&client, base, &gitea.token, &repo.full_name);

        for branch in &branches {
            // 分页遍历该分支的 commit
            // 按时间倒序，遇到整页都早于目标日期则停止
            let mut page = 1u32;
            const PAGE_LIMIT: u32 = 50;
            const MAX_PAGES: u32 = 3;

            loop {
                let commits_url = format!(
                    "{}/api/v1/repos/{}/commits?sha={}&limit={}&page={}",
                    base,
                    repo.full_name,
                    urlencoding::encode(branch),
                    PAGE_LIMIT,
                    page
                );

                let response = match client
                    .get(&commits_url)
                    .header("Authorization", format!("token {}", gitea.token))
                    .header("Accept", "application/json")
                    .send()
                {
                    Ok(r) => r,
                    Err(_) => break,
                };

                if !response.status().is_success() {
                    break;
                }

                let commits: Vec<GiteaCommit> = match response.json() {
                    Ok(c) => c,
                    Err(_) => break,
                };

                if commits.is_empty() {
                    break;
                }

                let mut all_before_target = true;

                for commit in &commits {
                    if seen_shas.contains(&commit.sha) {
                        continue;
                    }

                    // 解析 commit 本地日期
                    let raw_date = &commit.commit.author.date;
                    let commit_local_date = chrono::DateTime::parse_from_rfc3339(raw_date)
                        .map(|dt| dt.with_timezone(&chrono::Local).date_naive())
                        .unwrap_or_else(|_| {
                            chrono::NaiveDate::parse_from_str(
                                raw_date.get(..10).unwrap_or(""),
                                "%Y-%m-%d",
                            )
                            .unwrap_or(chrono::NaiveDate::MIN)
                        });

                    if commit_local_date >= target_date {
                        all_before_target = false;
                    }

                    if commit_local_date != target_date {
                        continue;
                    }

                    // 目标日期匹配，打印诊断信息
                    log::info!(
                        "Gitea: date match {} on {}:{} (author='{}', email='{}')",
                        &commit.sha[..8.min(commit.sha.len())],
                        repo.full_name,
                        branch,
                        commit.commit.author.name,
                        commit.commit.author.email
                    );

                    // Filter by username or email (case-insensitive)
                    let author_match = if !gitea.username.trim().is_empty() {
                        commit.commit.author.name.eq_ignore_ascii_case(&gitea.username)
                            || commit.commit.author.email.eq_ignore_ascii_case(&gitea.username)
                    } else if !config.user_email.trim().is_empty() {
                        commit.commit.author.email.eq_ignore_ascii_case(&config.user_email)
                            || commit.commit.author.name.eq_ignore_ascii_case(&config.user_email)
                    } else {
                        true
                    };

                    if !author_match {
                        log::debug!(
                            "Gitea: skip {} author mismatch (name='{}', email='{}')",
                            &commit.sha[..8.min(commit.sha.len())],
                            commit.commit.author.name,
                            commit.commit.author.email
                        );
                    }

                    if !author_match {
                        continue;
                    }

                    let title = commit
                        .commit
                        .message
                        .lines()
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string();

                    if title.is_empty() || is_merge_like(&title) {
                        continue;
                    }

                    let short_id = if commit.sha.len() >= 8 {
                        commit.sha[..8].to_string()
                    } else {
                        commit.sha.clone()
                    };

                    log::info!(
                        "Gitea commit: [{}:{}] {} ({})",
                        repo.full_name, branch, title, short_id
                    );

                    seen_shas.insert(commit.sha.clone());
                    all_commits.push(CommitInfo {
                        project_name: repo.full_name.clone(),
                        short_id,
                        title,
                        created_at: commit.commit.author.date.clone(),
                        url: commit.html_url.clone(),
                    });
                }

                if all_before_target {
                    break;
                }

                page += 1;
                if page > MAX_PAGES {
                    break;
                }
            }
        }
    }

    log::info!("Gitea: total {} commits", all_commits.len());
    Ok(all_commits)
}

/// 获取仓库的所有分支名，失败时返回仅含默认分支的 fallback
fn fetch_branches(client: &Client, base: &str, token: &str, repo_full_name: &str) -> Vec<String> {
    let url = format!(
        "{}/api/v1/repos/{}/branches?limit=50",
        base, repo_full_name
    );

    let response = match client
        .get(&url)
        .header("Authorization", format!("token {}", token))
        .header("Accept", "application/json")
        .send()
    {
        Ok(r) => r,
        Err(_) => return vec!["master".to_string()],
    };

    if !response.status().is_success() {
        return vec!["master".to_string()];
    }

    let branches: Vec<GiteaBranch> = match response.json() {
        Ok(b) => b,
        Err(_) => return vec!["master".to_string()],
    };

    if branches.is_empty() {
        return vec!["master".to_string()];
    }

    let names: Vec<String> = branches.into_iter().map(|b| b.name).collect();
    log::info!("Gitea: repo {} has {} branches", repo_full_name, names.len());
    names
}
