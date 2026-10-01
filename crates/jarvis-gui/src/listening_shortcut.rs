use parking_lot::Mutex;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

#[derive(Default)]
pub struct ListeningShortcut(Mutex<Option<Shortcut>>);

#[tauri::command]
pub fn set_listening_shortcut(app: tauri::AppHandle, state: tauri::State<'_, ListeningShortcut>, shortcut: String) -> Result<(), String> {
    let next = if shortcut.is_empty() { None } else {
        Some(shortcut.parse::<Shortcut>().map_err(|e| format!("Некорректное сочетание: {e}"))?)
    };
    let mut previous = state.0.lock();
    if *previous == next { return Ok(()); }
    if let Some(key) = next {
        app.global_shortcut().register(key).map_err(|e| format!("Сочетание занято или недоступно. Выберите другое: {e}"))?;
    }
    if let Some(key) = *previous {
        if let Err(error) = app.global_shortcut().unregister(key) {
            if let Some(key) = next { let _ = app.global_shortcut().unregister(key); }
            return Err(error.to_string());
        }
    }
    *previous = next;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn supported_combinations_parse() {
        for key in ["Ctrl+Alt+J", "Ctrl+Shift+F8", "Alt+Space", "Ctrl+ArrowUp"] {
            assert!(key.parse::<Shortcut>().is_ok(), "{key}");
        }
        assert!("Ctrl+NotAKey".parse::<Shortcut>().is_err());
    }
}
