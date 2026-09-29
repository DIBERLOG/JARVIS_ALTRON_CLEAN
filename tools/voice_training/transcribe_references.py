"""Make a reviewable transcript inventory; never treat ASR as ground truth."""

from __future__ import annotations

import argparse
import json
import re
from difflib import SequenceMatcher
from pathlib import Path

from faster_whisper import WhisperModel

from prepare_current_dataset import CLIPS
from prepare_dataset import SAMPLES


ROOT = Path(__file__).resolve().parent
REFERENCES = ROOT.parents[1] / "resources" / "tts" / "xtts-references" / "all"
OUTPUT = ROOT / "transcripts_draft.jsonl"


def known_prompt(filename: str) -> str | None:
    candidates = CLIPS if filename.startswith("ДЖАРВИС-") else SAMPLES
    matches = [text for marker, text in candidates if marker in filename]
    if len(matches) > 1:
        raise ValueError(f"Ambiguous prompt for {filename}")
    return matches[0] if matches else None


def normalized(text: str) -> str:
    return " ".join(re.findall(r"[а-яёa-z0-9]+", text.casefold()))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", default="small")
    parser.add_argument("--device", default="cpu")
    parser.add_argument("--compute-type", default="int8")
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    files = sorted(REFERENCES.glob("*.mp3"))
    if len(files) != 67:
        raise RuntimeError(f"Expected 67 unique references, got {len(files)}")

    existing = {}
    if args.output.is_file():
        for line in args.output.read_text(encoding="utf-8").splitlines():
            row = json.loads(line)
            existing[row["file"]] = row
    model = WhisperModel(args.model, device=args.device, compute_type=args.compute_type)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("a", encoding="utf-8") as output:
        for index, path in enumerate(files, 1):
            if path.name in existing:
                print(f"[{index:02d}/{len(files)}] cached {path.name}", flush=True)
                continue
            segments, info = model.transcribe(
                str(path), language="ru", beam_size=5, vad_filter=False,
                condition_on_previous_text=False,
            )
            segments = list(segments)
            recognized = " ".join(part.text.strip() for part in segments).strip()
            prompt = known_prompt(path.name)
            comparison = SequenceMatcher(None, normalized(prompt), normalized(recognized)).ratio() if prompt else None
            row = {
                "file": path.name,
                "duration_seconds": round(info.duration, 2),
                "prompt_text": prompt,
                "asr_text": recognized,
                "prompt_asr_similarity": round(comparison, 3) if comparison is not None else None,
                "mean_log_probability": round(sum(s.avg_logprob for s in segments) / len(segments), 3) if segments else None,
                "review_status": "needs_listening",
            }
            output.write(json.dumps(row, ensure_ascii=False) + "\n")
            output.flush()
            print(f"[{index:02d}/{len(files)}] {info.duration:.1f}s {path.name}", flush=True)


if __name__ == "__main__":
    main()
