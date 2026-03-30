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

        match request.command.as_str() {
            "initialize" => {
                response_body = Some(serde_json::json!({
                    "supportsConfigurationDoneRequest": true,
                    "supportsFunctionBreakpoints": false,
                }));
            }
            "launch" => {
                response_body = Some(serde_json::json!({}));
            }
            "disconnect" => {
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
    }

    Ok(())
}
