use std::{
    fs,
    path::PathBuf,
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{atomic::{AtomicU32, AtomicUsize, Ordering}, mpsc},
};
use once_cell::sync::OnceCell;

use crate::{config, APP_CONFIG_DIR, APP_DIR};

static PENDING_SPEECH: AtomicUsize = AtomicUsize::new(0);
static SPEECH_GENERATION: AtomicUsize = AtomicUsize::new(0);
static ACTIVE_TTS_PID: AtomicU32 = AtomicU32::new(0);
static SPEECH_QUEUE: OnceCell<mpsc::Sender<(String, Option<Command>, usize)>> = OnceCell::new();

fn kill_speech_process(pid: u32) {
    #[cfg(windows)]
    let result = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .output();
    #[cfg(not(windows))]
    let result = Command::new("kill").arg(pid.to_string()).output();
    if let Err(error) = result {
        warn!("Could not stop TTS process {pid}: {error}");
    }
}

/// Cancel the current speech and any replies waiting in this process's queue.
pub fn stop() {
    SPEECH_GENERATION.fetch_add(1, Ordering::SeqCst);
    let pid = ACTIVE_TTS_PID.swap(0, Ordering::SeqCst);
    if pid != 0 { kill_speech_process(pid); }
}

fn run_speech_command(mut command: Command, generation: usize) -> Result<(), String> {
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    let pid = child.id();
    ACTIVE_TTS_PID.store(pid, Ordering::SeqCst);
    if generation != SPEECH_GENERATION.load(Ordering::SeqCst) {
        kill_speech_process(pid);
    }
    let result = child.wait().map_err(|error| error.to_string());
    ACTIVE_TTS_PID.store(0, Ordering::SeqCst);
    match result {
        Ok(status) if status.success() || generation != SPEECH_GENERATION.load(Ordering::SeqCst) => Ok(()),
        Ok(status) => Err(format!("TTS process exited with {status}")),
        Err(error) => Err(error),
    }
}

#[cfg(all(test, windows))]
mod cancellation_tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn stop_interrupts_the_active_speech_process() {
        let generation = SPEECH_GENERATION.load(Ordering::SeqCst);
        let started = Instant::now();
        let worker = std::thread::spawn(move || {
            let mut command = Command::new("powershell");
            command.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 20"]);
            run_speech_command(command, generation)
        });
        while ACTIVE_TTS_PID.load(Ordering::SeqCst) == 0 && started.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_ne!(ACTIVE_TTS_PID.load(Ordering::SeqCst), 0, "test process did not start");
        stop();
        assert!(worker.join().unwrap().is_ok());
        assert!(started.elapsed() < Duration::from_secs(10), "speech was not interrupted promptly");
    }
}

struct SileroWorker {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl SileroWorker {
    fn start(tts_dir: &std::path::Path) -> Result<Self, String> {
        let python = training_python().ok_or("Training Python is unavailable")?;
        let script = tts_dir.join("SileroSpeak.py");
        if !script.is_file() { return Err("SileroSpeak.py is unavailable".into()); }
        let mut child = Command::new(python).arg(script).arg("--server")
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
            .spawn().map_err(|error| error.to_string())?;
        let input = child.stdin.take().ok_or("Silero stdin is unavailable")?;
        let mut output = BufReader::new(child.stdout.take().ok_or("Silero stdout is unavailable")?);
        let mut ready = String::new();
        output.read_line(&mut ready).map_err(|error| error.to_string())?;
        if ready.trim() != "READY" {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("Silero did not start: {}", ready.trim()));
        }
        Ok(Self { child, input, output })
    }

    fn speak(&mut self, text: &str) -> Result<(), String> {
        let message = serde_json::json!({ "text": text });
        writeln!(self.input, "{message}").map_err(|error| error.to_string())?;
        self.input.flush().map_err(|error| error.to_string())?;
        let mut result = String::new();
        self.output.read_line(&mut result).map_err(|error| error.to_string())?;
        if result.trim() == "OK" { Ok(()) } else { Err(format!("Silero: {}", result.trim())) }
    }
}

impl Drop for SileroWorker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn is_speaking() -> bool {
    PENDING_SPEECH.load(Ordering::SeqCst) > 0
}

pub fn speak(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }

    let tts_dir = APP_DIR.join("resources").join("tts");
    let use_silero = selected_tts_mode() == "silero" && silero_speaker_command(&tts_dir, text).is_some();
    let command = if use_silero { None } else { speaker_command(text) };
    if !use_silero && command.is_none() {
        warn!("No configured TTS speaker is available.");
        return false;
    }

    let queue = speech_queue();
    PENDING_SPEECH.fetch_add(1, Ordering::SeqCst);
    let generation = SPEECH_GENERATION.load(Ordering::SeqCst);
    if queue.send((text.to_owned(), command, generation)).is_err() {
        PENDING_SPEECH.fetch_sub(1, Ordering::SeqCst);
        warn!("TTS queue is unavailable.");
        return false;
    }
    true
}

fn speech_queue() -> &'static mpsc::Sender<(String, Option<Command>, usize)> {
    SPEECH_QUEUE.get_or_init(|| {
        let (sender, receiver) = mpsc::channel::<(String, Option<Command>, usize)>();
        std::thread::spawn(move || {
            let tts_dir = APP_DIR.join("resources").join("tts");
            let mut silero = if selected_tts_mode() == "silero" { SileroWorker::start(&tts_dir).ok() } else { None };
            for (spoken_text, command, generation) in receiver {
                if generation != SPEECH_GENERATION.load(Ordering::SeqCst) {
                    PENDING_SPEECH.fetch_sub(1, Ordering::SeqCst);
                    continue;
                }
                info!("TTS speaking: {}", spoken_text);
                if let Some(command) = command {
                    if let Err(error) = run_speech_command(command, generation) {
                        warn!("TTS process failed: {error}");
                    }
                } else {
                    if silero.is_none() { silero = SileroWorker::start(&tts_dir).ok(); }
                    if generation != SPEECH_GENERATION.load(Ordering::SeqCst) {
                        silero = None;
                        PENDING_SPEECH.fetch_sub(1, Ordering::SeqCst);
                        continue;
                    }
                    ACTIVE_TTS_PID.store(silero.as_ref().map(|worker| worker.child.id()).unwrap_or(0), Ordering::SeqCst);
                    if generation != SPEECH_GENERATION.load(Ordering::SeqCst) {
                        let pid = ACTIVE_TTS_PID.swap(0, Ordering::SeqCst);
                        if pid != 0 { kill_speech_process(pid); }
                    }
                    let result = if generation == SPEECH_GENERATION.load(Ordering::SeqCst) {
                        silero.as_mut().ok_or("Silero worker unavailable".to_string())
                            .and_then(|worker| worker.speak(&spoken_text))
                    } else {
                        Err("Speech cancelled".to_string())
                    };
                    ACTIVE_TTS_PID.store(0, Ordering::SeqCst);
                    if generation != SPEECH_GENERATION.load(Ordering::SeqCst) {
                        silero = None;
                    } else if let Err(error) = result {
                        warn!("{error}; retrying on next reply");
                        silero = None;
                        if let Some(fallback) = silero_speaker_command(&tts_dir, &spoken_text) {
                            if let Err(error) = run_speech_command(fallback, generation) {
                                warn!("Silero one-shot fallback failed: {error}");
                            }
                        }
                    }
                }
                PENDING_SPEECH.fetch_sub(1, Ordering::SeqCst);
            }
        });
        sender
    })
}

pub fn prewarm_silero() {
    if selected_tts_mode() != "silero" { return; }
    let _ = speech_queue();
}

fn speaker_command(text: &str) -> Option<Command> {
    let tts_dir = APP_DIR.join("resources").join("tts");
    if selected_tts_mode() == "xtts" {
        if let Some(command) = xtts_speaker_command(&tts_dir, text) {
            info!("Using local XTTS voice (direct output).");
            return Some(command);
        }
        warn!("XTTS voice is unavailable; falling back to Silero.");
    }
    if let Some(command) = silero_speaker_command(&tts_dir, text) {
        info!("Using Silero TTS (Voicemod cable).");
        return Some(command);
    }
    if let Some((python, script, model, config)) = neural_speaker_paths() {
        let mut command = Command::new(python);
        command
            .arg(script)
            .arg("--model")
            .arg(model)
            .arg("--config")
            .arg(config)
            .arg("--text")
            .arg(text);
        info!("Using experimental neural TTS.");
        return Some(command);
    }

    let speaker = APP_DIR.join("resources").join("tts").join("Speak.exe");
    if !speaker.exists() {
        return None;
    }
    let mut command = Command::new(speaker);
    command.arg(text);
    Some(command)
}

fn selected_tts_mode() -> String {
    let Some(config_dir) = APP_CONFIG_DIR.get() else {
        return "xtts".to_string();
    };
    fs::read_to_string(config_dir.join(config::DB_FILE_NAME))
        .ok()
        .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
        .and_then(|settings| settings.get("tts_mode")?.as_str().map(str::to_owned))
        .unwrap_or_else(|| "xtts".to_string())
}

pub fn mode_status() -> (String, bool) {
    let mode = selected_tts_mode();
    let tts_dir = APP_DIR.join("resources").join("tts");
    let available = if mode == "xtts" {
        xtts_speaker_command(&tts_dir, "check").is_some()
    } else {
        silero_speaker_command(&tts_dir, "check").is_some()
    };
    (mode, available)
}

fn training_python() -> Option<PathBuf> {
    let workspace = APP_DIR.parent()?.parent()?;
    let python = workspace.join("tools").join("voice_training").join(".venv").join("Scripts").join("python.exe");
    python.is_file().then_some(python)
}

fn xtts_speaker_command(tts_dir: &std::path::Path, text: &str) -> Option<Command> {
    let python = training_python()?;
    let script = tts_dir.join("TrainedXttsSpeak.py");
    let run_name = fs::read_to_string(tts_dir.join("trained-checkpoint.txt")).ok()?;
    let run_name = run_name.trim();
    if !run_name.starts_with("jarvis_xtts_pilot_") || run_name.contains(['/', '\\']) || run_name.contains("..") {
        return None;
    }
    let workspace = APP_DIR.parent()?.parent()?;
    let run = workspace.join("tools").join("voice_training").join("xtts_pilot_output").join(run_name);
    let checkpoint = run.join("best_model.pth");
    let config = run.join("config.json");
    let cache = PathBuf::from(std::env::var_os("LOCALAPPDATA")?)
        .join("tts").join("tts_models--multilingual--multi-dataset--xtts_v2");
    if !script.is_file() || !checkpoint.is_file() || !config.is_file()
        || !cache.join("vocab.json").is_file() || !cache.join("speakers_xtts.pth").is_file() {
        return None;
    }
    let mut command = Command::new(python);
    command.arg(script).arg("--checkpoint").arg(checkpoint)
        .arg("--config").arg(config).arg("--text").arg(text);
    Some(command)
}

fn silero_speaker_command(tts_dir: &std::path::Path, text: &str) -> Option<Command> {
    let python = training_python()?;
    let script = tts_dir.join("SileroSpeak.py");
    if !script.is_file() {
        return None;
    }
    let mut command = Command::new(python);
    command.arg(script).arg("--text").arg(text);
    Some(command)
}

fn neural_speaker_paths() -> Option<(PathBuf, PathBuf, PathBuf, PathBuf)> {
    let tts_dir = APP_DIR.join("resources").join("tts");
    if fs::read_to_string(tts_dir.join("neural-enabled.txt")).ok()?.trim() != "true" {
        return None;
    }
    let workspace = APP_DIR.parent()?.parent()?;
    let training_root = workspace.join("tools").join("voice_training");
    let python = training_root.join(".venv").join("Scripts").join("python.exe");
    let script = tts_dir.join("NeuralSpeak.py");
    if !python.exists() || !script.exists() {
        return None;
    }

    let mut runs: Vec<_> = fs::read_dir(training_root.join("smoke_output")).ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join("best_model.pth").is_file() && path.join("config.json").is_file())
        .collect();
    runs.sort_by_key(|path| fs::metadata(path).and_then(|meta| meta.modified()).ok());
    let run = runs.pop()?;
    Some((python, script, run.join("best_model.pth"), run.join("config.json")))
}
