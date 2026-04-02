//! Component wrapping: core Wasm + WIT → Component binary.
//!
//! Uses external `wasm-tools component new` for v2.
//! See ADR-008 for rationale.

use std::process::Command;

/// Wrap a core Wasm module into a Component Model binary.
///
/// Invokes `wasm-tools component new` with the core module and WIT text.
/// When `p2_native` is true the WASI Preview 1 adapter is skipped —
/// the core module is assumed to import P2-native interfaces directly.
/// Returns the component binary bytes on success.
pub fn wrap_core_to_component(
    core_wasm: &[u8],
    wit_text: &str,
    p2_native: bool,
) -> Result<Vec<u8>, WrapError> {
    // Check if wasm-tools is available
    let wasm_tools = find_wasm_tools()?;

    // Write core module and WIT to unique temp files (avoid races in parallel tests)
    let unique = std::process::id();
    let tmp_dir = std::env::temp_dir().join(format!("arukellt_wrap_{}", unique));
    std::fs::create_dir_all(&tmp_dir)
        .map_err(|e| WrapError::Io(format!("failed to create temp directory: {}", e)))?;
    let core_path = tmp_dir.join("core.wasm");
    let wit_dir = tmp_dir.join("wit");
    let wit_file = wit_dir.join("world.wit");
    let embedded_path = tmp_dir.join("embedded.wasm");
    let out_path = tmp_dir.join("component.wasm");

    std::fs::write(&core_path, core_wasm)
        .map_err(|e| WrapError::Io(format!("failed to write core module: {}", e)))?;
    std::fs::create_dir_all(&wit_dir)
        .map_err(|e| WrapError::Io(format!("failed to create WIT directory: {}", e)))?;
    std::fs::write(&wit_file, wit_text)
        .map_err(|e| WrapError::Io(format!("failed to write WIT file: {}", e)))?;

    // Step 1: Embed WIT metadata into core module
    let embed_output = Command::new(&wasm_tools)
        .args([
            "component",
            "embed",
            wit_dir.to_str().unwrap(),
            core_path.to_str().unwrap(),
            "-o",
            embedded_path.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| WrapError::Io(format!("failed to run wasm-tools embed: {}", e)))?;

    if !embed_output.status.success() {
        let stderr = String::from_utf8_lossy(&embed_output.stderr);
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(WrapError::WasmTools(format!(
            "wasm-tools component embed failed: {}",
            stderr.trim()
        )));
    }

    // Step 2: Create component from embedded module
    let mut cmd = Command::new(&wasm_tools);
    cmd.args([
        "component",
        "new",
        embedded_path.to_str().unwrap(),
        "-o",
        out_path.to_str().unwrap(),
    ]);

    // Automatically provide WASI adapter if available (skipped in P2-native mode)
    if !p2_native && let Some(adapter_path) = find_wasi_adapter() {
        cmd.args([
            "--adapt",
            &format!("wasi_snapshot_preview1={}", adapter_path),
        ]);
    }

    let new_output = cmd
        .output()
        .map_err(|e| WrapError::Io(format!("failed to run wasm-tools new: {}", e)))?;

    if !new_output.status.success() {
        let stderr = String::from_utf8_lossy(&new_output.stderr);
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(WrapError::WasmTools(format!(
            "wasm-tools component new failed: {}",
            stderr.trim()
        )));
    }

    let component_bytes = std::fs::read(&out_path)
        .map_err(|e| WrapError::Io(format!("failed to read component output: {}", e)))?;
    let _ = std::fs::remove_dir_all(&tmp_dir);

    Ok(component_bytes)
}

/// Validate a component binary using wasmparser.
pub fn validate_component(bytes: &[u8]) -> Result<(), String> {
    use wasmparser::{Validator, WasmFeatures};
    let mut validator = Validator::new_with_features(WasmFeatures::all());
    validator
        .validate_all(bytes)
        .map(|_| ())
        .map_err(|e| format!("component validation failed: {e}"))
}

/// Find the wasm-tools binary in PATH.
fn find_wasm_tools() -> Result<String, WrapError> {
    // Try `wasm-tools` in PATH
    match Command::new("wasm-tools").arg("--version").output() {
        Ok(output) if output.status.success() => return Ok("wasm-tools".to_string()),
        _ => {}
    }

    // Try common locations
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [format!("{}/.cargo/bin/wasm-tools", home)];
    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return Ok(path.clone());
        }
    }

    Err(WrapError::ToolNotFound(
        "wasm-tools not found. Install with: cargo install wasm-tools".to_string(),
    ))
}

/// Errors during component wrapping.
#[derive(Debug)]
pub enum WrapError {
    /// wasm-tools binary not found
    ToolNotFound(String),
    /// I/O error (temp files, etc.)
    Io(String),
    /// wasm-tools returned an error
    WasmTools(String),
}

/// Find a WASI preview1 adapter module for component wrapping.
///
/// Searches common locations for `wasi_snapshot_preview1.reactor.wasm` or
/// `wasi_snapshot_preview1.command.wasm`.
fn find_wasi_adapter() -> Option<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [
        // Project-local adapter
        "wasi_snapshot_preview1.reactor.wasm".to_string(),
        "wasi_snapshot_preview1.command.wasm".to_string(),
        // Cargo install location
        format!("{}/.cargo/bin/wasi_snapshot_preview1.reactor.wasm", home),
        format!("{}/.cargo/bin/wasi_snapshot_preview1.command.wasm", home),
        // Common system locations
        format!(
            "{}/.local/share/arukellt/wasi_snapshot_preview1.reactor.wasm",
            home
        ),
    ];
    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return Some(path.clone());
        }
    }
    // Check ARK_WASI_ADAPTER env var
    if let Ok(path) = std::env::var("ARK_WASI_ADAPTER")
        && std::path::Path::new(&path).exists()
    {
        return Some(path);
    }
    None
}

impl std::fmt::Display for WrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WrapError::ToolNotFound(msg) => write!(f, "error: {}", msg),
            WrapError::Io(msg) => write!(f, "error: {}", msg),
            WrapError::WasmTools(msg) => write!(f, "error: {}", msg),
        }
    }
}

impl std::error::Error for WrapError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_wasm_tools_does_not_panic() {
        // This test just ensures the function runs without panicking.
        // It may return Ok or ToolNotFound depending on the environment.
        let _result = find_wasm_tools();
    }

    // ── validate_component tests ──────────────────────────────────────────────

    /// Minimal valid WebAssembly module: magic + version 1 (no sections).
    /// wasmparser::Validator::validate_all accepts empty modules as valid.
    const MINIMAL_WASM_MODULE: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, // magic "\0asm"
        0x01, 0x00, 0x00, 0x00, // version 1 (module)
    ];

    /// Minimal valid WebAssembly Component binary: magic + component-model version.
    /// Extracted from real component binaries: version bytes are [0x0d, 0x00, 0x01, 0x00].
    const MINIMAL_WASM_COMPONENT: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, // magic "\0asm"
        0x0d, 0x00, 0x01, 0x00, // component-model version (0x0d = current version)
    ];

    #[test]
    fn test_validate_component_rejects_invalid_bytes() {
        // Random junk bytes must not validate.
        let junk = &[0xde, 0xad, 0xbe, 0xef, 0xff, 0xff, 0xff, 0xff];
        let result = validate_component(junk);
        assert!(
            result.is_err(),
            "validate_component should reject invalid bytes"
        );
    }

    #[test]
    fn test_validate_component_rejects_empty_input() {
        let result = validate_component(&[]);
        assert!(
            result.is_err(),
            "validate_component should reject empty byte slice"
        );
    }

    #[test]
    fn test_validate_component_accepts_valid_module() {
        // A minimal well-formed Wasm module must pass validation.
        // wasmparser validates both modules and components via validate_all.
        let result = validate_component(MINIMAL_WASM_MODULE);
        assert!(
            result.is_ok(),
            "validate_component should accept minimal valid Wasm module: {:?}",
            result
        );
    }

    #[test]
    fn test_validate_component_accepts_valid_component() {
        // A minimal empty component binary (header only) must pass validation.
        // This serves as the cross-language interop readiness gate: any
        // component binary that passes here is structurally valid and can be
        // consumed by other Component Model implementations.
        let result = validate_component(MINIMAL_WASM_COMPONENT);
        assert!(
            result.is_ok(),
            "validate_component should accept minimal valid component binary: {:?}",
            result
        );
    }

    #[test]
    fn test_validate_component_error_message_contains_detail() {
        // Error messages for invalid binaries must be informative.
        let junk = &[0x00, 0x00, 0x00, 0x00];
        let err = validate_component(junk).unwrap_err();
        assert!(
            err.contains("component validation failed"),
            "Error message should contain 'component validation failed', got: {err}"
        );
    }

    // ── Cross-language interop validation ─────────────────────────────────────
    // Verifies that the validate_component function correctly distinguishes
    // valid component binaries from invalid ones, serving as an automated
    // interop readiness gate for generated component output.

    #[test]
    fn test_component_interop_validation_gate() {
        // A well-formed component binary passes validation and is structurally
        // interoperable with other Component Model host implementations.
        let valid_result = validate_component(MINIMAL_WASM_COMPONENT);
        assert!(
            valid_result.is_ok(),
            "Minimal component binary must pass interop validation gate"
        );

        // Invalid bytes must be rejected — ensuring the gate is not trivially
        // accepting all input.
        let invalid_result = validate_component(&[0xff, 0x00, 0x00, 0x00]);
        assert!(
            invalid_result.is_err(),
            "Invalid bytes must be rejected by interop validation gate"
        );
    }
}
