use serde::Deserialize;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Pack<T> {
    pub directory: PathBuf,
    pub definitions: Vec<T>,
}

#[derive(Debug)]
pub struct Catalog<T> {
    pub packs: Vec<Pack<T>>,
    pub warnings: Vec<String>,
}

#[derive(Deserialize)]
struct Manifest<T> {
    commands: Vec<T>,
}

/// The manifest schema belongs to the caller, not to this kernel.
/// Invalid individual packs are reported without disabling valid packs.
pub fn load<T: serde::de::DeserializeOwned>(directory: &Path) -> io::Result<Catalog<T>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() && path.join("command.toml").is_file() {
            paths.push(path);
        }
    }
    paths.sort();
    let mut result = Catalog {
        packs: Vec::new(),
        warnings: Vec::new(),
    };
    for path in paths {
        let manifest = path.join("command.toml");
        let parsed = fs::read_to_string(&manifest)
            .map_err(|e| e.to_string())
            .and_then(|text| toml::from_str::<Manifest<T>>(&text).map_err(|e| e.to_string()));
        match parsed {
            Ok(pack) => result.packs.push(Pack {
                directory: path,
                definitions: pack.commands,
            }),
            Err(error) => result
                .warnings
                .push(format!("{}: {error}", manifest.display())),
        }
    }
    if result.packs.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "No valid command packs",
        ));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, Deserialize)]
    struct TestCommand {
        id: String,
    }

    #[test]
    fn reports_invalid_packs_and_keeps_deterministic_order() {
        let root = tempfile::tempdir().unwrap();
        for name in ["z", "broken", "a"] {
            let path = root.path().join(name);
            fs::create_dir(&path).unwrap();
            fs::write(
                path.join("command.toml"),
                if name == "broken" {
                    "[[commands]]\nid = "
                } else {
                    "[[commands]]\nid = 'test'"
                },
            )
            .unwrap();
        }
        let result = load::<TestCommand>(root.path()).unwrap();
        assert_eq!(result.packs.len(), 2);
        assert!(result.packs[0].directory.ends_with("a"));
        assert_eq!(result.packs[0].definitions[0].id, "test");
        assert_eq!(result.warnings.len(), 1);
    }
}
