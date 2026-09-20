"""One-epoch GPU smoke test for the prepared Russian voice dataset.

This is intentionally not a production-quality model: eleven long clips are
only enough to verify that the data, tokenizer and CUDA training pipeline work.
"""

from __future__ import annotations

import os
from pathlib import Path

os.environ.setdefault("PYTHONUTF8", "1")

# TorchCodec needs the FFmpeg shared DLLs to be discoverable by this Python
# process on Windows. The static FFmpeg build used for dataset conversion is
# not enough, so prefer the separately installed shared build.
_winget_root = Path(os.environ["LOCALAPPDATA"]) / "Microsoft" / "WinGet" / "Packages"
for _bin_dir in _winget_root.glob("BtbN.FFmpeg.GPL.Shared.9.0*/**/bin"):
    if (_bin_dir / "avcodec-63.dll").is_file():
        os.add_dll_directory(str(_bin_dir))
        os.environ["PATH"] = f"{_bin_dir}{os.pathsep}{os.environ['PATH']}"
        break

import torch
from trainer import Trainer, TrainerArgs
from TTS.tts.configs.vits_config import VitsConfig
from TTS.tts.configs.shared_configs import BaseDatasetConfig, CharactersConfig
from TTS.tts.datasets import load_tts_samples
from TTS.tts.models.vits import Vits
from TTS.tts.utils.text.tokenizer import TTSTokenizer
from TTS.utils.audio import AudioProcessor


ROOT = Path(__file__).resolve().parent
DATASET = ROOT / "dataset"
OUTPUT = ROOT / "smoke_output"


def main() -> None:
    if not torch.cuda.is_available():
        raise RuntimeError("CUDA is not available to PyTorch.")
    if not (DATASET / "metadata.csv").is_file():
        raise FileNotFoundError("Dataset is missing. Run prepare_dataset.ps1 first.")

    # All letters used by the prepared Russian transcript, plus common symbols.
    characters = CharactersConfig(
        pad="_",
        eos="~",
        bos="^",
        blank="@",
        characters=(
            "абвгдеёжзийклмнопрстуфхцчшщъыьэюя"
            "АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ"
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
        ),
        punctuations=" !'(),-.:;?—«»",
        is_unique=True,
        is_sorted=True,
    )
    dataset_config = BaseDatasetConfig(
        formatter="ljspeech",
        dataset_name="jarvis_ru_smoke",
        path=str(DATASET),
        meta_file_train="metadata.csv",
        language="ru",
    )
    config = VitsConfig(
        run_name="jarvis_ru_smoke_retry3",
        output_path=str(OUTPUT),
        datasets=[dataset_config],
        characters=characters,
        batch_size=1,
        eval_batch_size=1,
        num_loader_workers=0,
        num_eval_loader_workers=0,
        run_eval=False,
        print_eval=False,
        print_step=1,
        save_step=1,
        save_n_checkpoints=1,
        epochs=1,
        mixed_precision=True,
        use_grad_scaler=True,
        cudnn_benchmark=True,
        eval_split_size=0.0,
        test_sentences=[],
    )

    ap = AudioProcessor.init_from_config(config)
    tokenizer, config = TTSTokenizer.init_from_config(config)
    train_samples, _ = load_tts_samples(config.datasets, eval_split=False)
    print(f"CUDA device: {torch.cuda.get_device_name(0)}")
    print(f"Training clips: {len(train_samples)}")

    model = Vits(config, ap, tokenizer, speaker_manager=None)
    trainer = Trainer(
        TrainerArgs(),
        config,
        output_path=str(OUTPUT),
        model=model,
        train_samples=train_samples,
        eval_samples=None,
    )
    trainer.fit()


if __name__ == "__main__":
    main()
