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

## Transcript review for all current recordings

`transcripts_review.csv` covers the 67 unique recordings used by the XTTS
reference voice (about 11 minutes 38 seconds). It includes the original prompt
where one was saved (19 clips), two Whisper drafts, an independent Vosk check,
and a proposed full transcript. All 67 rows remain **unverified** because ASR
agreement cannot prove the exact spoken words. The eight rows marked
`сначала проверить` are particularly important: the old prompt stops before
the audio ends. Do not train from `dataset_current/metadata.csv` as-is; it has
truncated labels for those clips.

Listen to each source MP3 in `../../resources/tts/xtts-references/all/`, copy
the exact spoken text into `text_for_training`, and only then change its
`review_status` to `утверждено`. Leave unintelligible or noisy clips out of a
training set rather than guessing their words. The model files and cache are
local and are not included in the repository.

## Isolated XTTS feasibility pilot

`prepare_xtts_pilot.py` selected 26 clips shorter than nine seconds where two
Whisper passes agreed closely. Their transcripts are still provisional. The
one-epoch `train_xtts_pilot.py` run on 2026-09-29 completed with batch size 1
on the local RTX 5060; `preview_xtts_pilot.py` generated
`jarvis_xtts_pilot_1epoch.wav`. The checkpoint is under `xtts_pilot_output/`
and is **not** connected to the Jarvis voice setting. Judge it by listening,
not only by the training loss or ASR result. The dataset, auxiliary model
assets, and multi-gigabyte checkpoints are ignored by Git.
