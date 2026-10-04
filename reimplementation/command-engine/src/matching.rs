#[derive(Clone, Debug)]
pub struct Candidate {
    pub key: usize,
    pub id: String,
    pub phrase: String,
    pub allow_fuzzy: bool,
}

#[derive(Debug, PartialEq)]
pub enum Selection {
    Found(usize),
    Ambiguous(Vec<usize>),
    Missing,
}

pub fn normalize(text: &str) -> String {
    text.to_lowercase()
        .replace('ё', "е")
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || matches!(ch, '{' | '}') {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn request(text: &str) -> String {
    let normalized = normalize(text);
    let mut words: Vec<_> = normalized.split_whitespace().collect();
    while words
        .first()
        .is_some_and(|word| matches!(*word, "джарвис" | "jarvis" | "пожалуйста"))
    {
        words.remove(0);
    }
    while words
        .last()
        .is_some_and(|word| matches!(*word, "джарвис" | "jarvis" | "пожалуйста"))
    {
        words.pop();
    }
    words.join(" ")
}

pub fn is_negated(text: &str) -> bool {
    let text = normalize(text);
    text.split_whitespace()
        .any(|word| matches!(word, "не" | "нет" | "not" | "never" | "don"))
}

fn opposing_verbs(input: &str, alias: &str) -> bool {
    fn polarity(text: &str) -> i8 {
        let mut result = 0;
        for word in text.split_whitespace() {
            if [
                "откр",
                "включ",
                "запуст",
                "начни",
                "open",
                "start",
                "enable",
            ]
            .iter()
            .any(|prefix| word.starts_with(prefix))
            {
                result |= 1;
            }
            if [
                "закр",
                "выключ",
                "останов",
                "заверш",
                "close",
                "stop",
                "disable",
            ]
            .iter()
            .any(|prefix| word.starts_with(prefix))
            {
                result |= 2;
            }
        }
        result
    }
    matches!((polarity(input), polarity(alias)), (1, 2) | (2, 1))
}

pub fn template_matches(spoken: &str, pattern: &str) -> bool {
    let spoken = normalize(spoken);
    let pattern = normalize(pattern);
    let Some(start) = pattern.find('{') else {
        return false;
    };
    let Some(end) = pattern[start..].find('}').map(|offset| offset + start) else {
        return false;
    };
    if end == start + 1 || pattern[end + 1..].contains('{') {
        return false;
    }
    let before = &pattern[..start];
    let after = &pattern[end + 1..];
    spoken.starts_with(before)
        && spoken.ends_with(after)
        && spoken.len() > before.len() + after.len()
        && !spoken[before.len()..spoken.len() - after.len()]
            .trim()
            .is_empty()
}

fn unique<'a>(matches: impl Iterator<Item = &'a Candidate>) -> Selection {
    let mut ids = Vec::new();
    let mut keys = Vec::new();
    for candidate in matches {
        if !ids.contains(&candidate.id) {
            ids.push(candidate.id.clone());
            keys.push(candidate.key);
        }
    }
    match keys.len() {
        0 => Selection::Missing,
        1 => Selection::Found(keys[0]),
        _ => Selection::Ambiguous(keys),
    }
}

/// Exact aliases take precedence over parameter templates and approximate input.
pub fn select(text: &str, candidates: &[Candidate], fuzzy: bool) -> Selection {
    let text = request(text);
    if text.is_empty() {
        return Selection::Missing;
    }
    let exact = unique(
        candidates
            .iter()
            .filter(|entry| request(&entry.phrase) == text),
    );
    if exact != Selection::Missing {
        return exact;
    }
    // Never transform a negated instruction into a positive desktop action.
    if is_negated(&text) {
        return Selection::Missing;
    }
    let templated = unique(
        candidates
            .iter()
            .filter(|entry| template_matches(&text, &entry.phrase)),
    );
    if templated != Selection::Missing {
        return templated;
    }
    if !fuzzy || text.chars().count() > 256 {
        return Selection::Missing;
    }
    let mut scores: Vec<_> = candidates
        .iter()
        .filter(|entry| entry.allow_fuzzy && !entry.phrase.contains('{'))
        .filter(|entry| !opposing_verbs(&text, &request(&entry.phrase)))
        .map(|entry| (similarity(&text, &request(&entry.phrase)), entry))
        .filter(|(score, _)| *score >= 0.80)
        .collect();
    scores.sort_by(|a, b| b.0.total_cmp(&a.0));
    let Some((best, _)) = scores.first() else {
        return Selection::Missing;
    };
    unique(
        scores
            .iter()
            .filter(|(score, _)| best - score < 0.035)
            .map(|(_, entry)| *entry),
    )
}

fn similarity(a: &str, b: &str) -> f64 {
    let left: Vec<_> = a.chars().collect();
    let right: Vec<_> = b.chars().collect();
    if left.is_empty() || right.is_empty() {
        return 0.;
    }
    let mut row: Vec<_> = (0..=right.len()).collect();
    for (i, l) in left.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, r) in right.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (above + 1)
                .min(row[j] + 1)
                .min(diagonal + usize::from(l != r));
            diagonal = above;
        }
    }
    1. - row[right.len()] as f64 / left.len().max(right.len()) as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    fn candidate(key: usize, phrase: &str) -> Candidate {
        Candidate {
            key,
            id: format!("cmd-{key}"),
            phrase: phrase.into(),
            allow_fuzzy: true,
        }
    }
    #[test]
    fn full_unicode_and_wake_word_are_normalized() {
        assert_eq!(
            select(
                "Джарвис, открой счётчик!",
                &[candidate(1, "открой счетчик")],
                true
            ),
            Selection::Found(1)
        );
    }
    #[test]
    fn names_and_suffixes_are_supported() {
        assert!(template_matches(
            "погода в москве сегодня",
            "погода в {city} сегодня"
        ));
        assert!(!template_matches(
            "погода в сегодня",
            "погода в {city} сегодня"
        ));
        assert!(!template_matches("погода", "погода в {city}"));
    }
    #[test]
    fn overlapping_aliases_require_clarification() {
        assert_eq!(
            select(
                "покажи новости",
                &[
                    candidate(1, "покажи новости"),
                    candidate(2, "покажи новости")
                ],
                true
            ),
            Selection::Ambiguous(vec![1, 2])
        );
    }
    #[test]
    fn slight_recognition_error_is_allowed_but_negation_is_not() {
        let entries = [candidate(1, "открой калькулятор")];
        assert_eq!(
            select("открой калкулятор", &entries, true),
            Selection::Found(1)
        );
        assert_eq!(
            select("не открой калькулятор", &entries, true),
            Selection::Missing
        );
    }
    #[test]
    fn sensitive_actions_disable_approximate_selection() {
        let mut entry = candidate(1, "перезагрузи компьютер");
        entry.allow_fuzzy = false;
        assert_eq!(
            select("перезагрузи компютер", &[entry], true),
            Selection::Missing
        );
    }
    #[test]
    fn close_is_never_guessed_as_open() {
        assert_eq!(
            select("закрой браузер", &[candidate(1, "открой браузер")], true),
            Selection::Missing
        );
        assert_eq!(
            select("don't open browser", &[candidate(1, "open browser")], true),
            Selection::Missing
        );
    }
}
