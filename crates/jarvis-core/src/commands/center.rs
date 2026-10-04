use super::JCommand;

// Only implemented Center actions are advertised. Parameter examples are handled by the voice bridge.
pub fn available_commands() -> Vec<JCommand> {
    serde_json::from_str(r#"[
  {
    "id": "center_open",
    "type": "center",
    "description": "Открыть личный центр",
    "phrases": {
      "ru": [
        "открой центр"
      ]
    }
  },
  {
    "id": "center_news_open",
    "type": "center",
    "description": "Открыть новости",
    "phrases": {
      "ru": [
        "открой новости",
        "открой ленту"
      ]
    }
  },
  {
    "id": "center_news_refresh",
    "type": "center",
    "description": "Обновить новостную ленту",
    "phrases": {
      "ru": [
        "обнови новости",
        "обнови ленту",
        "обновить ленту"
      ]
    }
  },
  {
    "id": "center_news_translate",
    "type": "center",
    "description": "Перевести новость по номеру",
    "phrases": {
      "ru": [
        "переведи вторую новость",
        "переведи новость"
      ]
    }
  },
  {
    "id": "center_calendar",
    "type": "center",
    "description": "Открыть календарь",
    "phrases": {
      "ru": [
        "открой календарь"
      ]
    }
  },
  {
    "id": "center_schedule",
    "type": "center",
    "description": "Планы на сегодня",
    "phrases": {
      "ru": [
        "расписание на сегодня"
      ]
    }
  },
  {
    "id": "center_notes",
    "type": "center",
    "description": "Открыть заметки",
    "phrases": {
      "ru": [
        "открой заметки"
      ]
    }
  },
  {
    "id": "center_note_create",
    "type": "center",
    "description": "Создать заметку с диктовкой",
    "phrases": {
      "ru": [
        "создай заметку"
      ]
    }
  },
  {
    "id": "center_note_append",
    "type": "center",
    "description": "Дополнить заметку",
    "phrases": {
      "ru": [
        "дополни заметку"
      ]
    }
  },
  {
    "id": "center_checklist",
    "type": "center",
    "description": "Создать чек-лист",
    "phrases": {
      "ru": [
        "создай чек-лист"
      ]
    }
  },
  {
    "id": "center_voice_text",
    "type": "center",
    "description": "Открыть голос в текст",
    "phrases": {
      "ru": [
        "открой голос в текст"
      ]
    }
  },
  {
    "id": "center_timer_open",
    "type": "center",
    "description": "Открыть таймер",
    "phrases": {
      "ru": [
        "открой таймер"
      ]
    }
  },
  {
    "id": "center_timer_start",
    "type": "center",
    "description": "Запустить таймер: время или пресет",
    "phrases": {
      "ru": [
        "запусти таймер на десять минут",
        "запусти таймер учёба",
        "запусти пресет таймера"
      ]
    }
  },
  {
    "id": "center_timer_pause",
    "type": "center",
    "description": "Приостановить таймер",
    "phrases": {
      "ru": [
        "приостанови таймер"
      ]
    }
  },
  {
    "id": "center_timer_resume",
    "type": "center",
    "description": "Продолжить таймер",
    "phrases": {
      "ru": [
        "продолжи таймер"
      ]
    }
  },
  {
    "id": "center_timer_reset",
    "type": "center",
    "description": "Сбросить таймер",
    "phrases": {
      "ru": [
        "сбрось таймер"
      ]
    }
  },
  {
    "id": "center_reminders",
    "type": "center",
    "description": "Открыть напоминания",
    "phrases": {
      "ru": [
        "открой напоминания"
      ]
    }
  },
  {
    "id": "center_reminder_create",
    "type": "center",
    "description": "Создать напоминание",
    "phrases": {
      "ru": [
        "создай напоминание"
      ]
    }
  },
  {
    "id": "center_birthdays",
    "type": "center",
    "description": "Открыть дни рождения",
    "phrases": {
      "ru": [
        "открой дни рождения"
      ]
    }
  },
  {
    "id": "center_birthday_create",
    "type": "center",
    "description": "Добавить день рождения",
    "phrases": {
      "ru": [
        "добавь день рождения"
      ]
    }
  },
  {
    "id": "center_weather",
    "type": "center",
    "description": "Погода на 7 / 30 / 60 дней",
    "phrases": {
      "ru": [
        "погода на неделю",
        "погода на месяц",
        "погода на два месяца"
      ]
    }
  },
  {
    "id": "center_city",
    "type": "center",
    "description": "Изменить город прогноза",
    "phrases": {
      "ru": [
        "измени город для прогноза"
      ]
    }
  },
  {
    "id": "center_habits",
    "type": "center",
    "description": "Открыть привычки",
    "phrases": {
      "ru": [
        "открой привычки"
      ]
    }
  },
  {
    "id": "center_habit_mark",
    "type": "center",
    "description": "Отметить привычку по номеру",
    "phrases": {
      "ru": [
        "отметь привычку",
        "отметь вторую привычку"
      ]
    }
  },
  {
    "id": "center_training",
    "type": "center",
    "description": "Открыть тренировки",
    "phrases": {
      "ru": [
        "открой тренировки"
      ]
    }
  },
  {
    "id": "center_charge",
    "type": "center",
    "description": "Открыть зарядку",
    "phrases": {
      "ru": [
        "открой зарядку"
      ]
    }
  },
  {
    "id": "center_profile",
    "type": "center",
    "description": "Открыть мои параметры",
    "phrases": {
      "ru": [
        "открой мои параметры"
      ]
    }
  },
  {
    "id": "center_weight",
    "type": "center",
    "description": "Записать измерение веса",
    "phrases": {
      "ru": [
        "запиши вес"
      ]
    }
  },
  {
    "id": "center_stats",
    "type": "center",
    "description": "Открыть статистику тренировок",
    "phrases": {
      "ru": [
        "открой статистику"
      ]
    }
  },
  {
    "id": "center_vault",
    "type": "center",
    "description": "Открыть хранилище паролей",
    "phrases": {
      "ru": [
        "открой хранилище",
        "открой пароли"
      ]
    }
  },
  {
    "id": "center_history",
    "type": "center",
    "description": "Открыть историю запросов",
    "phrases": {
      "ru": [
        "открой историю запросов",
        "открой живые логи"
      ]
    }
  },
  {
    "id": "center_speech_stop",
    "type": "center",
    "description": "Остановить озвучку",
    "phrases": {
      "ru": [
        "останови озвучку"
      ]
    }
  }
]"#).expect("valid built-in Center command catalog")
}
pub fn matches_phrase(text: &str) -> bool {
    let normalized = text.to_lowercase().replace('ё', "е");
    available_commands().iter().any(|command|command.get_all_phrases().iter().any(|phrase|phrase.replace('ё',"е")==normalized))
}
#[cfg(test)]
mod tests {
    #[test]
    fn catalog_has_unique_ids_and_refresh_alias() {
        let commands=super::available_commands();
        let ids:std::collections::HashSet<_>=commands.iter().map(|c|&c.id).collect();
        assert_eq!(commands.len(),ids.len());
        assert!(super::matches_phrase("обнови ленту"));
        assert!(!super::matches_phrase("открой браузер"));
    }
}

