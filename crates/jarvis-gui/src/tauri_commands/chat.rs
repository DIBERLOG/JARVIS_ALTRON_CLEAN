use crate::AppState;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ChatReply {
    pub content: String,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Serialize)]
pub struct ChatConfig {
    pub provider: String,
    pub local_model: String,
    pub deepseek_model: String,
    pub deepseek_configured: bool,
    pub speak_responses: bool,
    pub personality: String,
}

#[tauri::command]
pub fn chat_search_web(query: String) -> Result<String, String> {
    let query = query.trim();
    if query.is_empty() || query.len() > 500 { return Err("Некорректный запрос поиска".into()); }
    let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(20)).build().map_err(|e| e.to_string())?;
    let url = reqwest::Url::parse_with_params("https://html.duckduckgo.com/html/", &[("q", query)])
        .map_err(|e| format!("Некорректный запрос поиска: {e}"))?;
    let html = client.get(url).header("User-Agent", "Mozilla/5.0 Jarvis/1.0")
        .send().map_err(|e| format!("Поиск недоступен: {e}"))?
        .text().map_err(|e| format!("Не удалось прочитать выдачу: {e}"))?;
    let mut facts = Vec::new();
    let mut rest = html.as_str();
    while facts.len() < 5 {
        let Some(marker) = rest.find("result__snippet") else { break; };
        rest = &rest[marker..];
        let Some(start) = rest.find('>') else { break; };
        rest = &rest[start + 1..];
        let Some(end) = rest.find("</") else { break; };
        let snippet = rest[..end].replace("<b>", "").replace("</b>", "").replace("&amp;", "&").replace("&#x27;", "'");
        let snippet = snippet.split_whitespace().collect::<Vec<_>>().join(" ");
        if snippet.len() > 20 { facts.push(snippet); }
        rest = &rest[end + 2..];
    }
    if facts.is_empty() { return Err("Поиск не вернул сниппеты. Попробуй другой запрос или выключи WEB INTEL.".into()); }
    Ok(facts.join("\n\n"))
}

fn speech_text(text: &str) -> String {
    text.replace("**", "").replace('`', "").replace("###", "").replace("##", "").replace('#', "")
        .lines().filter(|line| !line.trim_start().starts_with("http")).collect::<Vec<_>>().join(" ")
}

#[tauri::command]
pub fn chat_get_config(state: tauri::State<'_, AppState>) -> ChatConfig {
    ChatConfig {
        provider: state.settings.read("chat_provider").unwrap_or_else(|| "local".into()),
        local_model: state.settings.read("local_chat_model").unwrap_or_else(|| "qwen3:8b".into()),
        deepseek_model: state.settings.read("deepseek_chat_model").unwrap_or_else(|| "deepseek-flash".into()),
        deepseek_configured: state.settings.read("api_key__deepseek").is_some_and(|key| !key.trim().is_empty()),
        speak_responses: state.settings.read("chat_speak_responses").map(|v| v != "false").unwrap_or(true),
        personality: state.settings.read("assistant_personality").unwrap_or_else(|| "jarvis".into()),
    }
}

#[tauri::command]
pub fn chat_send(state: tauri::State<'_, AppState>, messages: Vec<ChatMessage>) -> Result<ChatReply, String> {
    if messages.is_empty() || messages.len() > 20 { return Err("История чата пуста или слишком длинная".into()); }
    if messages.iter().any(|m| m.content.trim().is_empty() || m.content.len() > 12_000) { return Err("Некорректное сообщение".into()); }
    let provider = state.settings.read("chat_provider").unwrap_or_else(|| "local".into());
    let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(90)).build().map_err(|e| e.to_string())?;
    if provider == "deepseek" {
        let key = state.settings.read("api_key__deepseek").unwrap_or_default();
        if key.trim().is_empty() { return Err("Добавь API-ключ DeepSeek в настройках этого чата".into()); }
        let model = state.settings.read("deepseek_chat_model").unwrap_or_else(|| "deepseek-flash".into());
        let response = client.post("https://api.deepseek.com/chat/completions")
            .bearer_auth(key.trim())
            .json(&json!({"model": model, "messages": messages, "temperature": 0.7, "max_tokens": 700, "stream": false}))
            .send().map_err(|e| format!("DeepSeek недоступен: {e}"))?;
        let status = response.status();
        let body: serde_json::Value = response.json().map_err(|e| format!("Некорректный ответ DeepSeek: {e}"))?;
        if !status.is_success() { return Err(body["error"]["message"].as_str().unwrap_or("Ошибка DeepSeek API").to_string()); }
        let content = body["choices"][0]["message"]["content"].as_str().unwrap_or("").trim().to_string();
        if content.is_empty() { return Err("DeepSeek вернул пустой ответ".into()); }
        if state.settings.read("chat_speak_responses").map(|v| v != "false").unwrap_or(true) { jarvis_core::tts::speak(&speech_text(&content)); }
        Ok(ChatReply { content, provider, model })
    } else {
        let model = state.settings.read("local_chat_model").unwrap_or_else(|| "qwen3:8b".into());
        let response = client.post("http://127.0.0.1:11434/api/chat")
            .json(&json!({"model": model, "messages": messages, "stream": false, "options": {"num_ctx": 4096}}))
            .send().map_err(|_| "Локальный Ollama не запущен. Установи Ollama и выполни команду загрузки модели ниже.".to_string())?;
        let status = response.status();
        let body: serde_json::Value = response.json().map_err(|e| format!("Некорректный ответ Ollama: {e}"))?;
        if !status.is_success() { return Err(body["error"].as_str().unwrap_or("Ошибка Ollama").to_string()); }
        let content = body["message"]["content"].as_str().unwrap_or("").trim().to_string();
        if content.is_empty() { return Err("Локальная модель вернула пустой ответ".into()); }
        if state.settings.read("chat_speak_responses").map(|v| v != "false").unwrap_or(true) { jarvis_core::tts::speak(&speech_text(&content)); }
        Ok(ChatReply { content, provider, model })
    }
}
