use crate::config::AppConfig;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct WorkItemWithSource {
    pub content: String,
    pub source: String, // "jira" or "gitlab"
}

#[derive(Debug, Serialize)]
pub struct FetchedItems {
    pub items: Vec<WorkItemWithSource>,
}

// const DAILY_BULLET_MIN: usize = 3;
const DAILY_BULLET_MAX: usize = 8;

fn compact_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn looks_like_jira_key(s: &str) -> bool {
    // e.g. ABC-123
    let bytes = s.as_bytes();
    if bytes.len() < 5 {
        return false;
    }
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_uppercase() {
        i += 1;
    }
    if i < 2 || i + 1 >= bytes.len() {
        return false;
    }
    if bytes[i] != b'-' {
        return false;
    }
    i += 1;
    let start_digits = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    i == bytes.len() && (i - start_digits) >= 1
}

fn looks_like_hex_hash(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 7 {
        return false;
    }
    t.chars().all(|c| c.is_ascii_hexdigit())
}

fn contains_forbidden_markers(line: &str) -> bool {
    // No regex: apply strong heuristics.
    if line.contains("http://") || line.contains("https://") {
        return true;
    }
    if line.contains('/') {
        // likely project path like group/repo
        return true;
    }
    for token in line
        .split(|c: char| c.is_whitespace() || c == '，' || c == ',' || c == ';' || c == '；')
        .filter(|t| !t.is_empty())
    {
        if looks_like_jira_key(token) {
            return true;
        }
        if looks_like_hex_hash(token) {
            return true;
        }
    }
    false
}

fn postprocess_bullets(mut lines: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in lines.drain(..) {
        let cleaned = compact_whitespace(raw.trim());
        if cleaned.is_empty() {
            continue;
        }
        if contains_forbidden_markers(&cleaned) {
            continue;
        }
        if out.iter().any(|x| x == &cleaned) {
            continue;
        }
        out.push(cleaned);
    }
    if out.len() > DAILY_BULLET_MAX {
        out.truncate(DAILY_BULLET_MAX);
    }
    out
}

fn sanitize_task_summary(summary: &str) -> String {
    // Remove bracketed IDs or paths that often appear in summaries.
    let mut s = summary.replace('[', "").replace(']', "");
    s = s.replace('(', "").replace(')', "");
    compact_whitespace(&s)
}

fn normalize_commit_title(title: &str) -> String {
    let mut t = compact_whitespace(title);

    // Drop common conventional-commit prefixes.
    let lower = t.to_lowercase();
    for prefix in [
        "feat:",
        "fix:",
        "chore:",
        "refactor:",
        "perf:",
        "test:",
        "docs:",
        "style:",
        "ci:",
        "build:",
    ] {
        if lower.starts_with(prefix) {
            t = t[prefix.len()..].trim().to_string();
            break;
        }
    }

    // If starts with type(scope): try to strip to after ':'.
    if let Some(idx) = t.find(':') {
        let before = &t[..idx];
        if before.contains('(') && before.contains(')') && before.len() <= 24 {
            t = t[idx + 1..].trim().to_string();
        }
    }

    t
}

fn extract_jira_keys(text: &str) -> Vec<String> {
    // Scan for patterns like ABC-123 without regex.
    let bytes = text.as_bytes();
    let mut keys = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_uppercase() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_uppercase() {
            i += 1;
        }
        let letters = i - start;
        if letters < 2 || i >= bytes.len() || bytes[i] != b'-' {
            continue;
        }
        i += 1;
        let digit_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let digits = i - digit_start;
        if digits >= 1 {
            let key = String::from_utf8_lossy(&bytes[start..i]).to_string();
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
    }
    keys
}

fn summarize_locally(date: &str, tasks: &[(String, String)], commits: &[String]) -> Vec<String> {
    // tasks: [(key, summary)] ; commits: normalized titles

    // Build mapping key -> related commits
    let mut key_to_commits: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for title in commits {
        for key in extract_jira_keys(title) {
            key_to_commits.entry(key).or_default().push(title.clone());
        }
    }

    let mut bullets: Vec<String> = Vec::new();

    // Prefer Jira summaries as backbone
    for (key, summary) in tasks {
        let mut s = sanitize_task_summary(summary);
        if s.is_empty() {
            continue;
        }
        // Add light hint when many related commits exist, but keep Chinese output.
        if let Some(rel) = key_to_commits.get(key) {
            if rel.len() >= 3 {
                // nudge towards result-oriented phrasing
                if !s.starts_with("优化") && !s.starts_with("修复") && !s.starts_with("修改") && !s.starts_with("添加") && !s.starts_with("完善") && !s.starts_with("测试") && !s.starts_with("联调") {
                    s = format!("完成{}", s);
                }
            }
        }
        bullets.push(s);
    }

    // If no Jira, derive from commit titles by keyword buckets
    if bullets.is_empty() {
        let mut buckets: std::collections::BTreeMap<&'static str, Vec<String>> = std::collections::BTreeMap::new();
        for c in commits {
            let lower = c.to_lowercase();
            let key = if lower.contains("fix") || lower.contains("bug") || c.contains("修复") {
                "修复"
            } else if lower.contains("opt") || lower.contains("perf") || c.contains("优化") {
                "优化"
            } else if lower.contains("test") || c.contains("测试") {
                "测试"
            } else if lower.contains("api") || c.contains("接口") {
                "接口"
            } else if lower.contains("export") || c.contains("导出") {
                "导出"
            } else if lower.contains("stat") || lower.contains("metric") || c.contains("统计") {
                "统计"
            } else {
                "其他"
            };
            buckets.entry(key).or_default().push(c.clone());
        }

        for (k, v) in buckets {
            if v.is_empty() {
                continue;
            }
            let bullet = match k {
                "修复" => "修复相关问题",
                "优化" => "优化相关逻辑",
                "测试" => "测试功能与接口",
                "接口" => "优化接口与联调",
                "导出" => "完善导出相关功能",
                "统计" => "修复并优化统计逻辑",
                _ => "处理日常开发事项",
            };
            bullets.push(bullet.to_string());
        }

        log::info!("Local summarizer produced {} bullets for date {}", bullets.len(), date);
    }

    // De-dup and clamp
    let mut uniq: Vec<String> = Vec::new();
    for b in bullets {
        let b = compact_whitespace(&b);
        if b.is_empty() {
            continue;
        }
        if contains_forbidden_markers(&b) {
            continue;
        }
        if uniq.iter().any(|x| x == &b) {
            continue;
        }
        uniq.push(b);
        if uniq.len() >= DAILY_BULLET_MAX {
            break;
        }
    }
    uniq
}

fn build_llm_input(date: &str, tasks: &[(String, String)], commits: &[String]) -> String {
    let mut s = String::new();
    s.push_str(&format!("【日期】{}\n", date));
    s.push_str("【硬性要求】\n");
    s.push_str("- 输出 3-8 条中文要点，每条一行\n");
    s.push_str("- 只输出要点，不要标题/解释\n");
    s.push_str("- 输出不得包含 Jira Key / GitLab 项目名或路径 / 提交 hash 或 short_id / URL\n");

    s.push_str("\n【Jira Done 任务】\n");
    if tasks.is_empty() {
        s.push_str("(无)\n");
    } else {
        for (key, summary) in tasks {
            s.push_str(&format!("key={} | summary={}\n", key, summary));
        }
    }

    s.push_str("\n【GitLab 提交摘要】\n");
    if commits.is_empty() {
        s.push_str("(无)\n");
    } else {
        for title in commits {
            s.push_str(&format!("- {}\n", title));
        }
    }

    s.push_str("\n【关联提示】\n");
    s.push_str("- 如果多个提交都指向同一 Jira 任务，请合并为更像结果导向的 1 条要点\n");
    s.push_str("- 允许理解 key/project 作为内部线索，但输出禁止出现它们\n");

    s
}

pub fn fetch_daily_items(config: &AppConfig, date: &str) -> Result<FetchedItems, String> {
    log::info!("开始自动获取: date={}", date);

    let jira_tasks = crate::jira::fetch_tasks(config, date)?;
    let gitlab_commits = crate::gitlab::fetch_commits(config, date)?;

    let mut items: Vec<WorkItemWithSource> = Vec::new();

    for task in &jira_tasks {
        let content = sanitize_task_summary(&task.summary);
        if !content.is_empty() {
            items.push(WorkItemWithSource {
                content,
                source: "jira".to_string(),
            });
        }
    }

    for commit in &gitlab_commits {
        let content = normalize_commit_title(&commit.title);
        if !content.is_empty() {
            items.push(WorkItemWithSource {
                content,
                source: "gitlab".to_string(),
            });
        }
    }

    log::info!("自动获取完成: jira={} 条, gitlab={} 条, 合计={} 条", jira_tasks.len(), gitlab_commits.len(), items.len());
    Ok(FetchedItems { items })
}

pub fn polish_daily_items(config: &AppConfig, date: &str, raw_items: &[WorkItemWithSource]) -> Result<Vec<String>, String> {
    log::info!("AI润色: date={}, 输入 {} 条", date, raw_items.len());

    let tasks_struct: Vec<(String, String)> = raw_items
        .iter()
        .filter(|i| i.source == "jira")
        .map(|i| (String::new(), i.content.clone()))
        .collect();

    let commits_titles: Vec<String> = raw_items
        .iter()
        .filter(|i| i.source == "gitlab")
        .map(|i| i.content.clone())
        .collect();

    log::info!("AI润色: jira={} 条, gitlab={} 条", tasks_struct.len(), commits_titles.len());

    let has_model = !config.model.base_url.trim().is_empty()
        && !config.model.api_key.trim().is_empty()
        && !config.model.model.trim().is_empty();

    if !has_model {
        log::warn!("AI润色: 未配置模型信息，无法润色");
        return Err("请先配置模型信息（base_url / api_key / model）".to_string());
    }

    log::info!("AI润色: 调用模型 {}", config.model.model);
    let input = build_llm_input(date, &tasks_struct, &commits_titles);
    let lines = match crate::llm::polish_with_openai(
        &config.model.base_url,
        &config.model.api_key,
        &config.model.model,
        &input,
    ) {
        Ok(lines) => {
            let cleaned = crate::llm::postprocess_daily_bullets(lines);
            if cleaned.is_empty() {
                log::warn!("AI润色: 模型返回为空，回退到本地规则");
                summarize_locally(date, &tasks_struct, &commits_titles)
            } else {
                log::info!("AI润色: 模型返回 {} 条", cleaned.len());
                cleaned
            }
        }
        Err(e) => {
            log::warn!("AI润色: 模型调用失败，回退到本地规则: {}", e);
            summarize_locally(date, &tasks_struct, &commits_titles)
        }
    };

    let mut final_lines = postprocess_bullets(lines);
    if final_lines.len() > DAILY_BULLET_MAX {
        final_lines.truncate(DAILY_BULLET_MAX);
    }

    log::info!("AI润色完成: 输出 {} 条", final_lines.len());
    Ok(final_lines)
}
