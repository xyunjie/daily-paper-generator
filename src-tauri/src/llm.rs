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

pub fn summarize_week_with_openai(
    base_url: &str,
    api_key: &str,
    model: &str,
    week_items: &[String],
    system_prompt: &str,
) -> Result<String, String> {
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
    let client = Client::new();

    let items_text = week_items.join("\n- ");
    let prompt = format!(
        "【本周工作内容】\n- {}\n\n请生成本周工作总结（不超过200字）：",
        items_text
    );

    let req = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ],
        temperature: 0.3,
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

    Ok(content.trim().to_string())
}

pub fn polish_with_openai(
    base_url: &str,
    api_key: &str,
    model: &str,
    input: &str,
    system_prompt: &str,
    few_shot_prompt: &str,
) -> Result<Vec<String>, String> {
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
    let client = Client::new();

    let prompt = format!(
        "{}\n\n{}\n\n【开始处理】\n{}",
        "请严格按规则输出。",
        few_shot_prompt,
        input
    );

    let req = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
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
