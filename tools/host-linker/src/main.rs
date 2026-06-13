//! Run user Wasm modules with WASI Preview 1 and conditional `arukellt_host` imports.

use arukellt_host_linker::{run_wasm, RuntimeCaps};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut dirs: Vec<String> = Vec::new();
    let mut wasm_path: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--dir" {
            if i + 1 >= args.len() {
                eprintln!("arukellt-host-run: --dir requires an argument");
                process::exit(2);
            }
            dirs.push(args[i + 1].clone());
            i += 2;
            continue;
        }
        if args[i].starts_with("--dir=") {
            dirs.push(args[i]["--dir=".len()..].to_string());
            i += 1;
            continue;
        }
        if args[i].starts_with('-') {
            eprintln!("arukellt-host-run: unknown flag: {}", args[i]);
            process::exit(2);
        }
        wasm_path = Some(PathBuf::from(&args[i]));
        i += 1;
    }

    let wasm_path = match wasm_path {
        Some(p) => p,
        None => {
            eprintln!("usage: arukellt-host-run [--dir <path>] <module.wasm>");
            process::exit(2);
        }
    };

    let wasm_bytes = match fs::read(&wasm_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("arukellt-host-run: failed to read {}: {}", wasm_path.display(), e);
            process::exit(1);
        }
    };

    let caps = RuntimeCaps::from_cli(&dirs);
    if let Err(e) = run_wasm(&wasm_bytes, &caps) {
        if !e.is_empty() {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}
