#[cfg(test)]
mod tests {
    use crate::lua::{CommandContext, LuaError, SandboxLevel, execute};

    use std::path::PathBuf;
    use std::time::Duration;
    use tempfile::tempdir;
    use std::fs;
    
    fn create_test_context(cmd_path: PathBuf) -> CommandContext {
        CommandContext {
            phrase: "test phrase".to_string(),
            command_id: "test_cmd".to_string(),
            command_path: cmd_path,
            language: "en".to_string(),
            slots: None,
        }
    }

    #[test]
    fn weather_script_parses() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../resources/commands/weather/script.lua");
        let source = fs::read_to_string(path).unwrap();
        mlua::Lua::new().load(&source).into_function().unwrap();
    }

    #[test]
    fn set_city_uses_slots_and_strips_russian_and_english_prepositions() {
        let source = include_str!("../../../../resources/commands/weather/set_city.lua");
        for (phrase, slot, expected) in [
            ("измени город на Казань", None, "Казань"),
            ("change city to London", None, "London"),
            ("установи город распознано неверно", Some("Котельники"), "Котельники"),
        ] {
            let lua = mlua::Lua::new();
            lua.load(r#"
                jarvis = {
                    context = { language = 'ru', slots = {} },
                    state = { set = function(key, value) saved_city = value end },
                    log = function() end, speak = function() end,
                    system = { notify = function() end }, audio = { play_not_found = function() end }
                }
            "#).exec().unwrap();
            let jarvis: mlua::Table = lua.globals().get("jarvis").unwrap();
            let context: mlua::Table = jarvis.get("context").unwrap();
            context.set("phrase", phrase).unwrap();
            if let Some(city) = slot {
                let slots: mlua::Table = context.get("slots").unwrap();
                slots.set("city", city).unwrap();
            }
            lua.load(source).exec().unwrap();
            assert_eq!(lua.globals().get::<String>("saved_city").unwrap(), expected);
        }
    }
    
    #[test]
    fn test_minimal_sandbox() {
        let dir = tempdir().unwrap();
        let script_path = dir.path().join("test.lua");
        
        fs::write(&script_path, r#"
            jarvis.log("info", "test log")
            return { chain = false }
        "#).unwrap();
        
        let context = create_test_context(dir.path().to_path_buf());
        let result = execute(
            &script_path,
            context,
            SandboxLevel::Minimal,
            Duration::from_secs(5),
        );
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap().chain, false);
    }
    
    #[test]
    fn test_state_persistence() {
        let dir = tempdir().unwrap();
        let script_path = dir.path().join("test.lua");
        
        // first run - set state
        fs::write(&script_path, r#"
            jarvis.state.set("key", "value")
            return true
        "#).unwrap();
        
        let context = create_test_context(dir.path().to_path_buf());
        execute(&script_path, context, SandboxLevel::Standard, Duration::from_secs(5)).unwrap();
        
        // second run - read state
        fs::write(&script_path, r#"
            local val = jarvis.state.get("key")
            if val == "value" then
                return true
            else
                error("State not persisted")
            end
        "#).unwrap();
        
        let context = create_test_context(dir.path().to_path_buf());
        let result = execute(&script_path, context, SandboxLevel::Standard, Duration::from_secs(5));
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_timeout() {
        let dir = tempdir().unwrap();
        let script_path = dir.path().join("test.lua");
        
        fs::write(&script_path, r#"
            while true do end
        "#).unwrap();
        
        let context = create_test_context(dir.path().to_path_buf());
        let result = execute(
            &script_path,
            context,
            SandboxLevel::Minimal,
            Duration::from_millis(100),
        );
        
        assert!(matches!(result, Err(LuaError::Timeout)));
    }
    
    #[test]
    fn test_sandbox_fs_escape() {
        let dir = tempdir().unwrap();
        let script_path = dir.path().join("test.lua");
        
        fs::write(&script_path, r#"
            local ok, err = pcall(function()
                jarvis.fs.read("../../../etc/passwd")
            end)
            if ok then
                error("Should have been blocked")
            end
            return true
        "#).unwrap();
        
        let context = create_test_context(dir.path().to_path_buf());
        let result = execute(&script_path, context, SandboxLevel::Standard, Duration::from_secs(5));
        
        assert!(result.is_ok());
    }
}
