"""Wait for verified local weights, quantize conservatively, and render one WAV.

No cloud synthesis, no training, no modification of the production voice.
"""
import json
import struct
import subprocess
import time
from pathlib import Path

import numpy as np
import soundfile as sf

ROOT = Path(__file__).resolve().parent
TEST = ROOT / "s2_local_test"
PROJECT = ROOT.parents[1]
status = {"state": "waiting_for_download", "production_voice_changed": False}

def update(**values):
    status.update(values)
    temporary = TEST / "test_status.tmp"
    temporary.write_text(json.dumps(status, ensure_ascii=False, indent=2), encoding="utf-8")
    temporary.replace(TEST / "test_status.json")

def sidecars(model):
    """Read embedded JSONs from GGUF metadata, following the upstream recipe."""
    names, offsets, payload = [], [], None
    sizes = {0:1, 1:1, 2:2, 3:2, 4:4, 5:4, 6:4, 7:1, 10:8, 11:8, 12:8}
    with model.open("rb") as source:
        def u32(): return struct.unpack("<I", source.read(4))[0]
        def u64(): return struct.unpack("<Q", source.read(8))[0]
        def string(): return source.read(u64()).decode("utf-8")
        def skip(kind):
            if kind in sizes:
                source.seek(sizes[kind], 1)
            elif kind == 8:
                source.seek(u64(), 1)
            elif kind == 9:
                element, count = u32(), u64()
                if element in sizes:
                    source.seek(count * sizes[element], 1)
                else:
                    for _ in range(count): skip(element)
            else:
                raise RuntimeError(f"Unsupported GGUF metadata type {kind}")
        if source.read(4) != b"GGUF" or u32() not in (2, 3):
            raise RuntimeError("Invalid GGUF header")
        u64()
        count = u64()
        for _ in range(count):
            name, kind = string(), u32()
            if name == "audiocpp.embedded_files.names":
                if kind != 9 or u32() != 8: raise RuntimeError("Unexpected names type")
                names = [string() for _ in range(u64())]
            elif name == "audiocpp.embedded_files.offsets":
                if kind != 9 or u32() != 10: raise RuntimeError("Unexpected offsets type")
                offsets = [u64() for _ in range(u64())]
            elif name == "audiocpp.embedded_files.data":
                if kind != 9 or u32() != 0: raise RuntimeError("Unexpected embedded data type")
                length = u64()
                if length > 100 << 20: raise RuntimeError("Unexpected embedded data size")
                payload = source.read(length)
            else:
                skip(kind)
    if payload is None or len(offsets) < len(names):
        raise RuntimeError("GGUF embedded configuration missing")
    directory = TEST / "sidecars"
    directory.mkdir(exist_ok=True)
    result = []
    for index, name in enumerate(names):
        if name.endswith(".json"):
            if Path(name).name != name:
                raise RuntimeError("Unexpected embedded file path")
            end = offsets[index + 1] if index + 1 < len(offsets) else len(payload)
            data = payload[offsets[index]:end]
            json.loads(data)
            path = directory / name
            path.write_bytes(data)
            result.extend(["--sidecar", f"{path}={name}"])
    return result

def main():
    update()
    deadline = time.monotonic() + 6 * 3600
    while not (TEST / "download_metadata.json").exists():
        if time.monotonic() > deadline:
            raise RuntimeError("Download did not finish within six hours; originals unchanged")
        time.sleep(10)
    download = json.loads((TEST / "download_metadata.json").read_text(encoding="utf-8"))
    model = Path(download["model"]["path"])
    compressed = model.parent / "fish-audio-s2-pro-q4_k-preserved-v2.gguf"
    update(state="quantizing", original_model=str(model))
    args = [str(TEST / "runtime/audiocpp_gguf.exe"), "--input", str(model),
            "--output", str(compressed), "--type", "q4_k", "--family", "fish_audio",
            "--allow-missing-model-spec", "--root", str(TEST / "sidecars")]
    for prefix in ("model_weights/fast_", "model_weights/codebook_embeddings",
                   "model_weights/embeddings", "model_weights/norm", "codec_weights"):
        args.extend(["--keep-type", prefix + "*=orig"])
    args.extend(sidecars(model))
    if not compressed.exists():
        with (TEST / "quantization.log").open("w", encoding="utf-8") as log:
            subprocess.run(args, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=1200)
    reference = PROJECT / "resources/tts/xtts-references/jarvis_ru_012.wav"
    row = next(line for line in (ROOT / "dataset_current/metadata.csv").read_text(encoding="utf-8").splitlines()
               if line.startswith("jarvis_ru_012|"))
    transcript = row.split("|")[1]
    text = "Добрый вечер, сэр. Я на связи. Давайте спокойно разберём ваши планы и выберем самое важное."
    output = TEST / "s2_pro_q4_reference_012.wav"
    update(state="synthesizing", model=str(compressed), reference=str(reference), text=text)
    command = [str(TEST / "runtime/audiocpp_cli.exe"), "--task", "tts", "--family", "fish_audio",
               "--model", str(compressed), "--backend", "vulkan", "--device", "1",
               "--text", text, "--voice-ref", str(reference), "--reference-text", transcript,
               "--seed", "28", "--temperature", "0.7", "--top-p", "0.8", "--top-k", "30",
               "--request-option", "max_new_tokens=512", "--out", str(output), "--metrics", "--log"]
    started = time.monotonic()
    with (TEST / "synthesis.log").open("w", encoding="utf-8") as log:
        subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=900)
    audio, rate = sf.read(output)
    if not audio.size or not np.isfinite(audio).all() or np.max(np.abs(audio)) < 1e-4:
        raise RuntimeError("Invalid or silent S2 output")
    metadata = dict(text=text, reference=str(reference), reference_transcript=transcript, model=str(compressed),
                    source_model=download["model"], seed=28, temperature=.7, top_p=.8, top_k=30,
                    sample_rate=rate, duration_seconds=len(audio)/rate,
                    wall_seconds=time.monotonic()-started, postprocessing=False,
                    note="Different engine and sampling settings from XTTS; listening evaluation required")
    (TEST / "sample_metadata.json").write_text(json.dumps(metadata, ensure_ascii=False, indent=2), encoding="utf-8")
    update(state="complete", output=str(output), metadata=str(TEST / "sample_metadata.json"))

if __name__ == "__main__":
    try:
        main()
    except BaseException as error:
        update(state="failed", error=str(error))
        raise
