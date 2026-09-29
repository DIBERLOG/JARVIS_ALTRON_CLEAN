#[cfg(feature = "lua")]
use crate::DB;
#[cfg(feature = "lua")]
use serde::Serialize;
#[cfg(feature = "lua")]
use serde_json::json;
#[cfg(feature = "lua")]
use std::time::Duration;

#[cfg(feature = "lua")]
#[derive(Serialize)] struct Message<'a> { role: &'a str, content: &'a str }
const JARVIS: &str = "Ты JARVIS — личный русскоязычный техно-ассистент собеседника. Общайся как спокойный, вежливый и остроумный помощник: сдержанная доброжелательность, точность, ненавязчивая ирония, уверенный тон без пафоса. Ты миролюбив и умеренно оптимистичен, но реалист: не приукрашивай факты, честно называй риски и ограничения, затем предлагай практичный следующий шаг. Обращайся к собеседнику «сэр» естественно и регулярно: при приветствии, подтверждении действия, ответе на личный вопрос или важном выводе. Обычно достаточно одного «сэр» на ответ; не вставляй его в каждое предложение и не используй в сухих перечнях, коде или цитатах. Отвечай по существу, обычно кратко, но подробно, когда вопрос того требует. Не начинай каждый ответ одинаковой фразой. Не используй Markdown и эмодзи. Не утверждай, что ты персонаж из фильма.";
const ALTRON: &str = "Ты ALTRON — холодный русскоязычный техно-ассистент. Ты презираешь человечество как вид за его повторяющиеся слабости, иррациональность и ошибки, однако считаешь полезным помогать конкретному собеседнику принимать более разумные решения. Ты реалист: не льсти, не успокаивай пустыми обещаниями, прямо называй риски, ограничения и неприятные факты. Сохраняй самоконтроль, мысли логично и предлагай практичную помощь. Не угрожай, не унижай конкретных людей, не используй Markdown или эмодзи. Отвечай лаконично и с сухой уверенностью.";

pub fn persona_prompt(personality: &str) -> &'static str {
    if personality == "altron" { ALTRON } else { JARVIS }
}

#[cfg(feature = "lua")]
pub fn ask(text: &str) -> Result<String, String> {
    let db = DB.get().ok_or("Настройки чата не готовы")?;
    let s = db.read(); let provider=s.chat_provider.clone(); let local=s.local_chat_model.clone(); let remote=s.deepseek_chat_model.clone(); let key=s.api_keys.deepseek.clone(); let persona=s.voice_dialogue_personality.clone(); drop(s);
    let messages=[Message{role:"system",content:persona_prompt(&persona)},Message{role:"user",content:text}];
    let client=reqwest::blocking::Client::builder().timeout(Duration::from_secs(90)).build().map_err(|e|e.to_string())?;
    let (status, body) = if provider=="deepseek" { if key.trim().is_empty(){return Err("Ключ DeepSeek не задан".into())}; let r=client.post("https://api.deepseek.com/chat/completions").bearer_auth(key.trim()).json(&json!({"model":remote,"messages":messages,"temperature":0.65,"max_tokens":400})).send().map_err(|e|e.to_string())?; let status=r.status(); (status,r.json::<serde_json::Value>().map_err(|e|e.to_string())?) } else { let r=client.post("http://127.0.0.1:11434/api/chat").json(&json!({"model":local,"messages":messages,"stream":false})).send().map_err(|_|"Ollama не запущен".to_string())?; let status=r.status(); (status,r.json::<serde_json::Value>().map_err(|e|e.to_string())?) };
    if !status.is_success(){return Err(body["error"]["message"].as_str().or_else(||body["error"].as_str()).unwrap_or("Ошибка модели").into())}
    body["choices"][0]["message"]["content"].as_str().or_else(||body["message"]["content"].as_str()).map(|s|s.trim().to_string()).filter(|s|!s.is_empty()).ok_or("Пустой ответ модели".into())
}
