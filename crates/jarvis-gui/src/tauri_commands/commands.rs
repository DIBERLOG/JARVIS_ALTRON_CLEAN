use jarvis_core::commands::{self, JCommand};

#[tauri::command]
pub fn get_commands_count() -> usize {
    get_commands_list().len()
}

#[tauri::command]
pub fn get_commands_list() -> Vec<JCommand> {
    commands::parse_commands().unwrap_or_default()
        .iter()
        .flat_map(|list| list.commands.clone())
        .chain(commands::center::available_commands())
        .collect()
}
