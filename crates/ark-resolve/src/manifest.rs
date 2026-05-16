//! `ark.toml` manifest parsing.
//!
//! This module was migrated from `crates/ark-manifest` when that crate was
//! retired in #580 (Phase 7 of #529).  `ark-resolve` is the sole consumer.

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
    #[serde(default)]
    pub lint: Option<LintConfig>,
    #[serde(default)]
    pub registry: Option<RegistrySection>,
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

/// [registry] section — package registry configuration.
///
/// The `url` field specifies the registry endpoint. Use `file://<path>` for a
/// local directory-based mock registry (useful for offline testing).
/// HTTP(S) URLs refer to a real registry service.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RegistrySection {
    /// Registry URL. May be:
    /// - `https://registry.arukellt.dev/v1` (default)
    /// - `file://./path/to/local/dir` (local mock, relative to manifest dir)
    pub url: Option<String>,
}

/// [lint] section — per-project lint rule severity overrides.
///
/// Rules are identified by their diagnostic code (e.g. "W0006") or a
/// human-readable alias (e.g. "unused-import").
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LintConfig {
    /// Rules to suppress (no diagnostic emitted).
    #[serde(default)]
    pub allow: Vec<String>,
    /// Rules to report as warnings (default for most lint rules).
    #[serde(default)]
    pub warn: Vec<String>,
    /// Rules to report as hard errors (compilation fails).
    #[serde(default)]
    pub deny: Vec<String>,
}

impl LintConfig {
    /// Returns the configured severity for a rule, or None if not configured.
    pub fn severity_for(&self, rule: &str) -> Option<LintLevel> {
        if self.allow.iter().any(|r| r == rule) {
            Some(LintLevel::Allow)
        } else if self.warn.iter().any(|r| r == rule) {
            Some(LintLevel::Warn)
        } else if self.deny.iter().any(|r| r == rule) {
            Some(LintLevel::Deny)
        } else {
            None
        }
    }
}

/// Lint severity levels configurable via ark.toml.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintLevel {
    Allow,
    Warn,
    Deny,
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

    /// Discover the starting package root plus all cycle-safe path dependency
    /// package roots reachable through `[dependencies]`.
    pub fn discover_path_dependency_roots(root: &Path) -> Result<Vec<PathBuf>, ManifestError> {
        let mut ordered_roots = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut pending = vec![root.to_path_buf()];

        while let Some(next_root) = pending.pop() {
            let canonical = std::fs::canonicalize(&next_root).unwrap_or_else(|_| next_root.clone());
            if !visited.insert(canonical) {
                continue;
            }

            let normalized_root =
                std::fs::canonicalize(&next_root).unwrap_or_else(|_| next_root.clone());
            let manifest = Self::load_from_dir(&normalized_root)?;
            ordered_roots.push(normalized_root.clone());

            for dep in manifest.dependencies.values() {
                let DependencySpec::Path { path } = dep else {
                    continue;
                };

                let dep_start = normalized_root.join(path);
                if let Some(dep_root) = Self::find_root(&dep_start) {
                    pending.push(dep_root);
                }
            }
        }

        Ok(ordered_roots)
    }

    /// Return the [bin] section, or a clear error if missing.
    pub fn require_bin(&self) -> Result<&BinSection, ManifestError> {
        self.bin
            .as_ref()
            .ok_or_else(|| ManifestError::MissingField("[bin]".to_string()))
    }
}
