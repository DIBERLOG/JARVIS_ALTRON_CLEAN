"""Rough intelligibility check using the project's Russian Vosk model."""

import json
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import resample_poly
from vosk import KaldiRecognizer, Model, SetLogLevel


root = Path(__file__).resolve().parent
model_path = root.parents[1] / "resources" / "vosk" / "vosk-model-small-ru-0.22"
SetLogLevel(-1)
model = Model(str(model_path))
for path in [
    root / "dataset_current" / "wavs" / "jarvis_ru_012.wav",
    root / "jarvis_ru_current_preview.wav",
    root / "jarvis_ru_current_preview_after_4_epochs.wav",
    root / "jarvis_xtts_preview.wav",
    root / "jarvis_xtts_preview_multi3.wav",
    root / "jarvis_xtts_preview_multi4.wav",
    root / "jarvis_xtts_refined_calm.wav",
    root / "jarvis_xtts_refined_natural.wav",
    root / "jarvis_xtts_refined_diction.wav",
    root / "jarvis_xtts_refined_deeper.wav",
    root / "jarvis_xtts_polish_a.wav",
    root / "jarvis_xtts_polish_b.wav",
    root / "jarvis_xtts_polish_c.wav",
    root / "jarvis_xtts_polished.wav",
    root / "jarvis_xtts_integration_test.wav",
    root / "jarvis_xtts_all_recordings_test.wav",
    root / "jarvis_xtts_all_blend35_test.wav",
    root / "jarvis_xtts_all_blend15_test.wav",
    root / "jarvis_xtts_all_blend35_v3_test.wav",
    root / "jarvis_xtts_all_blend10_v3_test.wav",
    root / "jarvis_xtts_pilot_1epoch.wav",
]:
    audio, rate = sf.read(path)
    if audio.ndim == 2:
        audio = audio.mean(axis=1)
    samples = resample_poly(audio, 16000, rate)
    pcm = (np.clip(samples, -1, 1) * 32767).astype("<i2").tobytes()
    recognizer = KaldiRecognizer(model, 16000)
    recognizer.AcceptWaveform(pcm)
    print(path.name, json.dumps(json.loads(recognizer.FinalResult())["text"], ensure_ascii=True))
