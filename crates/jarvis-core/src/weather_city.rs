use std::path::Path;

fn read_city(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|value| value.get("city").and_then(|city| city.as_str()).map(str::to_owned))
        .filter(|city| !city.trim().is_empty())
}

/// Shared by the voice process and the GUI, including installs in different folders.
pub fn get() -> String {
    crate::APP_CONFIG_DIR.get().and_then(|dir| read_city(&dir.join("weather-city.json")))
        .or_else(|| read_city(&crate::APP_DIR.join(crate::config::COMMANDS_PATH).join("weather/.state.json")))
        .unwrap_or_else(|| "Москва".into())
}

pub fn set(city: &str) -> Result<(), String> {
    let dir = crate::APP_CONFIG_DIR.get().ok_or("Каталог настроек не инициализирован")?;
    save(&dir.join("weather-city.json"), city)
}

fn save(path: &Path, city: &str) -> Result<(), String> {
    let city = city.trim();
    if city.is_empty() || city.chars().count() > 100 {
        return Err("Укажите название города длиной от 1 до 100 символов".into());
    }
    let mut file = tempfile::NamedTempFile::new_in(path.parent().ok_or("Нет каталога настроек")?)
        .map_err(|e| format!("Не удалось сохранить город: {e}"))?;
    serde_json::to_writer(file.as_file_mut(), &serde_json::json!({ "city": city }))
        .map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| format!("Не удалось сохранить город: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn city_is_shared_persistent_and_invalid_updates_preserve_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("weather-city.json");
        save(&path, "  Казань  ").unwrap();
        assert_eq!(read_city(&path).as_deref(), Some("Казань"));
        save(&path, "Котельники").unwrap();
        assert_eq!(read_city(&path).as_deref(), Some("Котельники"));
        assert!(save(&path, "   ").is_err());
        assert_eq!(read_city(&path).as_deref(), Some("Котельники"));
    }
}
