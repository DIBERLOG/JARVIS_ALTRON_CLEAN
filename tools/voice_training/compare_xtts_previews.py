"""Compare XTTS conditioning from several rights-cleared Jarvis recordings."""

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

from TTS.api import TTS


root = Path(__file__).resolve().parent
samples = root / "dataset_current" / "wavs"
model = TTS("tts_models/multilingual/multi-dataset/xtts_v2").to("cuda")
text = "Здравствуйте, сэр. Я проверил данные и готов продолжить работу."
variants = [
    ("jarvis_xtts_preview_multi3.wav", [12, 14, 19], 20, 0.55),
    ("jarvis_xtts_preview_multi4.wav", [12, 17, 18, 19], 25, 0.65),
]
for filename, clip_ids, cond_len, temperature in variants:
    refs = [str(samples / f"jarvis_ru_{clip_id:03d}.wav") for clip_id in clip_ids]
    output = root / filename
    model.tts_to_file(
        text=text,
        speaker_wav=refs,
        language="ru",
        file_path=str(output),
        gpt_cond_len=cond_len,
        gpt_cond_chunk_len=6,
        max_ref_len=20,
        temperature=temperature,
    )
    print(output)
