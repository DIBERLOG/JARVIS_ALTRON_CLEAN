"""Sequential isolated preparation, 15-minute training, and local preview."""
import json
import subprocess
import sys
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent
experiment = ROOT / "all_recordings_experiments" / datetime.now().strftime("%Y%m%d-%H%M%S")
experiment.mkdir(parents=True, exist_ok=False)
status = dict(state="preparing", experiment=str(experiment))

def save():
    (experiment / "status.json").write_text(json.dumps(status, indent=2), encoding="utf-8")

def run(script, *args):
    subprocess.run([sys.executable, str(ROOT / script), *map(str, args)], cwd=ROOT.parents[1], check=True)

try:
    save()
    run("prepare_all_experiment.py", experiment / "dataset")
    before = set((ROOT / "xtts_timed_runs").iterdir())
    status["state"] = "training"
    save()
    run("train_xtts_15min.py", "--dataset", experiment / "dataset", "--minutes", 15,
        "--checkpoint", ROOT / "xtts_timed_runs/20261004-131203/final.pth",
        "--learning-rate", "2e-7", "--compact")
    created = set((ROOT / "xtts_timed_runs").iterdir()) - before
    if len(created) != 1:
        raise RuntimeError("Cannot identify isolated training run")
    training = created.pop()
    status.update(state="preview", training_run=str(training))
    save()
    run("preview_xtts_references.py", training)
    status["state"] = "complete"
    save()
except BaseException as error:
    status.update(state="failed", error=str(error))
    save()
    raise
