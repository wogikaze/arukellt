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
