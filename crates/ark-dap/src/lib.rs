use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub seq: i64,
    pub type_: String,
    pub command: String,
    pub arguments: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub seq: i64,
    pub type_: String,
    pub request_seq: i64,
    pub success: bool,
    pub command: String,
    pub message: Option<String>,
    pub body: Option<serde_json::Value>,
}

/// Session state shared across the DAP message loop.
struct DapSession {
    /// Path to the .ark source file being debugged, if launched.
    source_path: Option<String>,
    /// Whether the program has been launched and terminated.
    terminated: bool,
}

impl DapSession {
    fn new() -> Self {
        DapSession {
            source_path: None,
            terminated: false,
        }
    }
}

fn make_event(seq: i64, event: &str, body: Option<serde_json::Value>) -> String {
    let msg = serde_json::json!({
        "seq": seq,
        "type": "event",
        "event": event,
        "body": body,
    });
    let json = serde_json::to_string(&msg).unwrap_or_default();
    format!("Content-Length: {}\r\n\r\n{}", json.len(), json)
}

pub async fn run_dap() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let session = Arc::new(Mutex::new(DapSession::new()));
    let mut seq_counter: i64 = 5000;

    loop {
        let mut header = String::new();
        let mut content_length = 0;

        loop {
            header.clear();
            reader.read_line(&mut header).await?;
            if header == "\r\n" || header == "\n" {
                break;
            }
            if let Some(len_str) = header.strip_prefix("Content-Length: ") {
                content_length = len_str.trim().parse::<usize>()?;
            }
        }

        if content_length == 0 {
            break;
        }

        let mut body = vec![0u8; content_length];
        reader.read_exact(&mut body).await?;

        let request: Request = serde_json::from_slice(&body)?;
        let mut response_body = None;
        let mut events_after: Vec<String> = Vec::new();
        seq_counter += 1;
        let event_seq = seq_counter;

        match request.command.as_str() {
            "initialize" => {
                response_body = Some(serde_json::json!({
                    "supportsConfigurationDoneRequest": true,
                    "supportsFunctionBreakpoints": false,
                    "supportsConditionalBreakpoints": false,
                    "supportsSetVariable": false,
                    "supportsSteppingGranularity": false,
                    "supportsTerminateRequest": true,
                }));
            }
            "launch" => {
                let source = request
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("program"))
                    .and_then(|p| p.as_str())
                    .map(|s| s.to_string());
                if let Ok(mut sess) = session.lock() {
                    sess.source_path = source;
                    sess.terminated = false;
                }
                response_body = Some(serde_json::json!({}));
                events_after.push(make_event(event_seq, "initialized", None));
            }
            "configurationDone" => {
                // Compile and run the program; emit output events, then terminated.
                let source_path = session.lock().ok().and_then(|s| s.source_path.clone());
                response_body = Some(serde_json::json!({}));

                if let Some(path) = source_path {
                    // Run the program via `arukellt run <path>` and capture output
                    let run_result = tokio::process::Command::new("arukellt")
                        .args(["run", &path])
                        .output()
                        .await;

                    match run_result {
                        Ok(out) => {
                            let stdout_text = String::from_utf8_lossy(&out.stdout);
                            let stderr_text = String::from_utf8_lossy(&out.stderr);
                            if !stdout_text.is_empty() {
                                events_after.push(make_event(
                                    event_seq + 1,
                                    "output",
                                    Some(serde_json::json!({
                                        "category": "stdout",
                                        "output": stdout_text,
                                    })),
                                ));
                            }
                            if !stderr_text.is_empty() {
                                events_after.push(make_event(
                                    event_seq + 2,
                                    "output",
                                    Some(serde_json::json!({
                                        "category": "stderr",
                                        "output": stderr_text,
                                    })),
                                ));
                            }
                            let exit_code = out.status.code().unwrap_or(-1);
                            events_after.push(make_event(
                                event_seq + 3,
                                "exited",
                                Some(serde_json::json!({ "exitCode": exit_code })),
                            ));
                        }
                        Err(e) => {
                            events_after.push(make_event(
                                event_seq + 1,
                                "output",
                                Some(serde_json::json!({
                                    "category": "stderr",
                                    "output": format!("arukellt run failed: {e}\n"),
                                })),
                            ));
                        }
                    }
                } else {
                    events_after.push(make_event(
                        event_seq + 1,
                        "output",
                        Some(serde_json::json!({
                            "category": "stderr",
                            "output": "DAP launch: no program path specified\n",
                        })),
                    ));
                }

                if let Ok(mut sess) = session.lock() {
                    sess.terminated = true;
                }
                events_after.push(make_event(event_seq + 4, "terminated", None));
            }
            "threads" => {
                response_body = Some(serde_json::json!({
                    "threads": [
                        { "id": 1, "name": "main" }
                    ]
                }));
            }
            "stackTrace" => {
                // No live execution — return empty stack
                response_body = Some(serde_json::json!({
                    "stackFrames": [],
                    "totalFrames": 0,
                }));
            }
            "scopes" => {
                // No live execution — return empty scope list
                response_body = Some(serde_json::json!({
                    "scopes": [],
                }));
            }
            "variables" => {
                // No live execution — return empty variable list
                response_body = Some(serde_json::json!({
                    "variables": [],
                }));
            }
            "continue" | "next" | "stepIn" | "stepOut" => {
                response_body = Some(serde_json::json!({ "allThreadsContinued": true }));
            }
            "setBreakpoints" => {
                // Accept breakpoints but report them as unverified (runtime breakpoints not yet supported)
                let breakpoints = request
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("breakpoints"))
                    .and_then(|b| b.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|bp| {
                                serde_json::json!({
                                    "verified": false,
                                    "message": "Breakpoints are not yet supported (runtime hooks needed)",
                                    "line": bp.get("line").and_then(|l| l.as_i64()).unwrap_or(0),
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                response_body = Some(serde_json::json!({ "breakpoints": breakpoints }));
            }
            "setFunctionBreakpoints" | "setExceptionBreakpoints" => {
                response_body = Some(serde_json::json!({}));
            }
            "terminate" | "disconnect" => {
                let response = Response {
                    seq: request.seq + 1000,
                    type_: "response".to_string(),
                    request_seq: request.seq,
                    success: true,
                    command: request.command,
                    message: None,
                    body: Some(serde_json::json!({})),
                };
                let response_json = serde_json::to_string(&response)?;
                let full_response = format!(
                    "Content-Length: {}\r\n\r\n{}",
                    response_json.len(),
                    response_json
                );
                stdout.write_all(full_response.as_bytes()).await?;
                stdout.flush().await?;
                break;
            }
            _ => {
                // Unknown command: respond with empty success to avoid debugger hang
                response_body = Some(serde_json::json!({}));
            }
        }

        let response = Response {
            seq: request.seq + 1000,
            type_: "response".to_string(),
            request_seq: request.seq,
            success: true,
            command: request.command,
            message: None,
            body: response_body,
        };

        let response_json = serde_json::to_string(&response)?;
        let full_response = format!(
            "Content-Length: {}\r\n\r\n{}",
            response_json.len(),
            response_json
        );
        stdout.write_all(full_response.as_bytes()).await?;
        stdout.flush().await?;

        for event_msg in events_after {
            stdout.write_all(event_msg.as_bytes()).await?;
            stdout.flush().await?;
        }
    }

    Ok(())
}
