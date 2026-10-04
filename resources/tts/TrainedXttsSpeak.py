"""Speak with the locally fine-tuned Jarvis XTTS checkpoint.

The multi-gigabyte checkpoint stays in the local training folder and is never
bundled with the application or committed to Git.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from contextlib import redirect_stdout
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

from XttsSpeak import add_ffmpeg_dll_path


def main() -> int:
    # Rust sends UTF-8 JSON through pipes; Windows' locale encoding corrupts Russian.
    if hasattr(sys.stdin, "reconfigure"):
        sys.stdin.reconfigure(encoding="utf-8")
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser()
    parser.add_argument("--text")
    parser.add_argument("--server", action="store_true")
    parser.add_argument("--checkpoint", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--no-play", action="store_true")
    args = parser.parse_args()
    text = (args.text or "").strip()
    if not text and not args.server:
        return 0
    if not args.checkpoint.is_file() or not args.config.is_file():
        raise FileNotFoundError("Trained Jarvis XTTS checkpoint is missing")

    cache = Path(os.environ["LOCALAPPDATA"]) / "tts" / "tts_models--multilingual--multi-dataset--xtts_v2"
    if not all((cache / name).is_file() for name in ("vocab.json", "speakers_xtts.pth")):
        raise FileNotFoundError("XTTS tokenizer or speakers file is missing from the local Coqui cache")
    references = [Path(__file__).resolve().parent / "xtts-references" / "jarvis_ru_012.wav"]
    if not references[0].is_file():
        raise RuntimeError("The selected Jarvis reference recording is missing")
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
    with redirect_stdout(sys.stderr):
        model.load_checkpoint(
            config, checkpoint_path=str(args.checkpoint), vocab_path=str(cache / "vocab.json"),
            speaker_file_path=str(cache / "speakers_xtts.pth"), use_deepspeed=False)
    model = model.to("cuda" if torch.cuda.is_available() else "cpu")
    gpt_latent, speaker_embedding = model.get_conditioning_latents(
        audio_path=[str(path) for path in references],
        gpt_cond_len=25, gpt_cond_chunk_len=5, max_ref_length=20,
        sound_norm_refs=True,
    )
    audio_cache = Path(os.environ["LOCALAPPDATA"]) / "JarvisVoiceStudio" / "speech-cache"
    audio_cache.mkdir(parents=True, exist_ok=True)
    profile = f"{args.checkpoint.resolve()}:{args.checkpoint.stat().st_mtime_ns}:{references[0].stat().st_mtime_ns}:28:.42:.72:1.0"

    def speak(text):
        key = hashlib.sha256((profile + text).encode("utf-8")).hexdigest()
        polished = audio_cache / (key + ".wav")
        if not polished.is_file():
            torch.manual_seed(28)
            with redirect_stdout(sys.stderr):
                generated = model.inference(text, "ru", gpt_latent, speaker_embedding,
                    temperature=.42, top_p=.72, speed=1.0, enable_text_splitting=True)
            temporary = polished.with_suffix(".tmp.wav")
            sf.write(temporary, generated["wav"], 24000, subtype="PCM_16")
            temporary.replace(polished)
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(polished, args.output)
        if not args.no_play:
            audio, sample_rate = sf.read(polished, dtype="float32")
            from AudioOutput import select_output
            sd.play(audio, samplerate=sample_rate, device=select_output(sd))
            sd.wait()
    if args.server:
        print("READY", flush=True)
        for line in sys.stdin:
            try:
                text = json.loads(line)["text"].strip()
                if text:
                    speak(text)
                print("OK", flush=True)
            except Exception as error:
                print("ERROR " + str(error).replace("\n", " "), flush=True)
    else:
        speak(text)
    if args.output:
        print(args.output, flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
