# JARVIS Voice Studio · Fish Speech 1.5.1

Separate Windows application with a local webview and system-tray icon. Closing
the window hides it; use the tray menu to quit. No audio is uploaded. Model/code
downloads require internet. Existing JARVIS/XTTS settings remain unchanged.

## Files and launch

Run `dist/JARVIS Voice Studio.exe` from this project checkout. The executable
contains only the interface and controller, not the multi-gigabyte model.
An independent ML environment is under `runtime/.venv`, and the pinned official
Fish source is under `runtime/fish-speech`. Python 3.10, git, ffmpeg/ffprobe and
the Windows WebView2 runtime are needed.

Settings, reviewed recordings, preview WAV files, logs and training runs are
stored in `%LOCALAPPDATA%/JarvisVoiceStudio`. They are not committed to Git.
Existing transcription candidates are imported as **unverified synthetic**
material. Review text against the complete audio, not the truncated MP3 name.
Do not change synthetic audio's provenance to human.

## Installation and inference

Use Settings → Install environment. Installation is isolated from XTTS.
CUDA 12.8 PyTorch 2.8, or an already installed 2.11 build, is used for RTX 5060; inference and training compatibility
must be validated on the actual GPU. Download weights only after reviewing the
model's CC-BY-NC-SA-4.0 non-commercial license. If Hugging Face requests access,
accept its conditions yourself in your account and configure authentication
in the dedicated environment; no tokens are embedded in this application.

Approve a clean reference of 3–30 seconds, select it under Voice, and load the
model. The local server stays loaded at `127.0.0.1:9764`, uses an ephemeral
authentication token and is not exposed to the network. WAV previews are saved.
The studio currently previews Fish speech separately; it does not replace the
JARVIS speech engine or install a Windows startup entry automatically.

## One-click voice audition

Open **Испытать голос**, enter a phrase and click **Испытать голос**. Choose
the trained result or the original model. The studio automatically chooses a
reviewed reference, exports the latest completed training run if needed, loads
the model and shows an audio player. Later phrases reuse the resident model.
Unfinished runs and verification-only runs are never presented as trained results.
Audition is unavailable while training; it never changes JARVIS's active voice.

## Timed training session

The duration is configurable from **1 to 240 minutes**. The initial default is
60 minutes; your saved setting is preserved. The timer begins on the first training
batch, not at download, preprocessing, token extraction or model loading.
At the deadline, or after Stop, the current batch finishes and Lightning saves
`checkpoints/studio-final.ckpt`. Saving adds time beyond the timer. Periodic
checkpoints are kept every 100 steps, up to two plus the latest checkpoint.
Training can end earlier on an error; a finished timer is not a quality score.
Power loss cannot guarantee saving. GPU out-of-memory is reported, not hidden.

Minimum launch gate: ten approved clips of 1–30 seconds, with exact text. This
is a technical minimum, not enough to promise realism. Only approved clips enter
the new isolated dataset. Original files are never overwritten. Synthetic
material needs the explicit experimental checkbox and may worsen quality.
The existing CSV's ASR drafts are not automatically approved.

The 8 GB profile uses LoRA rank 8, batch size 1, length 512, accumulation 4,
bf16 and a single GPU. It is a feasibility profile, not a guarantee of fitting.
Inference is unloaded before training. There is no endless retraining loop.
Each run gets its own manifest and checkpoints; new runs do not overwrite old
ones. Export merges LoRA into a new model used only for studio preview. Return
to Base voice to compare. Runs can be found with Open results folder.

## Build and tests

UI environment: Python 3.10 with pywebview 5.4, PyInstaller 6.16.0,
pystray 0.19.5 and Pillow 11.3.0. Manrope fonts are bundled locally.
Run `build.ps1` to produce the executable. Unit tests:
`.venv/Scripts/python.exe -m unittest test_studio.py`.
UI behavior tests: install `jsdom`, `@testing-library/dom` and
`@testing-library/user-event` under `runtime/ui-tests`, then run `node test_ui.cjs`.

Official references:
- https://github.com/fishaudio/fish-speech/tree/v1.5.1
- https://github.com/fishaudio/fish-speech/blob/v1.5.1/docs/en/finetune.md
- https://github.com/fishaudio/fish-speech/blob/v1.5.1/docs/en/inference.md
- https://huggingface.co/fishaudio/fish-speech-1.5
