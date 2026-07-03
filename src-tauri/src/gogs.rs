use crate::config::AppConfig;
use crate::gitlab::CommitInfo;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::HashSet;

// Gogs 的 API 与 Gitea 同源但更精简：没有「分页列出分支提交」的端点，
// 只能通过「列分支拿到 tip 提交」+「按 sha 逐个获取单提交（含 parents）」回溯提交历史。

#[derive(Debug, Clone, Deserialize)]
struct GogsRepo {
    full_name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GogsBranch {
    name: String,
    commit: Option<GogsBranchCommit>,
}

/// 分支对象内嵌的 tip 提交（PayloadCommit）：字段名与单提交端点不同
#[derive(Debug, Clone, Deserialize)]
struct GogsBranchCommit {
    id: String,
    #[serde(default)]
    timestamp: String,
}

/// 单提交端点返回结构（Commit）
#[derive(Debug, Clone, Deserialize)]
struct GogsCommit {
    sha: String,
    #[serde(default)]
    html_url: String,
    commit: GogsCommitDetail,
    #[serde(default)]
    parents: Vec<GogsCommitMeta>,
}

#[derive(Debug, Clone, Deserialize)]
struct GogsCommitDetail {
    message: String,
    author: GogsAuthor,
}

#[derive(Debug, Clone, Deserialize)]
struct GogsAuthor {
    #[serde(default)]
    name: String,
    #[serde(default)]
    email: String,
    #[serde(default)]
    date: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GogsCommitMeta {
    sha: String,
}

fn is_merge_like(title: &str) -> bool {
    let lower = title.trim().to_lowercase();
    lower.starts_with("merge ")
        || lower.contains("merge branch")
        || lower.contains("merge pull request")
        || lower.contains("merge remote-tracking")
}

pub fn fetch_commits(config: &AppConfig, date: &str) -> Result<Vec<CommitInfo>, String> {
    let gogs = &config.gogs;

    if gogs.base_url.trim().is_empty() || gogs.token.trim().is_empty() {
        return Ok(Vec::new());
    }

    log::info!(
        "Gogs fetch_commits: base_url='{}', username='{}', date='{}'",
        gogs.base_url,
        gogs.username,
        date
    );

    let client = Client::new();
    let base = gogs.base_url.trim().trim_end_matches('/');
    // token 若带首尾空白会被判为匿名请求（403），统一 trim 后再用
    let token = gogs.token.trim();

    let target_date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date format: {}", e))?;

    // 1. 拉取当前用户可访问的仓库（Gogs /user/repos 返回纯数组，无分页）
    let repos_url = format!("{}/api/v1/user/repos", base);
    let response = client
        .get(&repos_url)
        .header("Authorization", format!("token {}", token))
        .header("Accept", "application/json")
        .send()
        .map_err(|e| format!("Gogs repos request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(format!(
                "Gogs 鉴权失败（{}）：请确认 Access Token 有效且未过期，并检查 Token 是否误带空格。原始响应：{}",
                status, body
            ));
        }
        return Err(format!("Gogs API error: {} - {}", status, body));
    }

    let repos: Vec<GogsRepo> = response
        .json()
        .map_err(|e| format!("Failed to parse Gogs repos: {}", e))?;

    log::info!("Gogs: found {} repos", repos.len());

    let mut all_commits: Vec<CommitInfo> = Vec::new();
    let mut seen_shas: HashSet<String> = HashSet::new();
    // 已请求过的 sha（跨分支/跨仓库去重，避免重复拉取共享历史）
    let mut walked: HashSet<String> = HashSet::new();

    const MAX_WALK_PER_REPO: usize = 400;
    const MAX_TOTAL_WALK: usize = 3000;
    let mut total_walk = 0usize;

    'repos: for repo in &repos {
        if total_walk >= MAX_TOTAL_WALK {
            break;
        }

        let branches = fetch_branches(&client, base, token, &repo.full_name);
        let mut repo_walk = 0usize;

        for branch in &branches {
            let tip = match &branch.commit {
                Some(c) if !c.id.is_empty() => c,
                _ => continue,
            };

            // tip 早于目标日期 → 整条分支不可能有目标日期提交，直接跳过
            if !tip.timestamp.is_empty() && crate::utils::to_cst_date(&tip.timestamp) < target_date {
                continue;
            }

            // 从 tip 沿 parents 链回溯（DFS）
            let mut stack: Vec<String> = vec![tip.id.clone()];
            while let Some(sha) = stack.pop() {
                if sha.is_empty() || walked.contains(&sha) {
                    continue;
                }
                if repo_walk >= MAX_WALK_PER_REPO || total_walk >= MAX_TOTAL_WALK {
                    log::warn!(
                        "Gogs: repo {} branch {} 回溯达到上限，可能遗漏更早提交",
                        repo.full_name,
                        branch.name
                    );
                    continue 'repos;
                }
                walked.insert(sha.clone());
                repo_walk += 1;
                total_walk += 1;

                let commit =
                    match fetch_single_commit(&client, base, token, &repo.full_name, &sha) {
                        Some(c) => c,
                        None => continue,
                    };

                let commit_local_date = crate::utils::to_cst_date(&commit.commit.author.date);

                // 提交日期 >= 目标日期时继续回溯其父提交（更早的历史）
                if commit_local_date >= target_date {
                    for p in &commit.parents {
                        if !p.sha.is_empty() && !walked.contains(&p.sha) {
                            stack.push(p.sha.clone());
                        }
                    }
                }

                if commit_local_date != target_date {
                    continue;
                }

                if seen_shas.contains(&commit.sha) {
                    continue;
                }

                log::info!(
                    "Gogs: date match {} on {}:{} (author='{}', email='{}')",
                    &commit.sha[..8.min(commit.sha.len())],
                    repo.full_name,
                    branch.name,
                    commit.commit.author.name,
                    commit.commit.author.email
                );

                // 按用户名 / 邮箱过滤作者（大小写不敏感）
                let author_match = if !gogs.username.trim().is_empty() {
                    commit.commit.author.name.eq_ignore_ascii_case(&gogs.username)
                        || commit.commit.author.email.eq_ignore_ascii_case(&gogs.username)
                } else if !config.user_email.trim().is_empty() {
                    commit.commit.author.email.eq_ignore_ascii_case(&config.user_email)
                        || commit.commit.author.name.eq_ignore_ascii_case(&config.user_email)
                } else {
                    true
                };

                if !author_match {
                    log::debug!(
                        "Gogs: skip {} author mismatch (name='{}', email='{}')",
                        &commit.sha[..8.min(commit.sha.len())],
                        commit.commit.author.name,
                        commit.commit.author.email
                    );
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
                    "Gogs commit: [{}:{}] {} ({})",
                    repo.full_name,
                    branch.name,
                    title,
                    short_id
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
        }
    }

    log::info!("Gogs: total {} commits", all_commits.len());
    Ok(all_commits)
}

/// 获取仓库所有分支（含各分支 tip 提交）。失败时返回空列表。
fn fetch_branches(client: &Client, base: &str, token: &str, repo_full_name: &str) -> Vec<GogsBranch> {
    let url = format!("{}/api/v1/repos/{}/branches", base, repo_full_name);

    let response = match client
        .get(&url)
        .header("Authorization", format!("token {}", token))
        .header("Accept", "application/json")
        .send()
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    if !response.status().is_success() {
        return Vec::new();
    }

    let branches: Vec<GogsBranch> = response.json().unwrap_or_default();
    if !branches.is_empty() {
        log::info!(
            "Gogs: repo {} has {} branches",
            repo_full_name,
            branches.len()
        );
    }
    branches
}

/// 按 sha 获取单个提交（含 parents / html_url / 作者信息）。失败时返回 None。
fn fetch_single_commit(
    client: &Client,
    base: &str,
    token: &str,
    repo_full_name: &str,
    sha: &str,
) -> Option<GogsCommit> {
    let url = format!("{}/api/v1/repos/{}/commits/{}", base, repo_full_name, sha);

    let response = client
        .get(&url)
        .header("Authorization", format!("token {}", token))
        .header("Accept", "application/json")
        .send()
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    response.json().ok()
}
