"""Standalone XTTS prosody/tempo comparisons based on the selected four clips."""

from __future__ import annotations

import os
from pathlib import Path

os.environ["COQUI_TOS_AGREED"] = "1"
os.environ["TORCH_FORCE_NO_WEIGHTS_ONLY_LOAD"] = "1"
winget_root = Path(os.environ["LOCALAPPDATA"]) / "Microsoft" / "WinGet" / "Packages"
for bin_dir in winget_root.glob("BtbN.FFmpeg.GPL.Shared.9.0*/**/bin"):
    if (bin_dir / "avcodec-63.dll").is_file():
        os.add_dll_directory(str(bin_dir))
        os.environ["PATH"] = f"{bin_dir}{os.pathsep}{os.environ['PATH']}"
        break

import torch
from TTS.api import TTS


root = Path(__file__).resolve().parent
wavs = root / "dataset_current" / "wavs"
model = TTS("tts_models/multilingual/multi-dataset/xtts_v2").to("cuda")
config = model.synthesizer.tts_model.config
refs = [str(wavs / f"jarvis_ru_{i:03d}.wav") for i in [12, 17, 18, 19]]
base_text = "Здравствуйте, сэр. Я проверил данные и готов продолжить работу."
variants = [
    # name, text, conditioning seconds, temperature, top-p, speed
    ("jarvis_xtts_refined_calm.wav", base_text, 25, 0.42, 0.72, 0.92),
    ("jarvis_xtts_refined_natural.wav", base_text, 25, 0.60, 0.85, 0.97),
    ("jarvis_xtts_refined_diction.wav", "Здравствуйте, сэр. Я проверил данные. Готов продолжить работу.", 25, 0.48, 0.78, 0.94),
]
config.gpt_cond_len = 25
config.gpt_cond_chunk_len = 5
config.max_ref_len = 20
config.sound_norm_refs = True
for filename, text, _, temperature, top_p, speed in variants:
    torch.manual_seed(28)
    output = root / filename
    model.tts_to_file(
        text=text,
        speaker_wav=refs,
        language="ru",
        file_path=str(output),
        temperature=temperature,
        top_p=top_p,
        speed=speed,
    )
    print(output)
