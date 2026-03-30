use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Manifest {
    pub package: Package,
    #[serde(default)]
    pub scripts: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub authors: Vec<String>,
    pub description: Option<String>,
}

#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("manifest not found")]
    NotFound,
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
}
