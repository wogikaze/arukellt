//! One-shot probe: `arukellt-debug-probe <wasm> <line> <repo_root> [source.ark]`

use arukellt_host_linker::{run_until_breakpoint, RuntimeCaps};
use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() < 3 || args.len() > 4 {
        eprintln!("usage: arukellt-debug-probe <wasm> <line> <repo_root> [source.ark]");
        process::exit(2);
    }
    let wasm_bytes = fs::read(&args[0]).unwrap_or_else(|e| {
        eprintln!("read wasm: {}", e);
        process::exit(1);
    });
    let line: u32 = args[1].parse().unwrap_or_else(|_| {
        eprintln!("invalid line");
        process::exit(2);
    });
    let caps = RuntimeCaps::from_cli(&[args[2].clone()]);
    let ark_path = if args.len() == 4 {
        Path::new(&args[3]).to_path_buf()
    } else {
        Path::new(&args[0]).with_extension("ark")
    };
    let source = fs::read_to_string(&ark_path).unwrap_or_else(|e| {
        eprintln!("read ark source {}: {}", ark_path.display(), e);
        process::exit(1);
    });
    let pause = run_until_breakpoint(&wasm_bytes, line, &caps, Some(&source)).unwrap_or_else(|e| {
        eprintln!("debug run failed: {}", e);
        process::exit(1);
    });
    let mut out = String::from("{\"source_line\":");
    out.push_str(&pause.source_line.to_string());
    out.push_str(",\"locals\":[");
    for (i, local) in pause.locals.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"name\":\"");
        out.push_str(&local.name);
        out.push_str("\",\"value\":\"");
        out.push_str(&local.value);
        out.push_str("\"}");
    }
    out.push_str("]}");
    println!("{}", out);
}
