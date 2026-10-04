use std::{
    path::{Path, PathBuf},
    time::Duration,
};

/// Backend-neutral command definition supplied by the compatibility adapter.
pub struct Definition<'a> {
    pub kind: &'a str,
    pub executable: &'a str,
    pub executable_arguments: &'a [String],
    pub cli: &'a str,
    pub cli_arguments: &'a [String],
    pub script: &'a str,
}

#[derive(Debug, PartialEq)]
pub enum Plan {
    Acknowledge,
    Script(PathBuf),
    Launch {
        program: PathBuf,
        arguments: Vec<String>,
    },
    Restart {
        program: PathBuf,
        arguments: Vec<String>,
        grace: Duration,
    },
    Exit {
        grace: Duration,
    },
    EndChain,
}

/// Construct and validate a plan without starting programs or speaking.
pub fn plan(pack: &Path, definition: &Definition<'_>) -> Result<Plan, String> {
    match definition.kind {
        "voice" => Ok(Plan::Acknowledge),
        "stop_chaining" => Ok(Plan::EndChain),
        "terminate" => Ok(Plan::Exit {
            grace: Duration::from_secs(2),
        }),
        "cli" if definition.cli.trim().is_empty() => Err("CLI executable is not configured".into()),
        "cli" => Ok(Plan::Launch {
            program: definition.cli.into(),
            arguments: definition.cli_arguments.to_vec(),
        }),
        "ahk" | "restart" => {
            let program = super::process::locate(pack, definition.executable)
                .map_err(|error| error.to_string())?;
            let arguments = definition.executable_arguments.to_vec();
            if definition.kind == "restart" {
                Ok(Plan::Restart {
                    program,
                    arguments,
                    grace: Duration::from_millis(500),
                })
            } else {
                Ok(Plan::Launch { program, arguments })
            }
        }
        "lua" => {
            let script = pack.join(if definition.script.is_empty() {
                "script.lua"
            } else {
                definition.script
            });
            if !script.is_file() {
                return Err(format!("Script not found: {}", script.display()));
            }
            Ok(Plan::Script(script))
        }
        kind => Err(format!("Unsupported command type: {kind}")),
    }
}

/// Power/restart commands must be chosen by an explicit alias, not a guess.
pub fn allows_approximate(id: &str, kind: &str) -> bool {
    !matches!(kind, "restart" | "terminate")
        && !["restart", "shutdown", "delete", "remove", "format"]
            .iter()
            .any(|word| id.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn definition(kind: &str) -> Definition<'_> {
        Definition {
            kind,
            executable: "missing.exe",
            executable_arguments: &[],
            cli: "powershell.exe",
            cli_arguments: &[],
            script: "",
        }
    }
    #[test]
    fn plans_do_not_run_actions() {
        assert_eq!(
            plan(Path::new("."), &definition("voice")).unwrap(),
            Plan::Acknowledge
        );
        assert_eq!(
            plan(Path::new("."), &definition("stop_chaining")).unwrap(),
            Plan::EndChain
        );
        assert!(matches!(
            plan(Path::new("."), &definition("cli")).unwrap(),
            Plan::Launch { .. }
        ));
        assert!(plan(Path::new("."), &definition("ahk")).is_err());
        assert!(plan(Path::new("."), &definition("invalid")).is_err());
    }
    #[test]
    fn destructive_commands_need_explicit_selection() {
        assert!(!allows_approximate("computer_restart", "cli"));
        assert!(!allows_approximate("jarvis_restart", "restart"));
        assert!(allows_approximate("windows_minimize_all", "cli"));
    }
}
