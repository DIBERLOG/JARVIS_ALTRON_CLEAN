# Local voice-training preparation

`prepare_dataset.ps1` creates a LJSpeech-compatible dataset from the eleven
generated MP3 files in Downloads. It excludes the unrelated 19:17 recording.

The script needs Python 3.10 and `ffmpeg` on `PATH`:

```powershell
.\prepare_dataset.ps1
```

It creates `dataset\wavs\*.wav` and `dataset\metadata.csv`. Check the clips
and their text before any training run. The current eleven clips are useful for
a pipeline smoke test only; they are far too little speech for a convincing
standalone voice model. Add many short recordings with exact transcriptions
before fine-tuning.
