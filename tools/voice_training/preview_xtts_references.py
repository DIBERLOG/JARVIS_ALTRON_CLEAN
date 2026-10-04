"""Local reference-only comparison; no training or production settings changes."""
import json
import os
import sys
from datetime import datetime
from pathlib import Path

from train_xtts_pilot import CACHE, ROOT, add_ffmpeg_dll_path


def main():
    run = Path(sys.argv[1]).resolve()
    status = json.loads((run / "status.json").read_text(encoding="utf-8"))
    if status["state"] != "saved" or not (run / "final.pth").is_file():
        raise RuntimeError("A saved checkpoint is required")
    os.environ["TORCH_FORCE_NO_WEIGHTS_ONLY_LOAD"] = "1"
    add_ffmpeg_dll_path()
    import numpy as np
    import soundfile as sf
    import torch
    from TTS.tts.configs.xtts_config import XttsConfig
    from TTS.tts.models.xtts import Xtts

    refs = sorted((ROOT.parents[1] / "resources/tts/xtts-references").glob("jarvis_ru_*.wav"))
    if len(refs) != 4:
        raise RuntimeError("Four-reference profile is incomplete")
    config = XttsConfig()
    config.load_json(str(run / "config.json"))
    model = Xtts.init_from_config(config)
    model.load_checkpoint(config, checkpoint_path=str(run / "final.pth"),
                          vocab_path=str(CACHE / "vocab.json"),
                          speaker_file_path=str(CACHE / "speakers_xtts.pth"), use_deepspeed=False)
    model.to("cuda")
    output = run / ("reference_comparison_" + datetime.now().strftime("%Y%m%d-%H%M%S"))
    output.mkdir(exist_ok=False)
    text = "Добрый вечер, сэр. Я на связи. Давайте спокойно разберём ваши планы и выберем самое важное."
    metadata = {"text": text, "checkpoint": str(run / "final.pth"), "seed": 28,
                "speed": 1.0, "temperature": .42, "top_p": .72,
                "postprocessing": False, "variants": []}
    for index, reference in enumerate(refs, 1):
        latent, speaker = model.get_conditioning_latents(audio_path=[str(reference)],
            gpt_cond_len=25, gpt_cond_chunk_len=5, max_ref_length=20, sound_norm_refs=True)
        torch.manual_seed(28)
        result = model.inference(text, "ru", latent, speaker, temperature=.42,
                                 top_p=.72, speed=1.0, enable_text_splitting=True)
        audio = np.asarray(result["wav"])
        if not audio.size or not np.isfinite(audio).all():
            raise RuntimeError("Invalid synthesis output")
        filename = f"variant_{index}.wav"
        sf.write(output / filename, audio, 24000, subtype="PCM_16")
        metadata["variants"].append({"file": filename, "reference": str(reference),
                                     "seconds": len(audio) / 24000})
        print(f"Saved {filename}: {len(audio) / 24000:.2f}s", flush=True)
    (output / "metadata.json").write_text(json.dumps(metadata, ensure_ascii=False, indent=2), encoding="utf-8")
    print(output, flush=True)


if __name__ == "__main__":
    main()
