"""Build a separate training set from the verified legacy clips and new prompts."""

from __future__ import annotations

import shutil
import subprocess
import wave
from pathlib import Path


ROOT = Path(__file__).resolve().parent
SOURCE = Path.home() / "Downloads"
OUTPUT = ROOT / "dataset_current"
CLIPS = [
    ("23-13-Сэр,-напоминание", "Сэр, напоминание установлено на завтра, на девять часов утра. При необходимости я изменю время"),
    ("23-14-Открываю-список", "Открываю список задач. Два пункта завершены, ещё три ожидают вашего решения"),
    ("23-16-Сэр,-я-нашёл", "Сэр, я нашёл нужную заметку. Она была создана во вторник и обновлена сегодня утром"),
    ("23-17-Не-спешите", "Не спешите, сэр. Сначала сохраним изменения, затем закроем программу и проверим результат"),
    ("23-17-«Звук-работает", "Звук работает нормально. Если ответ кажется слишком тихим, я помогу проверить громкость приложения"),
    ("23-25-«Добрый-вечер", "Добрый вечер, сэр. Я проверил напоминания: на сегодня срочных дел больше нет. Можно немного отдохнуть"),
    ("23-26-Открываю-нужную", "Открываю нужную страницу, сэр. Если это не тот сайт, уточните название — я исправлюсь"),
    ("23-27-«Сэр,-я-не-уверен", "Сэр, я не уверен в результате. Позвольте проверить данные ещё раз, прежде чем дать окончательный ответ"),
]


def main() -> None:
    ffmpeg = shutil.which("ffmpeg")
    if ffmpeg is None:
        raise RuntimeError("ffmpeg not found")
    output_wavs = OUTPUT / "wavs"
    output_wavs.mkdir(parents=True, exist_ok=True)
    metadata = (ROOT / "dataset" / "metadata.csv").read_text(encoding="utf-8").splitlines()
    for row in metadata:
        clip_id = row.split("|", 1)[0]
        shutil.copy2(ROOT / "dataset" / "wavs" / f"{clip_id}.wav", output_wavs / f"{clip_id}.wav")
    for index, (prefix, transcript) in enumerate(CLIPS, start=12):
        matches = list(SOURCE.glob(f"ДЖАРВИС-2026-09-28-{prefix}*.mp3"))
        if len(matches) != 1:
            raise RuntimeError(f"Expected one recording for {prefix}; got {len(matches)}")
        clip_id = f"jarvis_ru_{index:03d}"
        target = output_wavs / f"{clip_id}.wav"
        subprocess.run([ffmpeg, "-y", "-i", str(matches[0]), "-ac", "1", "-ar", "22050", "-c:a", "pcm_s16le", str(target)], check=True, capture_output=True)
        metadata.append(f"{clip_id}|{transcript}|{transcript}")
    (OUTPUT / "metadata.csv").write_text("\n".join(metadata) + "\n", encoding="utf-8")
    duration = 0.0
    for file in output_wavs.glob("*.wav"):
        with wave.open(str(file)) as wav:
            duration += wav.getnframes() / wav.getframerate()
    print(f"Prepared {len(metadata)} recordings, {duration:.1f} seconds at {OUTPUT}")
    print("Transcripts are from supplied prompts; listen and confirm exact spoken words before production training.")


if __name__ == "__main__":
    main()
