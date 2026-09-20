"""Create a small LJSpeech-compatible Russian dataset from the generated clips.

This script deliberately copies only the eleven clips whose text is known.  It
does not use the unrelated "Сканирование макета" recording because an audio
file without an exact transcript would harm a training run.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import wave
from pathlib import Path


SAMPLES = [
    ("Здравствуйте.-Я-готов", "Здравствуйте. Я готов помочь. Доброе утро. Как ваши дела? Добрый день. Чем я могу быть полезен? Добрый вечер. Система работает в штатном режиме. Рад вас слышать. Спасибо. Выполняю команду. Готово. Задача завершена."),
    ("К-сожалению,-я-не-понял", "К сожалению, я не понял команду. Повторите, пожалуйста, немного медленнее. Открываю браузер. Запускаю калькулятор. Открываю Discord. Запускаю Visual Studio Code. Открываю YouTube. Открываю ChatGPT. Переключаюсь в игровой режим."),
    ("Показываю-расписание", "Показываю расписание. Закрываю текущее приложение. Проверяю погоду. Сегодня ожидается переменная облачность. Температура воздуха плюс восемнадцать градусов. В Москве возможен небольшой дождь. Скорость ветра — пять метров в секунду."),
    ("Влажность-воздуха", "Влажность воздуха составляет семьдесят два процента. Сейчас девять часов тридцать минут. Сегодня двадцатое сентября две тысячи двадцать шестого года. Напомнить вам об этом через десять минут? До следующего события осталось два часа."),
    ("Уровень-громкости", "Уровень громкости установлен на пятьдесят процентов. Микрофон включён. Микрофон выключен. Соединение с сетью установлено. Нет подключения к интернету. Обнаружено новое уведомление. У вас три непрочитанных сообщения."),
    ("Сканирование-завершено", "Сканирование завершено успешно. Внимание. Обнаружена ошибка. Перезагрузка помощника начнётся через три секунды. Компьютер будет перезагружен после подтверждения. Я здесь. Как настроение? У меня всё отлично. Это интересный вопрос."),
    ("Сейчас-разберёмся", "Сейчас разберёмся. Хорошо. Продолжаю работу. Я не могу выполнить это действие. Для этого потребуется ваше подтверждение. Скажите команду после ключевого слова. Джарвис, открой браузер. Джарвис, включи музыку."),
    ("Джарвис,-какая", "Джарвис, какая сегодня погода? Джарвис, запусти игру. Джарвис, расскажи шутку. Один, два, три, четыре, пять, шесть, семь, восемь, девять, десять. Ноль целых семьдесят пять сотых."),
    ("Двести-сорок", "Двести сорок восемь тысяч девятьсот шестнадцать. Пятнадцать процентов заряда батареи. Версия программы: ноль точка один точка ноль. Адрес сервера: сто девяносто два точка сто шестьдесят восемь точка ноль точка один."),
    ("Файл-сохранён", "Файл сохранён в папке «Документы». Путь к проекту содержит английские буквы, цифры и дефисы. Тест связи: А, Б, В, Г, Д, Е, Ё, Ж, З. Ш, Щ, Ч, Ц — важные звуки русской речи. В четверг вечером шёл сильный дождь."),
    ("Быстрый-браузер", "Быстрый браузер бережно обработал большой буфер данных. Съешь ещё этих мягких французских булок, да выпей чаю. Чёткий человек часто читает чужие чертежи. Жёлтый жук жужжит возле живой изгороди."),
]


def locate_clip(source_dir: Path, prefix: str) -> Path:
    matches = list(source_dir.glob(f"Jarvis-3.0-*-{prefix}*.mp3"))
    if len(matches) != 1:
        raise FileNotFoundError(f"Expected exactly one clip starting with '{prefix}', found {len(matches)}.")
    return matches[0]


def wav_duration_seconds(path: Path) -> float:
    with wave.open(str(path), "rb") as audio:
        return audio.getnframes() / audio.getframerate()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True, help="Directory containing the generated MP3 files")
    parser.add_argument("--output", type=Path, required=True, help="Output dataset directory")
    parser.add_argument("--ffmpeg", default="ffmpeg", help="Path to ffmpeg.exe")
    args = parser.parse_args()

    ffmpeg = shutil.which(args.ffmpeg) or (Path(args.ffmpeg) if Path(args.ffmpeg).is_file() else None)
    if not ffmpeg:
        print("ffmpeg was not found. Install it, then run this script again.", file=sys.stderr)
        return 2

    wavs_dir = args.output / "wavs"
    wavs_dir.mkdir(parents=True, exist_ok=True)
    metadata = []
    total_seconds = 0.0

    for index, (prefix, text) in enumerate(SAMPLES, start=1):
        source = locate_clip(args.source, prefix)
        filename = f"jarvis_ru_{index:03d}.wav"
        target = wavs_dir / filename
        subprocess.run(
            [str(ffmpeg), "-y", "-i", str(source), "-ac", "1", "-ar", "22050", "-c:a", "pcm_s16le", str(target)],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )
        duration = wav_duration_seconds(target)
        total_seconds += duration
        # The LJSpeech formatter appends the .wav suffix itself.
        metadata.append(f"{Path(filename).stem}|{text}|{text}")
        print(f"[{index:02d}/{len(SAMPLES)}] {filename}: {duration:.1f}s")

    (args.output / "metadata.csv").write_text("\n".join(metadata) + "\n", encoding="utf-8")
    (args.output / "README.txt").write_text(
        "Prepared from generated, rights-cleared audio.\n"
        f"Clips: {len(SAMPLES)}\nDuration: {total_seconds:.1f} seconds\n"
        "Format: mono PCM WAV, 22050 Hz; metadata: LJSpeech-compatible CSV.\n"
        "Do not add clips unless their transcript exactly matches the spoken audio.\n",
        encoding="utf-8",
    )
    print(f"\nDataset prepared: {args.output}\nTotal usable speech: {total_seconds / 60:.1f} min")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
