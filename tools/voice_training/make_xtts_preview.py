"""Standalone, personal-use XTTS-v2 voice-reference preview."""

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
speaker_wav = root / "dataset_current" / "wavs" / "jarvis_ru_012.wav"
output = root / "jarvis_xtts_preview.wav"
model = TTS("tts_models/multilingual/multi-dataset/xtts_v2").to("cuda")
model.tts_to_file(
    text="Здравствуйте, сэр. Я проверил данные и готов продолжить работу.",
    speaker_wav=str(speaker_wav),
    language="ru",
    file_path=str(output),
)
print(output)
