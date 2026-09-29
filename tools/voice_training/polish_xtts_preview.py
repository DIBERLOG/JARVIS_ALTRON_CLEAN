"""Generate pronunciation/prosody candidates from the selected four references."""

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
refs = [str(wavs / f"jarvis_ru_{i:03d}.wav") for i in [12, 17, 18, 19]]
model = TTS("tts_models/multilingual/multi-dataset/xtts_v2").to("cuda")
config = model.synthesizer.tts_model.config
config.gpt_cond_len = 25
config.gpt_cond_chunk_len = 5
config.max_ref_len = 20
config.sound_norm_refs = True
variants = [
    ("jarvis_xtts_polish_a.wav", "Здравствуйте, сэр. Я проверил данные и готов продолжить работу.", 41, 0.40, 0.73, 0.98),
    ("jarvis_xtts_polish_b.wav", "Здравствуйте, сэр. Я проверил данные. И готов продолжить работу.", 28, 0.42, 0.72, 0.98),
    ("jarvis_xtts_polish_c.wav", "Здравствуйте, сэр. Я проверил данные и готов продолжить работу.", 73, 0.46, 0.78, 1.00),
]
for filename, text, seed, temperature, top_p, speed in variants:
    torch.manual_seed(seed)
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
