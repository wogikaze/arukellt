use serde::{Deserialize, Serialize};
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

pub async fn run_dap() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);

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
        let mut send_after_response: Option<String> = None;

        match request.command.as_str() {
            "initialize" => {
                response_body = Some(serde_json::json!({
                    "supportsConfigurationDoneRequest": true,
                    "supportsFunctionBreakpoints": false,
                    "supportsConditionalBreakpoints": false,
                    "supportsSetVariable": false,
                    "supportsSteppingGranularity": false,
                }));
            }
            "launch" => {
                response_body = Some(serde_json::json!({}));
                // Send initialized event after launch response
                let initialized_event = serde_json::json!({
                    "seq": request.seq + 2000,
                    "type": "event",
                    "event": "initialized",
                });
                let event_json = serde_json::to_string(&initialized_event)?;
                let event_msg =
                    format!("Content-Length: {}\r\n\r\n{}", event_json.len(), event_json);
                // event will be sent after the response below
                send_after_response = Some(event_msg);
            }
            "configurationDone" => {
                response_body = Some(serde_json::json!({}));
                // Send terminated event — we don't actually run yet
                let terminated_event = serde_json::json!({
                    "seq": request.seq + 2000,
                    "type": "event",
                    "event": "terminated",
                });
                let event_json = serde_json::to_string(&terminated_event)?;
                let event_msg =
                    format!("Content-Length: {}\r\n\r\n{}", event_json.len(), event_json);
                send_after_response = Some(event_msg);
            }
            "threads" => {
                response_body = Some(serde_json::json!({
                    "threads": [
                        { "id": 1, "name": "main" }
                    ]
                }));
            }
            "setBreakpoints" => {
                // Accept breakpoints but report them as unverified (not yet supported)
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
                                    "message": "Breakpoints are not yet supported by Arukellt DAP",
                                    "line": bp.get("line").and_then(|l| l.as_i64()).unwrap_or(0),
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                response_body = Some(serde_json::json!({ "breakpoints": breakpoints }));
            }
            "disconnect" => {
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
            _ => {}
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

        if let Some(event_msg) = send_after_response {
            stdout.write_all(event_msg.as_bytes()).await?;
            stdout.flush().await?;
        }
    }

    Ok(())
}
