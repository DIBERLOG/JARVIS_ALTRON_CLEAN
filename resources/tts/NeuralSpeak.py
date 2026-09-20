"""Play a sentence with the locally trained experimental Coqui VITS model."""

from __future__ import annotations

import argparse
import os
import tempfile
from pathlib import Path


def add_ffmpeg_dll_path() -> None:
    packages = Path(os.environ["LOCALAPPDATA"]) / "Microsoft" / "WinGet" / "Packages"
    for bin_dir in packages.glob("BtbN.FFmpeg.GPL.Shared.9.0*/**/bin"):
        if (bin_dir / "avcodec-63.dll").is_file():
            os.add_dll_directory(str(bin_dir))
            os.environ["PATH"] = f"{bin_dir}{os.pathsep}{os.environ['PATH']}"
            return


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--text", required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--no-play", action="store_true")
    args = parser.parse_args()

    add_ffmpeg_dll_path()
    from TTS.utils.synthesizer import Synthesizer

    # The test checkpoint was trained with lowercase text. Lowercasing is
    # deliberate until a longer retraining run replaces this checkpoint.
    text = args.text.strip().lower()
    if not text:
        return 0
    synthesizer = Synthesizer(
        tts_checkpoint=str(args.model),
        tts_config_path=str(args.config),
        use_cuda=True,
    )
    wav = synthesizer.tts(text)
    temporary = args.output is None
    output = args.output or Path(tempfile.gettempdir()) / "jarvis-neural-voice.wav"
    synthesizer.save_wav(wav, str(output))
    print(output, flush=True)

    if not args.no_play:
        import winsound
        winsound.PlaySound(str(output), winsound.SND_FILENAME)
    if temporary:
        output.unlink(missing_ok=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
