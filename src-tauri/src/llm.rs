use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
    content: String,
}

const DAILY_BULLET_MIN: usize = 3;
const DAILY_BULLET_MAX: usize = 8;

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
    // Fast heuristics (no regex dependency)
    if line.contains("http://") || line.contains("https://") {
        return true;
    }
    // likely project path like group/repo
    if line.contains('/') {
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

fn clean_line(line: &str) -> String {
    line.trim()
        .trim_start_matches("- ")
        .trim_start_matches("• ")
        .trim_start_matches("* ")
        .trim_start_matches("· ")
        .trim()
        .to_string()
}

fn compact_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn postprocess_daily_bullets(lines: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();

    for raw in lines {
        let cleaned = compact_whitespace(&clean_line(&raw));
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

pub fn polish_with_openai(
    base_url: &str,
    api_key: &str,
    model: &str,
    input: &str,
) -> Result<Vec<String>, String> {
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
    let client = Client::new();

    let system = format!(
        "你是日报助手。请将输入信息整合为可直接填日报的中文要点。\n\
硬性规则：\n\
1) 只输出要点列表，每条一行，不要标题/解释/前后缀。\n\
2) 输出条数为 {}-{} 条（信息确实很少时可少于 {} 条，但不要胡编）。\n\
3) 每条必须是纯中文要点，尽量以动词开头（如：优化/修复/测试/联调/完善/修改/添加）。\n\
4) 严禁在输出中出现 Jira Key（如 ABC-123）、GitLab 项目名/路径（如 group/repo）、提交 hash/short_id、URL。\n\
5) 优先合并同一主题/同一任务的多条提交，避免碎片化。",
        DAILY_BULLET_MIN,
        DAILY_BULLET_MAX,
        DAILY_BULLET_MIN
    );

    let few_shot = "【示例输入】\n\
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
优化分析与数据接口";

    let prompt = format!(
        "{}\n\n{}\n\n【开始处理】\n{}",
        "请严格按规则输出。",
        few_shot,
        input
    );

    let req = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system,
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ],
        temperature: 0.2,
    };

    let res = client
        .post(url)
        .bearer_auth(api_key)
        .json(&req)
        .send()
        .map_err(|e| format!("模型请求失败: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("模型接口错误: {}", res.status()));
    }

    let data: ChatResponse = res
        .json()
        .map_err(|e| format!("模型响应解析失败: {}", e))?;

    let content = data
        .choices
        .get(0)
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    let raw_lines = content
        .lines()
        .map(|l| l.to_string())
        .collect::<Vec<_>>();

    let lines = postprocess_daily_bullets(raw_lines);
    Ok(lines)
}
