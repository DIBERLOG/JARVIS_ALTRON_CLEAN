use jarvis_core::commands::{self, JCommand};

#[tauri::command]
pub fn get_commands_count() -> usize {
    commands::parse_commands().unwrap_or_default()
        .iter()
        .map(|list| list.commands.len())
        .sum()
}

#[tauri::command]
pub fn get_commands_list() -> Vec<JCommand> {
    commands::parse_commands().unwrap_or_default()
        .iter()
        .flat_map(|list| list.commands.clone())
        .collect()
}
