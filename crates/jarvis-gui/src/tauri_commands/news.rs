use chrono::{DateTime, Utc};
use quick_xml::{events::Event, name::QName, Reader};
use serde::Serialize;
use std::time::Duration;

const FEEDS: [(&str, &str); 4] = [
    ("Интерфакс", "https://www.interfax.ru/rss.asp"),
    ("Лента.ру", "https://lenta.ru/rss/news"),
    ("BBC News", "https://feeds.bbci.co.uk/russian/rss.xml"),
    ("DW", "https://rss.dw.com/xml/rss-ru-all"),
];

#[derive(Clone, Debug, Serialize)]
pub struct NewsItem {
    pub source: String,
    pub title: String,
    pub url: String,
    pub published_at: String,
}

#[tauri::command]
pub fn chat_get_news() -> Result<Vec<NewsItem>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8)).build().map_err(|e| e.to_string())?;
    let mut feeds = Vec::new();
    let mut errors = Vec::new();
    for (source, url) in FEEDS {
        match client.get(url).send().and_then(|r| r.error_for_status()).and_then(|r| r.text()) {
            Ok(xml) => match parse_feed(source, &xml) {
                Ok(feed) if feed.is_empty() => errors.push(format!("{source}: нет публикаций с датой")),
                Ok(feed) => feeds.push(feed.into_iter().take(3).collect::<Vec<_>>()),
                Err(e) => errors.push(format!("{source}: {e}")),
            },
            Err(e) => errors.push(format!("{source}: {e}")),
        }
    }
    if feeds.is_empty() {
        return Err(format!("Новостные источники недоступны: {}", errors.join("; ")));
    }
    let mut items = Vec::new();
    for rank in 0..3 {
        for feed in &feeds {
            if let Some(item) = feed.get(rank) { items.push(item.clone()); }
        }
    }
    Ok(items)
}

pub fn parse_feed(source: &str, xml: &str) -> Result<Vec<NewsItem>, String> {
    let mut reader = Reader::from_str(xml);
    let mut in_item = false;
    let mut title = String::new();
    let mut url = String::new();
    let mut published_at = String::new();
    let mut items = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) if e.name() == QName(b"item") => {
                in_item = true;
                title.clear(); url.clear(); published_at.clear();
            }
            Ok(Event::Start(e)) if in_item => {
                let name = e.name();
                if matches!(name.as_ref(), b"title" | b"link" | b"pubDate") {
                    let raw = reader.read_text(name).map_err(|e| e.to_string())?;
                    let raw = raw.trim().strip_prefix("<![CDATA[").unwrap_or(raw.trim());
                    let raw = raw.strip_suffix("]]>").unwrap_or(raw);
                    let value = quick_xml::escape::unescape(raw).map_err(|e| e.to_string())?.into_owned();
                    match name.as_ref() {
                        b"title" => title = value,
                        b"link" => url = value,
                        b"pubDate" => published_at = value,
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) if e.name() == QName(b"item") => {
                in_item = false;
                let date = DateTime::parse_from_rfc2822(&published_at)
                    .or_else(|_| DateTime::parse_from_rfc3339(&published_at));
                if let Ok(date) = date {
                    if !title.is_empty() && url.starts_with("https://") {
                        items.push(NewsItem { source: source.into(), title: title.clone(), url: url.clone(), published_at: date.with_timezone(&Utc).to_rfc3339() });
                    }
                }
                if items.len() >= 5 { break; }
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
    use super::{chat_get_news, parse_feed};
    #[test]
    fn parses_rss_with_date_and_link() {
        let xml = r#"<rss><channel><item><title>Новость &amp; факт</title><link>https://example.org/a</link><pubDate>Tue, 29 Sep 2026 12:30:00 +0300</pubDate></item></channel></rss>"#;
        let items = parse_feed("Источник", xml).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Новость & факт");
        assert_eq!(items[0].published_at, "2026-09-29T09:30:00+00:00");
    }
    #[test]
    fn removes_cdata_from_bbc_titles() {
        let xml = r#"<rss><channel><item><title><![CDATA[Заголовок BBC]]></title><link>https://bbc.com/test</link><pubDate>Tue, 29 Sep 2026 12:30:00 +0300</pubDate></item></channel></rss>"#;
        assert_eq!(parse_feed("BBC", xml).unwrap()[0].title, "Заголовок BBC");
    }

    #[test]
    #[ignore = "requires live network access"]
    fn live_news_sources() {
        let items = chat_get_news().unwrap();
        assert!(items.len() >= 4, "only {} stories", items.len());
        assert!(items.iter().all(|item| item.url.starts_with("https://")));
        assert!(items.iter().all(|item| !item.title.contains("CDATA")));
    }
}
