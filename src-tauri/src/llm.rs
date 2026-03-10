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

pub fn polish_with_openai(base_url: &str, api_key: &str, model: &str, input: &str) -> Result<Vec<String>, String> {
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
    let client = Client::new();

    let prompt = format!(
        "请将以下工作内容拆分为多条清晰的日报条目，并进行语义优化。返回每条一行：\n{}",
        input
    );

    let req = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: "你是日报助手，输出简洁的中文条目。".to_string() },
            ChatMessage { role: "user".to_string(), content: prompt },
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

    let lines = content
        .lines()
        .map(|l| l.trim().trim_start_matches("- ").trim_start_matches("• "))
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect::<Vec<_>>();

    Ok(lines)
}
