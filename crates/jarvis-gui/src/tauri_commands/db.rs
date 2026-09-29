use crate::AppState;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SettingEntry {
    key: String,
    val: String,
}

#[tauri::command]
pub fn db_read(state: tauri::State<'_, AppState>, key: &str) -> String {
    state.settings.read(key).unwrap_or_default()
}

#[tauri::command]
pub fn db_write(state: tauri::State<'_, AppState>, key: &str, val: &str) -> bool {
    match state.settings.write(key, val) {
        Ok(()) => true,
        Err(e) => {
            log::warn!("db_write('{}', '{}'): {}", key, val, e);
            false
        }
    }
}

#[tauri::command]
pub fn db_write_many(state: tauri::State<'_, AppState>, entries: Vec<SettingEntry>) -> Result<(), String> {
    let pairs: Vec<(&str, &str)> = entries.iter()
        .map(|entry| (entry.key.as_str(), entry.val.as_str()))
        .collect();
    state.settings.write_many(&pairs)
}
