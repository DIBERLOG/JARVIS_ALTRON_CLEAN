"""Fish 1.5.1 training wrapper: clock starts on the first batch; safe stop and final checkpoint."""
import argparse
import json
from pathlib import Path
import sys
import time

def expired(start, seconds, now):
    return start is not None and now - start >= seconds

def atomic_status(path, data, attempts=6):
    """Progress is best effort: a Windows reader lock must never abort training."""
    tmp = path.with_suffix(".tmp")
    try:
        tmp.write_text(json.dumps(data), encoding="utf-8")
        for attempt in range(attempts):
            try:
                tmp.replace(path)
                return True
            except PermissionError:
                if attempt + 1 < attempts:
                    time.sleep(0.02)
    except OSError:
        pass
    return False

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--seconds", type=int, default=3600)
    parser.add_argument("--check-config", action="store_true")
    args, overrides = parser.parse_known_args()
    if not 60 <= args.seconds <= 14400:
        raise ValueError("Training time must be 1–240 minutes")
    sys.path.insert(0, str(args.repo))
    import lightning as L
    from hydra import compose, initialize_config_dir
    from fish_speech.train import train
    from fish_speech import utils
    class Deadline(L.Callback):
        def __init__(self):
            self.started = None
        def status(self, trainer, phase):
            elapsed = time.monotonic() - self.started if self.started is not None else 0
            loss = trainer.callback_metrics.get("train/loss")
            atomic_status(args.run / "status.json", {"phase": phase, "elapsed": round(elapsed),
                        "remaining": max(0, args.seconds-round(elapsed)), "limit": args.seconds,
                        "step": trainer.global_step, "loss": float(loss) if loss is not None else None})
        def on_train_batch_start(self, trainer, module, batch, batch_idx):
            if self.started is None: self.started = time.monotonic()
            self.status(trainer, "training")
        def on_train_batch_end(self, trainer, module, outputs, batch, batch_idx):
            if expired(self.started, args.seconds, time.monotonic()) or (args.run / "stop.request").exists():
                trainer.should_stop = True
            self.status(trainer, "saving" if trainer.should_stop else "training")
        def on_train_end(self, trainer, module):
            destination = args.run / "checkpoints/studio-final.ckpt"
            destination.parent.mkdir(parents=True, exist_ok=True)
            trainer.save_checkpoint(str(destination))
            self.status(trainer, "saved")
    original = utils.instantiate_callbacks
    utils.instantiate_callbacks = lambda cfg: [*original(cfg), Deadline()]
    with initialize_config_dir(version_base="1.3", config_dir=str(args.repo.resolve() / "fish_speech/configs")):
        cfg = compose(config_name="text2semantic_finetune", overrides=[arg for arg in overrides if arg != "--"])
        if args.check_config:
            print("CONFIG_OK", cfg.data.batch_size, cfg.trainer.strategy, cfg.callbacks.model_checkpoint.save_last)
            return
        train(cfg)

if __name__ == "__main__":
    main()
