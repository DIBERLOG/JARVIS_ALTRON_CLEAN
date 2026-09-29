"""Speak with the locally fine-tuned Jarvis XTTS checkpoint.

The multi-gigabyte checkpoint stays in the local training folder and is never
bundled with the application or committed to Git.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

from XttsSpeak import add_ffmpeg_dll_path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--text", required=True)
    parser.add_argument("--checkpoint", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--no-play", action="store_true")
    args = parser.parse_args()
    text = args.text.strip()
    if not text:
        return 0
    if not args.checkpoint.is_file() or not args.config.is_file():
        raise FileNotFoundError("Trained Jarvis XTTS checkpoint is missing")

    cache = Path(os.environ["LOCALAPPDATA"]) / "tts" / "tts_models--multilingual--multi-dataset--xtts_v2"
    if not all((cache / name).is_file() for name in ("vocab.json", "speakers_xtts.pth")):
        raise FileNotFoundError("XTTS tokenizer or speakers file is missing from the local Coqui cache")
    references = sorted((Path(__file__).resolve().parent / "xtts-references").glob("jarvis_ru_*.wav"))
    if len(references) != 4:
        raise RuntimeError("The four approved Jarvis reference recordings are missing")
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        raise RuntimeError("ffmpeg is required for the trained Jarvis voice")

    add_ffmpeg_dll_path()
    os.environ["TORCH_FORCE_NO_WEIGHTS_ONLY_LOAD"] = "1"
    import sounddevice as sd
    import soundfile as sf
    import torch
    from TTS.tts.configs.xtts_config import XttsConfig
    from TTS.tts.models.xtts import Xtts

    config = XttsConfig()
    config.load_json(str(args.config))
    model = Xtts.init_from_config(config)
    model.load_checkpoint(
        config,
        checkpoint_path=str(args.checkpoint),
        vocab_path=str(cache / "vocab.json"),
        speaker_file_path=str(cache / "speakers_xtts.pth"),
        use_deepspeed=False,
    )
    model = model.to("cuda" if torch.cuda.is_available() else "cpu")
    gpt_latent, speaker_embedding = model.get_conditioning_latents(
        audio_path=[str(path) for path in references],
        gpt_cond_len=25, gpt_cond_chunk_len=5, max_ref_length=20,
        sound_norm_refs=True,
    )
    torch.manual_seed(28)
    generated = model.inference(
        text, "ru", gpt_latent, speaker_embedding,
        temperature=0.42, top_p=0.72, speed=0.92,
        enable_text_splitting=True,
    )
    with tempfile.TemporaryDirectory(prefix="jarvis-trained-xtts-") as temporary:
        raw = Path(temporary) / "raw.wav"
        polished = args.output or Path(temporary) / "polished.wav"
        polished.parent.mkdir(parents=True, exist_ok=True)
        sf.write(raw, generated["wav"], 24000)
        subprocess.run(
            [ffmpeg, "-hide_banner", "-loglevel", "error", "-y", "-i", str(raw),
             "-af", "rubberband=pitch=0.985:tempo=1.04,equalizer=f=3200:width_type=o:width=1:g=1.2",
             "-ar", "24000", "-ac", "1", "-c:a", "pcm_s16le", str(polished)],
            check=True,
        )
        if not args.no_play:
            audio, sample_rate = sf.read(polished, dtype="float32")
            sd.play(audio, samplerate=sample_rate)
            sd.wait()
    if args.output:
        print(args.output, flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
