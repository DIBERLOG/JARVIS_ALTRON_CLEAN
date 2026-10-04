use std::{process::{Command, Stdio}, time::Duration};

static START_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn ensure_running() -> Result<(), String> {
    let _guard = START_LOCK.lock().map_err(|_| "Не удалось подготовить Ollama")?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(1)).build().map_err(|_| "Не удалось подключиться к Ollama")?;
    let ready = || client.get("http://127.0.0.1:11434/api/tags").send()
        .map(|r| r.status().is_success()).unwrap_or(false);
    if ready() { return Ok(()); }
    let mut command = Command::new("ollama");
    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        if let Some(path) = std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from)
            .map(|p| p.join("Programs/Ollama/ollama.exe")).filter(|p| p.is_file()) {
            command = Command::new(path);
        }
        command.creation_flags(0x08000000);
    }
    let mut child = command.arg("serve").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().map_err(|_| "Не удалось автоматически запустить Ollama. Проверьте, что она установлена.")?;
    // The server remains available after JARVIS closes; reap the child when it eventually exits.
    std::thread::spawn(move || { let _ = child.wait(); });
    for _ in 0..40 {
        std::thread::sleep(Duration::from_millis(250));
        if ready() { return Ok(()); }
    }
    Err("Ollama запускается дольше обычного. Повторите перевод через несколько секунд.".into())
}

pub fn start_in_background() {
    std::thread::spawn(|| {
        if let Err(error) = ensure_running() { log::warn!("{error}"); }
    });
}
