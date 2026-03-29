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
}
