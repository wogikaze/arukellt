//! Component wrapping: core Wasm + WIT → Component binary.
//!
//! Uses external `wasm-tools component new` for v2.
//! See ADR-008 for rationale.

use std::process::Command;

/// Wrap a core Wasm module into a Component Model binary.
///
/// Invokes `wasm-tools component new` with the core module and WIT text.
/// Returns the component binary bytes on success.
pub fn wrap_core_to_component(
    core_wasm: &[u8],
    wit_text: &str,
) -> Result<Vec<u8>, WrapError> {
    // Check if wasm-tools is available
    let wasm_tools = find_wasm_tools()?;

    // Write core module to temp file
    let tmp_dir = std::env::temp_dir();
    let core_path = tmp_dir.join("arukellt_core.wasm");
    let wit_path = tmp_dir.join("arukellt_component.wit");
    let out_path = tmp_dir.join("arukellt_component.wasm");

    std::fs::write(&core_path, core_wasm).map_err(|e| {
        WrapError::Io(format!("failed to write core module: {}", e))
    })?;
    std::fs::write(&wit_path, wit_text).map_err(|e| {
        WrapError::Io(format!("failed to write WIT file: {}", e))
    })?;

    // Invoke wasm-tools component new
    let output = Command::new(&wasm_tools)
        .args([
            "component",
            "new",
            core_path.to_str().unwrap(),
            "--wit",
            wit_path.to_str().unwrap(),
            "-o",
            out_path.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| WrapError::Io(format!("failed to run wasm-tools: {}", e)))?;

    // Clean up temp files
    let _ = std::fs::remove_file(&core_path);
    let _ = std::fs::remove_file(&wit_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = std::fs::remove_file(&out_path);
        return Err(WrapError::WasmTools(format!(
            "wasm-tools component new failed: {}",
            stderr.trim()
        )));
    }

    let component_bytes = std::fs::read(&out_path).map_err(|e| {
        WrapError::Io(format!("failed to read component output: {}", e))
    })?;
    let _ = std::fs::remove_file(&out_path);

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
    let candidates = [
        format!("{}/.cargo/bin/wasm-tools", home),
    ];
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
}
