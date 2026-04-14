//! Package registry resolution (ADR-023).
//!
//! Implements the third tier of import resolution:
//!   1. Local path
//!   2. Standard library
//!   3. Registry (this module)
//!
//! In v1, only a local file-based mock registry is supported. The mock is
//! configured with `url = "file://<path>"` in the project `[registry]` section
//! of `ark.toml`. Network registry access is a planned follow-up.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink, Span};
use ark_manifest::{DependencySpec, Manifest};

/// Sentinel path string returned by `resolve_registry_path` when an error was
/// already emitted and the rest of the resolver should skip this import.
pub(crate) const REGISTRY_ERROR_SENTINEL: &str = "<registry-error>";

/// Registry configuration derived from a project's `ark.toml`.
pub(crate) struct RegistryConfig {
    /// Absolute path to the local mock registry directory.
    /// `None` when the registry URL is a non-file URL (HTTP/S) or no `[registry]`
    /// section is present.
    pub local_base: Option<PathBuf>,
    /// Whether a `[registry]` section exists in `ark.toml`.
    pub configured: bool,
    /// The raw URL string for use in error messages.
    pub url: Option<String>,
    /// Package names that are registry dependencies (version-string entries in
    /// `[dependencies]`), mapped to their declared version constraint.
    pub packages: HashMap<String, String>,
}

impl RegistryConfig {
    /// Build a `RegistryConfig` from a `Manifest` rooted at `manifest_dir`.
    pub(crate) fn from_manifest(manifest: &Manifest, manifest_dir: &Path) -> Self {
        // Collect all version-string (registry) dependencies.
        let packages: HashMap<String, String> = manifest
            .dependencies
            .iter()
            .filter_map(|(name, spec)| {
                if let DependencySpec::Version(ver) = spec {
                    Some((name.clone(), ver.clone()))
                } else {
                    None
                }
            })
            .collect();

        let (configured, url, local_base) = match &manifest.registry {
            None => (false, None, None),
            Some(reg) => {
                let raw_url = reg.url.clone();
                let local_base = raw_url.as_deref().and_then(|u| parse_file_url(u, manifest_dir));
                (true, raw_url, local_base)
            }
        };

        Self {
            local_base,
            configured,
            url,
            packages,
        }
    }

    /// Return `true` if `module_name` is declared as a registry dependency.
    pub(crate) fn is_registry_dep(&self, module_name: &str) -> bool {
        self.packages.contains_key(module_name)
    }
}

/// Parse a `file://` URL into an absolute `PathBuf` relative to `base`.
///
/// Supports both `file:///abs/path` and `file://./rel/path` forms.
/// Returns `None` for non-file URLs.
fn parse_file_url(url: &str, base: &Path) -> Option<PathBuf> {
    let rest = url.strip_prefix("file://")?;
    let p = if rest.starts_with('/') {
        // Absolute: file:///abs/path
        PathBuf::from(rest)
    } else {
        // Relative: file://./rel or file://rel
        let stripped = rest.strip_prefix("./").unwrap_or(rest);
        base.join(stripped)
    };
    Some(p)
}

/// Resolve a registry package import path.
///
/// Called when a `use <module>` could not be satisfied by local or stdlib
/// paths and the module name is a declared registry dependency. Looks for:
///   `<local_base>/<module_name>.ark`
///   `<local_base>/<module_name>/mod.ark`
///
/// On error, emits the appropriate E012x diagnostic and returns the
/// `REGISTRY_ERROR_SENTINEL` path so the caller can skip loading.
pub(crate) fn resolve_registry_path(
    module_name: &str,
    config: &RegistryConfig,
    span: Span,
    sink: &mut DiagnosticSink,
) -> PathBuf {
    let version = config
        .packages
        .get(module_name)
        .map(|s| s.as_str())
        .unwrap_or("?");

    if !config.configured {
        sink.emit(
            Diagnostic::new(DiagnosticCode::E0124)
                .with_message(format!(
                    "no registry configured; add `[registry]` to ark.toml \
                     (needed to resolve registry dependency `{}`)",
                    module_name,
                ))
                .with_label(span, "registry dependency declared here"),
        );
        return PathBuf::from(REGISTRY_ERROR_SENTINEL);
    }

    let Some(base) = &config.local_base else {
        // Registry is configured with a non-file URL — network is not
        // implemented in v1 (local mock only).
        let url = config.url.as_deref().unwrap_or("<unknown>");
        sink.emit(
            Diagnostic::new(DiagnosticCode::E0120)
                .with_message(format!(
                    "registry unreachable: {} (network registry not supported in v1; \
                     use `url = \"file://./path\"` for a local mock registry)",
                    url,
                ))
                .with_label(span, "registry dependency"),
        );
        return PathBuf::from(REGISTRY_ERROR_SENTINEL);
    };

    // Try `<base>/<name>.ark` then `<base>/<name>/mod.ark`.
    let file_path = base.join(format!("{}.ark", module_name));
    if file_path.exists() {
        return file_path;
    }
    let mod_path = base.join(module_name).join("mod.ark");
    if mod_path.exists() {
        return mod_path;
    }

    // Package not found in the local mock registry.
    sink.emit(
        Diagnostic::new(DiagnosticCode::E0121)
            .with_message(format!(
                "package '{}' not found in registry (version '{}' declared in ark.toml)",
                module_name, version,
            ))
            .with_label(span, "registry package not found"),
    );
    PathBuf::from(REGISTRY_ERROR_SENTINEL)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_manifest::{DependencySpec, Manifest};
    use std::collections::HashMap;

    fn make_manifest_with_registry(
        dep_name: &str,
        dep_version: &str,
        registry_url: Option<&str>,
    ) -> Manifest {
        let toml = format!(
            r#"
[package]
name = "test-pkg"
version = "0.1.0"

[dependencies]
{dep_name} = "{dep_version}"
{}
"#,
            match registry_url {
                Some(url) => format!("[registry]\nurl = {:?}", url),
                None => String::new(),
            }
        );
        Manifest::from_toml(&toml).expect("valid toml")
    }

    #[test]
    fn test_is_registry_dep_detects_version_deps() {
        let m = make_manifest_with_registry("my_pkg", "1.0.0", None);
        let config = RegistryConfig::from_manifest(&m, Path::new("/tmp"));
        assert!(config.is_registry_dep("my_pkg"));
        assert!(!config.is_registry_dep("other_pkg"));
    }

    #[test]
    fn test_config_not_configured_without_registry_section() {
        let m = make_manifest_with_registry("my_pkg", "1.0.0", None);
        let config = RegistryConfig::from_manifest(&m, Path::new("/tmp"));
        assert!(!config.configured);
        assert!(config.local_base.is_none());
    }

    #[test]
    fn test_config_local_base_from_file_url() {
        let m = make_manifest_with_registry("my_pkg", "1.0.0", Some("file://./mock_reg"));
        let config = RegistryConfig::from_manifest(&m, Path::new("/project"));
        assert!(config.configured);
        assert_eq!(
            config.local_base.as_deref(),
            Some(Path::new("/project/mock_reg"))
        );
    }

    #[test]
    fn test_parse_file_url_absolute() {
        let result = parse_file_url("file:///abs/registry", Path::new("/ignored"));
        assert_eq!(result, Some(PathBuf::from("/abs/registry")));
    }

    #[test]
    fn test_parse_file_url_relative() {
        let result = parse_file_url("file://./mock", Path::new("/project"));
        assert_eq!(result, Some(PathBuf::from("/project/mock")));
    }

    #[test]
    fn test_parse_file_url_non_file_returns_none() {
        let result = parse_file_url("https://registry.example.com", Path::new("/project"));
        assert!(result.is_none());
    }
}
