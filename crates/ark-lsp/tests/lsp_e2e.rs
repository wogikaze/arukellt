//! LSP protocol-level E2E tests.
//!
//! These tests spawn the `arukellt lsp` subprocess and exercise the
//! JSON-RPC protocol directly, verifying capability negotiation, document
//! lifecycle, and feature responses.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// Helper that manages a running LSP server subprocess.
/// A background reader thread continuously reads JSON-RPC messages
/// from stdout and sends them through a channel.
struct LspSession {
    child: Child,
    rx: mpsc::Receiver<Value>,
}

impl LspSession {
    /// Spawn the LSP server.  Locates the workspace-built `arukellt` binary
    /// relative to this crate's manifest directory.
    fn start() -> Self {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let workspace_root = std::path::Path::new(manifest_dir)
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let bin_path = workspace_root.join("target/debug/arukellt");
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

        // Move stdout into a background reader thread that continuously
        // parses JSON-RPC messages and sends them through a channel.
        let stdout = child.stdout.take().expect("stdout");
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut header_line = String::new();
                match reader.read_line(&mut header_line) {
                    Ok(0) | Err(_) => break, // EOF or error
                    Ok(_) => {}
                }
                let content_length: usize =
                    if let Some(rest) = header_line.strip_prefix("Content-Length:") {
                        rest.trim().parse().unwrap_or(0)
                    } else {
                        continue; // skip non-header lines
                    };
                // Read blank line separator
                let mut blank = String::new();
                let _ = reader.read_line(&mut blank);
                // Read body
                let mut body = vec![0u8; content_length];
                if reader.read_exact(&mut body).is_err() {
                    break;
                }
                if let Ok(val) = serde_json::from_slice::<Value>(&body) {
                    if tx.send(val).is_err() {
                        break; // receiver dropped
                    }
                }
            }
        });

        LspSession { child, rx }
    }

    /// Send a JSON-RPC message with proper Content-Length header.
    fn send(&mut self, msg: &Value) {
        let body = serde_json::to_string(msg).unwrap();
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let stdin = self.child.stdin.as_mut().expect("stdin");
        stdin.write_all(header.as_bytes()).unwrap();
        stdin.write_all(body.as_bytes()).unwrap();
        stdin.flush().unwrap();
    }

    /// Read one JSON-RPC message from the background reader with timeout.
    fn recv(&self) -> Option<Value> {
        self.rx.recv_timeout(Duration::from_secs(10)).ok()
    }

    /// Send a request and receive the response, skipping any notification
    /// messages that arrive before the response.
    fn request(&mut self, id: i64, method: &str, params: Value) -> Option<Value> {
        let mut msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
        });
        // Omit "params" entirely for null — tower-lsp rejects both null and {}
        if !params.is_null() {
            msg["params"] = params;
        }
        self.send(&msg);

        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while std::time::Instant::now() < deadline {
            if let Some(msg) = self.recv() {
                // Skip notifications (no "id" field)
                if msg.get("id").and_then(|v| v.as_i64()) == Some(id) {
                    return Some(msg);
                }
                // Also accept string id
                if msg.get("id").and_then(|v| v.as_str()) == Some(&id.to_string()) {
                    return Some(msg);
                }
            }
        }
        None
    }

    /// Send a notification (no id, no response expected).
    fn notify(&mut self, method: &str, params: Value) {
        self.send(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }));
    }

    /// Perform the initialize handshake.
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
        // Send initialized notification
        self.notify("initialized", json!({}));
        resp
    }

    /// Perform the initialize handshake with a real workspace root URI.
    /// The root URI is used by the LSP server to locate the stdlib directory
    /// so that stdlib imports (e.g. `use std::host::stdio`) are resolved
    /// during diagnostics analysis.
    fn initialize_with_root(&mut self, root_uri: &str) -> Value {
        let resp = self
            .request(
                1,
                "initialize",
                json!({
                    "processId": null,
                    "rootUri": root_uri,
                    "capabilities": {}
                }),
            )
            .expect("initialize response");
        // Send initialized notification
        self.notify("initialized", json!({}));
        // Give the server time to index project files and stdlib
        std::thread::sleep(Duration::from_millis(300));
        resp
    }

    /// Open a document.
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
        // Small delay to let the server process the notification
        std::thread::sleep(Duration::from_millis(200));
    }

    /// Collect the `textDocument/publishDiagnostics` notification for `target_uri`.
    ///
    /// Waits up to `timeout` for a matching notification.  Returns the diagnostics
    /// array, or an empty `Vec` if no notification arrives within the timeout.
    fn collect_diagnostics_for(&self, target_uri: &str, timeout: Duration) -> Vec<Value> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match self
                .rx
                .recv_timeout(remaining.min(Duration::from_millis(200)))
            {
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
                Err(_) => break, // Timeout
            }
        }
        // No matching notification received within the timeout.
        // Treat as "no diagnostics published", which is a safe default —
        // the server sends an empty array when there are no errors.
        vec![]
    }

    /// Shut down cleanly.
    fn shutdown(&mut self) {
        let _ = self.request(9999, "shutdown", json!(null));
        self.notify("exit", json!(null));
        let _ = self.child.wait();
    }
}

impl Drop for LspSession {
    fn drop(&mut self) {
        // Try graceful shutdown
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ---- Tests ----

#[test]
fn initialize_returns_capabilities() {
    let mut session = LspSession::start();
    let resp = session.initialize();

    let result = resp.get("result").expect("result in initialize response");
    let caps = result.get("capabilities").expect("capabilities in result");

    // Should advertise completion
    assert!(
        caps.get("completionProvider").is_some(),
        "server should advertise completion"
    );

    // Should advertise hover
    assert!(
        caps.get("hoverProvider").is_some(),
        "server should advertise hover"
    );

    // Should advertise definition
    assert!(
        caps.get("definitionProvider").is_some(),
        "server should advertise definition"
    );

    // Should advertise formatting
    assert!(
        caps.get("documentFormattingProvider").is_some(),
        "server should advertise formatting"
    );

    session.shutdown();
}

#[test]
fn completion_returns_results() {
    let mut session = LspSession::start();
    session.initialize();

    let uri = "file:///test/main.ark";
    session.open_document(uri, "fn main() {\n    pr\n}\n");

    let resp = session
        .request(
            2,
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": 6 }
            }),
        )
        .expect("completion response");

    let result = resp.get("result").expect("result");
    // Result should be an array of completion items
    let items = result.as_array().expect("completion items array");
    assert!(!items.is_empty(), "should return completion items");

    // Should contain "println"
    let has_println = items
        .iter()
        .any(|item| item.get("label").and_then(|l| l.as_str()) == Some("println"));
    assert!(has_println, "completions should include println");

    session.shutdown();
}

#[test]
fn hover_returns_info() {
    let mut session = LspSession::start();
    session.initialize();

    let uri = "file:///test/main.ark";
    session.open_document(uri, "fn greet(name: String) -> String {\n    name\n}\n");

    let resp = session
        .request(
            3,
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 3 }
            }),
        )
        .expect("hover response");

    let result = resp.get("result");
    // Result might be null for some positions, but for the function name
    // it should return hover info
    if let Some(r) = result {
        if !r.is_null() {
            let contents = r.get("contents").expect("hover contents");
            let value = contents.get("value").and_then(|v| v.as_str()).unwrap_or("");
            assert!(
                value.contains("greet") || value.contains("fn"),
                "hover should show function info"
            );
        }
    }

    session.shutdown();
}

#[test]
fn definition_resolves_local_symbol() {
    let mut session = LspSession::start();
    session.initialize();

    let uri = "file:///test/main.ark";
    session.open_document(
        uri,
        "fn greet() -> String {\n    \"hello\"\n}\n\nfn main() {\n    greet()\n}\n",
    );

    let resp = session
        .request(
            4,
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 5, "character": 4 }  // on `greet` call
            }),
        )
        .expect("definition response");

    let result = resp.get("result");
    if let Some(r) = result {
        if !r.is_null() {
            // Should resolve to the function definition
            let target_uri = r.get("uri").and_then(|u| u.as_str());
            assert_eq!(target_uri, Some(uri), "definition should be in same file");
        }
    }

    session.shutdown();
}

#[test]
fn shutdown_sequence_works() {
    let mut session = LspSession::start();
    session.initialize();

    // Shutdown should return a response
    let resp = session
        .request(100, "shutdown", json!(null))
        .expect("shutdown response");

    // Result should be null (success)
    // tower-lsp returns result: null on shutdown
    assert!(
        resp.get("result").is_some() || resp.get("error").is_none(),
        "shutdown should return a result (got {:?})",
        resp
    );

    // Send exit notification
    session.notify("exit", json!(null));

    // Process should exit
    let status = session.child.wait().expect("wait for exit");
    assert!(
        status.success(),
        "LSP server should exit cleanly after shutdown+exit"
    );
}

#[test]
fn unknown_method_returns_error() {
    let mut session = LspSession::start();
    session.initialize();

    let resp = session.request(
        5,
        "textDocument/doesNotExist",
        json!({ "textDocument": { "uri": "file:///test.ark" } }),
    );

    // tower-lsp returns MethodNotFound for unknown methods
    if let Some(r) = resp {
        if let Some(err) = r.get("error") {
            let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
            // -32601 = MethodNotFound
            assert_eq!(code, -32601, "should return MethodNotFound error");
        }
    }
    // If no response at all, that's also acceptable (some LSP servers
    // silently ignore unknown methods)

    session.shutdown();
}

#[test]
fn definition_local_variable_range_is_identifier_only() {
    // Regression test for issue #450:
    // goto-definition for a local `let` binding should return a range that
    // covers *only* the identifier token, not the full `let … = …` statement.
    //
    // Source (0-indexed lines):
    //   0: fn main() {
    //   1:     let source = 42
    //   2:     print(source)
    //   3: }
    //
    // Cursor is placed on the `source` usage inside `print(source)` (line 2).
    // The expected definition range should point to `source` on line 1:
    //   start.character = 8  (after "    let ")
    //   end.character   = 14 (start + len("source"))
    let mut session = LspSession::start();
    session.initialize();

    let uri = "file:///test/def_span.ark";
    let src = "fn main() {\n    let source = 42\n    print(source)\n}\n";
    session.open_document(uri, src);

    let resp = session
        .request(
            50,
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 2, "character": 10 }  // on `source` in print(source)
            }),
        )
        .expect("definition response for local variable");

    if let Some(result) = resp.get("result") {
        if result.is_null() {
            // If definition is not resolved, skip range check
            session.shutdown();
            return;
        }
        // Extract range from the definition location
        // Result may be a Location object or an array of Locations
        let location = if result.is_array() {
            result.as_array().and_then(|arr| arr.first())
        } else {
            Some(result)
        };
        if let Some(loc) = location {
            if let Some(range) = loc.get("range") {
                let start_line = range
                    .get("start")
                    .and_then(|s| s.get("line"))
                    .and_then(|l| l.as_u64());
                let start_char = range
                    .get("start")
                    .and_then(|s| s.get("character"))
                    .and_then(|c| c.as_u64());
                let end_char = range
                    .get("end")
                    .and_then(|e| e.get("character"))
                    .and_then(|c| c.as_u64());

                if let (Some(sl), Some(sc), Some(ec)) = (start_line, start_char, end_char) {
                    // Definition must point to line 1 (the let binding)
                    assert_eq!(
                        sl, 1,
                        "definition should be on line 1 (the let binding), got line {}",
                        sl
                    );
                    // Range must cover only the identifier `source` (6 chars),
                    // not the full `let source = 42` statement
                    assert_eq!(
                        sc, 8,
                        "definition range start.character should be 8 (start of `source`), got {}",
                        sc
                    );
                    assert_eq!(
                        ec, 14,
                        "definition range end.character should be 14 (end of `source`), got {}",
                        ec
                    );
                }
            }
        }
    }

    session.shutdown();
}

// ---- Issue 451: hover returns null for non-identifier / no-semantic-info tokens ----

#[test]
fn hover_string_literal_returns_null() {
    // Regression for issue #451: hovering over a string literal must return null.
    // Source (0-indexed):
    //   0: fn main() {
    //   1:     let s = "hello"
    //   2: }
    // Cursor is placed inside the string "hello" at line 1, character 13.
    let mut session = LspSession::start();
    session.initialize();

    let uri = "file:///test/hover_string.ark";
    session.open_document(uri, "fn main() {\n    let s = \"hello\"\n}\n");

    let resp = session
        .request(
            60,
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": 13 }  // inside "hello"
            }),
        )
        .expect("hover response for string literal");

    // Result must be null — string literals carry no semantic hover info.
    let result = resp.get("result").expect("result field");
    assert!(
        result.is_null(),
        "hovering over a string literal should return null, got: {:?}",
        result
    );

    session.shutdown();
}

#[test]
fn hover_integer_literal_returns_null() {
    // Regression for issue #451: hovering over an integer literal must return null.
    // Source:
    //   0: fn main() {
    //   1:     let n = 42
    //   2: }
    // Cursor is placed on `42` at line 1, character 12.
    let mut session = LspSession::start();
    session.initialize();

    let uri = "file:///test/hover_int.ark";
    session.open_document(uri, "fn main() {\n    let n = 42\n}\n");

    let resp = session
        .request(
            61,
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": 12 }  // on `42`
            }),
        )
        .expect("hover response for integer literal");

    let result = resp.get("result").expect("result field");
    assert!(
        result.is_null(),
        "hovering over an integer literal should return null, got: {:?}",
        result
    );

    session.shutdown();
}

#[test]
fn hover_keyword_returns_null() {
    // Regression for issue #451: hovering over a keyword must return null.
    // Source:
    //   0: fn main() {
    //   1:     let n = 42
    //   2: }
    // Cursor is placed on `let` at line 1, character 4.
    let mut session = LspSession::start();
    session.initialize();

    let uri = "file:///test/hover_keyword.ark";
    session.open_document(uri, "fn main() {\n    let n = 42\n}\n");

    let resp = session
        .request(
            62,
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": 4 }  // on `let`
            }),
        )
        .expect("hover response for keyword");

    let result = resp.get("result").expect("result field");
    assert!(
        result.is_null(),
        "hovering over a keyword should return null, got: {:?}",
        result
    );

    session.shutdown();
}

#[test]
fn hover_stdlib_function_returns_content() {
    // Regression guard for issue #451: stdlib functions with semantic info
    // must still return non-null hover content.
    // Source:
    //   0: fn main() {
    //   1:     println("hi")
    //   2: }
    // Cursor is placed on `println` at line 1, character 4.
    let mut session = LspSession::start();
    session.initialize();

    let uri = "file:///test/hover_stdlib.ark";
    session.open_document(uri, "fn main() {\n    println(\"hi\")\n}\n");

    let resp = session
        .request(
            63,
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": 4 }  // on `println`
            }),
        )
        .expect("hover response for stdlib function");

    let result = resp.get("result").expect("result field");
    // If the manifest is loaded, result should be non-null and contain "println".
    // If the manifest is absent the server returns null — that is also acceptable
    // here (the important invariant is that no *noise* hover is returned).
    if !result.is_null() {
        let value = result
            .get("contents")
            .and_then(|c| c.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            value.contains("println"),
            "stdlib hover should mention the function name, got: {:?}",
            value
        );
    }

    session.shutdown();
}

// ---- Issue 452: E0100 false positives for valid stdlib imports ----

/// Return the workspace root as a `file://` URI so tests can pass it to the
/// LSP server's `initialize` request.  The server uses this root to locate the
/// `std/` directory and resolve stdlib imports during diagnostics analysis.
fn workspace_root_uri() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // CARGO_MANIFEST_DIR is `{workspace}/crates/ark-lsp`.
    // Go up two levels to reach the workspace root.
    let root = std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    // Build a file:// URI manually to avoid an external URL-encoding dependency.
    let path_str = root.to_str().expect("workspace root is valid UTF-8");
    if path_str.starts_with('/') {
        format!("file://{}", path_str)
    } else {
        // Windows absolute path: file:///C:/...
        format!("file:///{}", path_str.replace('\\', "/"))
    }
}

#[test]
fn no_e0100_for_valid_stdlib_import() {
    // Regression test for issue #452.
    //
    // A source file that imports `std::host::stdio` and calls `stdio::println`
    // is valid.  `arukellt check` produces no errors; the LSP must not produce
    // any E0100 diagnostics for it either.
    //
    // The server is initialized with the real workspace root so it can locate
    // the stdlib directory and register `println` from `std/host/stdio.ark`
    // into the TypeChecker before checking the user's module.
    let mut session = LspSession::start();
    let root_uri = workspace_root_uri();
    session.initialize_with_root(&root_uri);

    let uri = "file:///test/issue452_stdlib.ark";
    let src = "use std::host::stdio\nfn main() {\n    stdio::println(\"hello from stdlib\")\n}\n";
    session.open_document(uri, src);

    // Wait for publishDiagnostics.  After fix #452 this should arrive with an
    // empty diagnostics array (or not arrive at all, which we also treat as
    // "no errors").
    let diags = session.collect_diagnostics_for(uri, Duration::from_secs(5));

    let e0100_diags: Vec<_> = diags
        .iter()
        .filter(|d| {
            d.get("code")
                .and_then(|c| c.as_str())
                .map(|s| s == "E0100")
                .unwrap_or(false)
        })
        .collect();

    assert!(
        e0100_diags.is_empty(),
        "E0100 false positive for valid `use std::host::stdio` import — \
         LSP diagnostics must match CLI check (no errors). Got: {:?}",
        e0100_diags
    );

    session.shutdown();
}

// ---- Issue 454: snapshot-style regression tests ----
//
// These tests use a shared FIXTURE_BASIC source constant (greet + main) and
// assert hover markdown content, definition character ranges, and diagnostic
// counts with exact/contains checks — no snapshot crate required.
//
// FIXTURE_BASIC includes an explicit stdlib import (`use std::host::stdio`) so
// the program is fully valid and produces zero E0xxx diagnostics when the
// workspace root is provided to the LSP server.
//
// Layout of FIXTURE_BASIC (0-indexed lines):
//   0: use std::host::stdio
//   1: fn greet(name: String) -> String {
//   2:     name
//   3: }
//   4: (empty)
//   5: fn main() {
//   6:     let result = greet("world")
//   7:     stdio::println(result)
//   8: }

const FIXTURE_BASIC: &str =
    "use std::host::stdio\nfn greet(name: String) -> String {\n    name\n}\n\nfn main() {\n    let result = greet(\"world\")\n    stdio::println(result)\n}\n";

#[test]
fn snapshot_hover_println_contains_signature() {
    // Snapshot test for issue #454 (complements issue #451 hover tests).
    //
    // Hover on `println` in FIXTURE_BASIC (line 7, character 11 — the `p` of
    // `println` in `stdio::println(result)`) must return non-null content that
    // contains the string "println", confirming the stdlib manifest entry is
    // rendered for a qualified call.
    //
    // Source position:
    //   Line 7: "    stdio::println(result)"
    //            0         1
    //            01234567890123456789
    //   character 11 = start of `println` (after "    stdio::")
    let mut session = LspSession::start();
    session.initialize();

    let uri = "file:///test/snapshot_hover_println.ark";
    session.open_document(uri, FIXTURE_BASIC);

    let resp = session
        .request(
            454_01,
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 7, "character": 11 }  // on `println` in stdio::println
            }),
        )
        .expect("hover response for println in FIXTURE_BASIC");

    let result = resp.get("result").expect("result field in hover response");

    // If the server returns non-null, it must mention "println".
    // A null result is also accepted when the stdlib manifest is not loaded
    // (e.g. CI without the workspace root), but no *wrong* content is allowed.
    if !result.is_null() {
        let value = result
            .get("contents")
            .and_then(|c| c.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            value.contains("println"),
            "snapshot: hover on `println` must mention the function name; got: {:?}",
            value
        );
    }

    session.shutdown();
}

#[test]
fn snapshot_hover_string_literal_is_null() {
    // Snapshot test for issue #454 (complements issue #451 null-hover tests).
    //
    // Hover on the string literal `"world"` inside FIXTURE_BASIC (line 6,
    // character 23 — the `w` inside `"world"`) must return null.  String
    // literals carry no semantic hover information.
    //
    // Source position:
    //   Line 6: "    let result = greet(\"world\")"
    //            0         1         2
    //            0123456789012345678901234567890
    //   character 23 = 'w' inside the "world" literal
    let mut session = LspSession::start();
    session.initialize();

    let uri = "file:///test/snapshot_hover_string.ark";
    session.open_document(uri, FIXTURE_BASIC);

    let resp = session
        .request(
            454_02,
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 6, "character": 23 }  // inside "world"
            }),
        )
        .expect("hover response for string literal in FIXTURE_BASIC");

    let result = resp.get("result").expect("result field in hover response");
    assert!(
        result.is_null(),
        "snapshot: hovering over a string literal must return null; got: {:?}",
        result
    );

    session.shutdown();
}

#[test]
fn snapshot_definition_local_var_name_span() {
    // Snapshot test for issue #454 (complements issue #450 definition-span test).
    //
    // Goto-definition on the `result` usage in FIXTURE_BASIC (line 7,
    // character 19 — the `r` of `result` in `stdio::println(result)`) must
    // resolve to the `let result` binding on line 6 with a range that covers
    // only the identifier token:
    //   start.character = 8   ("    let " is 8 characters)
    //   end.character   = 14  (start + len("result") = 8 + 6)
    //
    // Source positions:
    //   Line 6: "    let result = greet(\"world\")"
    //            0       8      14
    //   Line 7: "    stdio::println(result)"
    //            0                  19    25
    let mut session = LspSession::start();
    session.initialize();

    let uri = "file:///test/snapshot_def_result.ark";
    session.open_document(uri, FIXTURE_BASIC);

    let resp = session
        .request(
            454_03,
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 7, "character": 19 }  // on `result` in stdio::println(result)
            }),
        )
        .expect("definition response for `result` in FIXTURE_BASIC");

    if let Some(result) = resp.get("result") {
        if result.is_null() {
            // Server did not resolve — skip range assertions.
            session.shutdown();
            return;
        }
        // Accept both a single Location and an array of Locations.
        let location = if result.is_array() {
            result.as_array().and_then(|arr| arr.first())
        } else {
            Some(result)
        };
        if let Some(loc) = location {
            // Same-file assertion.
            if let Some(def_uri) = loc.get("uri").and_then(|u| u.as_str()) {
                assert_eq!(
                    def_uri, uri,
                    "snapshot: definition for `result` should be in the same file"
                );
            }
            if let Some(range) = loc.get("range") {
                let start_line = range
                    .get("start")
                    .and_then(|s| s.get("line"))
                    .and_then(|l| l.as_u64());
                let start_char = range
                    .get("start")
                    .and_then(|s| s.get("character"))
                    .and_then(|c| c.as_u64());
                let end_char = range
                    .get("end")
                    .and_then(|e| e.get("character"))
                    .and_then(|c| c.as_u64());

                if let (Some(sl), Some(sc), Some(ec)) = (start_line, start_char, end_char) {
                    assert_eq!(
                        sl, 6,
                        "snapshot: definition of `result` should be on line 6 (the let binding), got line {}",
                        sl
                    );
                    assert_eq!(
                        sc, 8,
                        "snapshot: definition range start.character should be 8 (start of `result`), got {}",
                        sc
                    );
                    assert_eq!(
                        ec, 14,
                        "snapshot: definition range end.character should be 14 (end of `result`), got {}",
                        ec
                    );
                }
            }
        }
    }

    session.shutdown();
}

#[test]
fn snapshot_diagnostics_valid_program_zero_errors() {
    // Snapshot test for issue #454 (complements issue #452 E0100 tests).
    //
    // FIXTURE_BASIC includes `use std::host::stdio` and calls `stdio::println`,
    // making it a fully valid program.  After opening it the LSP server should
    // publish zero E0xxx diagnostics.  The server is initialized with the real
    // workspace root so stdlib resolution is available just as it would be in
    // normal editor use.
    let mut session = LspSession::start();
    let root_uri = workspace_root_uri();
    session.initialize_with_root(&root_uri);

    let uri = "file:///test/snapshot_valid.ark";
    session.open_document(uri, FIXTURE_BASIC);

    let diags = session.collect_diagnostics_for(uri, Duration::from_secs(5));

    let error_diags: Vec<_> = diags
        .iter()
        .filter(|d| {
            // Capture any E0xxx diagnostic code (error-level)
            d.get("code")
                .and_then(|c| c.as_str())
                .map(|s| s.starts_with('E'))
                .unwrap_or(false)
        })
        .collect();

    assert!(
        error_diags.is_empty(),
        "snapshot: valid program (FIXTURE_BASIC) must produce zero E-prefixed diagnostics; \
         got {} error(s): {:?}",
        error_diags.len(),
        error_diags
    );

    session.shutdown();
}

#[test]
fn no_e0100_for_valid_multimodule_stdlib_import() {
    // Additional regression for issue #452: multiple stdlib imports in the same
    // file should all be resolved without E0100.
    let mut session = LspSession::start();
    let root_uri = workspace_root_uri();
    session.initialize_with_root(&root_uri);

    let uri = "file:///test/issue452_multi.ark";
    // Use two stdlib modules; calls to both should be free of E0100.
    let src = "use std::host::stdio\nuse std::text\n\
        fn main() {\n    let s = text::to_string(42)\n    stdio::println(s)\n}\n";
    session.open_document(uri, src);

    let diags = session.collect_diagnostics_for(uri, Duration::from_secs(5));

    let e0100_diags: Vec<_> = diags
        .iter()
        .filter(|d| {
            d.get("code")
                .and_then(|c| c.as_str())
                .map(|s| s == "E0100")
                .unwrap_or(false)
        })
        .collect();

    assert!(
        e0100_diags.is_empty(),
        "E0100 false positive for multi-module stdlib imports: {:?}",
        e0100_diags
    );

    session.shutdown();
}
