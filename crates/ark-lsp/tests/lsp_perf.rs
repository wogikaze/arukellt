//! LSP performance smoke tests.
//!
//! These tests measure response times for core LSP operations on large
//! fixtures and emit warnings (non-fatal) when times exceed baseline
//! thresholds.  They are intentionally separate from the quick-pass suite
//! and are only run in the full / release verification pass.
//!
//! Results are written to `target/lsp-perf-results.json` (workspace root)
//! for optional CI trend tracking; the file is not committed.
//!
//! Baseline constants can be overridden via environment variables:
//!   LSP_PERF_BASELINE_HOVER_MS
//!   LSP_PERF_BASELINE_DEFINITION_MS
//!   LSP_PERF_BASELINE_DIAGNOSE_MS
//!   LSP_PERF_BASELINE_STDLIB_MS

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

// ── Baseline constants (ms) ──────────────────────────────────────────────────

/// Baseline for hover response time (milliseconds).
const BASELINE_HOVER_MS: u64 = 500;
/// Baseline for definition response time (milliseconds).
const BASELINE_DEFINITION_MS: u64 = 500;
/// Baseline for open + diagnostics time on large_module (milliseconds).
const BASELINE_DIAGNOSE_MS: u64 = 2000;
/// Baseline for open + diagnostics time on stdlib_heavy (milliseconds).
const BASELINE_STDLIB_MS: u64 = 2000;

fn read_baseline_env(env_var: &str, default: u64) -> u64 {
    std::env::var(env_var)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

// ── LspSession (same pattern as lsp_e2e.rs) ─────────────────────────────────

struct LspSession {
    child: Child,
    rx: mpsc::Receiver<Value>,
}

impl LspSession {
    fn start() -> Self {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let workspace_root = std::path::Path::new(manifest_dir)
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        // Prefer release binary when running in --release, fall back to debug.
        let bin_path = {
            let release = workspace_root.join("target/release/arukellt");
            let debug = workspace_root.join("target/debug/arukellt");
            if release.exists() { release } else { debug }
        };
        assert!(
            bin_path.exists(),
            "arukellt binary not found at {:?} — run `cargo build -p arukellt` first",
            bin_path
        );

        let mut child = Command::new(&bin_path)
            .arg("lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn LSP server at {:?}: {}", bin_path, e));

        let stdout = child.stdout.take().expect("stdout");
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut header_line = String::new();
                match reader.read_line(&mut header_line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                let content_length: usize =
                    if let Some(rest) = header_line.strip_prefix("Content-Length:") {
                        rest.trim().parse().unwrap_or(0)
                    } else {
                        continue;
                    };
                let mut blank = String::new();
                let _ = reader.read_line(&mut blank);
                let mut body = vec![0u8; content_length];
                if reader.read_exact(&mut body).is_err() {
                    break;
                }
                if let Ok(val) = serde_json::from_slice::<Value>(&body) {
                    if tx.send(val).is_err() {
                        break;
                    }
                }
            }
        });

        LspSession { child, rx }
    }

    fn send(&mut self, msg: &Value) {
        let body = serde_json::to_string(msg).unwrap();
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let stdin = self.child.stdin.as_mut().expect("stdin");
        stdin.write_all(header.as_bytes()).unwrap();
        stdin.write_all(body.as_bytes()).unwrap();
        stdin.flush().unwrap();
    }

    fn recv(&self) -> Option<Value> {
        self.rx.recv_timeout(Duration::from_secs(15)).ok()
    }

    fn request(&mut self, id: i64, method: &str, params: Value) -> Option<Value> {
        let mut msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
        });
        if !params.is_null() {
            msg["params"] = params;
        }
        self.send(&msg);

        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            if let Some(msg) = self.recv() {
                if msg.get("id").and_then(|v| v.as_i64()) == Some(id) {
                    return Some(msg);
                }
                if msg.get("id").and_then(|v| v.as_str()) == Some(&id.to_string()) {
                    return Some(msg);
                }
            }
        }
        None
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }));
    }

    fn initialize(&mut self) -> Value {
        let resp = self
            .request(
                1,
                "initialize",
                json!({
                    "processId": null,
                    "rootUri": null,
                    "capabilities": {}
                }),
            )
            .expect("initialize response");
        self.notify("initialized", json!({}));
        resp
    }

    fn open_document(&mut self, uri: &str, text: &str) {
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "arukellt",
                    "version": 1,
                    "text": text
                }
            }),
        );
        std::thread::sleep(Duration::from_millis(200));
    }

    fn request_hover(&mut self, uri: &str, line: u32, col: u32) -> Value {
        use std::sync::atomic::{AtomicI64, Ordering};
        static NEXT_ID: AtomicI64 = AtomicI64::new(80001);
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let resp = self.request(
            id,
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": col }
            }),
        );
        resp.and_then(|r| r.get("result").cloned())
            .unwrap_or(Value::Null)
    }

    fn request_definition(&mut self, uri: &str, line: u32, col: u32) -> Value {
        use std::sync::atomic::{AtomicI64, Ordering};
        static NEXT_ID: AtomicI64 = AtomicI64::new(81001);
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let resp = self.request(
            id,
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": col }
            }),
        );
        resp.and_then(|r| r.get("result").cloned())
            .unwrap_or(Value::Null)
    }

    fn collect_diagnostics_for(&self, target_uri: &str, timeout: Duration) -> Vec<Value> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match self.rx.recv_timeout(remaining.min(Duration::from_millis(200))) {
                Ok(msg) => {
                    let method = msg.get("method").and_then(|m| m.as_str());
                    if method == Some("textDocument/publishDiagnostics") {
                        let uri_matches = msg
                            .get("params")
                            .and_then(|p| p.get("uri"))
                            .and_then(|u| u.as_str())
                            == Some(target_uri);
                        if uri_matches {
                            return msg
                                .get("params")
                                .and_then(|p| p.get("diagnostics"))
                                .and_then(|d| d.as_array())
                                .cloned()
                                .unwrap_or_default();
                        }
                    }
                }
                Err(_) => break,
            }
        }
        vec![]
    }

    fn shutdown(&mut self) {
        let _ = self.request(9999, "shutdown", json!(null));
        self.notify("exit", json!(null));
        let _ = self.child.wait();
    }
}

impl Drop for LspSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ── Result collection ────────────────────────────────────────────────────────

struct PerfResult {
    test: String,
    elapsed_ms: u64,
    baseline_ms: u64,
    warned: bool,
}

fn record_result(results: &mut Vec<PerfResult>, test: &str, elapsed: Duration, baseline_ms: u64) {
    let elapsed_ms = elapsed.as_millis() as u64;
    let threshold = baseline_ms * 5;
    let warned = elapsed_ms > threshold;
    if warned {
        println!(
            "PERF WARNING: {} took {}ms (> 5× baseline {}ms = {}ms)",
            test, elapsed_ms, baseline_ms, threshold
        );
    } else {
        println!("PERF OK: {} took {}ms (baseline {}ms)", test, elapsed_ms, baseline_ms);
    }
    results.push(PerfResult {
        test: test.to_string(),
        elapsed_ms,
        baseline_ms,
        warned,
    });
}

fn write_results_json(results: &[PerfResult]) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let target_dir = workspace_root.join("target");
    // target/ should exist (cargo build creates it), but create_dir_all just in case
    let _ = std::fs::create_dir_all(&target_dir);
    let out_path = target_dir.join("lsp-perf-results.json");

    // Read existing results and merge, so parallel test runs accumulate entries.
    let mut existing: Vec<Value> = std::fs::read_to_string(&out_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let new_entries: Vec<Value> = results
        .iter()
        .map(|r| {
            json!({
                "test": r.test,
                "elapsed_ms": r.elapsed_ms,
                "baseline_ms": r.baseline_ms,
                "warned": r.warned
            })
        })
        .collect();

    // Remove any existing entries with the same test name, then append fresh ones.
    let new_names: std::collections::HashSet<&str> =
        results.iter().map(|r| r.test.as_str()).collect();
    existing.retain(|e| {
        e.get("test")
            .and_then(|v| v.as_str())
            .map(|n| !new_names.contains(n))
            .unwrap_or(true)
    });
    existing.extend(new_entries);

    let json_str = serde_json::to_string_pretty(&existing).unwrap_or_default();
    let _ = std::fs::write(&out_path, json_str);
    println!("LSP perf results written to: {}", out_path.display());
}

// ── Fixture paths ─────────────────────────────────────────────────────────────

fn fixture_uri(name: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let path = workspace_root
        .join("tests/fixtures/lsp_perf")
        .join(name);
    format!("file://{}", path.display())
}

fn fixture_text(name: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let path = workspace_root
        .join("tests/fixtures/lsp_perf")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {:?}: {}", path, e))
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn perf_hover_large_module() {
    let baseline_ms = read_baseline_env("LSP_PERF_BASELINE_HOVER_MS", BASELINE_HOVER_MS);

    let uri = fixture_uri("large_module.ark");
    let text = fixture_text("large_module.ark");

    let mut session = LspSession::start();
    session.initialize();
    session.open_document(&uri, &text);

    // Request hover at line 3 (func_001 definition: `fn func_001() -> i32 { 1 }`)
    let t0 = Instant::now();
    let _result = session.request_hover(&uri, 3, 3);
    let elapsed = t0.elapsed();

    let mut results = vec![];
    record_result(&mut results, "perf_hover_large_module", elapsed, baseline_ms);
    write_results_json(&results);

    session.shutdown();
    // Test always passes; warning is informational only
}

#[test]
fn perf_definition_large_module() {
    let baseline_ms = read_baseline_env("LSP_PERF_BASELINE_DEFINITION_MS", BASELINE_DEFINITION_MS);

    let uri = fixture_uri("large_module.ark");
    let text = fixture_text("large_module.ark");

    let mut session = LspSession::start();
    session.initialize();
    session.open_document(&uri, &text);

    // Request definition at the call site in main(): line ~214, col 20 (func_001)
    // The main() body calls func_001() on the line after `fn main() {`
    // large_module.ark has 210 fn definitions on lines 2..211, blank on 212,
    // then fn main() on 213, then `let result: i32 = func_001()` on 214.
    let t0 = Instant::now();
    let _result = session.request_definition(&uri, 214, 23);
    let elapsed = t0.elapsed();

    let mut results = vec![];
    record_result(&mut results, "perf_definition_large_module", elapsed, baseline_ms);
    write_results_json(&results);

    session.shutdown();
}

#[test]
fn perf_open_and_diagnose_large_module() {
    let baseline_ms = read_baseline_env("LSP_PERF_BASELINE_DIAGNOSE_MS", BASELINE_DIAGNOSE_MS);

    let uri = fixture_uri("large_module.ark");
    let text = fixture_text("large_module.ark");

    let mut session = LspSession::start();
    session.initialize();

    let t0 = Instant::now();
    session.open_document(&uri, &text);
    // Wait up to 8 seconds for diagnostics
    let _diags = session.collect_diagnostics_for(&uri, Duration::from_secs(8));
    let elapsed = t0.elapsed();

    let mut results = vec![];
    record_result(&mut results, "perf_open_and_diagnose_large_module", elapsed, baseline_ms);
    write_results_json(&results);

    session.shutdown();
}

#[test]
fn perf_open_stdlib_heavy() {
    let baseline_ms = read_baseline_env("LSP_PERF_BASELINE_STDLIB_MS", BASELINE_STDLIB_MS);

    let uri = fixture_uri("stdlib_heavy.ark");
    let text = fixture_text("stdlib_heavy.ark");

    let mut session = LspSession::start();
    session.initialize();

    let t0 = Instant::now();
    session.open_document(&uri, &text);
    // Wait up to 8 seconds for diagnostics
    let _diags = session.collect_diagnostics_for(&uri, Duration::from_secs(8));
    let elapsed = t0.elapsed();

    let mut results = vec![];
    record_result(&mut results, "perf_open_stdlib_heavy", elapsed, baseline_ms);
    write_results_json(&results);

    session.shutdown();
}
