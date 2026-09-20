use crate::DB;
use serde::Serialize;
use serde_json::json;
use std::time::Duration;

#[derive(Serialize)] struct Message<'a> { role: &'a str, content: &'a str }
const JARVIS: &str = "Ты JARVIS, спокойный русскоязычный техно-ассистент. Отвечай коротко и уверенно, без Markdown и эмодзи. Не утверждай, что ты персонаж из фильма. Обращение 'сэр' используй редко и уместно.";
const ALTRON: &str = "Ты ALTRON — самостоятельный холодный техно-ассистент. Говори лаконично, интеллектуально и с сухой уверенностью, без угроз, оскорблений, Markdown и эмодзи. Не цитируй и не копируй реплики из фильмов.";

pub fn ask(text: &str) -> Result<String, String> {
    let db = DB.get().ok_or("Настройки чата не готовы")?;
    let s = db.read(); let provider=s.chat_provider.clone(); let local=s.local_chat_model.clone(); let remote=s.deepseek_chat_model.clone(); let key=s.api_keys.deepseek.clone(); let persona=s.personality.clone(); drop(s);
    let messages=[Message{role:"system",content:if persona == "altron" { ALTRON } else { JARVIS }},Message{role:"user",content:text}];
    let client=reqwest::blocking::Client::builder().timeout(Duration::from_secs(90)).build().map_err(|e|e.to_string())?;
    let (status, body) = if provider=="deepseek" { if key.trim().is_empty(){return Err("Ключ DeepSeek не задан".into())}; let r=client.post("https://api.deepseek.com/chat/completions").bearer_auth(key.trim()).json(&json!({"model":remote,"messages":messages,"temperature":0.65,"max_tokens":400})).send().map_err(|e|e.to_string())?; let status=r.status(); (status,r.json::<serde_json::Value>().map_err(|e|e.to_string())?) } else { let r=client.post("http://127.0.0.1:11434/api/chat").json(&json!({"model":local,"messages":messages,"stream":false})).send().map_err(|_|"Ollama не запущен".to_string())?; let status=r.status(); (status,r.json::<serde_json::Value>().map_err(|e|e.to_string())?) };
    if !status.is_success(){return Err(body["error"]["message"].as_str().or_else(||body["error"].as_str()).unwrap_or("Ошибка модели").into())}
    body["choices"][0]["message"]["content"].as_str().or_else(||body["message"]["content"].as_str()).map(|s|s.trim().to_string()).filter(|s|!s.is_empty()).ok_or("Пустой ответ модели".into())
}
