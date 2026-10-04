"""Isolated timed XTTS experiment. Never changes the production voice."""
import json
import argparse
import os
import shutil
import time
from datetime import datetime
from pathlib import Path

import train_xtts_pilot as pilot

ROOT = Path(__file__).resolve().parent
RUN = ROOT / "xtts_timed_runs" / datetime.now().strftime("%Y%m%d-%H%M%S")


class Deadline(Exception):
    pass


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--minutes", type=int, default=15)
    parser.add_argument("--checkpoint", type=Path)
    parser.add_argument("--learning-rate", type=float, default=1e-6)
    parser.add_argument("--seconds", type=int)
    parser.add_argument("--compact", action="store_true")
    parser.add_argument("--dataset", type=Path)
    args = parser.parse_args()
    if not 1 <= args.minutes <= 15 or not 0 < args.learning_rate <= 1e-6:
        raise ValueError("Unsafe duration or learning rate")
    if args.checkpoint and not args.checkpoint.is_file():
        raise FileNotFoundError(args.checkpoint)
    limit = args.minutes * 60
    if args.seconds is not None:
        if not 1 <= args.seconds <= 900:
            raise ValueError("Unsafe time limit")
        limit = args.seconds
    RUN.mkdir(parents=True, exist_ok=False)
    status = {"state": "preparing", "limit_seconds": limit, "base": str(pilot.CACHE), "run": str(RUN),
              "initial_checkpoint": str(args.checkpoint.resolve()) if args.checkpoint else str(pilot.CACHE / "model.pth"),
              "learning_rate": args.learning_rate}

    def update(**values):
        status.update(values)
        temporary = RUN / "status.tmp"
        temporary.write_text(json.dumps(status, ensure_ascii=False, indent=2), encoding="utf-8")
        for _ in range(10):
            try:
                temporary.replace(RUN / "status.json")
                return
            except PermissionError:
                time.sleep(.02)

    update()
    try:
        approved = json.loads((Path(os.environ["LOCALAPPDATA"]) / "JarvisVoiceStudio" / "recordings.json").read_text(encoding="utf-8"))
        allowed = {r["name"]: r for r in approved if r.get("approved") and r.get("seconds", 999) <= 9.1}
        source = ROOT / "dataset_xtts_pilot_short"
        manifest = json.loads((source / "manifest.json").read_text(encoding="utf-8"))
        selected = [r for r in manifest if r["source"] in allowed and r.get("asr_agreement", 0) >= .97
                    and r["transcript"] == allowed[r["source"]]["text"]]
        if len(selected) < 16:
            raise RuntimeError(f"Too few reliable approved clips: {len(selected)}")
        if args.dataset:
            source = args.dataset.resolve()
            selected = json.loads((source / "manifest.json").read_text(encoding="utf-8"))
            for row in selected:
                row["transcript"] = row["transcript"].replace(", Са.", ", сэр.").replace(", са.", ", сэр.")
                if "Открываю-ChatGPT" in row["source"]:
                    row["transcript"] = row["transcript"].replace("Открываю счёт GPTSA.", "Открываю ChatGPT, сэр.")
                if Path(row["source"]).name == "notes_open.mp3":
                    row["transcript"] = row["transcript"].replace("Открываю ваши заметкися,", "Открываю ваши заметки, сэр.")
            import soundfile as sf
            if len(selected) < 16:
                raise RuntimeError("Full dataset is incomplete")
            for row in selected:
                if Path(row["id"]).name != row["id"] or not row["transcript"].strip() or any(c in row["transcript"] for c in "|\r\n"):
                    raise ValueError("Invalid dataset row")
                duration = sf.info(str(source / "wavs" / (row["id"] + ".wav"))).duration
                if not .3 <= duration <= 9.1 or len(row["transcript"]) > 195:
                    raise ValueError("Dataset fragment exceeds training limits")
            update(dataset_source=str(source), unique_sources=len({r["source_hash"] for r in selected}),
                   transcript_quality="experimental local ASR; not manually verified")
        dataset = RUN / "dataset"
        (dataset / "wavs").mkdir(parents=True)
        for row in selected:
            shutil.copy2(source / "wavs" / (row["id"] + ".wav"), dataset / "wavs" / (row["id"] + ".wav"))
        (dataset / "metadata.csv").write_text("\n".join(f'{r["id"]}|{r["transcript"]}|{r["transcript"]}' for r in selected) + "\n", encoding="utf-8")
        (RUN / "dataset_manifest.json").write_text(json.dumps(selected, ensure_ascii=False, indent=2), encoding="utf-8")
        pilot.add_ffmpeg_dll_path()
        os.environ["TORCH_FORCE_NO_WEIGHTS_ONLY_LOAD"] = "1"
        import trainer
        from TTS.tts.layers.xtts.trainer import gpt_trainer
        original_config = gpt_trainer.GPTTrainerConfig
        original_trainer = trainer.Trainer
        started = None

        def progress(instance):
            nonlocal started
            if started is None:
                started = time.monotonic()
            elapsed = time.monotonic() - started
            update(state="training", elapsed_seconds=round(elapsed, 2), step=instance.total_steps_done,
                   checkpoint_directory=str(instance.output_path), clips=len(selected))
            if elapsed >= limit:
                raise Deadline()

        def config(**kwargs):
            if args.checkpoint:
                kwargs["model_args"].xtts_checkpoint = str(args.checkpoint.resolve())
            kwargs.update(epochs=10000, run_name=f"jarvis_xtts_{args.minutes}min", lr=args.learning_rate, save_step=250,
                          plot_step=100000, save_n_checkpoints=1)
            return original_config(**kwargs)

        class TimedTrainer(original_trainer):
            def __init__(self, *args, **kwargs):
                kwargs["callbacks"] = {"on_train_step_end": progress}
                super().__init__(*args, **kwargs)

            def save_best_model(self):
                if not args.compact:
                    return super().save_best_model()

            def save_checkpoint(self):
                if not args.compact:
                    return super().save_checkpoint()
                import torch
                temporary = RUN / "final.tmp"
                torch.save({"model": self.model.state_dict(), "step": self.total_steps_done,
                            "epoch": self.epochs_done}, temporary)
                temporary.replace(RUN / "final.pth")

            def fit(self):
                try:
                    self._fit()
                except Deadline:
                    self.save_checkpoint()
                    checkpoint = RUN / "final.pth" if args.compact else Path(self.output_path) / f"checkpoint_{self.total_steps_done}.pth"
                    if not checkpoint.is_file():
                        raise FileNotFoundError(checkpoint)
                    final = RUN / "final.pth"
                    if checkpoint != final:
                        shutil.copy2(checkpoint, final)
                    shutil.copy2(Path(self.output_path) / "config.json", RUN / "config.json")
                    update(state="saved", final_checkpoint=str(final), elapsed_seconds=round(time.monotonic() - started, 2))
                finally:
                    self.dashboard_logger.finish()

        trainer.Trainer = TimedTrainer
        gpt_trainer.GPTTrainerConfig = config
        pilot.DATASET = dataset
        pilot.OUTPUT = RUN / "training"
        pilot.main()
    except BaseException as error:
        update(state="failed", error=str(error))
        raise


if __name__ == "__main__":
    main()
