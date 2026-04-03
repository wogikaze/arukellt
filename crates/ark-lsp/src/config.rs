//! LSP initialization configuration.
//!
//! [`LspConfig`] is populated from the `initializationOptions` block sent by
//! the VS Code extension (issue #478) and re-sent on
//! `workspace/didChangeConfiguration`.  The five keys mirror the settings
//! declared in `package.json` (#477):
//!
//! | JSON key                   | Field                    | Default  |
//! |----------------------------|--------------------------|----------|
//! | `enableCodeLens`           | `enable_code_lens`       | `true`   |
//! | `hoverDetailLevel`         | `hover_detail_level`     | `"normal"` |
//! | `diagnosticsReportLevel`   | `diagnostics_report_level` | `"all"` |
//! | `useSelfHostBackend`       | `use_self_host_backend`  | `false`  |
//! | `checkOnSave`              | `check_on_save`          | `true`   |
//!
//! This struct exists for **parsing and storage only** — behavioral changes
//! driven by these settings are introduced in subsequent issues.

/// Parsed representation of the five extension-side initialization settings.
///
/// Constructed via [`LspConfig::from_initialization_options`] which is called
/// inside the `initialize` request handler.  All fields have sensible
/// defaults so a missing key is never an error.
#[derive(Debug, Clone, PartialEq)]
pub struct LspConfig {
    /// When `false` the server should return an empty array for all
    /// `textDocument/codeLens` requests (CodeLens feature disabled).
    /// Corresponds to `arukellt.enableCodeLens`.
    pub enable_code_lens: bool,

    /// Controls hover response verbosity.
    /// Valid values: `"minimal"`, `"normal"`, `"verbose"`.
    /// Corresponds to `arukellt.hoverDetailLevel`.
    pub hover_detail_level: String,

    /// Controls which diagnostic severity levels are forwarded to the editor.
    /// Valid values: `"error"`, `"warning"`, `"hint"`, `"all"`.
    /// Corresponds to `arukellt.diagnostics.reportLevel`.
    pub diagnostics_report_level: String,

    /// When `true` the server should prefer the self-hosted (ark-compiled)
    /// backend once it reaches Stage 2 fixpoint.  Currently a no-op.
    /// Corresponds to `arukellt.useSelfHostBackend`.
    pub use_self_host_backend: bool,

    /// When `true` the server triggers a diagnostics pass whenever the user
    /// saves a file (in addition to the normal `didChange` trigger).
    /// Corresponds to `arukellt.checkOnSave`.
    pub check_on_save: bool,
}

impl Default for LspConfig {
    fn default() -> Self {
        LspConfig {
            enable_code_lens: true,
            hover_detail_level: "normal".to_string(),
            diagnostics_report_level: "all".to_string(),
            use_self_host_backend: false,
            check_on_save: true,
        }
    }
}

impl LspConfig {
    /// Parse an [`LspConfig`] from the JSON value provided as
    /// `initializationOptions` in the LSP `initialize` request (or from the
    /// `settings` object in `workspace/didChangeConfiguration`).
    ///
    /// Unknown or missing keys are silently ignored; the corresponding field
    /// retains its default value.
    pub fn from_initialization_options(value: &serde_json::Value) -> Self {
        let mut cfg = LspConfig::default();

        if let Some(b) = value.get("enableCodeLens").and_then(|v| v.as_bool()) {
            cfg.enable_code_lens = b;
        }
        if let Some(s) = value.get("hoverDetailLevel").and_then(|v| v.as_str()) {
            cfg.hover_detail_level = s.to_string();
        }
        if let Some(s) = value.get("diagnosticsReportLevel").and_then(|v| v.as_str()) {
            cfg.diagnostics_report_level = s.to_string();
        }
        if let Some(b) = value.get("useSelfHostBackend").and_then(|v| v.as_bool()) {
            cfg.use_self_host_backend = b;
        }
        if let Some(b) = value.get("checkOnSave").and_then(|v| v.as_bool()) {
            cfg.check_on_save = b;
        }

        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_values_are_sensible() {
        let cfg = LspConfig::default();
        assert!(cfg.enable_code_lens);
        assert_eq!(cfg.hover_detail_level, "normal");
        assert_eq!(cfg.diagnostics_report_level, "all");
        assert!(!cfg.use_self_host_backend);
        assert!(cfg.check_on_save);
    }

    #[test]
    fn from_initialization_options_parses_all_five_keys() {
        let opts = json!({
            "enableCodeLens": false,
            "hoverDetailLevel": "verbose",
            "diagnosticsReportLevel": "error",
            "useSelfHostBackend": true,
            "checkOnSave": false
        });
        let cfg = LspConfig::from_initialization_options(&opts);
        assert!(!cfg.enable_code_lens);
        assert_eq!(cfg.hover_detail_level, "verbose");
        assert_eq!(cfg.diagnostics_report_level, "error");
        assert!(cfg.use_self_host_backend);
        assert!(!cfg.check_on_save);
    }

    #[test]
    fn missing_keys_keep_defaults() {
        let opts = json!({ "checkOnSave": false });
        let cfg = LspConfig::from_initialization_options(&opts);
        // Only checkOnSave was overridden
        assert!(cfg.enable_code_lens);
        assert_eq!(cfg.hover_detail_level, "normal");
        assert_eq!(cfg.diagnostics_report_level, "all");
        assert!(!cfg.use_self_host_backend);
        assert!(!cfg.check_on_save);
    }

    #[test]
    fn empty_object_yields_defaults() {
        let opts = json!({});
        let cfg = LspConfig::from_initialization_options(&opts);
        assert_eq!(cfg, LspConfig::default());
    }

    #[test]
    fn from_initialization_options_null_value_yields_defaults() {
        let opts = serde_json::Value::Null;
        let cfg = LspConfig::from_initialization_options(&opts);
        assert_eq!(cfg, LspConfig::default());
    }
}
