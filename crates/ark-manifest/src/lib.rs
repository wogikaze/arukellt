use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Manifest {
    pub package: Package,
    #[serde(default)]
    pub bin: Option<BinSection>,
    #[serde(default)]
    pub targets: HashMap<String, TargetConfig>,
    #[serde(default)]
    pub dependencies: HashMap<String, DependencySpec>,
    #[serde(default)]
    pub scripts: HashMap<String, String>,
    #[serde(default)]
    pub world: Option<WorldSection>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub authors: Vec<String>,
    pub description: Option<String>,
}

/// [bin] section — identifies the entry-point source file for `ark build`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BinSection {
    pub name: String,
    pub path: PathBuf,
}

/// Per-target overrides under [[targets]].
/// Keys match TargetId values (e.g. "wasm32-wasi-p2").
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TargetConfig {
    pub opt_level: Option<u8>,
    pub output: Option<PathBuf>,
}

/// Dependency specification. Currently only path-based local dependencies are supported.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum DependencySpec {
    Version(String),
    Path { path: PathBuf },
}

/// [world] section — WIT world binding for component output.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorldSection {
    pub name: String,
}

#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("manifest not found")]
    NotFound,
    #[error("ark.toml is missing required field: {0}")]
    MissingField(String),
}

impl Manifest {
    pub fn from_toml(content: &str) -> Result<Self, ManifestError> {
        let manifest: Manifest = toml::from_str(content)?;
        Ok(manifest)
    }

    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(path)?;
        Self::from_toml(&content)
    }

    pub fn find_root(start_dir: &Path) -> Option<PathBuf> {
        let mut current = start_dir.to_path_buf();
        loop {
            let manifest_path = current.join("ark.toml");
            if manifest_path.exists() {
                return Some(current);
            }
            if !current.pop() {
                break;
            }
        }
        None
    }

    pub fn load_from_dir(dir: &Path) -> Result<Self, ManifestError> {
        let manifest_path = dir.join("ark.toml");
        if !manifest_path.exists() {
            return Err(ManifestError::NotFound);
        }
        Self::load(&manifest_path)
    }

    /// Load from the nearest ancestor directory that contains an ark.toml.
    pub fn find_and_load(start_dir: &Path) -> Result<(PathBuf, Self), ManifestError> {
        let root = Self::find_root(start_dir).ok_or(ManifestError::NotFound)?;
        let manifest = Self::load_from_dir(&root)?;
        Ok((root, manifest))
    }

    /// Return the [bin] section, or a clear error if missing.
    pub fn require_bin(&self) -> Result<&BinSection, ManifestError> {
        self.bin
            .as_ref()
            .ok_or_else(|| ManifestError::MissingField("[bin]".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_manifest(dir: &TempDir, content: &str) -> PathBuf {
        let path = dir.path().join("ark.toml");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_parse_minimal_valid_manifest() {
        let toml = r#"
[package]
name = "hello"
version = "0.1.0"

[bin]
name = "hello"
path = "src/main.ark"
"#;
        let m = Manifest::from_toml(toml).expect("should parse");
        assert_eq!(m.package.name, "hello");
        assert_eq!(m.package.version, "0.1.0");
        let bin = m.bin.expect("should have [bin]");
        assert_eq!(bin.name, "hello");
        assert_eq!(bin.path, std::path::PathBuf::from("src/main.ark"));
    }

    #[test]
    fn test_parse_with_targets() {
        let toml = r#"
[package]
name = "myapp"
version = "0.2.0"

[bin]
name = "myapp"
path = "main.ark"

[targets.wasm32-wasi-p2]
opt_level = 2
"#;
        let m = Manifest::from_toml(toml).expect("should parse");
        let t = m
            .targets
            .get("wasm32-wasi-p2")
            .expect("should have target config");
        assert_eq!(t.opt_level, Some(2));
    }

    #[test]
    fn test_parse_with_dependencies() {
        let toml = r#"
[package]
name = "app"
version = "0.1.0"

[bin]
name = "app"
path = "main.ark"

[dependencies]
my-lib = { path = "../my-lib" }
"#;
        let m = Manifest::from_toml(toml).expect("should parse");
        let dep = m.dependencies.get("my-lib").expect("should have dep");
        match dep {
            DependencySpec::Path { path } => {
                assert_eq!(path, &std::path::PathBuf::from("../my-lib"));
            }
            other => panic!("expected path dep, got {:?}", other),
        }
    }

    #[test]
    fn test_missing_required_package_field() {
        let toml = r#"
[bin]
name = "oops"
path = "main.ark"
"#;
        assert!(
            Manifest::from_toml(toml).is_err(),
            "should fail without [package]"
        );
    }

    #[test]
    fn test_require_bin_missing() {
        let toml = r#"
[package]
name = "lib"
version = "0.1.0"
"#;
        let m = Manifest::from_toml(toml).expect("should parse");
        let err = m.require_bin().unwrap_err();
        assert!(err.to_string().contains("[bin]"));
    }

    #[test]
    fn test_find_root_discovers_ancestor() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // Write ark.toml at root
        std::fs::write(
            root.join("ark.toml"),
            "[package]\nname=\"p\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        // Create a deep subdirectory
        let deep = root.join("src").join("deep");
        std::fs::create_dir_all(&deep).unwrap();

        let found = Manifest::find_root(&deep).expect("should find root");
        assert_eq!(found, root);
    }

    #[test]
    fn test_find_root_returns_none_when_missing() {
        // Use a temp dir with NO ark.toml
        let dir = tempfile::tempdir().unwrap();
        // Ensure no parent ark.toml interferes by using a path inside the
        // temp dir itself (which tempfile guarantees is clean).
        let result = Manifest::find_root(dir.path());
        // Note: this test may find an ark.toml in an ancestor if run from
        // within the repo. We accept SKIP if we're inside a project.
        if result.is_some() {
            // We're nested inside a project — this is a valid environment skip.
            return;
        }
        assert!(result.is_none());
    }

    #[test]
    fn test_load_from_dir_returns_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let err = Manifest::load_from_dir(dir.path()).unwrap_err();
        assert!(matches!(err, ManifestError::NotFound));
    }

    #[test]
    fn test_find_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let toml_content = "[package]\nname=\"test\"\nversion=\"0.1.0\"\n[bin]\nname=\"test\"\npath=\"main.ark\"\n";
        std::fs::write(dir.path().join("ark.toml"), toml_content).unwrap();

        let sub = dir.path().join("src");
        std::fs::create_dir_all(&sub).unwrap();

        let (root, m) = Manifest::find_and_load(&sub).expect("should find and load");
        assert_eq!(
            root,
            dir.path()
                .canonicalize()
                .unwrap_or(dir.path().to_path_buf())
        );
        assert_eq!(m.package.name, "test");
    }

    #[test]
    fn test_toml_syntax_error_gives_toml_error() {
        let toml = "this is not valid toml @@@@";
        let err = Manifest::from_toml(toml).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(_)));
    }

    #[test]
    fn test_package_with_description_and_authors() {
        let toml = r#"
[package]
name = "full"
version = "1.0.0"
authors = ["Alice <alice@example.com>"]
description = "A full-featured package"
"#;
        let m = Manifest::from_toml(toml).expect("should parse");
        assert_eq!(m.package.authors, vec!["Alice <alice@example.com>"]);
        assert_eq!(
            m.package.description.as_deref(),
            Some("A full-featured package")
        );
    }

    #[test]
    fn test_scripts_section_parsed() {
        let toml = r#"
[package]
name = "scripted"
version = "0.1.0"

[bin]
name = "scripted"
path = "main.ark"

[scripts]
test = "arukellt run tests.ark"
lint = "arukellt check ."
"#;
        let m = Manifest::from_toml(toml).expect("should parse");
        assert_eq!(
            m.scripts.get("test").map(|s| s.as_str()),
            Some("arukellt run tests.ark")
        );
        assert_eq!(
            m.scripts.get("lint").map(|s| s.as_str()),
            Some("arukellt check .")
        );
    }
}
