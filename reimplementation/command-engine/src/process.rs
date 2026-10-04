use std::{
    io,
    path::{Path, PathBuf},
    process::{Child, Command},
};

/// Pass each argument literally, never through an implicit cmd/sh wrapper.
pub fn launch(program: &Path, arguments: &[String]) -> io::Result<Child> {
    if program.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Empty executable path",
        ));
    }
    let mut command = Command::new(program);
    command.args(arguments);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command.spawn()
}

pub fn locate(pack: &Path, configured: &str) -> io::Result<PathBuf> {
    let requested = Path::new(configured);
    if configured.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Executable not configured",
        ));
    }
    let resolved = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        pack.join(requested)
    };
    if !resolved.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Executable not found: {}", resolved.display()),
        ));
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lookup_reports_missing_file_without_starting_process() {
        let root = tempfile::tempdir().unwrap();
        assert!(locate(root.path(), "missing.exe").is_err());
        assert!(launch(Path::new(""), &[]).is_err());
    }
    #[test]
    fn keeps_separate_arguments_without_shell_expansion() {
        #[cfg(windows)]
        let program = Path::new("powershell.exe");
        #[cfg(not(windows))]
        let program = Path::new("printf");
        #[cfg(windows)]
        let root = tempfile::tempdir().unwrap();
        #[cfg(windows)]
        let script = root.path().join("literal.ps1");
        #[cfg(windows)]
        std::fs::write(
            &script,
            "param([string] $inputText)\nif ($inputText -ne 'a & b | c > d') { exit 2 }\nexit 0\n",
        )
        .unwrap();
        #[cfg(windows)]
        let args = vec![
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-File".into(),
            script.to_string_lossy().into_owned(),
            "-inputText".into(),
            "a & b | c > d".into(),
        ];
        #[cfg(not(windows))]
        let args = vec!["%s".into(), "a & b".into()];
        assert!(launch(program, &args).unwrap().wait().unwrap().success());
    }
}
