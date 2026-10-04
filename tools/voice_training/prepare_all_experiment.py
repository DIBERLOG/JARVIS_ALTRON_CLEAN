"""Prepare every unique local recording without changing original audio or production voice."""
import hashlib
import json
import sys
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import resample_poly
from train_xtts_pilot import ROOT, add_ffmpeg_dll_path


def main():
    destination = Path(sys.argv[1]).resolve()
    destination.mkdir(parents=True, exist_ok=False)
    (destination / "wavs").mkdir()
    add_ffmpeg_dll_path()
    import torch
    import whisper
    model = whisper.load_model("medium", device="cuda")
    sources = {}
    for folder in (ROOT.parents[1] / "resources/tts/xtts-references/all",
                   ROOT.parents[1] / "resources/sound/command-replies/ru"):
        for path in sorted(folder.glob("*.mp3")):
            sources.setdefault(hashlib.sha256(path.read_bytes()).hexdigest(), path)
    manifest = []
    total = 0.
    for index, (digest, path) in enumerate(sources.items(), 1):
        audio, rate = sf.read(str(path), dtype="float32", always_2d=True)
        audio = audio.mean(axis=1)
        total += len(audio) / rate
        result = model.transcribe(str(path), language="ru", fp16=True, word_timestamps=True,
                                  condition_on_previous_text=False, beam_size=5, verbose=None)
        words = [w for segment in result["segments"] for w in segment.get("words", []) if w["word"].strip()]
        if not words:
            raise RuntimeError(f"No transcript for {path}; refusing to silently omit recording")
        groups, current = [], []
        for word in words:
            candidate = current + [word]
            text = "".join(w["word"] for w in candidate).strip()
            if current and (word["end"] - current[0]["start"] > 8.4 or len(text) > 185):
                groups.append(current)
                current = []
            current.append(word)
        if current:
            groups.append(current)
        for number, group in enumerate(groups):
            start = max(0., group[0]["start"] - .10)
            end = min(len(audio) / rate, group[-1]["end"] + .12)
            chunk = audio[int(start * rate):int(end * rate)]
            if len(chunk) / rate < .3 or len(chunk) / rate > 9.1:
                raise RuntimeError(f"Invalid alignment: {path}")
            text = "".join(w["word"] for w in group).strip().replace("|", " ").replace("\n", " ")
            identifier = f"all_{index:03d}_{number:02d}"
            import math
            divisor = math.gcd(rate, 22050)
            chunk = resample_poly(chunk, 22050 // divisor, rate // divisor)
            sf.write(destination / "wavs" / (identifier + ".wav"), chunk, 22050, subtype="PCM_16")
            manifest.append(dict(id=identifier, source=str(path), source_hash=digest, transcript=text,
                                 start=start, end=end, transcription="whisper-medium provisional"))
        (destination / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
        print(f"Prepared {index}/{len(sources)} recordings; {len(manifest)} fragments", flush=True)
    (destination / "summary.json").write_text(json.dumps(dict(unique_recordings=len(sources),
        source_seconds=total, fragments=len(manifest), speech_seconds=sum(r["end"]-r["start"] for r in manifest),
        transcripts_manually_verified=False), indent=2), encoding="utf-8")
    print("Dataset complete", flush=True)


if __name__ == "__main__":
    main()
