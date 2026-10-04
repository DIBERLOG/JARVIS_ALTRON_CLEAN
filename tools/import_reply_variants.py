"""Import the user's October 4 recordings without modifying their audio."""
import hashlib
import json
import re
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FILES = sorted((Path.home() / 'Downloads').glob('ДЖАРВИС-2026-10-04-*.mp3'))
DEST = ROOT / 'resources/sound/command-replies/ru'
# Prefixes identify the supplied files, whose download names truncate long phrases.
GROUPS = {
 'news_open_ironic': ['Открываю новости, сэр. Постараемся'],
 'note_saved_ironic': ['Мысль записана, сэр. Память'],
 'checklist_saved_ironic': ['Чек-лист готов, сэр. Теперь'],
 'timer_start_ironic': ['Таймер запущен, сэр. Со временем'],
 'habit_marked_ironic': ['Привычка отмечена, сэр. Будущий'],
 'charge_open_ironic': ['Открываю зарядку, сэр. Героизм'],
 'reaction_reply': ['На связи, сэр', 'Слушаю вас, сэр', 'Да, сэр. Чем займёмся', 'Я здесь. Говорите', 'К вашим услугам, сэр', 'Готов, сэр. Каков план'],
 'greet_morning': ['Доброе утро'], 'greet_day': ['Добрый день'],
 'greet_evening': ['Добрый вечер', 'С возвращением', 'Рад вас слышать'],
 'reaction_ok': ['Принято', 'Хорошо, сэр. Сейчас сделаю', 'Займусь этим', 'Понял вас', 'Есть, сэр', 'Сделаю, сэр'],
 'reaction_not_found': ['Не совсем расслышал', 'Я понял только часть команды'],
 'action_cancelled': ['Отменено, сэр', 'Хорошо, сэр. Оставляем как было'],
 'confirm_again': ['Подтвердите действие', 'Проверьте данные на экране'],
 'news_open': ['Открываю вашу новостную ленту', 'Посмотрим, что произошло'],
 'news_refresh': ['Обновляю новости', 'Запрашиваю свежую ленту'],
 'news_translate': ['Перевожу выбранную новость'],
 'news_translation_done': ['Готово, сэр. Перевод уже'],
 'calendar_open': ['Открываю календарь'], 'schedule_today': ['Проверяю ваши дела'],
 'notes_open': ['Открываю заметки'], 'note_create': ['Как назовём новую заметку'],
 'note_ask_content': ['Теперь продиктуйте текст'],
 'note_ask_append_text': ['Что добавить к выбранной заметке'],
 'note_saved': ['Сохранено, сэр. Мысль'],
 'checklist_create': ['Создадим чек-лист'], 'checklist_ask_items': ['Продиктуйте пункты'],
 'checklist_saved': ['Чек-лист готов, сэр'],
 'dictation_open': ['Начинаю запись'], 'dictation_stop': ['Запись завершена'],
 'dictation_format': ['Добавляю пунктуацию'],
 'timer_ask_duration': ['На какое время запустить таймер'],
 'timer_ask_preset': ['Какой пресет выбрать'],
 'timer_start': ['Таймер запущен, сэр', 'Отсчёт начался'],
 'timer_pause': ['Таймер на паузе'], 'timer_resume': ['Продолжаю отсчёт'],
 'timer_reset': ['Таймер сброшен'], 'timer_five_minutes': ['Сэр, осталось пять минут'],
 'timer_one_minute': ['Сэр, осталась одна минута'],
 'timer_finished': ['Время вышло', 'Таймер завершён'],
 'habits_open': ['Открываю привычки'], 'habit_ask_number': ['Какую привычку отметить'],
 'habit_marked': ['Привычка отмечена, сэр'], 'habit_progress_added': ['Добавил одну отметку'],
 'habit_target_reached': ['Сегодняшняя цель уже'],
 'training_open': ['Открываю тренировочный центр'], 'training_generate': ['Подбираю тренировку'],
 'charge_open': ['Открываю зарядку'], 'profile_open': ['Открываю ваши параметры'],
 'weight_add': ['Назовите новое значение веса'], 'weight_saved': ['Измерение сохранено'],
 'stats_open': ['Показываю статистику, сэр'], 'weather': ['Проверяю погоду'],
 'weather_city': ['Для какого города'], 'weather_ask_period': ['На какой период показать погоду'],
 'weather_city_saved': ['Город изменён'], 'weather_unavailable': ['Сейчас погодный сервис'],
 'vault_open': ['Открываю хранилище'], 'vault_lock': ['Хранилище заблокировано'],
 'logs_open': ['Открываю журнал событий'], 'dialogue_start': ['Включаю разговорный режим', 'Я слушаю, сэр. О чём'],
 'dialogue_stop': ['Завершаю разговорный режим'],
 'jarvis_joke': ['Мой план на день прост', 'Я бы рассказал шутку', 'Самое сложное в списке дел', 'Искусственный интеллект не устаёт', 'У меня нет плохих привычек', 'Если мысль не записана'],
}

def normalized(value):
    return re.sub(r'[^а-яa-z0-9]', '', value.lower().replace('ё', 'е'))

def phrase(path):
    return re.sub(r'^ДЖАРВИС-\d{4}-\d{2}-\d{2}-\d{2}-\d{2}-', '', path.stem)

def main():
    DEST.mkdir(parents=True, exist_ok=True)
    used = set()
    manifest = []
    # Longest matching prefix wins: short neutral phrases must not absorb jokes.
    prefixes = sorted([(normalized(p), key) for key, values in GROUPS.items() for p in values], reverse=True, key=lambda x: len(x[0]))
    counters = {}
    for source in FILES:
        title = normalized(phrase(source))
        key = next((key for prefix, key in prefixes if title.startswith(prefix)), None)
        if key:
            # Ironic alternatives are retained in the library, not randomly used for sensitive actions.
            ironic = any(title.startswith(normalized(p)) for p in ['К вашим услугам, сэр. Кофе', 'Таймер запущен, сэр. Со временем', 'Привычка отмечена, сэр. Будущий', 'Открываю зарядку, сэр. Героизм', 'Чек-лист готов, сэр. Теперь'])
            if ironic and not key.endswith('_ironic'):
                key = None
        if key is None:
            key = 'library_' + hashlib.sha256(source.name.encode()).hexdigest()[:12]
        counters[key] = counters.get(key, 0) + 1
        count = counters[key]
        # Existing base replies are kept; new recordings are additional alternatives.
        target = DEST / (f'jarvis_joke_{100 + count}.mp3' if key == 'jarvis_joke' else f'{key}__{count}.mp3')
        shutil.copy2(source, target)
        used.add(source)
        manifest.append({'source': source.name, 'file': target.name, 'reply_id': key, 'sha256': hashlib.sha256(source.read_bytes()).hexdigest(), 'connected': not key.startswith('library_')})
    (DEST / 'variants-20261004.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding='utf-8')
    print(json.dumps({'imported': len(used), 'connected': sum(x['connected'] for x in manifest), 'library': sum(not x['connected'] for x in manifest)}, ensure_ascii=False))

if __name__ == '__main__':
    main()
