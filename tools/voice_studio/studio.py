"""Local Fish Speech 1.5.1 studio. UI runtime and ML runtime are isolated."""
from __future__ import annotations
import base64
import csv
import json
import os
from pathlib import Path
import secrets
import shutil
import subprocess
import sys
import threading
import time
import urllib.request
import uuid

ASSETS = Path(getattr(sys, "_MEIPASS", Path(__file__).parent))
CODEC = "firefly-gan-vq-fsq-8x1024-21hz-generator.pth"
NO_WINDOW = getattr(subprocess, "CREATE_NO_WINDOW", 0)

def atomic_json(path, data):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(".tmp")
    tmp.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
    tmp.replace(path)

def workspace_root():
    candidates = [Path(sys.executable).parent if getattr(sys, "frozen", False) else Path(__file__).parent]
    for base in candidates:
        for root in [base, *base.parents]:
            if (root / "Cargo.toml").is_file() and (root / "resources/tts").is_dir():
                return root
    raise RuntimeError("Поместите приложение в папку проекта JARVIS или tools/voice_studio/dist.")

def approved_rows(rows, allow_synthetic=False):
    valid = []
    for row in rows:
        if not row.get("approved"):
            continue
        if not row.get("text", "").strip():
            raise ValueError("В утверждённой записи отсутствует полный текст.")
        if not Path(row["path"]).is_file():
            raise ValueError("Файл утверждённой записи не найден: " + row["path"])
        if not 1 <= float(row.get("seconds", 0)) <= 30:
            raise ValueError("Для обучения нужны отдельные фразы длительностью 1–30 секунд.")
        if row.get("origin") != "human" and not allow_synthetic:
            raise ValueError("Материал синтетический или происхождение не указано. Включите экспериментальный режим либо добавьте живые записи.")
        valid.append(dict(row))
    if len(valid) < 10:
        raise ValueError("Утвердите минимум 10 записей с точными текстами. Это только минимальный порог запуска, не гарантия качества.")
    return valid

def training_overrides(dataset, run, weights):
    # These are documented Fish Speech 1.5.1 Hydra keys. No model weights are overwritten.
    return [
        "project=" + run.name, "paths.run_dir=" + run.as_posix(),
        "pretrained_ckpt_path=" + weights.as_posix(),
        "train_dataset.proto_files=[" + (dataset / "protos").as_posix() + "]",
        "val_dataset.proto_files=[" + (dataset / "protos").as_posix() + "]",
        "+lora@model.model.lora_config=r_8_alpha_16",
        "data.batch_size=1", "data.num_workers=0", "max_length=512",
        "trainer.accumulate_grad_batches=4", "trainer.devices=1",
        "trainer.strategy=auto", "trainer.precision=bf16-true",
        "trainer.max_steps=100000", "trainer.limit_val_batches=1",
        "+trainer.min_epochs=0", "+trainer.min_steps=0",
        "trainer.val_check_interval=100", "callbacks.model_checkpoint.save_last=true",
        "callbacks.model_checkpoint.save_top_k=2", "callbacks.model_checkpoint.every_n_train_steps=100",
    ]

class Studio:
    def prepare_audio_compat(self):
        # Fish 1.5 predates TorchAudio 2.9's removal of the legacy audio APIs.
        shim = '''
# JARVIS: legacy Fish audio IO on modern TorchAudio; isolated ML runtime only.
if not hasattr(torchaudio, "list_audio_backends"):
    import soundfile as _studio_soundfile
    def _studio_audio_load(source, backend=None, **kwargs):
        samples, rate = _studio_soundfile.read(source, dtype="float32", always_2d=True)
        return torch.from_numpy(samples.T.copy()), rate
    torchaudio.list_audio_backends = lambda: ["soundfile"]
    torchaudio.load = _studio_audio_load
'''
        for relative in ["fish_speech/inference_engine/reference_loader.py", "tools/vqgan/extract_vq.py"]:
            path = self.repo / relative
            content = path.read_text(encoding="utf-8")
            if "# JARVIS: legacy Fish audio IO" not in content:
                path.write_text(content.replace("import torchaudio\n", "import torchaudio\n" + shim, 1), encoding="utf-8")
        dataset_path = self.repo / "fish_speech/datasets/semantic.py"
        dataset_source = dataset_path.read_text(encoding="utf-8")
        adjusted = dataset_source.replace("persistent_workers=True,", "persistent_workers=self.num_workers > 0,")
        if adjusted != dataset_source:
            dataset_path.write_text(adjusted, encoding="utf-8")

    def __init__(self, root=None, storage=None):
        self.root = Path(root or workspace_root())
        self.home = Path(storage or (Path(os.environ.get("LOCALAPPDATA", str(Path.home()))) / "JarvisVoiceStudio"))
        self.home.mkdir(parents=True, exist_ok=True)
        self.runtime = self.root / "tools/voice_studio/runtime"
        self.repo = self.runtime / "fish-speech"
        self.python = self.runtime / ".venv/Scripts/python.exe"
        self.weights = self.repo / "checkpoints/fish-speech-1.5"
        self.settings_path = self.home / "settings.json"
        self.rows_path = self.home / "recordings.json"
        self.settings = {"minutes": 60, "synthetic": False, "reference": "", "model": "", "workspace": str(self.root)}
        if self.settings_path.exists():
            self.settings.update(json.loads(self.settings_path.read_text(encoding="utf-8")))
        self.rows = json.loads(self.rows_path.read_text(encoding="utf-8")) if self.rows_path.exists() else self.seed_rows()
        self.logs = []
        self.lock = threading.RLock()
        self.busy = False
        self.phase = "Готов к настройке"
        self.error = ""
        self.job = ""
        self.server = None
        self.loaded_model = None
        self.process = None
        self.run = Path(self.settings["last_run"]) if self.settings.get("last_run") else None
        self.window = None
        self.tray = None
        self.exiting = False
        self.token = secrets.token_urlsafe(32)
        self.cancel = threading.Event()
        self.duration = 3600
        self.log("Fish Speech 1.5.1 · локальная студия. Таймер обучения: 60 минут.")

    def seed_rows(self):
        source = self.root / "tools/voice_training/transcripts_review.csv"
        rows = []
        if source.exists():
            with source.open(encoding="utf-8-sig", newline="") as stream:
                for row in csv.DictReader(stream):
                    rows.append({"id": uuid.uuid4().hex, "path": row["audio_path"], "name": row["source_file"],
                                 "seconds": float(row.get("duration_seconds") or 0),
                                 "text": row.get("text_for_training") or row.get("candidate_text") or "",
                                 "approved": row.get("review_status") == "утверждено", "origin": "synthetic"})
        atomic_json(self.rows_path, rows)
        return rows

    def log(self, message):
        with self.lock:
            line = time.strftime("%H:%M:%S") + "  " + str(message).rstrip()
            self.logs.append(line)
            self.logs = self.logs[-250:]
            with (self.home / "studio.log").open("a", encoding="utf-8") as stream:
                stream.write(line + "\n")

    def get_state(self):
        with self.lock:
            training = {}
            if self.run and (self.run / "status.json").exists():
                try:
                    training = json.loads((self.run / "status.json").read_text(encoding="utf-8"))
                except (OSError, ValueError):
                    pass
            server = self.server is not None and self.server.poll() is None
            if training.get("phase") in ["training", "saving"] and not (self.busy and self.job == "Обучение"):
                training["phase"] = "saved" if self.run and (self.run / "checkpoints/studio-final.ckpt").is_file() else "interrupted"
            return {"busy": self.busy, "phase": self.phase, "error": self.error, "job": self.job,
                    "settings": dict(self.settings), "rows": [dict(r) for r in self.rows],
                    "logs": list(self.logs), "server": server, "training": training,
                    "installed": self.python.is_file() and (self.runtime / "environment-ready.json").is_file(),
                    "weights": self.has_weights(), "free_gb": round(shutil.disk_usage(self.home).free / 1024**3, 1),
                    "run": str(self.run or ""), "home": str(self.home),
                    "test_run": str(self.latest_finished_run() or ""),
                    "test_info": getattr(self, "test_info", "")}

    def latest_finished_run(self):
        runs = self.home / "runs"
        if not runs.exists():
            return None
        completed = [p for p in runs.iterdir() if p.is_dir() and not p.name.startswith("verification-")
                     and (p / "checkpoints/studio-final.ckpt").is_file()]
        return max(completed, key=lambda p: (p / "checkpoints/studio-final.ckpt").stat().st_mtime, default=None)

    def has_weights(self):
        return all((self.weights / name).is_file() for name in ["model.pth", "config.json", "tokenizer.tiktoken", CODEC])

    def env(self):
        env = os.environ.copy()
        env.update(PYTHONUTF8="1", PYTHONUNBUFFERED="1", USE_LIBUV="0",
                   TORCH_FORCE_NO_WEIGHTS_ONLY_LOAD="1", WANDB_MODE="disabled")
        return env

    def run_command(self, args, cwd=None):
        self.log("Запуск: " + " ".join(str(a) for a in args))
        if self.cancel.is_set():
            raise RuntimeError("Операция отменена.")
        proc = subprocess.Popen([str(a) for a in args], cwd=str(cwd or self.repo),
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                                encoding="utf-8", errors="replace", env=self.env(), creationflags=NO_WINDOW)
        self.process = proc
        try:
            for line in proc.stdout:
                self.log(line)
            code = proc.wait()
            if code:
                raise RuntimeError("Процесс завершился с ошибкой. Подробности в журнале.")
            if self.cancel.is_set():
                raise RuntimeError("Операция отменена.")
        finally:
            self.process = None

    def task(self, name, fn):
        with self.lock:
            if self.busy:
                raise ValueError("Дождитесь текущей операции.")
            self.busy = True
            self.job = name
            self.error = ""
            self.phase = name
            self.cancel.clear()
        def work():
            try:
                fn()
                self.phase = "Операция завершена"
            except Exception as exc:
                self.error = str(exc)
                self.phase = "Нужно внимание"
                self.log(self.error)
                if self.job == "Обучение" and self.run:
                    from train_runner import atomic_status
                    try:
                        status = json.loads((self.run / "status.json").read_text(encoding="utf-8"))
                    except (OSError, ValueError):
                        status = {}
                    status.update(phase="failed", error=self.error)
                    atomic_status(self.run / "status.json", status)
            finally:
                self.busy = False
        threading.Thread(target=work, daemon=True).start()
        return {"ok": True}

    def save_settings(self, data):
        minutes = int(data.get("minutes", self.settings["minutes"]))
        if not 1 <= minutes <= 240:
            raise ValueError("Время обучения: от 1 до 240 минут.")
        reference = str(data.get("reference", self.settings["reference"]))
        if reference and reference not in [r["id"] for r in self.rows]:
            raise ValueError("Выберите запись из списка.")
        with self.lock:
            self.settings.update(minutes=minutes, reference=reference,
                                 synthetic=bool(data.get("synthetic", self.settings["synthetic"])))
            atomic_json(self.settings_path, self.settings)
        return {"ok": True}

    def save_row(self, data):
        if self.busy:
            raise ValueError("Во время операции изменение записей заблокировано.")
        with self.lock:
            row = next(r for r in self.rows if r["id"] == data["id"])
            text = str(data.get("text", row["text"])).strip()
            if data.get("approved") and not text:
                raise ValueError("Введите полный текст записи.")
            row.update(text=text, approved=bool(data.get("approved")), origin=data.get("origin", row["origin"]))
            if row["origin"] not in ["human", "synthetic", "unknown"]:
                row["origin"] = "unknown"
            atomic_json(self.rows_path, self.rows)
        return {"ok": True}

    def choose_audio(self):
        import webview
        files = self.window.create_file_dialog(webview.OPEN_DIALOG, allow_multiple=True,
                                              file_types=("Аудиозаписи (*.wav;*.flac;*.mp3)",))
        if not files:
            return {"ok": True}
        def load():
            for file in files:
                path = Path(file).resolve()
                if path.suffix.lower() not in [".wav", ".flac", ".mp3"]:
                    continue
                if any(Path(row["path"]) == path for row in self.rows):
                    continue
                result = subprocess.check_output(["ffprobe", "-v", "error", "-show_entries", "format=duration",
                        "-of", "default=noprint_wrappers=1:nokey=1", str(path)], creationflags=NO_WINDOW)
                lab = path.with_suffix(".lab")
                self.rows.append({"id": uuid.uuid4().hex, "path": str(path), "name": path.name,
                                  "seconds": float(result), "text": lab.read_text(encoding="utf-8").strip() if lab.exists() else "",
                                  "approved": False, "origin": "unknown"})
            atomic_json(self.rows_path, self.rows)
        return self.task("Импорт записей", load)

    def audio(self, row_id):
        row = next(r for r in self.rows if r["id"] == row_id)
        path = Path(row["path"])
        if path.stat().st_size > 20 * 1024**2:
            raise ValueError("Для предпросмотра выберите запись до 20 МБ.")
        mime = "audio/mpeg" if path.suffix.lower() == ".mp3" else "audio/flac" if path.suffix.lower() == ".flac" else "audio/wav"
        return "data:" + mime + ";base64," + base64.b64encode(path.read_bytes()).decode()

    def install(self):
        if shutil.disk_usage(self.home).free < 15 * 1024**3:
            raise ValueError("Для установки освободите хотя бы 15 ГБ.")
        self.runtime.mkdir(parents=True, exist_ok=True)
        base_python = Path(os.environ.get("LOCALAPPDATA", "")) / "Programs/Python/Python310/python.exe"
        if not self.repo.exists():
            self.run_command(["git", "clone", "--branch", "v1.5.1", "--depth", "1",
                              "https://github.com/fishaudio/fish-speech.git", self.repo], self.runtime)
        if subprocess.check_output(["git", "-C", str(self.repo), "rev-parse", "HEAD"], creationflags=NO_WINDOW).decode().strip() != "58046eaa1a4cefb0c8cc3a3a667b34186ea02dde":
            raise ValueError("Ожидался официальный Fish Speech v1.5.1; версия каталога отличается.")
        if not self.python.exists():
            if not base_python.exists():
                raise ValueError("Нужен Python 3.10 для отдельной среды модели.")
            self.run_command([base_python, "-m", "venv", self.runtime / ".venv"], self.runtime)
        check = subprocess.run([str(self.python), "-c",
            "import importlib.metadata as m; print(m.version('torch'))"], capture_output=True, text=True, creationflags=NO_WINDOW)
        if check.returncode or check.stdout.strip() not in ["2.8.0+cu128", "2.11.0+cu128"]:
            self.run_command([self.python, "-m", "pip", "install", "torch==2.8.0", "torchaudio==2.8.0",
                              "--index-url", "https://download.pytorch.org/whl/cu128", "--only-binary=:all:"])
        self.run_command([self.python, "-m", "pip", "install", "-e", ".", "transformers==4.46.3",
                          "lightning==2.5.5", "gradio<6", "huggingface-hub<1", "torchcodec"], self.repo)
        self.run_command([self.python, "-c", "import torch; import tools.api_server; from omegaconf import OmegaConf; OmegaConf.clear_resolver('eval'); import fish_speech.train; assert torch.cuda.is_available(), 'CUDA недоступна'; print(torch.cuda.get_device_name(0)); print(torch.ones(4,device='cuda').sum().item())"])
        atomic_json(self.runtime / "environment-ready.json", {"version": "1.5.1", "checked_at": time.time()})
        self.log("Среда проверена. Теперь скачайте веса после принятия лицензии модели.")

    def download(self):
        if not self.python.exists():
            raise ValueError("Сначала установите среду.")
        self.run_command([self.python, "-c",
                          "from huggingface_hub import snapshot_download; snapshot_download('fishaudio/fish-speech-1.5',local_dir=r'" +
                          self.weights.as_posix() + "',allow_patterns=['*.pth','*.json','*.tiktoken','README.md'])"])
        if not self.has_weights():
            raise ValueError("Не все веса скачаны. Проверьте доступ к модели на Hugging Face.")

    def stop_server(self):
        if self.server and self.server.poll() is None:
            self.server.terminate()
            try:
                self.server.wait(timeout=20)
            except subprocess.TimeoutExpired:
                self.server.kill()
                self.server.wait()
        self.server = None
        self.loaded_model = None

    def start_server(self):
        if not self.has_weights():
            raise ValueError("Веса Fish Speech ещё не установлены.")
        self.stop_server()
        self.prepare_audio_compat()
        model = Path(self.settings.get("model") or self.weights)
        args = [self.python, "-m", "tools.api_server", "--listen", "127.0.0.1:9764",
                "--llama-checkpoint-path", model, "--decoder-checkpoint-path", self.weights / CODEC,
                "--decoder-config-name", "firefly_gan_vq", "--device", "cuda", "--half", "--api-key", self.token]
        self.server = subprocess.Popen([str(a) for a in args], cwd=self.repo, stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT, text=True, encoding="utf-8", errors="replace", env=self.env(), creationflags=NO_WINDOW)
        proc = self.server
        def reader():
            for line in proc.stdout:
                self.log(line)
        threading.Thread(target=reader, daemon=True).start()
        for _ in range(180):
            if self.cancel.is_set():
                self.stop_server()
                raise ValueError("Запуск отменён.")
            if proc.poll() is not None:
                raise ValueError("Модель не загрузилась. Проверьте журнал и видеопамять.")
            try:
                req = urllib.request.Request("http://127.0.0.1:9764/v1/health", headers={"Authorization": "Bearer " + self.token})
                with urllib.request.urlopen(req, timeout=2) as response:
                    if response.status == 200:
                        self.loaded_model = str(model)
                        self.log("Модель загружена и остаётся в памяти до остановки. GPU занят озвучкой.")
                        return
            except Exception:
                time.sleep(1)
        self.stop_server()
        raise ValueError("Модель не успела загрузиться за 3 минуты.")

    def preview(self, text):
        if not self.server or self.server.poll() is not None:
            raise ValueError("Сначала загрузите модель в разделе озвучки.")
        row = next((r for r in self.rows if r["id"] == self.settings["reference"]), None)
        if not row or not row["approved"]:
            raise ValueError("Выберите утверждённую запись-образец с точным текстом.")
        if not 3 <= row["seconds"] <= 30:
            raise ValueError("Для образца выберите чистую запись от 3 до 30 секунд.")
        text = str(text).strip()
        if not text or len(text) > 1500:
            raise ValueError("Введите текст длиной до 1500 символов.")
        payload = {"text": text, "format": "wav", "streaming": False, "use_memory_cache": "on",
                   "references": [{"audio": base64.b64encode(Path(row["path"]).read_bytes()).decode(), "text": row["text"]}],
                   "temperature": 0.7, "top_p": 0.7, "repetition_penalty": 1.2, "max_new_tokens": 1024}
        request = urllib.request.Request("http://127.0.0.1:9764/v1/tts", data=json.dumps(payload).encode(),
                    headers={"Content-Type": "application/json", "Authorization": "Bearer " + self.token})
        with urllib.request.urlopen(request, timeout=180) as response:
            wav = response.read()
        if not wav.startswith(b"RIFF"):
            raise ValueError("Сервер не вернул WAV.")
        output = self.home / "previews"
        output.mkdir(exist_ok=True)
        file = output / (time.strftime("%Y%m%d-%H%M%S") + ".wav")
        file.write_bytes(wav)
        self.preview_audio = "data:audio/wav;base64," + base64.b64encode(wav).decode()
        self.log("Предпросмотр сохранён: " + str(file))

    def train(self):
        rows = approved_rows(self.rows, self.settings["synthetic"])
        if not self.has_weights():
            raise ValueError("Сначала установите веса модели.")
        if shutil.disk_usage(self.home).free < 12 * 1024**3:
            raise ValueError("Для контрольных точек обучения нужно минимум 12 ГБ свободного места.")
        self.prepare_audio_compat()
        self.stop_server()  # The 8 GB GPU cannot hold inference and training concurrently.
        self.run = self.home / "runs" / (time.strftime("%Y%m%d-%H%M%S") + "-" + uuid.uuid4().hex[:6])
        self.run.mkdir(parents=True)
        self.settings["last_run"] = str(self.run)
        atomic_json(self.settings_path, self.settings)
        self.duration = int(self.settings["minutes"]) * 60
        atomic_json(self.run / "manifest.json", {"version": "1.5.1", "duration_seconds": self.duration,
                    "synthetic_experiment": self.settings["synthetic"], "recordings": rows})
        dataset = self.run / "data"
        speaker = dataset / "jarvis"
        speaker.mkdir(parents=True)
        self.phase = "Подготовка записей — таймер ещё не идёт"
        for index, row in enumerate(rows):
            dest = speaker / f"clip_{index:04d}.wav"
            self.run_command(["ffmpeg", "-nostdin", "-v", "error", "-i", row["path"],
                        "-af", "loudnorm=I=-20:TP=-2:LRA=7", "-ar", "44100", "-ac", "1", "-c:a", "pcm_s16le", dest])
            dest.with_suffix(".lab").write_text(row["text"], encoding="utf-8")
        self.phase = "Извлечение токенов — таймер ещё не идёт"
        self.run_command([self.python, "tools/vqgan/extract_vq.py", dataset, "--num-workers", "1",
                          "--batch-size", "1", "--config-name", "firefly_gan_vq", "--checkpoint-path", self.weights / CODEC])
        self.run_command([self.python, "tools/llama/build_dataset.py", "--input", dataset,
                          "--output", dataset / "protos", "--text-extension", ".lab", "--num-workers", "1"])
        self.phase = "Обучение: таймер стартует с первого пакета"
        self.run_command([self.python, ASSETS / "train_runner.py", "--repo", self.repo, "--run", self.run,
                          "--seconds", str(self.duration), "--", *training_overrides(dataset, self.run, self.weights)])
        final = self.run / "checkpoints/studio-final.ckpt"
        if not final.exists():
            raise ValueError("Обучение завершилось без финальной контрольной точки. Проверьте журнал.")
        self.log("Результат сохранён отдельно. Перед применением нужен экспорт и прослушивание.")

    def export_model(self, run=None):
        run = run or self.run
        if not run or not (run / "checkpoints/studio-final.ckpt").is_file():
            raise ValueError("Нет завершённого результата обучения в этой сессии.")
        output = run / "merged-model"
        checkpoint = run / "checkpoints/studio-final.ckpt"
        merged = output / "model.pth"
        if not merged.is_file() or merged.stat().st_mtime < checkpoint.stat().st_mtime:
            self.run_command([self.python, "tools/llama/merge_lora.py", "--lora-config", "r_8_alpha_16",
                              "--base-weight", self.weights, "--lora-weight", checkpoint,
                              "--output", output])
        for name in ["config.json", "tokenizer.tiktoken"]:
            if not (output / name).exists():
                shutil.copy2(self.weights / name, output / name)
        self.settings["model"] = str(output)
        atomic_json(self.settings_path, self.settings)
        self.log("Экспорт выбран только для предпросмотра в студии. Голос JARVIS не изменён.")

    def test_voice(self, text, variant="trained"):
        text = str(text).strip()
        if not text or len(text) > 1500:
            raise ValueError("Введите текст длиной от 1 до 1500 символов.")
        if variant not in ["trained", "base"]:
            raise ValueError("Выберите обученный или исходный голос.")
        eligible = [r for r in self.rows if r.get("approved") and r.get("text", "").strip()
                    and 3 <= r.get("seconds", 0) <= 30 and Path(r["path"]).is_file()]
        if not eligible:
            raise ValueError("Для испытания подтвердите хотя бы одну запись-образец длительностью 3–30 секунд.")
        if not self.has_weights():
            raise ValueError("Сначала установите веса в разделе «Настройка».")
        if variant == "trained":
            run = self.latest_finished_run()
            if not run:
                raise ValueError("Сохранённого результата обучения пока нет. Дождитесь завершения или выберите «Исходный голос».")
            self.phase = "Подготавливаю обученный голос"
            merged = run / "merged-model/model.pth"
            if self.loaded_model != str(run / "merged-model") or not merged.is_file() or merged.stat().st_mtime < (run / "checkpoints/studio-final.ckpt").stat().st_mtime:
                self.stop_server()
            self.export_model(run)
            label = "После обучения · " + run.name
        else:
            self.settings["model"] = ""
            label = "Исходный голос Fish Speech 1.5.1"
        reference = next((r for r in eligible if r["id"] == self.settings["reference"]), eligible[0])
        self.settings["reference"] = reference["id"]
        atomic_json(self.settings_path, self.settings)
        wanted = str(Path(self.settings.get("model") or self.weights))
        if not self.server or self.server.poll() is not None or self.loaded_model != wanted:
            self.phase = "Загружаю голос для испытания"
            self.start_server()
        self.phase = "Озвучиваю ваш текст"
        self.preview(text)
        self.test_info = label

    def stop(self):
        self.cancel.set()
        if self.run and self.job == "Обучение":
            (self.run / "stop.request").touch()
            self.log("Запрошена остановка. Текущий шаг завершится, затем модель сохранится.")
        elif self.process and self.process.poll() is None:
            self.process.terminate()
        return {"ok": True}

    def action(self, name, data=None):
        data = data or {}
        try:
            if name == "settings": return self.save_settings(data)
            if name == "row": return self.save_row(data)
            if name == "stop": return self.stop()
            if name == "import": return self.choose_audio()
            if name == "hide":
                if not self.tray: raise ValueError("Значок в трее недоступен. Сверните окно обычной кнопкой.")
                self.window.hide()
                return {"ok": True}
            if name == "folder":
                os.startfile(self.home)
                return {"ok": True}
            if name == "reset_model":
                if self.busy: raise ValueError("Дождитесь операции.")
                self.stop_server()
                self.settings["model"] = ""
                atomic_json(self.settings_path, self.settings)
                return {"ok": True}
            actions = {"install": ("Установка среды", self.install),
                       "download": ("Загрузка весов", self.download),
                       "start_server": ("Загрузка модели", self.start_server),
                       "stop_server": ("Выгрузка модели", self.stop_server),
                       "preview": ("Генерация предпросмотра", lambda: self.preview(data.get("text", ""))),
                       "test_voice": ("Испытание голоса", lambda: self.test_voice(data.get("text", ""), data.get("variant", "trained"))),
                       "train": ("Обучение", self.train), "export": ("Экспорт модели", self.export_model)}
            if name not in actions: raise ValueError("Неизвестное действие.")
            if name == "download" and not data.get("license_accepted"):
                raise ValueError("Ознакомьтесь с лицензией весов CC-BY-NC-SA-4.0 и подтвердите некоммерческое использование.")
            return self.task(*actions[name])
        except Exception as exc:
            return {"ok": False, "error": str(exc)}

    def get_preview(self):
        return getattr(self, "preview_audio", "")

class Api:
    """Expose only the four UI operations, never environment or process-launch helpers."""
    def __init__(self, studio):
        self._studio = studio
    def get_state(self):
        return self._studio.get_state()
    def action(self, name, data=None):
        return self._studio.action(name, data)
    def audio(self, row_id):
        return self._studio.audio(row_id)
    def get_preview(self):
        return self._studio.get_preview()

def main():
    mutex = None
    if os.name == "nt":
        import ctypes
        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel.CreateMutexW.restype = ctypes.c_void_p
        kernel.CreateMutexW.argtypes = [ctypes.c_void_p, ctypes.c_bool, ctypes.c_wchar_p]
        mutex = kernel.CreateMutexW(None, False, "Local\\JarvisVoiceStudio_1_5_1")
        if ctypes.get_last_error() == 183:
            user = ctypes.windll.user32
            user.FindWindowW.restype = ctypes.c_void_p
            user.FindWindowW.argtypes = [ctypes.c_wchar_p, ctypes.c_wchar_p]
            handle = user.FindWindowW(None, "JARVIS · Voice Studio")
            if handle:
                user.ShowWindow(ctypes.c_void_p(handle), 9)
                user.SetForegroundWindow(ctypes.c_void_p(handle))
            return
    import webview
    from PIL import Image, ImageDraw
    import pystray
    studio = Studio()
    window = webview.create_window("JARVIS · Voice Studio", str(ASSETS / "index.html"), js_api=Api(studio),
                                  width=1120, height=790, min_size=(850, 640), background_color="#071418")
    studio.window = window
    image = Image.new("RGB", (64, 64), "#0b2027")
    draw = ImageDraw.Draw(image)
    draw.ellipse((10, 10, 54, 54), outline="#65eee1", width=3)
    draw.line((23, 32, 28, 40, 40, 22), fill="#65eee1", width=4)
    def quit_app(*_):
        if studio.busy:
            studio.stop()
            studio.log("Дождитесь завершения операции перед выходом.")
            window.show()
            return
        studio.stop_server()
        studio.exiting = True
        if studio.tray: studio.tray.stop()
        window.destroy()
    icon = pystray.Icon("jarvis-voice-studio", image, "JARVIS Voice Studio",
               menu=pystray.Menu(pystray.MenuItem("Открыть студию", lambda *_: window.show(), default=True),
                                pystray.MenuItem("Выйти", quit_app)))
    def tray():
        try:
            studio.tray = icon
            icon.run()
        except Exception as exc:
            studio.tray = None
            studio.log("Трей недоступен: " + str(exc))
    threading.Thread(target=tray, daemon=True).start()
    def closing():
        if studio.exiting:
            return True
        if studio.tray:
            window.hide()
            return False
        if studio.busy:
            return False
    window.events.closing += closing
    if "--train" in sys.argv:
        launched = threading.Event()
        def start_requested_training():
            if launched.is_set():
                return
            launched.set()
            window.evaluate_js("tab('training')")
            result = studio.action("train")
            if not result.get("ok"):
                studio.error = result.get("error", "Не удалось начать обучение")
                studio.log(studio.error)
        window.events.loaded += start_requested_training
    webview.start()

if __name__ == "__main__":
    main()
