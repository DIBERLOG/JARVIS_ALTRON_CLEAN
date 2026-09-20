use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::APP_DIR;

static IS_SPEAKING: AtomicBool = AtomicBool::new(false);

pub fn is_speaking() -> bool {
    IS_SPEAKING.load(Ordering::SeqCst)
}

pub fn speak(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() || IS_SPEAKING.swap(true, Ordering::SeqCst) {
        return false;
    }

    let Some(mut command) = speaker_command(text) else {
        warn!("No configured TTS speaker is available.");
        IS_SPEAKING.store(false, Ordering::SeqCst);
        return false;
    };

    match command.spawn() {
        Ok(mut child) => {
            info!("TTS speaking: {}", text);
            std::thread::spawn(move || {
                if let Err(error) = child.wait() {
                    warn!("TTS process failed: {}", error);
                }
                IS_SPEAKING.store(false, Ordering::SeqCst);
            });
            true
        }
        Err(error) => {
            warn!("Unable to start TTS: {}", error);
            IS_SPEAKING.store(false, Ordering::SeqCst);
            false
        }
    }
}

fn speaker_command(text: &str) -> Option<Command> {
    let tts_dir = APP_DIR.join("resources").join("tts");
    if fs::read_to_string(tts_dir.join("silero-enabled.txt")).ok().is_some_and(|v| v.trim() == "true") {
        let workspace = APP_DIR.parent()?.parent()?;
        let python = workspace.join("tools").join("voice_training").join(".venv").join("Scripts").join("python.exe");
        let script = tts_dir.join("SileroSpeak.py");
        if python.exists() && script.exists() {
            let mut command = Command::new(python);
            command.arg(script).arg("--text").arg(text);
            info!("Using Silero TTS.");
            return Some(command);
        }
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
