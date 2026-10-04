pub mod manager;
pub mod structs;

use crate::{config, APP_CONFIG_DIR};

use altron_command_engine::persistence;
use log::info;
use std::io;
use std::path::{Path, PathBuf};

pub use manager::SettingsManager;

fn get_db_file_path() -> PathBuf {
    PathBuf::from(format!(
        "{}/{}",
        APP_CONFIG_DIR.get().unwrap().display(),
        config::DB_FILE_NAME
    ))
}

pub fn init_settings() -> structs::Settings {
    let db_file_path = get_db_file_path();

    info!(
        "Loading settings db file located at: {}",
        db_file_path.display()
    );

    if let Some(settings) = read_settings_at(&db_file_path) {
        info!("Settings loaded.");
        return settings;
    }

    warn!("No settings file found or there was an error parsing it. Creating default struct.");
    structs::Settings::default()
}

/// init settings and return a SettingsManager ready to use
pub fn init() -> SettingsManager {
    let settings = init_settings();
    SettingsManager::new(settings)
}

pub fn save_settings(settings: &structs::Settings) -> Result<(), std::io::Error> {
    let db_file_path = get_db_file_path();
    save_settings_at(&db_file_path, settings)?;

    info!("Settings saved to: {:#}", db_file_path.display());
    Ok(())
}

fn read_settings_at(path: &Path) -> Option<structs::Settings> {
    persistence::read(path).ok()
}

fn save_settings_at(path: &Path, settings: &structs::Settings) -> io::Result<()> {
    persistence::replace(path, settings)
}

/// Serializes read-modify-write across GUI and assistant processes. Each write
/// starts from the latest disk state, so a stale process cannot reset settings.
pub fn update_settings<F>(
    fallback: &structs::Settings,
    change: F,
) -> Result<structs::Settings, String>
where
    F: FnOnce(&mut structs::Settings) -> Result<(), String>,
{
    let path = get_db_file_path();
    let updated = persistence::transaction(&path, fallback, change)?;
    info!("ALTRON kernel: settings transaction committed");
    Ok(updated)
}

pub fn latest_settings() -> Option<structs::Settings> {
    read_settings_at(&get_db_file_path())
}

#[cfg(test)]
mod persistence_tests {
    use super::*;

    #[test]
    fn settings_replace_existing_file_and_survive_reload() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("app.db");
        let mut settings = structs::Settings::default();
        save_settings_at(&path, &settings).unwrap();
        settings.set("tts_mode", "silero").unwrap();
        save_settings_at(&path, &settings).unwrap();
        assert_eq!(
            read_settings_at(&path).unwrap().get("tts_mode").as_deref(),
            Some("silero")
        );
        settings.set("tts_mode", "xtts").unwrap();
        save_settings_at(&path, &settings).unwrap();
        assert_eq!(
            read_settings_at(&path).unwrap().get("tts_mode").as_deref(),
            Some("xtts")
        );
    }
}
