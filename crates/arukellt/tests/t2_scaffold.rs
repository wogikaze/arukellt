use std::path::{Path, PathBuf};
use std::process::Command;

use ark_target::TargetId;
use wasmparser::{ExternalKind, Parser, Payload, Validator};

fn arukellt_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_arukellt")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("target")
                .join("debug")
                .join("arukellt")
        })
}

#[test]
fn t2_scaffold_emits_valid_core_wasm_for_empty_fixture() {
    let profile = TargetId::Wasm32Freestanding.profile();
    assert!(
        profile.implemented,
        "wasm32-freestanding should remain exposed as an implemented compile-only target"
    );
    assert!(
        !profile.run_supported,
        "wasm32-freestanding scaffold must stay compile-only until runtime wiring exists"
    );

    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let fixture = workspace_root.join("tests/fixtures/regression/t2_scaffold.ark");
    let out_dir = tempfile::tempdir().expect("failed to create tempdir");
    let output_path = out_dir.path().join("t2_scaffold.wasm");

    let output = Command::new(arukellt_bin())
        .arg("compile")
        .arg("--target")
        .arg("wasm32-freestanding")
        .arg(&fixture)
        .arg("-o")
        .arg(&output_path)
        .output()
        .expect("failed to run arukellt compile");

    assert!(
        output.status.success(),
        "T2 compile failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let bytes = std::fs::read(&output_path).expect("failed to read emitted wasm");
    Validator::new()
        .validate_all(&bytes)
        .expect("emitted T2 wasm must validate");

    let mut imports = Vec::new();
    let mut exports = Vec::new();
    for payload in Parser::new(0).parse_all(&bytes) {
        match payload.expect("parser should accept scaffold wasm") {
            Payload::ImportSection(reader) => {
                for import in reader {
                    let import = import.expect("import entry should parse");
                    imports.push(format!("{}::{}", import.module, import.name));
                }
            }
            Payload::ExportSection(reader) => {
                for export in reader {
                    let export = export.expect("export entry should parse");
                    exports.push((export.name.to_string(), export.kind));
                }
            }
            _ => {}
        }
    }

    assert!(imports.is_empty(), "T2 scaffold must not import WASI");
    assert!(
        exports
            .iter()
            .any(|(name, kind)| name == "memory" && *kind == ExternalKind::Memory),
        "T2 scaffold should export linear memory"
    );
    assert!(
        exports
            .iter()
            .any(|(name, kind)| name == "_start" && *kind == ExternalKind::Func),
        "T2 scaffold should export _start"
    );
}
