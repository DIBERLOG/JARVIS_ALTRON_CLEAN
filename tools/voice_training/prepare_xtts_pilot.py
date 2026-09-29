"""Build an isolated, provisional XTTS pilot set; never overwrite approved data."""

from __future__ import annotations

import argparse
import csv
import json
import subprocess
from difflib import SequenceMatcher
from pathlib import Path

from transcribe_references import REFERENCES, ROOT, normalized


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--max-duration", type=float, default=11)
    parser.add_argument("--output", type=Path, default=ROOT / "dataset_xtts_pilot")
    args = parser.parse_args()
    output_dir = args.output.resolve()
    if output_dir.parent != ROOT.resolve() or not output_dir.name.startswith("dataset_xtts_pilot"):
        raise ValueError("Pilot output must be a dataset_xtts_pilot* directory inside voice_training")
    if output_dir.exists():
        raise FileExistsError(f"Pilot dataset already exists: {output_dir}")
    review = list(csv.DictReader((ROOT / "transcripts_review.csv").open(encoding="utf-8-sig", newline="")))
    small = {row["file"]: row for row in (
        json.loads(line) for line in (ROOT / "transcripts_draft.jsonl").read_text(encoding="utf-8").splitlines()
    )}
    medium = {row["file"]: row for row in (
        json.loads(line) for line in (ROOT / "transcripts_medium.jsonl").read_text(encoding="utf-8").splitlines()
    )}
    selected = []
    for row in review:
        if row["review_status"] != "проверить" or float(row["duration_seconds"]) > args.max_duration:
            continue
        name = row["source_file"]
        agreement = SequenceMatcher(None, normalized(small[name]["asr_text"]), normalized(medium[name]["asr_text"])).ratio()
        if agreement < 0.9:
            continue
        text = row["candidate_text"].strip()
        if not text or len(text) > 200 or "|" in text or "\n" in text:
            continue
        selected.append((row["id"], name, text, round(agreement, 3)))
    if len(selected) < 20:
        raise RuntimeError(f"Only {len(selected)} high-agreement clips; refusing pilot preparation")

    wavs = output_dir / "wavs"
    wavs.mkdir(parents=True)
    metadata = []
    manifest = []
    for clip_id, name, text, agreement in selected:
        source = REFERENCES / name
        target = wavs / f"{clip_id}.wav"
        subprocess.run(
            ["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", str(source),
             "-ac", "1", "-ar", "22050", "-c:a", "pcm_s16le", str(target)],
            check=True,
        )
        metadata.append(f"{clip_id}|{text}|{text}")
        manifest.append({"id": clip_id, "source": name, "transcript": text, "asr_agreement": agreement})
    (output_dir / "metadata.csv").write_text("\n".join(metadata) + "\n", encoding="utf-8")
    (output_dir / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
    (output_dir / "README.txt").write_text(
        "EXPERIMENTAL ONLY. These transcripts are ASR candidates, not human-verified.\n"
        "Do not use this dataset for a final voice model. The app's current voice is untouched.\n",
        encoding="utf-8",
    )
    print(f"Prepared {len(selected)} isolated pilot clips at {output_dir}")


if __name__ == "__main__":
    main()
