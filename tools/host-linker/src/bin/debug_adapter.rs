//! Minimal DAP server backed by wasmtime debug hooks (#638).

use arukellt_host_linker::{run_until_breakpoint, RuntimeCaps};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct DapState {
    seq: i32,
    program: Option<PathBuf>,
    breakpoints: Vec<i32>,
    paused_line: Option<i32>,
    live_locals: Vec<(String, String)>,
    repo_root: PathBuf,
}

impl DapState {
    fn new(repo_root: PathBuf) -> Self {
        Self {
            seq: 1,
            program: None,
            breakpoints: Vec::new(),
            paused_line: None,
            live_locals: Vec::new(),
            repo_root,
        }
    }

    fn next_seq(&mut self) -> i32 {
        let s = self.seq;
        self.seq += 1;
        s
    }
}

fn main() {
    let repo_root = std::env::var("ARUKELLT_REPO_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                    .map(|d| d.join("../../.."))
                    .unwrap_or_else(|| PathBuf::from(".")),
            )
        });
    let mut state = DapState::new(repo_root);
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut buffer = String::new();
    let mut input = String::new();
    while stdin.lock().read_line(&mut buffer).unwrap_or(0) > 0 {
        input.push_str(&buffer);
        buffer.clear();
        while let Some((msg, rest)) = take_frame(&input) {
            input = rest;
            if let Ok(value) = serde_json::from_str::<Value>(&msg) {
                handle_message(&mut state, &value, &mut stdout);
            }
        }
    }
}

fn take_frame(input: &str) -> Option<(String, String)> {
    let header_end = input.find("\r\n\r\n")?;
    let header = &input[..header_end];
    let len = header
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length:").map(str::trim))
        .and_then(|n| n.parse::<usize>().ok())?;
    let body_start = header_end + 4;
    if input.len() < body_start + len {
        return None;
    }
    let body = input[body_start..body_start + len].to_string();
    let rest = input[body_start + len..].to_string();
    Some((body, rest))
}

fn write_frame(stdout: &mut impl Write, body: &str) {
    let _ = write!(
        stdout,
        "Content-Length: {}\r\n\r\n{}",
        body.as_bytes().len(),
        body
    );
    let _ = stdout.flush();
}

fn handle_message(state: &mut DapState, msg: &Value, stdout: &mut impl Write) {
    if msg.get("type").and_then(Value::as_str) != Some("request") {
        return;
    }
    let command = msg.get("command").and_then(Value::as_str).unwrap_or("");
    let request_seq = msg.get("seq").and_then(Value::as_i64).unwrap_or(0) as i32;
    match command {
        "initialize" => {
            let seq = state.next_seq();
            write_frame(
                stdout,
                &json!({
                    "seq": seq,
                    "type": "response",
                    "request_seq": request_seq,
                    "success": true,
                    "command": "initialize",
                    "body": { "supportsConfigurationDoneRequest": true }
                })
                .to_string(),
            );
            let evt = state.next_seq();
            write_frame(
                stdout,
                &json!({ "seq": evt, "type": "event", "event": "initialized" }).to_string(),
            );
        }
        "launch" => {
            if let Some(program) = msg
                .get("arguments")
                .and_then(|a| a.get("program"))
                .and_then(Value::as_str)
            {
                state.program = Some(PathBuf::from(program));
            }
            respond_ok(state, stdout, request_seq, command, json!({}));
        }
        "setBreakpoints" => {
            state.breakpoints.clear();
            if let Some(bps) = msg
                .get("arguments")
                .and_then(|a| a.get("breakpoints"))
                .and_then(Value::as_array)
            {
                for bp in bps {
                    if let Some(line) = bp.get("line").and_then(Value::as_i64) {
                        state.breakpoints.push(line as i32);
                    }
                }
            }
            let verified: Vec<Value> = state
                .breakpoints
                .iter()
                .map(|line| json!({ "verified": true, "line": line }))
                .collect();
            respond_ok(
                state,
                stdout,
                request_seq,
                command,
                json!({ "breakpoints": verified }),
            );
        }
        "configurationDone" => {
            respond_ok(state, stdout, request_seq, command, json!({}));
            if let Err(e) = run_debug_session(state) {
                eprintln!("debug session error: {}", e);
            }
            if state.paused_line.is_some() {
                let evt = state.next_seq();
                write_frame(
                    stdout,
                    &json!({
                        "seq": evt,
                        "type": "event",
                        "event": "stopped",
                        "body": { "reason": "breakpoint", "threadId": 1, "allThreadsStopped": true }
                    })
                    .to_string(),
                );
            }
        }
        "threads" => {
            respond_ok(
                state,
                stdout,
                request_seq,
                command,
                json!({ "threads": [{ "id": 1, "name": "main" }] }),
            );
        }
        "stackTrace" => {
            let line = state.paused_line.unwrap_or(1);
            let program = state
                .program
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "main.ark".to_string());
            respond_ok(
                state,
                stdout,
                request_seq,
                command,
                json!({
                    "stackFrames": [{
                        "id": 1,
                        "name": "main",
                        "line": line,
                        "column": 1,
                        "source": { "name": Path::new(&program).file_name().and_then(|s| s.to_str()).unwrap_or("main.ark"), "path": program }
                    }],
                    "totalFrames": 1
                }),
            );
        }
        "scopes" => {
            respond_ok(
                state,
                stdout,
                request_seq,
                command,
                json!({ "scopes": [{ "name": "Locals", "variablesReference": 1, "expensive": false }] }),
            );
        }
        "variables" => {
            let vars: Vec<Value> = state
                .live_locals
                .iter()
                .enumerate()
                .map(|(i, (name, value))| {
                    json!({
                        "name": name,
                        "value": value,
                        "type": "i32",
                        "variablesReference": 0,
                        "evaluateName": name,
                        "indexedVariables": 0,
                        "namedVariables": 0,
                        "presentationHint": { "kind": "property", "attributes": ["readOnly"] },
                        "id": i + 1
                    })
                })
                .collect();
            respond_ok(state, stdout, request_seq, command, json!({ "variables": vars }));
        }
        "disconnect" | "terminate" => {
            respond_ok(state, stdout, request_seq, command, json!({}));
        }
        _ => respond_ok(state, stdout, request_seq, command, json!({})),
    }
}

fn respond_ok(state: &mut DapState, stdout: &mut impl Write, request_seq: i32, command: &str, body: Value) {
    let seq = state.next_seq();
    write_frame(
        stdout,
        &json!({
            "seq": seq,
            "type": "response",
            "request_seq": request_seq,
            "success": true,
            "command": command,
            "body": body
        })
        .to_string(),
    );
}

fn run_debug_session(state: &mut DapState) -> Result<(), String> {
    let program = state
        .program
        .as_ref()
        .ok_or_else(|| "missing program".to_string())?
        .clone();
    let breakpoint_line = state
        .breakpoints
        .first()
        .copied()
        .ok_or_else(|| "no breakpoints".to_string())?;
    let wasm_path = compile_to_wasm(&state.repo_root, &program)?;
    let wasm_bytes = std::fs::read(&wasm_path).map_err(|e| e.to_string())?;
    let caps = RuntimeCaps::from_cli(&[state.repo_root.display().to_string()]);
    let source = std::fs::read_to_string(&program).map_err(|e| e.to_string())?;
    let pause = run_until_breakpoint(&wasm_bytes, breakpoint_line as u32, &caps, Some(&source))?;
    state.paused_line = Some(pause.source_line as i32);
    state.live_locals = pause
        .locals
        .into_iter()
        .map(|l| (l.name, l.value))
        .collect();
    Ok(())
}

fn compile_to_wasm(repo_root: &Path, program: &Path) -> Result<PathBuf, String> {
    let wrapper = repo_root.join("scripts/run/arukellt-selfhost.sh");
    let pinned = repo_root.join("bootstrap/arukellt-selfhost.wasm");
    let build_dir = repo_root.join(".build").join("debug-dap");
    std::fs::create_dir_all(&build_dir).map_err(|e| e.to_string())?;
    let wasm_name = format!("{}.wasm", program.file_stem().and_then(|s| s.to_str()).unwrap_or("out"));
    let wasm_path = build_dir.join(&wasm_name);
    let wasm_rel = wasm_path.strip_prefix(repo_root).map_err(|_| "path error".to_string())?;
    let output = Command::new(&wrapper)
        .arg("compile")
        .arg(program.strip_prefix(repo_root).unwrap_or(program))
        .arg("-o")
        .arg(&wasm_rel)
        .current_dir(repo_root)
        .env("ARUKELLT_SELFHOST_WASM", pinned)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    if wasm_path.is_file() {
        Ok(wasm_path)
    } else {
        Err(format!("compiled wasm missing at {}", wasm_path.display()))
    }
}
