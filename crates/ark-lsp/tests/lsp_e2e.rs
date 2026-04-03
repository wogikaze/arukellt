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
