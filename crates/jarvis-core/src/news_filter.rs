//! Publisher-based exclusions for all news feeds. Article topics and language are irrelevant.
const DOMAINS: &[&str] = &[
    "kyivindependent.com", "kyivpost.com", "ukrinform.net", "ukrinform.ru",
    "ukrinform.de", "ukrinform.es", "ukrinform.fr", "ukrinform.jp",
    "suspilne.media", "unian.net", "unian.info", "korrespondent.net",
    "liga.net", "censor.net", "gordonua.com", "obozrevatel.com", "espreso.tv",
    "hromadske.radio", "ukraineworld.org", "united24media.com",
    "euromaidanpress.com", "glavred.info", "strana.news", "enovosty.com",
];
const NAMES: &[&str] = &[
    "the kyiv independent", "kyiv independent", "kyiv post", "ukrinform",
    "укринформ", "укрінформ", "unian", "униан", "уніан", "суспільне",
    "suspilne", "украинская правда", "українська правда", "ukrainska pravda",
    "європейська правда", "европейская правда", "european pravda",
    "экономическая правда", "економічна правда", "ліга.net", "liga.net",
    "цензор.нет", "censor.net", "obozrevatel", "обозреватель", "обозреватель.ua",
    "корреспондент.net", "korrespondent.net", "гордон", "gordon",
    "еспресо", "espreso", "hromadske", "громадське", "united24 media",
    "euromaidan press", "рбк-украина", "рбк-україна", "rbc-ukraine",
    "24 канал", "канал 24", "тсн", "tsn", "новое время", "nv",
    "babel", "бабель", "zn.ua", "зеркало недели", "дзеркало тижня",
];

pub fn excluded_publisher(name: &str, publisher_url: &str, article_url: &str) -> bool {
    [publisher_url, article_url].iter().any(|url| {
        url::Url::parse(url).ok().and_then(|url| url.host_str().map(str::to_owned))
            .map(|host| {
                let host = host.trim_end_matches('.').to_ascii_lowercase();
                host.ends_with(".ua") || DOMAINS.iter().any(|domain|
                    host == *domain || host.ends_with(&format!(".{domain}")))
            }).unwrap_or(false)
    }) || {
        let name = name.trim().to_lowercase();
        NAMES.iter().any(|blocked| name == *blocked)
    }
}

#[cfg(test)]
mod tests {
    use super::excluded_publisher;
    #[test]
    fn filters_domains_and_names_not_article_topics() {
        assert!(excluded_publisher("Unknown", "https://news.example.ua", "https://news.google.com/a"));
        assert!(excluded_publisher("Unknown", "https://www.kyivindependent.com", ""));
        assert!(excluded_publisher("УНИАН", "", "https://news.google.com/a"));
        assert!(!excluded_publisher("BBC Україна", "https://bbc.com", "https://bbc.com/ukraine"));
        assert!(!excluded_publisher("CNN", "https://cnn.com", "https://cnn.com/?source=unian.net"));
        assert!(!excluded_publisher("Other", "https://unian.net.example.com", ""));
    }
}
