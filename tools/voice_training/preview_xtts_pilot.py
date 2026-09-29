"""Generate a separate sample from the one-epoch pilot checkpoint."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

from train_xtts_pilot import add_ffmpeg_dll_path


ROOT = Path(__file__).resolve().parent
RUNS = ROOT / "xtts_pilot_output"
REFERENCES = ROOT.parents[1] / "resources" / "tts" / "xtts-references"
CACHE = Path(os.environ["LOCALAPPDATA"]) / "tts" / "tts_models--multilingual--multi-dataset--xtts_v2"
OUTPUT = ROOT / "jarvis_xtts_pilot_1epoch.wav"
TEXT = "Здравствуйте, сэр. Я проверил данные и готов продолжить работу."


def main() -> None:
    runs = sorted(RUNS.glob("jarvis_xtts_pilot_1epoch-*"))
    if not runs:
        raise FileNotFoundError("Pilot training run is missing")
    run = runs[-1]
    checkpoint = run / "best_model.pth"
    if not checkpoint.is_file():
        raise FileNotFoundError("Pilot checkpoint is missing")
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        raise RuntimeError("ffmpeg not found")
    os.environ["TORCH_FORCE_NO_WEIGHTS_ONLY_LOAD"] = "1"
    add_ffmpeg_dll_path()

    import soundfile as sf
    import torch
    from TTS.tts.configs.xtts_config import XttsConfig
    from TTS.tts.models.xtts import Xtts

    config = XttsConfig()
    config.load_json(str(run / "config.json"))
    model = Xtts.init_from_config(config)
    model.load_checkpoint(
        config, checkpoint_path=str(checkpoint),
        vocab_path=str(CACHE / "vocab.json"),
        speaker_file_path=str(CACHE / "speakers_xtts.pth"), use_deepspeed=False,
    )
    model = model.to("cuda" if torch.cuda.is_available() else "cpu")
    references = sorted(REFERENCES.glob("jarvis_ru_*.wav"))
    if len(references) != 4:
        raise RuntimeError("Approved four-reference profile is incomplete")
    gpt_latent, speaker_embedding = model.get_conditioning_latents(
        audio_path=[str(path) for path in references],
        gpt_cond_len=25, gpt_cond_chunk_len=5, max_ref_length=20,
        sound_norm_refs=True,
    )
    torch.manual_seed(28)
    generated = model.inference(
        TEXT, "ru", gpt_latent, speaker_embedding,
        temperature=0.42, top_p=0.72, speed=0.92,
        enable_text_splitting=True,
    )
    with tempfile.TemporaryDirectory(prefix="jarvis-xtts-pilot-") as temporary:
        raw = Path(temporary) / "raw.wav"
        sf.write(raw, generated["wav"], 24000)
        subprocess.run(
            [ffmpeg, "-hide_banner", "-loglevel", "error", "-y", "-i", str(raw),
             "-af", "rubberband=pitch=0.985:tempo=1.04,equalizer=f=3200:width_type=o:width=1:g=1.2",
             "-ar", "24000", "-ac", "1", "-c:a", "pcm_s16le", str(OUTPUT)],
            check=True,
        )
    print(OUTPUT, flush=True)


if __name__ == "__main__":
    main()
