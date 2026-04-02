//! Standard library for Arukellt.
//!
//! Provides manifest-driven stdlib metadata: function descriptors,
//! module lists, and stability information parsed from `std/manifest.toml`.

use serde::Deserialize;
use std::collections::BTreeSet;

/// Standard library function descriptor (legacy API, kept for compatibility).
#[derive(Debug, Clone)]
pub struct StdlibFn {
    pub name: String,
    pub module: &'static str,
    pub wasi_import: Option<(&'static str, &'static str)>,
}

/// Get all stdlib function descriptors (legacy hardcoded list).
pub fn stdlib_functions() -> Vec<StdlibFn> {
    vec![
        StdlibFn {
            name: "println".into(),
            module: "prelude",
            wasi_import: None,
        },
        StdlibFn {
            name: "print".into(),
            module: "prelude",
            wasi_import: None,
        },
        StdlibFn {
            name: "eprintln".into(),
            module: "prelude",
            wasi_import: None,
        },
        StdlibFn {
            name: "i32_to_string".into(),
            module: "prelude",
            wasi_import: None,
        },
        StdlibFn {
            name: "i64_to_string".into(),
            module: "prelude",
            wasi_import: None,
        },
        StdlibFn {
            name: "f64_to_string".into(),
            module: "prelude",
            wasi_import: None,
        },
        StdlibFn {
            name: "parse_i32".into(),
            module: "prelude",
            wasi_import: None,
        },
        StdlibFn {
            name: "parse_i64".into(),
            module: "prelude",
            wasi_import: None,
        },
        StdlibFn {
            name: "to_string".into(),
            module: "prelude",
            wasi_import: None,
        },
        StdlibFn {
            name: "parse_f64".into(),
            module: "prelude",
            wasi_import: None,
        },
        StdlibFn {
            name: "Vec_new_i32".into(),
            module: "vec",
            wasi_import: None,
        },
        StdlibFn {
            name: "Vec_new_i64".into(),
            module: "vec",
            wasi_import: None,
        },
        StdlibFn {
            name: "Vec_new_f64".into(),
            module: "vec",
            wasi_import: None,
        },
        StdlibFn {
            name: "Vec_new_String".into(),
            module: "vec",
            wasi_import: None,
        },
        StdlibFn {
            name: "sqrt".into(),
            module: "prelude",
            wasi_import: None,
        },
        StdlibFn {
            name: "abs".into(),
            module: "prelude",
            wasi_import: None,
        },
        StdlibFn {
            name: "min".into(),
            module: "prelude",
            wasi_import: None,
        },
        StdlibFn {
            name: "max".into(),
            module: "prelude",
            wasi_import: None,
        },
    ]
}

// ---------------------------------------------------------------------------
// Manifest-driven stdlib metadata (canonical source: std/manifest.toml)
// ---------------------------------------------------------------------------

/// Parsed stdlib manifest.
#[derive(Debug, Clone, Deserialize)]
pub struct StdlibManifest {
    pub metadata: ManifestMetadata,
    #[serde(default)]
    pub types: Vec<ManifestType>,
    #[serde(default)]
    pub values: Vec<ManifestValue>,
    #[serde(default)]
    pub modules: Vec<ManifestModule>,
    #[serde(default)]
    pub functions: Vec<ManifestFunction>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestMetadata {
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestType {
    pub name: String,
    #[serde(default)]
    pub generic_params: Vec<String>,
    #[serde(default)]
    pub prelude: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestValue {
    pub name: String,
    #[serde(default)]
    pub prelude: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestModule {
    pub name: String,
    #[serde(default)]
    pub target: Vec<String>,
    #[serde(default)]
    pub stability: Option<String>,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestFunction {
    pub name: String,
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub params: Vec<String>,
    #[serde(default)]
    pub returns: Option<String>,
    #[serde(default)]
    pub prelude: bool,
    #[serde(default)]
    pub stability: Option<String>,
    #[serde(default)]
    pub intrinsic: Option<String>,
    #[serde(default)]
    pub doc_category: Option<String>,
    #[serde(default)]
    pub deprecated_by: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub see_also: Option<String>,
    #[serde(default)]
    pub target: Vec<String>,
}

impl StdlibManifest {
    /// Parse a stdlib manifest from TOML text.
    pub fn parse(toml_text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_text)
    }

    /// Load the stdlib manifest from the default location relative to repo root.
    pub fn load_from_repo(repo_root: &std::path::Path) -> Result<Self, String> {
        let path = repo_root.join("std/manifest.toml");
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
        Self::parse(&text).map_err(|e| format!("failed to parse {}: {}", path.display(), e))
    }

    /// Get sorted list of unique module names (e.g. "std::host::stdio").
    pub fn module_names(&self) -> Vec<String> {
        let mut names: BTreeSet<String> = BTreeSet::new();
        for m in &self.modules {
            names.insert(m.name.clone());
        }
        // Also collect modules referenced by functions but not declared as [[modules]]
        for f in &self.functions {
            if let Some(ref m) = f.module {
                if !m.is_empty() && m != "prelude" {
                    names.insert(m.clone());
                }
            }
        }
        names.into_iter().collect()
    }

    /// Get import candidates: (short_alias, full_module_path) pairs
    /// suitable for auto-import in the LSP.
    pub fn import_candidates(&self) -> Vec<(String, String)> {
        let mut candidates = Vec::new();
        // From declared [[modules]]
        for m in &self.modules {
            let alias = m.name.rsplit("::").next().unwrap_or(&m.name).to_string();
            candidates.push((alias, m.name.clone()));
        }
        // From function module references (covers modules without [[modules]] entry)
        for f in &self.functions {
            if let Some(ref m) = f.module {
                if !m.is_empty() && m != "prelude" {
                    let alias = m.rsplit("::").next().unwrap_or(m).to_string();
                    candidates.push((alias, m.clone()));
                }
            }
        }
        candidates.sort();
        candidates.dedup();
        candidates
    }

    /// Get all function names grouped by module.
    pub fn functions_by_module(&self) -> std::collections::BTreeMap<String, Vec<String>> {
        let mut map: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        for f in &self.functions {
            let module = f.module.clone().unwrap_or_else(|| "prelude".to_string());
            map.entry(module).or_default().push(f.name.clone());
        }
        map
    }

    /// Get all prelude function names.
    pub fn prelude_function_names(&self) -> Vec<String> {
        self.functions
            .iter()
            .filter(|f| f.prelude)
            .map(|f| f.name.clone())
            .collect()
    }

    /// Get the expansion family label for a module.
    pub fn family_for_module(&self, module_name: &str) -> Option<String> {
        self.modules
            .iter()
            .find(|m| m.name == module_name)
            .and_then(|m| m.family.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn parse_manifest() {
        let manifest = StdlibManifest::load_from_repo(&repo_root()).unwrap();
        assert!(!manifest.functions.is_empty(), "should have functions");
        assert!(!manifest.modules.is_empty(), "should have modules");
        assert!(!manifest.types.is_empty(), "should have types");
    }

    #[test]
    fn module_names_include_host_modules() {
        let manifest = StdlibManifest::load_from_repo(&repo_root()).unwrap();
        let names = manifest.module_names();
        assert!(
            names.contains(&"std::host::stdio".to_string()),
            "missing stdio, got: {:?}",
            names
        );
        assert!(names.contains(&"std::host::fs".to_string()), "missing fs");
    }

    #[test]
    fn import_candidates_match_modules() {
        let manifest = StdlibManifest::load_from_repo(&repo_root()).unwrap();
        let candidates = manifest.import_candidates();
        let aliases: Vec<&str> = candidates.iter().map(|(a, _)| a.as_str()).collect();
        assert!(aliases.contains(&"stdio"), "should have stdio");
        assert!(aliases.contains(&"fs"), "should have fs");
        // std::math is not a module in the manifest - math functions are in prelude
    }

    #[test]
    fn prelude_functions_are_nonempty() {
        let manifest = StdlibManifest::load_from_repo(&repo_root()).unwrap();
        let prelude = manifest.prelude_function_names();
        // Many core functions have prelude=true in the manifest
        assert!(
            prelude.len() > 10,
            "prelude should have many functions, got {}",
            prelude.len()
        );
        // String_from, eq, etc. are marked prelude in the manifest
        assert!(
            prelude.contains(&"String_from".to_string()),
            "String_from should be prelude"
        );
    }

    #[test]
    fn lsp_import_candidates_are_subset_of_manifest() {
        let manifest = StdlibManifest::load_from_repo(&repo_root()).unwrap();
        let manifest_modules: BTreeSet<String> = manifest.module_names().into_iter().collect();

        // Modules that the LSP hardcodes for auto-import.
        // Some are real manifest modules, some are virtual (e.g. std::math, std::string).
        // Only validate the ones that exist in the manifest.
        let lsp_real_modules = [
            "std::host::stdio",
            "std::host::fs",
            "std::host::env",
            "std::path",
            "std::time",
            "std::test",
            "std::host::process",
            "std::host::clock",
            "std::host::random",
        ];

        for module in lsp_real_modules {
            assert!(
                manifest_modules.contains(module),
                "LSP module '{}' not found in manifest modules: {:?}",
                module,
                manifest_modules
            );
        }
    }

    #[test]
    fn family_field_parsed() {
        let manifest = StdlibManifest::load_from_repo(&repo_root()).unwrap();
        // http and sockets have family = "expansion"
        assert_eq!(
            manifest.family_for_module("std::host::http"),
            Some("expansion".to_string()),
            "http should have expansion family"
        );
        assert_eq!(
            manifest.family_for_module("std::host::sockets"),
            Some("expansion".to_string()),
            "sockets should have expansion family"
        );
    }
}
