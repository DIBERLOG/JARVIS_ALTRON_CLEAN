"""Combine source prompts and two ASR passes into an editable review sheet."""

from __future__ import annotations

import csv
import json
from difflib import SequenceMatcher
from pathlib import Path

from transcribe_references import REFERENCES, ROOT, normalized


def load(path: Path) -> dict[str, dict]:
    return {row["file"]: row for row in (
        json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()
    )}


def main() -> None:
    whisper = load(ROOT / "transcripts_draft.jsonl")
    vosk = load(ROOT / "transcripts_vosk.jsonl")
    medium = load(ROOT / "transcripts_medium.jsonl")
    if len(whisper) != 67 or whisper.keys() != vosk.keys() or whisper.keys() != medium.keys():
        raise RuntimeError("All three ASR passes must cover the same 67 recordings")
    path = ROOT / "transcripts_review.csv"
    counts: dict[str, int] = {}
    with path.open("w", encoding="utf-8-sig", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=[
            "id", "source_file", "audio_path", "duration_seconds", "source_prompt", "whisper_small",
            "whisper_medium", "vosk_crosscheck", "candidate_text", "text_for_training",
            "review_status", "review_notes",
        ])
        writer.writeheader()
        for index, name in enumerate(sorted(whisper), 1):
            first, second, stronger = whisper[name], vosk[name], medium[name]
            prompt = first["prompt_text"] or ""
            similarity = first["prompt_asr_similarity"]
            agreement = SequenceMatcher(None, normalized(first["asr_text"]), normalized(stronger["asr_text"])).ratio()
            medium_prompt_similarity = SequenceMatcher(None, normalized(prompt), normalized(stronger["asr_text"])).ratio() if prompt else None
            prompt_words = normalized(prompt).replace("ё", "е")
            spoken_words = normalized(stronger["asr_text"]).replace("ё", "е")
            extra_speech = bool(prompt) and len(spoken_words) > len(prompt_words) * 1.15
            notes = []
            if extra_speech:
                notes.append("Исходный текст неполный или отличается от аудио")
            elif prompt and similarity is not None and similarity < 0.9:
                notes.append("Сверить числа и слова с исходным текстом")
            elif not prompt:
                notes.append("Точный исходный текст не сохранился")
            if agreement < 0.75:
                notes.append("Модели Whisper заметно расходятся")
            if first["mean_log_probability"] is None or first["mean_log_probability"] < -0.5:
                notes.append("Низкая уверенность распознавания")
            candidate = prompt if prompt and not extra_speech else stronger["asr_text"]
            priority = "сначала проверить" if extra_speech or (not prompt and any(
                x in notes for x in (
                    "Модели Whisper заметно расходятся",
                    "Низкая уверенность распознавания",
                )
            )) else "проверить"
            counts[priority] = counts.get(priority, 0) + 1
            writer.writerow({
                "id": f"jarvis_{index:03d}",
                "source_file": name,
                "audio_path": str(REFERENCES / name),
                "duration_seconds": first["duration_seconds"],
                "source_prompt": prompt,
                "whisper_small": first["asr_text"],
                "whisper_medium": stronger["asr_text"],
                "vosk_crosscheck": second["asr_text"],
                "candidate_text": candidate,
                "text_for_training": "",
                "review_status": priority,
                "review_notes": "; ".join(notes),
            })
    print(f"Saved {len(whisper)} recordings to {path}")
    print(counts)


if __name__ == "__main__":
    main()
