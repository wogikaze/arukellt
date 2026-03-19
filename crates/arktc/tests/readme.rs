use std::fs;
use std::path::PathBuf;
use std::process::Command;

use tempfile::tempdir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn export_names(bytes: &[u8]) -> Vec<String> {
    let mut cursor = 8;
    while cursor < bytes.len() {
        let section_id = bytes[cursor];
        cursor += 1;
        let section_size = read_u32_leb(bytes, &mut cursor) as usize;
        let section_end = cursor + section_size;
        if section_id == 7 {
            let mut section_cursor = cursor;
            let export_count = read_u32_leb(bytes, &mut section_cursor);
            let mut exports = Vec::new();
            for _ in 0..export_count {
                let name_len = read_u32_leb(bytes, &mut section_cursor) as usize;
                let name = std::str::from_utf8(&bytes[section_cursor..section_cursor + name_len])
                    .expect("export name utf8")
                    .to_owned();
                section_cursor += name_len;
                section_cursor += 1;
                let _ = read_u32_leb(bytes, &mut section_cursor);
                exports.push(name);
            }
            return exports;
        }
        cursor = section_end;
    }
    Vec::new()
}

fn read_u32_leb(bytes: &[u8], cursor: &mut usize) -> u32 {
    let mut value = 0_u32;
    let mut shift = 0_u32;
    loop {
        let byte = bytes[*cursor];
        *cursor += 1;
        value |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return value;
        }
        shift += 7;
    }
}

#[test]
fn readme_check_command_example_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("check")
        .arg(repo_root().join("example/file_read.ar"))
        .arg("--json")
        .output()
        .expect("run arktc check");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["version"], "v0.1");
    assert_eq!(json["error_count"], 0);
}

#[test]
fn readme_build_commands_match_documented_wasm_contract() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("scalar.ar");
    fs::write(
        &source,
        "\
fn main() -> Int:
  42
",
    )
    .expect("write source");
    let wasm_js = dir.path().join("out-js.wasm");
    let wasm_wasi = dir.path().join("out-wasi.wasm");

    let js_output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&source)
        .arg("--target")
        .arg("wasm-js")
        .arg("--output")
        .arg(&wasm_js)
        .output()
        .expect("run wasm-js build");
    let wasi_output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&source)
        .arg("--target")
        .arg("wasm-wasi")
        .arg("--output")
        .arg(&wasm_wasi)
        .output()
        .expect("run wasm-wasi build");

    assert!(js_output.status.success(), "expected wasm-js build success");
    assert!(
        wasi_output.status.success(),
        "expected wasm-wasi build success"
    );

    let js_bytes = fs::read(&wasm_js).expect("read wasm-js");
    let wasi_bytes = fs::read(&wasm_wasi).expect("read wasm-wasi");
    assert_eq!(export_names(&js_bytes), vec!["main"]);
    assert_eq!(export_names(&wasi_bytes), vec!["_start"]);
}
