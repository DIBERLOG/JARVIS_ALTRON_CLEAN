//! Replacement kernel services. No dependency on the existing JARVIS core.
//! Planning and phrase selection never execute desktop operations.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod audio_history;
pub mod catalog;
pub mod dispatch;
pub mod matching;
pub mod persistence;
pub mod process;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    OpenPanel(&'static str),
    MinimizeWindows,
    RestoreWindows,
    PauseTimer,
    ResumeTimer,
    StopSpeaking,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Ready(Action),
    Clarify(Vec<Action>),
    Unknown,
}

#[derive(Default)]
pub struct Router {
    routes: Vec<(String, Action)>,
}

fn normalize_request(text: &str) -> String {
    matching::request(text)
}

impl Router {
    pub fn register(&mut self, action: Action, aliases: &[&str]) {
        for alias in aliases {
            let normalized = normalize_request(alias);
            if !normalized.is_empty()
                && !self
                    .routes
                    .iter()
                    .any(|(phrase, item)| phrase == &normalized && item == &action)
            {
                self.routes.push((normalized, action.clone()));
            }
        }
    }

    pub fn resolve(&self, request: &str) -> Decision {
        let candidates: Vec<_> = self
            .routes
            .iter()
            .enumerate()
            .map(|(key, (phrase, action))| matching::Candidate {
                key,
                id: format!("{action:?}"),
                phrase: phrase.clone(),
                allow_fuzzy: false,
            })
            .collect();
        match matching::select(request, &candidates, false) {
            matching::Selection::Found(key) => Decision::Ready(self.routes[key].1.clone()),
            matching::Selection::Ambiguous(keys) => {
                Decision::Clarify(keys.iter().map(|key| self.routes[*key].1.clone()).collect())
            }
            matching::Selection::Missing => Decision::Unknown,
        }
    }

    pub fn desktop_defaults() -> Self {
        let mut router = Self::default();
        router.register(
            Action::MinimizeWindows,
            &[
                "сверни все окна",
                "сверни окна",
                "покажи рабочий стол",
                "сверни все",
            ],
        );
        router.register(
            Action::RestoreWindows,
            &["верни окна", "восстанови окна", "отмени сворачивание"],
        );
        router.register(
            Action::PauseTimer,
            &["поставь таймер на паузу", "приостанови таймер"],
        );
        router.register(
            Action::ResumeTimer,
            &["продолжи таймер", "возобнови таймер"],
        );
        router.register(
            Action::StopSpeaking,
            &["перестань говорить", "останови озвучку", "замолчи"],
        );
        for (panel, aliases) in [
            ("news", &["открой новости", "покажи новости"][..]),
            ("notes", &["открой заметки", "покажи заметки"][..]),
            ("calendar", &["открой календарь", "покажи календарь"][..]),
            ("training", &["открой тренировки", "покажи тренировки"][..]),
            ("settings", &["открой настройки", "покажи настройки"][..]),
        ] {
            router.register(Action::OpenPanel(panel), aliases);
        }
        router
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_windows_command_with_wake_word_and_punctuation() {
        assert_eq!(
            Router::desktop_defaults().resolve("Джарвис, сверни все окна, пожалуйста!"),
            Decision::Ready(Action::MinimizeWindows)
        );
    }

    #[test]
    fn accepts_yo_and_uppercase() {
        assert_eq!(
            Router::desktop_defaults().resolve("СВЕРНИ ВСЁ"),
            Decision::Ready(Action::MinimizeWindows)
        );
    }

    #[test]
    fn negated_request_never_runs_positive_action() {
        assert_eq!(
            Router::desktop_defaults().resolve("не сверни все окна"),
            Decision::Unknown
        );
    }

    #[test]
    fn conflicting_alias_requires_clarification() {
        let mut router = Router::default();
        router.register(Action::PauseTimer, &["таймер"]);
        router.register(Action::ResumeTimer, &["таймер"]);
        assert_eq!(
            router.resolve("таймер"),
            Decision::Clarify(vec![Action::PauseTimer, Action::ResumeTimer])
        );
    }

    #[test]
    fn duplicate_registration_is_not_ambiguous() {
        let mut router = Router::default();
        router.register(Action::StopSpeaking, &["стоп", "Стоп!"]);
        assert_eq!(
            router.resolve("стоп"),
            Decision::Ready(Action::StopSpeaking)
        );
    }

    #[test]
    fn unrelated_or_empty_text_is_not_executed() {
        let router = Router::desktop_defaults();
        for text in [
            "",
            "джарвис",
            "расскажи про рабочий стол",
            "не открывай новости",
            "открой новости и удали заметки",
        ] {
            assert_eq!(router.resolve(text), Decision::Unknown);
        }
    }

    #[test]
    fn panel_and_timer_have_distinct_actions() {
        let router = Router::desktop_defaults();
        assert_eq!(
            router.resolve("покажи заметки"),
            Decision::Ready(Action::OpenPanel("notes"))
        );
        assert_eq!(
            router.resolve("возобнови таймер"),
            Decision::Ready(Action::ResumeTimer)
        );
    }
}
