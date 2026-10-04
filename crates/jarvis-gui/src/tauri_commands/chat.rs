use crate::AppState;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use jarvis_core::chat::{is_news_request, needs_live_info};

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
    let response = client.get(url).header("User-Agent", "Mozilla/5.0 Jarvis/1.0")
        .send().map_err(|e| format!("Поиск недоступен: {e}"))?
        .error_for_status().map_err(|e| format!("Поиск вернул ошибку: {e}"))?;
    let html = response
        .text().map_err(|e| format!("Не удалось прочитать выдачу: {e}"))?;
    let facts = extract_search_snippets(&html);
    if facts.is_empty() { return Err("Поиск не вернул сниппеты. Попробуй другой запрос или выключи WEB INTEL.".into()); }
    Ok(facts.join("\n\n"))
}

fn extract_search_snippets(html: &str) -> Vec<String> {
    let mut facts = Vec::new();
    let mut rest = html;
    while facts.len() < 5 {
        let Some(marker) = rest.find("result__snippet") else { break; };
        rest = &rest[marker..];
        let Some(start) = rest.find('>') else { break; };
        let link = rest[..start].split("href=\"").nth(1).and_then(|value| value.split('"').next())
            .and_then(|raw| {
                let raw = raw.replace("&amp;", "&");
                let absolute = if raw.starts_with("//") { format!("https:{raw}") } else { raw };
                let parsed = reqwest::Url::parse(&absolute).ok()?;
                if parsed.host_str() == Some("duckduckgo.com") {
                    parsed.query_pairs().find(|(key, _)| key == "uddg").map(|(_, value)| value.into_owned())
                } else { Some(parsed.to_string()) }
            }).filter(|url| url.starts_with("https://") || url.starts_with("http://"));
        rest = &rest[start + 1..];
        let Some(end) = rest.find("</a>") else { break; };
        let mut in_tag = false;
        let plain: String = rest[..end].chars().filter(|ch| match ch {
            '<' => { in_tag = true; false },
            '>' => { in_tag = false; false },
            _ => !in_tag,
        }).collect();
        let plain = plain.replace("&amp;", "&").replace("&quot;", "\"")
            .replace("&#x27;", "'").replace("&#39;", "'")
            .replace("&lt;", "<").replace("&gt;", ">");
        let snippet = plain.split_whitespace().collect::<Vec<_>>().join(" ");
        if snippet.chars().count() > 20 {
            if let Some(url) = link { facts.push(format!("{snippet}\nИсточник: {url}")); }
        }
        rest = &rest[end + "</a>".len()..];
    }
    facts
}

#[cfg(test)]
mod search_tests {
    use super::{chat_search_web, extract_search_snippets, needs_live_info};

    #[test]
    fn nested_markup_does_not_truncate_search_result() {
        let html = r#"<a class="result__snippet" href="https://example.org/test"><b>Погода</b> в Москве: сегодня тепло &amp; сухо.</a>"#;
        assert_eq!(extract_search_snippets(html), vec!["Погода в Москве: сегодня тепло & сухо.\nИсточник: https://example.org/test"]);
    }

    #[test]
    fn recognizes_fresh_information_requests() {
        assert!(needs_live_info("Какие новости сегодня?"));
        assert!(needs_live_info("Актуальная информация о запуске"));
        assert!(!needs_live_info("Объясни, что такое процессор"));
    }

    #[test]
    #[ignore = "requires live network access"]
    fn live_search_has_source_links() {
        let facts = chat_search_web("официальный сайт Python новости".into()).unwrap();
        assert!(facts.contains("Источник: https://"));
    }
}

fn speech_text(text: &str) -> String {
    text.replace("**", "").replace('`', "").replace("###", "").replace("##", "").replace('#', "")
        .lines().filter(|line| !line.trim_start().starts_with("http")).collect::<Vec<_>>().join(" ")
}

#[tauri::command]
pub fn chat_is_speaking() -> bool {
    jarvis_core::tts::is_speaking()
}

#[tauri::command]
pub fn chat_stop_speech() {
    jarvis_core::tts::stop();
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
pub async fn chat_send(state: tauri::State<'_, AppState>, client_messages: Vec<ChatMessage>, use_web_search: bool) -> Result<ChatReply, String> {
    let state = AppState { settings: state.settings.clone() };
    tauri::async_runtime::spawn_blocking(move || send_chat(&state, client_messages, use_web_search)).await
        .map_err(|e| format!("Не удалось получить ответ: {e}"))?
}

fn send_chat(state: &AppState, client_messages: Vec<ChatMessage>, use_web_search: bool) -> Result<ChatReply, String> {
    if client_messages.is_empty() || client_messages.len() > 20 { return Err("История чата пуста или слишком длинная".into()); }
    if client_messages.iter().any(|m| m.content.trim().is_empty() || m.content.len() > 12_000) { return Err("Некорректное сообщение".into()); }
    let provider = state.settings.read("chat_provider").unwrap_or_else(|| "local".into());
    let personality = state.settings.read("assistant_personality").unwrap_or_else(|| "jarvis".into());
    let mut messages = vec![ChatMessage { role: "system".into(), content: format!("{}\n{}", jarvis_core::chat::persona_prompt(&personality), jarvis_core::chat::web_evidence_prompt()) }];
    let latest = client_messages.last().unwrap().content.to_lowercase();
    messages.extend(client_messages);
    let mut source_footer = String::new();
    if use_web_search || needs_live_info(&latest) {
        let now = chrono::Local::now().to_rfc3339();
        let facts = if is_news_request(&latest) {
            let items = super::news::fetch_news()?;
            source_footer = items.iter().take(4).map(|item| format!("{} ({}) — {}", item.source, item.published_at, item.url)).collect::<Vec<_>>().join("\n");
            items.iter().take(10).map(|item| format!("{} | {} | {} | {}", item.source, item.published_at, item.title, item.url)).collect::<Vec<_>>().join("\n")
        } else {
            let results = chat_search_web(messages.last().unwrap().content.clone())?;
            source_footer = results.lines().filter(|line| line.starts_with("Источник: ")).take(4).collect::<Vec<_>>().join("\n");
            results
        };
        messages.last_mut().unwrap().content.push_str(&format!("\n\nТекущая дата: {now}. Ниже результаты свежего поиска. Используй их для ответа; называй источник, ссылку и дату публикации, если дата указана. Не выдумывай отсутствующие даты или факты. Если данных недостаточно, скажи об этом.\n{facts}"));
    }
    let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(90)).build().map_err(|e| e.to_string())?;
    if provider == "deepseek" {
        let key = state.settings.read("api_key__deepseek").unwrap_or_default();
        if key.trim().is_empty() { return Err("Добавь API-ключ DeepSeek в настройках этого чата".into()); }
        let model = state.settings.read("deepseek_chat_model").unwrap_or_else(|| "deepseek-flash".into());
        let response = client.post("https://api.deepseek.com/chat/completions")
            .bearer_auth(key.trim())
            .json(&json!({"model": model, "messages": messages, "temperature": 0.7, "max_tokens": 700, "stream": true}))
            .send().map_err(|e| format!("DeepSeek недоступен: {e}"))?;
        let speak = state.settings.read("chat_speak_responses").map(|v| v != "false").unwrap_or(true);
        let generation = jarvis_core::tts::generation();
        let content = jarvis_core::chat::read_chat_stream(response, true, |_| {})?;
        if speak && generation == jarvis_core::tts::generation() { jarvis_core::tts::speak(&speech_text(&content)); }
        if content.is_empty() { return Err("DeepSeek вернул пустой ответ".into()); }
        Ok(ChatReply { content: with_sources(content, &source_footer), provider, model })
    } else {
        let model = state.settings.read("local_chat_model").unwrap_or_else(|| "qwen3:8b".into());
        let response = client.post("http://127.0.0.1:11434/api/chat")
            .json(&json!({"model": model, "messages": messages, "stream": true, "think": false, "keep_alive": "15m", "options": {"num_ctx": 2048, "num_predict": 700}}))
            .send().map_err(|_| "Локальный Ollama не запущен. Установи Ollama и выполни команду загрузки модели ниже.".to_string())?;
        let speak = state.settings.read("chat_speak_responses").map(|v| v != "false").unwrap_or(true);
        let generation = jarvis_core::tts::generation();
        let content = jarvis_core::chat::read_ollama_reply(response, |_| {})?;
        if speak && generation == jarvis_core::tts::generation() { jarvis_core::tts::speak(&speech_text(&content)); }
        if content.is_empty() { return Err("Локальная модель вернула пустой ответ".into()); }
        Ok(ChatReply { content: with_sources(content, &source_footer), provider, model })
    }
}

fn with_sources(content: String, footer: &str) -> String {
    if footer.is_empty() { content } else { format!("{content}\n\nИсточники (откройте для проверки):\n{footer}") }
}
