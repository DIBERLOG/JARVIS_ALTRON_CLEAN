"""Local no-play smoke test for resident model and repeated-response cache."""
import json
import subprocess
import sys
import time
from pathlib import Path

root = Path(__file__).resolve().parents[2]
run = root / "tools/voice_training/xtts_timed_runs/20261004-131203"
started = time.monotonic()
process = subprocess.Popen([sys.executable, str(root / "resources/tts/TrainedXttsSpeak.py"),
    "--checkpoint", str(run / "final.pth"), "--config", str(run / "config.json"),
    "--server", "--no-play"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, encoding="utf-8")
try:
    ready = process.stdout.readline().strip()
    assert ready == "READY", ready
    print("model_ready_seconds", round(time.monotonic() - started, 3), flush=True)
    for _ in range(2):
        started = time.monotonic()
        process.stdin.write(json.dumps({"text": "Здравствуйте, сэр. Я готов к работе."}, ensure_ascii=False) + "\n")
        process.stdin.flush()
        result = process.stdout.readline().strip()
        assert result == "OK", result
        print("reply_seconds", round(time.monotonic() - started, 3), flush=True)
    process.stdin.close()
    assert process.wait(timeout=30) == 0
finally:
    if process.poll() is None:
        process.kill()
        process.wait()
