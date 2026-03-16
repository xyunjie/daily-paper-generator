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

pub fn generate_week_tasks_with_openai(
    base_url: &str,
    api_key: &str,
    model: &str,
    week_items: &[String],
) -> Result<(String, String), String> {
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
    let client = Client::new();

    let items_text = week_items.join("\n- ");
    let prompt = format!(
        "【本周工作内容】\n- {}\n\n\
        请根据以上工作内容生成两部分：\n\
        1. 本周重点任务（提炼 3-5 条最关键的任务/方向，每条一行）\n\
        2. 任务完成情况（对应每条重点任务的完成状态和进展）\n\n\
        严格按以下格式输出，不要添加额外内容：\n\
        【重点任务】\n\
        1. 任务描述\n\
        2. 任务描述\n\
        【完成情况】\n\
        1. 已完成/进行中 + 简要说明\n\
        2. 已完成/进行中 + 简要说明",
        items_text
    );

    let system = "你是周报助手。请根据工作内容提炼本周重点任务和对应完成情况。\n\
        硬性规则：\n\
        1) 重点任务 3-5 条，概括本周主要工作方向，不要逐条罗列琐碎条目。\n\
        2) 完成情况与重点任务一一对应，标注'已完成'或'进行中 XX%'并附简要说明。\n\
        3) 纯中文输出，禁止出现 Jira Key、项目路径、commit hash、URL。\n\
        4) 如实描述，不要编造没有的工作。";

    let req = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system.to_string(),
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

    // Parse the two sections
    let content = content.trim();
    let (key_tasks, completion_status) = parse_tasks_response(content);
    Ok((key_tasks, completion_status))
}

fn parse_tasks_response(content: &str) -> (String, String) {
    let mut key_tasks = String::new();
    let mut completion = String::new();
    let mut current_section = 0; // 0=none, 1=key_tasks, 2=completion

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains("重点任务") && trimmed.starts_with('【') {
            current_section = 1;
            continue;
        }
        if (trimmed.contains("完成情况") || trimmed.contains("完成状态"))
            && trimmed.starts_with('【')
        {
            current_section = 2;
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        match current_section {
            1 => {
                if !key_tasks.is_empty() {
                    key_tasks.push('\n');
                }
                key_tasks.push_str(trimmed);
            }
            2 => {
                if !completion.is_empty() {
                    completion.push('\n');
                }
                completion.push_str(trimmed);
            }
            _ => {}
        }
    }

    // Fallback: if parsing failed, put everything in key_tasks
    if key_tasks.is_empty() && completion.is_empty() {
        key_tasks = content.to_string();
    }

    (key_tasks, completion)
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
