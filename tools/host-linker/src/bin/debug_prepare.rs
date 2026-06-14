//! Write patched debug wasm: `arukellt-debug-prepare <wasm-in> <source.ark> <line> <wasm-out>`

use arukellt_host_linker::prepare_debug_wasm;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 4 {
        eprintln!("usage: arukellt-debug-prepare <wasm-in> <source.ark> <line> <wasm-out>");
        process::exit(2);
    }
    let raw = fs::read(&args[0]).unwrap_or_else(|e| {
        eprintln!("read wasm: {}", e);
        process::exit(1);
    });
    let source = fs::read_to_string(&args[1]).unwrap_or_else(|e| {
        eprintln!("read source: {}", e);
        process::exit(1);
    });
    let line: u32 = args[2].parse().unwrap_or_else(|_| {
        eprintln!("invalid line");
        process::exit(2);
    });
    let out = prepare_debug_wasm(&raw, &source, line).unwrap_or_else(|e| {
        eprintln!("prepare: {}", e);
        process::exit(1);
    });
    fs::write(&args[3], out).unwrap_or_else(|e| {
        eprintln!("write: {}", e);
        process::exit(1);
    });
}
