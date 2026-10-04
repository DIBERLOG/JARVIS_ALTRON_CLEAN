"""Speak with the locally cached XTTS-v2 model and Jarvis voice references.

The model weights stay in the user's Coqui cache; they are not part of Jarvis.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import shutil
import subprocess
import tempfile
from pathlib import Path


def add_ffmpeg_dll_path() -> None:
    packages = Path(os.environ["LOCALAPPDATA"]) / "Microsoft" / "WinGet" / "Packages"
    for bin_dir in packages.glob("BtbN.FFmpeg.GPL.Shared.9.0*/**/bin"):
        if (bin_dir / "avcodec-63.dll").is_file():
            os.add_dll_directory(str(bin_dir))
            os.environ["PATH"] = f"{bin_dir}{os.pathsep}{os.environ['PATH']}"
            break


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--text", required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--no-play", action="store_true")
    parser.add_argument("--all-weight", type=float, default=0.10, help="Weight of the profile made from every recording")
    args = parser.parse_args()
    text = args.text.strip()
    if not text:
        return 0

    cache = Path(os.environ["LOCALAPPDATA"]) / "tts" / "tts_models--multilingual--multi-dataset--xtts_v2"
    if not all((cache / name).is_file() for name in ("model.pth", "config.json", "vocab.json")):
        raise RuntimeError("XTTS-v2 model is not installed in the local Coqui cache")
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        raise RuntimeError("ffmpeg is required for the selected Jarvis voice")

    add_ffmpeg_dll_path()
    os.environ["COQUI_TOS_AGREED"] = "1"
    os.environ["TORCH_FORCE_NO_WEIGHTS_ONLY_LOAD"] = "1"

    import numpy as np
    import sounddevice as sd
    import soundfile as sf
    import torch
    from TTS.api import TTS

    refs_dir = Path(__file__).resolve().parent / "xtts-references" / "all"
    references = sorted(refs_dir.glob("*.mp3"))
    if not references:
        raise RuntimeError("Jarvis XTTS reference recordings are missing")
    # Preserve the exact four WAV references from the approved preview for
    # intonation. All unique MP3 recordings contribute to the broader timbre.
    chosen = sorted(refs_dir.parent.glob("jarvis_ru_*.wav"))
    if len(chosen) != 4:
        raise RuntimeError("The four approved Jarvis voice references are missing")
    manifest = hashlib.sha256("\n".join(
        f"{path.name}:{path.stat().st_size}" for path in [*chosen, *references]
    ).encode()).hexdigest()

    model = TTS("tts_models/multilingual/multi-dataset/xtts_v2").to(
        "cuda" if torch.cuda.is_available() else "cpu"
    )
    voice_model = model.synthesizer.tts_model
    profile_path = cache / "jarvis_all_recordings_profile_v3.npz"
    if profile_path.is_file():
        with np.load(profile_path, allow_pickle=False) as saved:
            valid = str(saved["manifest"]) == manifest
            if valid:
                gpt_latent = torch.from_numpy(saved["gpt_latent"].copy()).to(voice_model.device)
                all_embedding = torch.from_numpy(saved["all_embedding"].copy()).to(voice_model.device)
                chosen_embedding = torch.from_numpy(saved["chosen_embedding"].copy()).to(voice_model.device)
    else:
        valid = False
    if not valid:
        print(f"Preparing Jarvis voice profile from {len(references)} recordings...", flush=True)
        _, all_embedding = voice_model.get_conditioning_latents(
            audio_path=[str(path) for path in references],
            gpt_cond_len=25,
            gpt_cond_chunk_len=5,
            max_ref_length=20,
            sound_norm_refs=True,
        )
        gpt_latent, chosen_embedding = voice_model.get_conditioning_latents(
            audio_path=[str(path) for path in chosen],
            gpt_cond_len=25,
            gpt_cond_chunk_len=5,
            max_ref_length=20,
            sound_norm_refs=True,
        )
        np.savez_compressed(
            profile_path,
            manifest=manifest,
            gpt_latent=gpt_latent.detach().cpu().numpy(),
            all_embedding=all_embedding.detach().cpu().numpy(),
            chosen_embedding=chosen_embedding.detach().cpu().numpy(),
        )
    print(f"Jarvis voice profile: {len(references)} recordings", flush=True)
    if not 0 <= args.all_weight <= 1:
        raise ValueError("--all-weight must be between 0 and 1")
    speaker_embedding = chosen_embedding * (1 - args.all_weight) + all_embedding * args.all_weight
    torch.manual_seed(28)

    with tempfile.TemporaryDirectory(prefix="jarvis-xtts-") as temporary_dir:
        raw = Path(temporary_dir) / "raw.wav"
        polished = args.output or Path(temporary_dir) / "polished.wav"
        polished.parent.mkdir(parents=True, exist_ok=True)
        generated = voice_model.inference(
            text,
            "ru",
            gpt_latent,
            speaker_embedding,
            temperature=0.42,
            top_p=0.72,
            speed=0.92,
            enable_text_splitting=True,
        )
        sf.write(raw, generated["wav"], 24000)
        subprocess.run(
            [ffmpeg, "-hide_banner", "-loglevel", "error", "-y", "-i", str(raw),
             "-af", "rubberband=pitch=0.985:tempo=1.04,equalizer=f=3200:width_type=o:width=1:g=1.2",
             "-ar", "24000", "-ac", "1", "-c:a", "pcm_s16le", str(polished)],
            check=True,
        )
        if not args.no_play:
            audio, sample_rate = sf.read(polished, dtype="float32")
            from AudioOutput import select_output
            sd.play(audio, samplerate=sample_rate, device=select_output(sd))
            sd.wait()
        if args.output:
            print(args.output, flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
