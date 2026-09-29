"""One-epoch, isolated XTTS-v2 GPT fine-tuning feasibility test.

The transcripts are provisional ASR candidates. This script never changes the
model used by Jarvis and must not be treated as production voice training.
"""

from __future__ import annotations

import os
from pathlib import Path

import requests


ROOT = Path(__file__).resolve().parent
DATASET = ROOT / "dataset_xtts_pilot_short"
OUTPUT = ROOT / "xtts_pilot_output"
ASSETS = ROOT / "xtts_pilot_assets"
CACHE = Path(os.environ["LOCALAPPDATA"]) / "tts" / "tts_models--multilingual--multi-dataset--xtts_v2"


def download_asset(name: str) -> Path:
    ASSETS.mkdir(exist_ok=True)
    path = ASSETS / name
    if path.is_file() and path.stat().st_size > 1000:
        return path
    url = f"https://coqui.gateway.scarf.sh/hf-coqui/XTTS-v2/main/{name}"
    temporary = path.with_suffix(path.suffix + ".part")
    with requests.get(url, stream=True, timeout=120) as response:
        response.raise_for_status()
        with temporary.open("wb") as output:
            for chunk in response.iter_content(chunk_size=1 << 20):
                if chunk:
                    output.write(chunk)
    temporary.replace(path)
    return path


def add_ffmpeg_dll_path() -> None:
    packages = Path(os.environ["LOCALAPPDATA"]) / "Microsoft" / "WinGet" / "Packages"
    for bin_dir in packages.glob("BtbN.FFmpeg.GPL.Shared.9.0*/**/bin"):
        if (bin_dir / "avcodec-63.dll").is_file():
            os.add_dll_directory(str(bin_dir))
            os.environ["PATH"] = f"{bin_dir}{os.pathsep}{os.environ['PATH']}"
            break


def main() -> None:
    if not (DATASET / "metadata.csv").is_file():
        raise FileNotFoundError("Prepare the isolated pilot dataset first")
    if not all((CACHE / name).is_file() for name in ("model.pth", "vocab.json", "config.json")):
        raise FileNotFoundError("Local XTTS-v2 base model is missing")
    mel_stats = download_asset("mel_stats.pth")
    dvae = download_asset("dvae.pth")
    add_ffmpeg_dll_path()
    os.environ["TORCH_FORCE_NO_WEIGHTS_ONLY_LOAD"] = "1"
    os.environ["PYTHONUTF8"] = "1"

    import torch
    from trainer import Trainer, TrainerArgs
    from TTS.config.shared_configs import BaseDatasetConfig
    from TTS.tts.datasets import load_tts_samples
    from TTS.tts.layers.xtts.trainer.gpt_trainer import GPTArgs, GPTTrainer, GPTTrainerConfig, XttsAudioConfig

    if not torch.cuda.is_available():
        raise RuntimeError("CUDA is unavailable; the XTTS pilot requires the GPU")
    print(f"Starting isolated XTTS pilot on {torch.cuda.get_device_name(0)}", flush=True)
    dataset = BaseDatasetConfig(
        formatter="ljspeech", dataset_name="jarvis_xtts_pilot", path=str(DATASET),
        meta_file_train="metadata.csv", language="ru",
    )
    model_args = GPTArgs(
        max_conditioning_length=132300,
        min_conditioning_length=66150,
        max_wav_length=203000,
        max_text_length=200,
        mel_norm_file=str(mel_stats),
        dvae_checkpoint=str(dvae),
        xtts_checkpoint=str(CACHE / "model.pth"),
        tokenizer_file=str(CACHE / "vocab.json"),
        gpt_num_audio_tokens=1026,
        gpt_start_audio_token=1024,
        gpt_stop_audio_token=1025,
        gpt_use_masking_gt_prompt_approach=True,
        gpt_use_perceiver_resampler=True,
    )
    config = GPTTrainerConfig(
        epochs=1,
        output_path=str(OUTPUT),
        model_args=model_args,
        run_name="jarvis_xtts_pilot_1epoch",
        project_name="Jarvis XTTS pilot",
        run_description="Experimental ASR-labeled feasibility test; not the Jarvis production voice",
        dashboard_logger="tensorboard",
        audio=XttsAudioConfig(sample_rate=22050, dvae_sample_rate=22050, output_sample_rate=24000),
        batch_size=1,
        batch_group_size=0,
        eval_batch_size=1,
        num_loader_workers=0,
        eval_split_max_size=4,
        eval_split_size=0.1,
        print_step=5,
        plot_step=100,
        log_model_step=1000,
        save_step=10,
        save_n_checkpoints=1,
        save_checkpoints=True,
        print_eval=False,
        optimizer="AdamW",
        optimizer_wd_only_on_weights=True,
        optimizer_params={"betas": [0.9, 0.96], "eps": 1e-8, "weight_decay": 1e-2},
        lr=5e-6,
        lr_scheduler="MultiStepLR",
        lr_scheduler_params={"milestones": [50000, 150000, 300000], "gamma": 0.5, "last_epoch": -1},
        test_sentences=[],
    )
    model = GPTTrainer.init_from_config(config)
    train_samples, eval_samples = load_tts_samples(
        [dataset], eval_split=True,
        eval_split_max_size=config.eval_split_max_size,
        eval_split_size=config.eval_split_size,
    )
    print(f"Train clips: {len(train_samples)}; evaluation clips: {len(eval_samples)}", flush=True)
    trainer = Trainer(
        TrainerArgs(restore_path=None, skip_train_epoch=False, start_with_eval=False, grad_accum_steps=4),
        config, output_path=str(OUTPUT), model=model,
        train_samples=train_samples, eval_samples=eval_samples,
    )
    trainer.fit()
    print(f"Pilot training finished: {trainer.output_path}", flush=True)


if __name__ == "__main__":
    main()
