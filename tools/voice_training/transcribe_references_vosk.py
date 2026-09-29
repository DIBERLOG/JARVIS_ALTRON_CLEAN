"""Independent rough Vosk pass for spotting Whisper transcription conflicts."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

from vosk import KaldiRecognizer, Model, SetLogLevel


ROOT = Path(__file__).resolve().parent
REFERENCES = ROOT.parents[1] / "resources" / "tts" / "xtts-references" / "all"
MODEL = ROOT.parents[1] / "resources" / "vosk" / "vosk-model-small-ru-0.22"
OUTPUT = ROOT / "transcripts_vosk.jsonl"


def main() -> None:
    SetLogLevel(-1)
    model = Model(str(MODEL))
    existing = set()
    if OUTPUT.is_file():
        existing = {json.loads(line)["file"] for line in OUTPUT.read_text(encoding="utf-8").splitlines()}
    paths = sorted(REFERENCES.glob("*.mp3"))
    with OUTPUT.open("a", encoding="utf-8") as output:
        for index, path in enumerate(paths, 1):
            if path.name in existing:
                continue
            converted = subprocess.run(
                ["ffmpeg", "-hide_banner", "-loglevel", "error", "-i", str(path),
                 "-f", "s16le", "-ac", "1", "-ar", "16000", "pipe:1"],
                capture_output=True, check=True,
            ).stdout
            recognizer = KaldiRecognizer(model, 16000)
            chunks = []
            for offset in range(0, len(converted), 64000):
                if recognizer.AcceptWaveform(converted[offset:offset + 64000]):
                    chunks.append(json.loads(recognizer.Result())["text"])
            chunks.append(json.loads(recognizer.FinalResult())["text"])
            row = {"file": path.name, "asr_text": " ".join(x for x in chunks if x)}
            output.write(json.dumps(row, ensure_ascii=False) + "\n")
            output.flush()
            print(f"[{index:02d}/{len(paths)}] {path.name}", flush=True)


if __name__ == "__main__":
    main()
