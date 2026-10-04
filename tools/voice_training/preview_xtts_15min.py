"""Compare base XTTS and a completed isolated timed run locally."""
import gc
import json
import os
import sys
from pathlib import Path
from train_xtts_pilot import CACHE, ROOT, add_ffmpeg_dll_path


def main():
    run = Path(sys.argv[1]).resolve()
    state = json.loads((run / "status.json").read_text(encoding="utf-8"))
    if state["state"] != "saved" or not (run / "final.pth").is_file():
        raise RuntimeError("Training is not saved yet")
    os.environ["TORCH_FORCE_NO_WEIGHTS_ONLY_LOAD"] = "1"
    add_ffmpeg_dll_path()
    import torch
    import soundfile as sf
    from TTS.tts.configs.xtts_config import XttsConfig
    from TTS.tts.models.xtts import Xtts
    refs = sorted((ROOT.parents[1] / "resources/tts/xtts-references").glob("jarvis_ru_*.wav"))
    if len(refs) != 4:
        raise RuntimeError("Four-reference profile missing")
    text = "Добрый вечер, сэр. Я на связи. Давайте спокойно разберём ваши планы и выберем самое важное."
    output = run / "comparison"
    output.mkdir(exist_ok=True)
    for name, folder, checkpoint in [("original", CACHE, CACHE / "model.pth"), ("trained_15min", run, run / "final.pth")]:
        config = XttsConfig()
        config.load_json(str(folder / "config.json"))
        model = Xtts.init_from_config(config)
        model.load_checkpoint(config, checkpoint_path=str(checkpoint), vocab_path=str(CACHE / "vocab.json"),
                              speaker_file_path=str(CACHE / "speakers_xtts.pth"), use_deepspeed=False)
        model.to("cuda")
        latent, speaker = model.get_conditioning_latents(audio_path=[str(p) for p in refs],
            gpt_cond_len=25, gpt_cond_chunk_len=5, max_ref_length=20, sound_norm_refs=True)
        torch.manual_seed(28)
        result = model.inference(text, "ru", latent, speaker, temperature=.42, top_p=.72,
                                 speed=.92, enable_text_splitting=True)
        sf.write(output / f"{name}.wav", result["wav"], 24000)
        del model, latent, speaker, result
        gc.collect()
        torch.cuda.empty_cache()
    (output / "metadata.json").write_text(json.dumps({"text": text, "seed": 28,
        "temperature": .42, "top_p": .72, "speed": .92, "references": [str(p) for p in refs],
        "base": str(CACHE), "trained_checkpoint": str(run / "final.pth"), "postprocessing": False}, ensure_ascii=False, indent=2), encoding="utf-8")
    print(output)


if __name__ == "__main__":
    main()
