"""Second, stronger local transcription pass using OpenAI Whisper medium."""

from __future__ import annotations

import json
from pathlib import Path

import torch
import whisper


ROOT = Path(__file__).resolve().parent
REFERENCES = ROOT.parents[1] / "resources" / "tts" / "xtts-references" / "all"
OUTPUT = ROOT / "transcripts_medium.jsonl"


def main() -> None:
    model = whisper.load_model("medium", device="cuda" if torch.cuda.is_available() else "cpu")
    existing = set()
    if OUTPUT.is_file():
        existing = {json.loads(line)["file"] for line in OUTPUT.read_text(encoding="utf-8").splitlines()}
    files = sorted(REFERENCES.glob("*.mp3"))
    with OUTPUT.open("a", encoding="utf-8") as output:
        for index, path in enumerate(files, 1):
            if path.name in existing:
                continue
            result = model.transcribe(
                str(path), language="ru", fp16=model.device.type == "cuda",
                verbose=False, condition_on_previous_text=False, beam_size=5,
            )
            row = {"file": path.name, "asr_text": result["text"].strip()}
            output.write(json.dumps(row, ensure_ascii=False) + "\n")
            output.flush()
            print(f"[{index:02d}/{len(files)}] {path.name}", flush=True)


if __name__ == "__main__":
    main()
