use sysinfo::{System, Pid, ProcessRefreshKind, RefreshKind, CpuRefreshKind, Components};
use peak_alloc::PeakAlloc;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::time::Duration;
use serde::Serialize;
use crate::AppState;

#[tauri::command]
pub fn set_jarvis_terminal_visible(visible:bool)->Result<(),String>{
    #[cfg(windows)] {
        // Console visibility only; never terminate the assistant process.
        #[link(name="kernel32")]
        unsafe extern "system" {fn GetConsoleWindow()->*mut std::ffi::c_void;fn AttachConsole(pid:u32)->i32;fn FreeConsole()->i32;}
        #[link(name="user32")]
        unsafe extern "system" {fn ShowWindow(window:*mut std::ffi::c_void,command:i32)->i32;}
        static LOCK:Mutex<()>=Mutex::new(());
        let _guard=LOCK.lock().map_err(|_|"Не удалось изменить видимость терминала")?;
        let mut sys=System::new();sys.refresh_processes(sysinfo::ProcessesToUpdate::All,true);
        let pid=find_jarvis_app_pid(&sys).ok_or("Сначала запустите голосовой модуль JARVIS")?;
        unsafe {
            let mut window=GetConsoleWindow();let mut attached=false;
            if window.is_null(){if AttachConsole(pid.as_u32())==0{return Err("У голосового модуля нет доступного окна терминала".into())}attached=true;window=GetConsoleWindow();}
            if window.is_null(){if attached{FreeConsole();}return Err("Окно терминала недоступно".into())}
            ShowWindow(window,if visible{5}else{0});
            if attached{FreeConsole();}
        }
        Ok(())
    }
    #[cfg(not(windows))] {let _=visible;Err("Управление терминалом доступно только в Windows".into())}
}

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

static SYS: Lazy<Mutex<System>> = Lazy::new(|| {
    Mutex::new(System::new_with_specifics(
        RefreshKind::nothing()
            .with_processes(ProcessRefreshKind::nothing().with_memory().with_cpu())
            .with_cpu(CpuRefreshKind::everything())
    ))
});

static COMPONENTS: Lazy<Mutex<Components>> = Lazy::new(|| {
    Mutex::new(Components::new_with_refreshed_list())
});

/// Find jarvis-app process and return its PID
fn find_jarvis_app_pid(sys: &System) -> Option<Pid> {
    let expected = std::env::current_exe().ok()?.with_file_name(if cfg!(windows) { "jarvis-app.exe" } else { "jarvis-app" });
    for (pid, process) in sys.processes() {
        if process.exe().is_some_and(|path| path == expected) {
            return Some(*pid);
        }
    }
    None
}

#[derive(serde::Serialize)]
pub struct JarvisAppStats {
    pub running: bool,
    pub ram_mb: u64,
    pub cpu_usage: f32,
}

#[tauri::command]
pub fn get_jarvis_app_stats() -> JarvisAppStats {
    let mut sys = SYS.lock().unwrap();
    
    // refresh all processes to find jarvis-app
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    
    if let Some(pid) = find_jarvis_app_pid(&sys) {
        if let Some(proc) = sys.process(pid) {
            return JarvisAppStats {
                running: true,
                ram_mb: proc.memory() / 1024 / 1024,
                cpu_usage: proc.cpu_usage(),
            };
        }
    }
    
    JarvisAppStats {
        running: false,
        ram_mb: 0,
        cpu_usage: 0.0,
    }
}

#[tauri::command]
pub fn get_current_ram_usage() -> u64 {
    let mut sys = SYS.lock().unwrap();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    
    if let Some(pid) = find_jarvis_app_pid(&sys) {
        if let Some(proc) = sys.process(pid) {
            return proc.memory() / 1024 / 1024;
        }
    }
    
    0
}

#[tauri::command]
pub fn is_jarvis_app_running() -> bool {
    let mut sys = SYS.lock().unwrap();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    find_jarvis_app_pid(&sys).is_some()
}

#[tauri::command]
pub fn get_cpu_temp() -> String {
    let mut components = COMPONENTS.lock().unwrap();
    components.refresh(true);
    
    for component in components.iter() {
        let label = component.label().to_lowercase();
        if label.contains("cpu") || label.contains("core") || label.contains("package") {
            if let Some(temp) = component.temperature() {
                return format!("{:.1}", temp);
            }
        }
    }
    
    if let Some(component) = components.iter().next() {
        if let Some(temp) = component.temperature() {
            return format!("{:.1}", temp);
        }
    }
    
    String::from("N/A")
}

#[tauri::command]
pub fn get_cpu_usage() -> f32 {
    let mut sys = SYS.lock().unwrap();
    
    sys.refresh_cpu_all();
    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_cpu_all();
    
    sys.global_cpu_usage()
}

#[tauri::command]
pub fn get_peak_ram_usage() -> String {
    format!("{}", PEAK_ALLOC.peak_usage_as_gb())
}

#[tauri::command]
pub fn run_jarvis_app() -> Result<(), String> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?
        .parent()
        .ok_or("Failed to get exe directory")?
        .to_path_buf();
    
    #[cfg(target_os = "windows")]
    let jarvis_app_name = "jarvis-app.exe";
    
    #[cfg(not(target_os = "windows"))]
    let jarvis_app_name = "jarvis-app";
    
    let jarvis_app_path = exe_dir.join(jarvis_app_name);
    
    if !jarvis_app_path.exists() {
        return Err(format!("jarvis-app not found at: {}", jarvis_app_path.display()));
    }
    if is_jarvis_app_running() {
        return Ok(());
    }
    
    std::process::Command::new(&jarvis_app_path)
        .spawn()
        .map_err(|e| format!("Failed to start jarvis-app: {}", e))?;
    
    Ok(())
}

#[derive(Serialize)]
pub struct HealthStatus {
    pub microphone_ready: bool,
    pub microphone_name: String,
    pub neural_ready: bool,
    pub tts_mode: String,
    pub tts_ready: bool,
    pub voicemod_running: bool,
    pub internet_available: bool,
}

#[tauri::command]
pub fn get_health_status(state: tauri::State<'_, AppState>) -> HealthStatus {
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let assistant_running = find_jarvis_app_pid(&sys).is_some();
    let voicemod_running = sys.processes().values().any(|process| {
        process.name().to_string_lossy().eq_ignore_ascii_case("Voicemod.exe")
    });
    let devices = jarvis_core::recorder::get_audio_devices();
    let mic_index = state.settings.read("selected_microphone")
        .and_then(|value| value.parse::<i32>().ok()).unwrap_or(-1);
    let microphone_name = if mic_index < 0 { "Системный микрофон".to_string() }
        else { devices.get(mic_index as usize).cloned().unwrap_or_else(|| "Микрофон не найден".into()) };
    let microphone_ready = assistant_running && (mic_index < 0 || (mic_index as usize) < devices.len());
    let (tts_mode, tts_ready) = jarvis_core::tts::mode_status();
    let internet_available = match reqwest::blocking::Client::builder().timeout(Duration::from_secs(3)).build() {
        Ok(client) => client.get("https://html.duckduckgo.com/html/").send()
            .is_ok_and(|response| response.status().is_success())
            || client.get("https://www.google.com/generate_204").send()
                .is_ok_and(|response| response.status().is_success()),
        Err(_) => false,
    };
    HealthStatus {
        microphone_ready, microphone_name, neural_ready: assistant_running,
        tts_mode, tts_ready, voicemod_running, internet_available,
    }
}
