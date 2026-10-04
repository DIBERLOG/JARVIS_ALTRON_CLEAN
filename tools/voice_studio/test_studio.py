import tempfile
from pathlib import Path
import unittest
from unittest.mock import patch
from studio import Studio, approved_rows, training_overrides
from train_runner import expired, atomic_status

class StudioTests(unittest.TestCase):
    def test_hour_boundary(self):
        self.assertFalse(expired(None,3600,9999))
        self.assertFalse(expired(100,3600,3699))
        self.assertTrue(expired(100,3600,3700))

    def test_progress_reader_lock_is_retried_and_never_aborts_training(self):
        with tempfile.TemporaryDirectory() as temp:
            target = Path(temp)/"status.json"
            with patch.object(Path, "replace", side_effect=[PermissionError("reader lock"), None]) as replace, patch("train_runner.time.sleep"):
                self.assertTrue(atomic_status(target, {"step":1}))
                self.assertEqual(replace.call_count,2)
            with patch.object(Path, "replace", side_effect=PermissionError("locked")), patch("train_runner.time.sleep"):
                self.assertFalse(atomic_status(target, {"step":2}))

    def test_stale_training_status_is_not_shown_as_running(self):
        with tempfile.TemporaryDirectory() as temp:
            studio = Studio(root=Path(temp), storage=Path(temp)/"state")
            studio.run = Path(temp)/"run"; studio.run.mkdir()
            atomic_status(studio.run/"status.json", {"phase":"training","elapsed":74})
            self.assertEqual(studio.get_state()["training"]["phase"], "interrupted")
            studio.busy=True; studio.job="Обучение"
            self.assertEqual(studio.get_state()["training"]["phase"], "training")

    def test_only_reviewed_recordings(self):
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/"sample.wav";path.write_bytes(b"test")
            rows=[dict(id=str(i),path=str(path),text="Точный текст",seconds=5,approved=True,origin="human") for i in range(10)]
            rows.append(dict(approved=False,text="Не проверено",path="missing"))
            self.assertEqual(len(approved_rows(rows)),10)
            rows[0]["origin"]="synthetic"
            with self.assertRaises(ValueError):approved_rows(rows)
            self.assertEqual(len(approved_rows(rows,True)),10)
            rows[0]["text"]=""
            with self.assertRaises(ValueError):approved_rows(rows,True)

    def test_no_training_without_data(self):
        with tempfile.TemporaryDirectory() as temp:
            studio=Studio(root=Path(temp),storage=Path(temp)/"state")
            self.assertEqual(studio.settings["minutes"],60)
            with self.assertRaises(ValueError):studio.train()
            self.assertIsNone(studio.run)
            studio.save_settings(dict(minutes=30))
            self.assertEqual(Studio(root=Path(temp),storage=Path(temp)/"state").settings["minutes"],30)
            with self.assertRaises(ValueError):studio.save_settings(dict(minutes=0))

    def test_background_state_and_license_gate(self):
        with tempfile.TemporaryDirectory() as temp:
            studio=Studio(root=Path(temp),storage=Path(temp)/"state")
            self.assertFalse(studio.action("download")["ok"])
            self.assertFalse(studio.action("unknown")["ok"])
            self.assertFalse(studio.get_state()["server"])

    def test_training_args_are_isolated(self):
        args=training_overrides(Path("C:/data"),Path("C:/runs/session"),Path("C:/model"))
        self.assertIn("data.batch_size=1",args)
        self.assertIn("trainer.strategy=auto",args)
        self.assertIn("callbacks.model_checkpoint.save_last=true",args)
        self.assertIn("paths.run_dir=C:/runs/session",args)

    def test_runtime_compat_disables_persistent_workers_without_workers(self):
        with tempfile.TemporaryDirectory() as temp:
            studio = Studio(root=Path(temp), storage=Path(temp)/"state")
            for relative in ["fish_speech/inference_engine/reference_loader.py", "tools/vqgan/extract_vq.py"]:
                path = studio.repo / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("import torchaudio\n", encoding="utf-8")
            dataset = studio.repo / "fish_speech/datasets/semantic.py"
            dataset.parent.mkdir(parents=True, exist_ok=True)
            dataset.write_text("persistent_workers=True,\npersistent_workers=True,\n", encoding="utf-8")
            studio.prepare_audio_compat()
            expected = "persistent_workers=self.num_workers > 0,\n" * 2
            self.assertEqual(dataset.read_text(encoding="utf-8"), expected)
            studio.prepare_audio_compat()
            self.assertEqual(dataset.read_text(encoding="utf-8"), expected)

    def test_one_click_base_voice_selects_reference_and_loads_model(self):
        with tempfile.TemporaryDirectory() as temp:
            studio = Studio(root=Path(temp), storage=Path(temp)/"state")
            sample = Path(temp)/"sample.wav"
            sample.write_bytes(b"sample")
            studio.rows = [dict(id="approved", approved=True, text="Проверенный текст", seconds=5, path=str(sample))]
            with patch.object(studio, "has_weights", return_value=True), patch.object(studio, "start_server") as start, patch.object(studio, "preview") as preview:
                studio.test_voice("Проверка", "base")
                start.assert_called_once()
                preview.assert_called_once_with("Проверка")
                self.assertEqual(studio.settings["reference"], "approved")
                self.assertIn("Исходный", studio.test_info)
                with self.assertRaisesRegex(ValueError, "результата обучения пока нет"):
                    studio.test_voice("Проверка", "trained")
                self.assertEqual(preview.call_count, 1)

    def test_one_click_trained_voice_exports_automatically(self):
        with tempfile.TemporaryDirectory() as temp:
            studio = Studio(root=Path(temp), storage=Path(temp)/"state")
            sample = Path(temp)/"sample.wav"; sample.write_bytes(b"sample")
            studio.rows = [dict(id="approved", approved=True, text="Текст", seconds=5, path=str(sample))]
            run = studio.home/"runs"/"completed"
            (run/"checkpoints").mkdir(parents=True)
            (run/"checkpoints/studio-final.ckpt").write_bytes(b"checkpoint")
            def export(selected): studio.settings["model"] = str(selected/"merged-model")
            with patch.object(studio, "has_weights", return_value=True), patch.object(studio, "stop_server"), patch.object(studio, "start_server") as start, patch.object(studio, "export_model", side_effect=export) as prepare, patch.object(studio, "preview") as preview:
                studio.test_voice("Результат")
                prepare.assert_called_once_with(run)
                start.assert_called_once()
                preview.assert_called_once_with("Результат")
                self.assertIn("После обучения", studio.test_info)
if __name__=="__main__": unittest.main()
