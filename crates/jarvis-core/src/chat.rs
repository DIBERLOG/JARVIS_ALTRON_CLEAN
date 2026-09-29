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
const WEB_EVIDENCE: &str = "У тебя есть доступ к свежим данным только тогда, когда они явно приложены к запросу. Используй их по теме, называй источник и дату публикации, если она известна. Не выдумывай дату, ссылку или факт. Если поиск недоступен, не утверждай, что проверил интернет.";

pub fn persona_prompt(personality: &str) -> &'static str {
    if personality == "altron" { ALTRON } else { JARVIS }
}

pub fn web_evidence_prompt() -> &'static str { WEB_EVIDENCE }

pub fn is_news_request(text: &str) -> bool {
    let text = text.to_lowercase();
    ["новост", "событи", "сводк", "news"].iter().any(|word| text.contains(word))
}

pub fn needs_live_info(text: &str) -> bool {
    let text = text.to_lowercase();
    is_news_request(&text) || ["сегодня", "сейчас", "последн", "актуальн", "свеж", "вчера", "завтра", "2026", "курс валют", "погода"].iter().any(|word| text.contains(word))
}

#[cfg(feature = "lua")]
pub fn ask(text: &str) -> Result<String, String> {
    ask_with_history(text, &[])
}

#[cfg(feature = "lua")]
pub fn ask_with_history(text: &str, history: &[(String, String)]) -> Result<String, String> {
    // The GUI saves settings in a separate process; read the latest choice for
    // every turn so changing providers does not require restarting JARVIS.
    let s = crate::db::latest_settings()
        .or_else(|| DB.get().map(|db| db.read().clone()))
        .ok_or("Настройки чата не готовы")?;
    let provider=s.chat_provider; let local=s.local_chat_model; let remote=s.deepseek_chat_model; let key=s.api_keys.deepseek; let persona=s.voice_dialogue_personality;
    let system_prompt = format!("{}\n{}", persona_prompt(&persona), web_evidence_prompt());
    let live_context = if needs_live_info(text) { Some(voice_live_context(text)?) } else { None };
    let user_text = match live_context {
        Some(context) => format!("{text}\n\nТекущая дата: {}. Свежие данные из интернета (ссылки не диктуй вслух, но назови источник и дату):\n{context}", chrono::Local::now().to_rfc3339()),
        None => text.to_string(),
    };
    let mut messages=vec![Message{role:"system",content:&system_prompt}];
    for (question, answer) in history.iter().rev().take(8).rev() {
        messages.push(Message{role:"user",content:question});
        messages.push(Message{role:"assistant",content:answer});
    }
    messages.push(Message{role:"user",content:&user_text});
    let client=reqwest::blocking::Client::builder().timeout(Duration::from_secs(90)).build().map_err(|e|e.to_string())?;
    let (status, body) = if provider=="deepseek" { if key.trim().is_empty(){return Err("Ключ DeepSeek не задан".into())}; let r=client.post("https://api.deepseek.com/chat/completions").bearer_auth(key.trim()).json(&json!({"model":remote,"messages":messages,"temperature":0.65,"max_tokens":400})).send().map_err(|e|e.to_string())?; let status=r.status(); (status,r.json::<serde_json::Value>().map_err(|e|e.to_string())?) } else { let r=client.post("http://127.0.0.1:11434/api/chat").json(&json!({"model":local,"messages":messages,"stream":false})).send().map_err(|_|"Ollama не запущен".to_string())?; let status=r.status(); (status,r.json::<serde_json::Value>().map_err(|e|e.to_string())?) };
    if !status.is_success(){return Err(body["error"]["message"].as_str().or_else(||body["error"].as_str()).unwrap_or("Ошибка модели").into())}
    body["choices"][0]["message"]["content"].as_str().or_else(||body["message"]["content"].as_str()).map(|s|s.trim().to_string()).filter(|s|!s.is_empty()).ok_or("Пустой ответ модели".into())
}

#[cfg(feature = "lua")]
fn voice_live_context(query: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(10)).build().map_err(|e| e.to_string())?;
    if is_news_request(query) {
        let feeds = [
            ("Интерфакс", "https://www.interfax.ru/rss.asp"),
            ("Лента.ру", "https://lenta.ru/rss/news"),
            ("BBC News", "https://feeds.bbci.co.uk/russian/rss.xml"),
            ("DW", "https://rss.dw.com/xml/rss-ru-all"),
        ];
        let mut lines = Vec::new();
        let mut errors = Vec::new();
        for (source, url) in feeds {
            match client.get(url).send().and_then(|r| r.error_for_status()).and_then(|r| r.text()) {
                Ok(xml) => match voice_feed_items(source, &xml) {
                    Ok(items) if !items.is_empty() => lines.extend(items.into_iter().take(2)),
                    Ok(_) => errors.push(format!("{source}: нет публикаций с датой")),
                    Err(e) => errors.push(format!("{source}: {e}")),
                },
                Err(e) => errors.push(format!("{source}: {e}")),
            }
        }
        if lines.is_empty() { return Err(format!("Новостной поиск недоступен: {}", errors.join("; "))); }
        return Ok(lines.join("\n"));
    }
    let url = reqwest::Url::parse_with_params("https://html.duckduckgo.com/html/", &[("q", query)])
        .map_err(|e| e.to_string())?;
    let html = client.get(url).header("User-Agent", "Mozilla/5.0 Jarvis/1.0")
        .send().and_then(|r| r.error_for_status()).and_then(|r| r.text())
        .map_err(|e| format!("Веб-поиск недоступен: {e}"))?;
    let mut rest = html.as_str();
    let mut lines = Vec::new();
    while lines.len() < 5 {
        let Some(marker) = rest.find("result__snippet") else { break; };
        rest = &rest[marker..];
        let Some(end_tag) = rest.find('>') else { break; };
        let link = rest[..end_tag].split("href=\"").nth(1).and_then(|s| s.split('"').next())
            .and_then(|raw| {
                let raw = raw.replace("&amp;", "&");
                let absolute = if raw.starts_with("//") { format!("https:{raw}") } else { raw };
                let parsed = reqwest::Url::parse(&absolute).ok()?;
                if parsed.host_str() == Some("duckduckgo.com") {
                    parsed.query_pairs().find(|(key, _)| key == "uddg").map(|(_, value)| value.into_owned())
                } else { Some(parsed.to_string()) }
            }).filter(|url| url.starts_with("https://") || url.starts_with("http://"));
        rest = &rest[end_tag + 1..];
        let Some(end) = rest.find("</a>") else { break; };
        let mut in_tag = false;
        let plain: String = rest[..end].chars().filter(|ch| match ch {
            '<' => { in_tag = true; false },
            '>' => { in_tag = false; false },
            _ => !in_tag,
        }).collect();
        if let Some(link) = link {
            let snippet = plain.replace("&amp;", "&").replace("&quot;", "\"").replace("&#39;", "'");
            if snippet.chars().count() > 20 { lines.push(format!("{} | Источник: {link}", snippet.split_whitespace().collect::<Vec<_>>().join(" "))); }
        }
        rest = &rest[end + 4..];
    }
    if lines.is_empty() { return Err("Веб-поиск не вернул источники".into()); }
    Ok(lines.join("\n"))
}

#[cfg(feature = "lua")]
fn voice_feed_items(source: &str, xml: &str) -> Result<Vec<String>, String> {
    use quick_xml::{events::Event, name::QName, Reader};
    let mut reader = Reader::from_str(xml);
    let (mut title, mut link, mut date) = (String::new(), String::new(), String::new());
    let (mut in_item, mut items) = (false, Vec::new());
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) if e.name() == QName(b"item") => { in_item = true; title.clear(); link.clear(); date.clear(); }
            Ok(Event::Start(e)) if in_item => {
                let name = e.name();
                if matches!(name.as_ref(), b"title" | b"link" | b"pubDate") {
                    let raw = reader.read_text(name).map_err(|e| e.to_string())?;
                    let raw = raw.trim().strip_prefix("<![CDATA[").unwrap_or(raw.trim());
                    let raw = raw.strip_suffix("]]>").unwrap_or(raw);
                    let value = quick_xml::escape::unescape(raw).map_err(|e| e.to_string())?.into_owned();
                    match name.as_ref() { b"title" => title = value, b"link" => link = value, b"pubDate" => date = value, _ => {} }
                }
            }
            Ok(Event::End(e)) if e.name() == QName(b"item") => {
                in_item = false;
                if !title.is_empty() && link.starts_with("https://") {
                    if let Ok(date) = chrono::DateTime::parse_from_rfc2822(&date).or_else(|_| chrono::DateTime::parse_from_rfc3339(&date)) {
                        items.push(format!("{source} | {} | {title} | {link}", date.to_rfc3339()));
                    }
                }
                if items.len() >= 2 { break; }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.to_string()),
            _ => {}
        }
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::{is_news_request, needs_live_info, persona_prompt, web_evidence_prompt};
    #[test]
    fn both_personalities_share_web_rules() {
        assert_ne!(persona_prompt("jarvis"), persona_prompt("altron"));
        assert!(persona_prompt("altron").contains("ALTRON"));
        assert!(web_evidence_prompt().contains("источник"));
        assert!(needs_live_info("Какие новости сегодня?"));
        assert!(is_news_request("Новости мира"));
    }
    #[cfg(feature = "lua")]
    #[test]
    #[ignore = "requires live network access"]
    fn voice_news_fetches_sources() {
        let news = super::voice_live_context("Расскажи новости сегодня").unwrap();
        assert!(news.contains("https://"));
        assert!(!news.contains("CDATA"));
    }
}
