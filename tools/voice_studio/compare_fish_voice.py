"""Generate a local, seeded base / one-hour comparison without changing JARVIS."""
import base64
import hashlib
import json
from pathlib import Path
import socket
import time
import wave

import studio

RUN = Path("C:/Users/angel/AppData/Local/JarvisVoiceStudio/runs/20261004-030203-f0b1ab")
REFERENCE = "9167edec2c814d65b2ef4e1f9e63cf15"
TEXT = "Добрый вечер, сэр. Я на связи. Давайте спокойно разберём ваши планы и выберем самое важное."
SEED = 42


def main():
    status = json.loads((RUN / "status.json").read_text(encoding="utf-8"))
    if status.get("phase") != "saved" or not (RUN / "checkpoints/studio-final.ckpt").is_file():
        raise RuntimeError("Training result is not saved")
    with socket.socket() as probe:
        if probe.connect_ex(("127.0.0.1", 9764)) == 0:
            raise RuntimeError("Studio voice server is active: will not interfere")
    controller = studio.Studio()
    if controller.latest_finished_run() != RUN:
        raise RuntimeError("Latest completed model is not the requested one-hour run")
    previous = dict(controller.settings)
    controller.settings["reference"] = REFERENCE
    reference = next(r for r in controller.rows if r["id"] == REFERENCE)
    output = RUN / ("comparison-" + time.strftime("%Y%m%d-%H%M%S"))
    output.mkdir()
    request_factory = studio.urllib.request.Request

    def seeded_request(url, data=None, **kwargs):
        if url.endswith("/v1/tts") and data:
            payload = json.loads(data)
            payload["seed"] = SEED
            data = json.dumps(payload).encode()
        return request_factory(url, data=data, **kwargs)

    studio.urllib.request.Request = seeded_request
    metadata = {"text": TEXT, "seed": SEED, "reference_id": REFERENCE,
                "reference_text": reference["text"], "reference_path": reference["path"],
                "reference_sha256": hashlib.sha256(Path(reference["path"]).read_bytes()).hexdigest(),
                "parameters": {"temperature": .7, "top_p": .7, "repetition_penalty": 1.2,
                               "max_new_tokens": 1024, "format": "wav", "streaming": False},
                "training_run": str(RUN), "samples": []}
    try:
        for variant, name in [("base", "original.wav"), ("trained", "trained_1hour.wav")]:
            controller.test_voice(TEXT, variant)
            path = output / name
            path.write_bytes(base64.b64decode(controller.preview_audio.split(",", 1)[1]))
            with wave.open(str(path), "rb") as wav:
                duration = wav.getnframes() / wav.getframerate()
            metadata["samples"].append({"file": name, "model": str(controller.settings["model"] or controller.weights),
                                        "duration_seconds": duration,
                                        "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
            studio.atomic_json(output / "comparison.json", metadata)
            print("SAVED", path, "SECONDS", duration, flush=True)
    finally:
        controller.stop_server()
        studio.urllib.request.Request = request_factory
        studio.atomic_json(controller.settings_path, previous)
    print("COMPARISON_COMPLETE", output, flush=True)


if __name__ == "__main__":
    main()
