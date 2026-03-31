use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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

/// Execution state of the debugged program.
#[derive(Debug, Clone, PartialEq)]
enum ExecState {
    /// Not yet started.
    NotStarted,
    /// Paused at a breakpoint or step.
    Stopped { line: i64, reason: String },
    /// Running (between continue/next and next stop).
    Running,
    /// Program finished execution.
    Terminated,
}

/// A source-level breakpoint.
#[derive(Debug, Clone)]
struct Breakpoint {
    line: i64,
    verified: bool,
}

/// Session state shared across the DAP message loop.
struct DapSession {
    /// Path to the .ark source file being debugged.
    source_path: Option<String>,
    /// Source lines (1-indexed via vec index + 1).
    source_lines: Vec<String>,
    /// Breakpoints by source path -> set of line numbers.
    breakpoints: HashMap<String, Vec<Breakpoint>>,
    /// Current execution state.
    exec_state: ExecState,
    /// Current line (1-indexed) in source-level stepping.
    current_line: i64,
    /// Total line count.
    total_lines: i64,
    /// Executable lines (non-empty, non-comment, non-brace-only).
    executable_lines: HashSet<i64>,
    /// Program output captured from `arukellt run`.
    program_output: Option<String>,
    /// Program stderr captured from `arukellt run`.
    program_stderr: Option<String>,
    /// Program exit code.
    exit_code: Option<i32>,
    /// Whether the program has been run to completion.
    program_ran: bool,
    /// Stop-after-entry mode (stop at first executable line).
    stop_on_entry: bool,
}

impl DapSession {
    fn new() -> Self {
        DapSession {
            source_path: None,
            source_lines: Vec::new(),
            breakpoints: HashMap::new(),
            exec_state: ExecState::NotStarted,
            current_line: 0,
            total_lines: 0,
            executable_lines: HashSet::new(),
            program_output: None,
            program_stderr: None,
            exit_code: None,
            program_ran: false,
            stop_on_entry: false,
        }
    }

    fn load_source(&mut self, path: &str) {
        if let Ok(content) = std::fs::read_to_string(path) {
            self.source_lines = content.lines().map(|l| l.to_string()).collect();
            self.total_lines = self.source_lines.len() as i64;
            self.executable_lines = (1..=self.total_lines)
                .filter(|&line| {
                    let idx = (line - 1) as usize;
                    if idx >= self.source_lines.len() {
                        return false;
                    }
                    let trimmed = self.source_lines[idx].trim();
                    !trimmed.is_empty()
                        && !trimmed.starts_with("//")
                        && trimmed != "{"
                        && trimmed != "}"
                        && !trimmed.starts_with("import ")
                })
                .collect();
        }
    }

    /// Find the next executable line at or after `from` (1-indexed).
    fn next_executable_line(&self, from: i64) -> Option<i64> {
        (from..=self.total_lines).find(|l| self.executable_lines.contains(l))
    }

    /// Check if any breakpoint is set at the given line.
    fn has_breakpoint_at(&self, line: i64) -> bool {
        if let Some(path) = &self.source_path {
            if let Some(bps) = self.breakpoints.get(path) {
                return bps.iter().any(|bp| bp.line == line && bp.verified);
            }
        }
        false
    }

    /// Advance to next stop point. Returns the line to stop at and reason, or None if terminated.
    fn advance_to_next_stop(&self, mode: StepMode) -> Option<(i64, String)> {
        let start = self.current_line + 1;
        match mode {
            StepMode::Next => {
                // Step one executable line
                if let Some(next) = self.next_executable_line(start) {
                    Some((next, "step".to_string()))
                } else {
                    None // end of file
                }
            }
            StepMode::Continue => {
                // Run until next breakpoint or end
                for line in start..=self.total_lines {
                    if self.executable_lines.contains(&line) && self.has_breakpoint_at(line) {
                        return Some((line, "breakpoint".to_string()));
                    }
                }
                None // no more breakpoints, run to end
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum StepMode {
    Next,
    Continue,
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

fn make_response(req: &Request, body: Option<serde_json::Value>) -> String {
    let resp = serde_json::json!({
        "seq": req.seq + 1000,
        "type": "response",
        "request_seq": req.seq,
        "success": true,
        "command": req.command,
        "body": body,
    });
    let json = serde_json::to_string(&resp).unwrap_or_default();
    format!("Content-Length: {}\r\n\r\n{}", json.len(), json)
}

/// Run the program via `arukellt run` and capture output.
async fn run_program(path: &str) -> (Option<String>, Option<String>, i32) {
    let result = tokio::process::Command::new("arukellt")
        .args(["run", path])
        .output()
        .await;

    match result {
        Ok(out) => {
            let stdout_text = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr_text = String::from_utf8_lossy(&out.stderr).to_string();
            let code = out.status.code().unwrap_or(-1);
            (
                if stdout_text.is_empty() {
                    None
                } else {
                    Some(stdout_text)
                },
                if stderr_text.is_empty() {
                    None
                } else {
                    Some(stderr_text)
                },
                code,
            )
        }
        Err(e) => (None, Some(format!("arukellt run failed: {e}\n")), -1),
    }
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
        let mut messages: Vec<String> = Vec::new();
        seq_counter += 1;

        match request.command.as_str() {
            "initialize" => {
                messages.push(make_response(
                    &request,
                    Some(serde_json::json!({
                        "supportsConfigurationDoneRequest": true,
                        "supportsFunctionBreakpoints": false,
                        "supportsConditionalBreakpoints": false,
                        "supportsSetVariable": false,
                        "supportsSteppingGranularity": true,
                        "supportsTerminateRequest": true,
                        "supportsSingleThreadExecutionRequests": true,
                    })),
                ));
            }

            "launch" => {
                let args = request.arguments.as_ref();
                let source = args
                    .and_then(|a| a.get("program"))
                    .and_then(|p| p.as_str())
                    .map(|s| s.to_string());
                let stop_on_entry = args
                    .and_then(|a| a.get("stopOnEntry"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if let Ok(mut sess) = session.lock() {
                    if let Some(ref path) = source {
                        sess.load_source(path);
                    }
                    sess.source_path = source;
                    sess.exec_state = ExecState::NotStarted;
                    sess.stop_on_entry = stop_on_entry;
                }
                messages.push(make_response(&request, Some(serde_json::json!({}))));
                messages.push(make_event(seq_counter, "initialized", None));
            }

            "setBreakpoints" => {
                let args = request.arguments.as_ref();
                let source_path = args
                    .and_then(|a| a.get("source"))
                    .and_then(|s| s.get("path"))
                    .and_then(|p| p.as_str())
                    .unwrap_or("")
                    .to_string();

                let bp_requests = args
                    .and_then(|a| a.get("breakpoints"))
                    .and_then(|b| b.as_array())
                    .cloned()
                    .unwrap_or_default();

                let mut verified_bps = Vec::new();
                if let Ok(mut sess) = session.lock() {
                    let mut new_bps = Vec::new();
                    for bp_req in &bp_requests {
                        let line = bp_req.get("line").and_then(|l| l.as_i64()).unwrap_or(0);
                        let is_executable = sess.executable_lines.contains(&line);
                        let actual_line = if is_executable {
                            line
                        } else {
                            sess.next_executable_line(line).unwrap_or(line)
                        };
                        let verified = sess.executable_lines.contains(&actual_line);
                        new_bps.push(Breakpoint {
                            line: actual_line,
                            verified,
                        });
                        verified_bps.push(serde_json::json!({
                            "verified": verified,
                            "line": actual_line,
                            "source": { "path": source_path },
                        }));
                    }
                    sess.breakpoints.insert(source_path, new_bps);
                }

                messages.push(make_response(
                    &request,
                    Some(serde_json::json!({ "breakpoints": verified_bps })),
                ));
            }

            "setFunctionBreakpoints" | "setExceptionBreakpoints" => {
                messages.push(make_response(
                    &request,
                    Some(serde_json::json!({ "breakpoints": [] })),
                ));
            }

            "configurationDone" => {
                messages.push(make_response(&request, Some(serde_json::json!({}))));

                // Run the actual program in the background to capture output
                let source_path = session.lock().ok().and_then(|s| s.source_path.clone());

                if let Some(ref path) = source_path {
                    let (out, err, code) = run_program(path).await;
                    if let Ok(mut sess) = session.lock() {
                        sess.program_output = out;
                        sess.program_stderr = err;
                        sess.exit_code = Some(code);
                        sess.program_ran = true;
                    }
                }

                // Check if we should stop on entry or at first breakpoint
                let should_stop = if let Ok(mut sess) = session.lock() {
                    if sess.stop_on_entry {
                        if let Some(first_exec) = sess.next_executable_line(1) {
                            sess.current_line = first_exec;
                            sess.exec_state = ExecState::Stopped {
                                line: first_exec,
                                reason: "entry".to_string(),
                            };
                            Some(("entry".to_string(), first_exec))
                        } else {
                            None
                        }
                    } else {
                        // Check for first breakpoint
                        let stop = sess.advance_to_next_stop(StepMode::Continue);
                        if let Some((line, reason)) = stop.as_ref() {
                            sess.current_line = *line;
                            sess.exec_state = ExecState::Stopped {
                                line: *line,
                                reason: reason.clone(),
                            };
                        }
                        stop.map(|(line, reason)| (reason, line))
                    }
                } else {
                    None
                };

                if let Some((reason, _line)) = should_stop {
                    seq_counter += 1;
                    messages.push(make_event(
                        seq_counter,
                        "stopped",
                        Some(serde_json::json!({
                            "reason": reason,
                            "threadId": 1,
                            "allThreadsStopped": true,
                        })),
                    ));
                } else {
                    // No breakpoints — emit output and terminate
                    emit_program_results(&session, &mut messages, &mut seq_counter);
                }
            }

            "threads" => {
                messages.push(make_response(
                    &request,
                    Some(serde_json::json!({
                        "threads": [{ "id": 1, "name": "main" }]
                    })),
                ));
            }

            "stackTrace" => {
                let (frames, total) = if let Ok(sess) = session.lock() {
                    match &sess.exec_state {
                        ExecState::Stopped { line, .. } => {
                            let source_path = sess.source_path.clone().unwrap_or_default();
                            let source_line = if (*line as usize) <= sess.source_lines.len() {
                                sess.source_lines[(*line as usize) - 1].trim().to_string()
                            } else {
                                String::new()
                            };
                            // Determine function name from source context
                            let fn_name = find_enclosing_fn(&sess.source_lines, *line);
                            let frame = serde_json::json!({
                                "id": 1,
                                "name": fn_name,
                                "source": {
                                    "name": std::path::Path::new(&source_path)
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("unknown"),
                                    "path": source_path,
                                },
                                "line": line,
                                "column": 1,
                                "endLine": line,
                                "endColumn": source_line.len() + 1,
                            });
                            (vec![frame], 1)
                        }
                        _ => (vec![], 0),
                    }
                } else {
                    (vec![], 0)
                };

                messages.push(make_response(
                    &request,
                    Some(serde_json::json!({
                        "stackFrames": frames,
                        "totalFrames": total,
                    })),
                ));
            }

            "scopes" => {
                let scopes = if let Ok(sess) = session.lock() {
                    match &sess.exec_state {
                        ExecState::Stopped { .. } => {
                            vec![serde_json::json!({
                                "name": "Locals",
                                "variablesReference": 1,
                                "expensive": false,
                            })]
                        }
                        _ => vec![],
                    }
                } else {
                    vec![]
                };
                messages.push(make_response(
                    &request,
                    Some(serde_json::json!({ "scopes": scopes })),
                ));
            }

            "variables" => {
                let vars = if let Ok(sess) = session.lock() {
                    match &sess.exec_state {
                        ExecState::Stopped { line, .. } => {
                            extract_visible_variables(&sess.source_lines, *line)
                        }
                        _ => vec![],
                    }
                } else {
                    vec![]
                };
                messages.push(make_response(
                    &request,
                    Some(serde_json::json!({ "variables": vars })),
                ));
            }

            "continue" => {
                messages.push(make_response(
                    &request,
                    Some(serde_json::json!({ "allThreadsContinued": true })),
                ));

                let stop = if let Ok(sess) = session.lock() {
                    sess.advance_to_next_stop(StepMode::Continue)
                } else {
                    None
                };

                if let Some((line, reason)) = stop {
                    if let Ok(mut sess) = session.lock() {
                        sess.current_line = line;
                        sess.exec_state = ExecState::Stopped {
                            line,
                            reason: reason.clone(),
                        };
                    }
                    seq_counter += 1;
                    messages.push(make_event(
                        seq_counter,
                        "stopped",
                        Some(serde_json::json!({
                            "reason": reason,
                            "threadId": 1,
                            "allThreadsStopped": true,
                        })),
                    ));
                } else {
                    // No more breakpoints — run to completion
                    if let Ok(mut sess) = session.lock() {
                        sess.exec_state = ExecState::Terminated;
                    }
                    emit_program_results(&session, &mut messages, &mut seq_counter);
                }
            }

            "next" => {
                messages.push(make_response(
                    &request,
                    Some(serde_json::json!({ "allThreadsContinued": true })),
                ));

                let stop = if let Ok(sess) = session.lock() {
                    sess.advance_to_next_stop(StepMode::Next)
                } else {
                    None
                };

                if let Some((line, reason)) = stop {
                    if let Ok(mut sess) = session.lock() {
                        sess.current_line = line;
                        sess.exec_state = ExecState::Stopped {
                            line,
                            reason: reason.clone(),
                        };
                    }
                    seq_counter += 1;
                    messages.push(make_event(
                        seq_counter,
                        "stopped",
                        Some(serde_json::json!({
                            "reason": reason,
                            "threadId": 1,
                            "allThreadsStopped": true,
                        })),
                    ));
                } else {
                    if let Ok(mut sess) = session.lock() {
                        sess.exec_state = ExecState::Terminated;
                    }
                    emit_program_results(&session, &mut messages, &mut seq_counter);
                }
            }

            "stepIn" | "stepOut" => {
                // Treat same as next for single-threaded source stepping
                messages.push(make_response(
                    &request,
                    Some(serde_json::json!({ "allThreadsContinued": true })),
                ));

                let stop = if let Ok(sess) = session.lock() {
                    sess.advance_to_next_stop(StepMode::Next)
                } else {
                    None
                };

                if let Some((line, reason)) = stop {
                    if let Ok(mut sess) = session.lock() {
                        sess.current_line = line;
                        sess.exec_state = ExecState::Stopped {
                            line,
                            reason: reason.clone(),
                        };
                    }
                    seq_counter += 1;
                    messages.push(make_event(
                        seq_counter,
                        "stopped",
                        Some(serde_json::json!({
                            "reason": reason,
                            "threadId": 1,
                            "allThreadsStopped": true,
                        })),
                    ));
                } else {
                    if let Ok(mut sess) = session.lock() {
                        sess.exec_state = ExecState::Terminated;
                    }
                    emit_program_results(&session, &mut messages, &mut seq_counter);
                }
            }

            "terminate" | "disconnect" => {
                messages.push(make_response(&request, Some(serde_json::json!({}))));
                for msg in &messages {
                    stdout.write_all(msg.as_bytes()).await?;
                    stdout.flush().await?;
                }
                break;
            }

            _ => {
                messages.push(make_response(&request, Some(serde_json::json!({}))));
            }
        }

        for msg in &messages {
            stdout.write_all(msg.as_bytes()).await?;
            stdout.flush().await?;
        }
    }

    Ok(())
}

/// Emit captured program output and terminated/exited events.
fn emit_program_results(
    session: &Arc<Mutex<DapSession>>,
    messages: &mut Vec<String>,
    seq: &mut i64,
) {
    if let Ok(mut sess) = session.lock() {
        if let Some(ref out) = sess.program_output {
            *seq += 1;
            messages.push(make_event(
                *seq,
                "output",
                Some(serde_json::json!({
                    "category": "stdout",
                    "output": out,
                })),
            ));
        }
        if let Some(ref err) = sess.program_stderr {
            *seq += 1;
            messages.push(make_event(
                *seq,
                "output",
                Some(serde_json::json!({
                    "category": "stderr",
                    "output": err,
                })),
            ));
        }
        let code = sess.exit_code.unwrap_or(0);
        *seq += 1;
        messages.push(make_event(
            *seq,
            "exited",
            Some(serde_json::json!({ "exitCode": code })),
        ));
        *seq += 1;
        messages.push(make_event(*seq, "terminated", None));
        sess.exec_state = ExecState::Terminated;
    }
}

/// Find the enclosing function name for a given line (1-indexed).
fn find_enclosing_fn(lines: &[String], target_line: i64) -> String {
    for i in (0..target_line as usize).rev() {
        if i >= lines.len() {
            continue;
        }
        let trimmed = lines[i].trim();
        if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
            let name_start = if trimmed.starts_with("pub fn ") {
                "pub fn ".len()
            } else {
                "fn ".len()
            };
            let rest = &trimmed[name_start..];
            let name_end = rest.find('(').unwrap_or(rest.len());
            let name = rest[..name_end].trim();
            // Strip type parameters
            let name = name.split('<').next().unwrap_or(name);
            return name.to_string();
        }
    }
    "<module>".to_string()
}

/// Extract variables visible at the given line from static source analysis.
fn extract_visible_variables(lines: &[String], target_line: i64) -> Vec<serde_json::Value> {
    let mut vars = Vec::new();
    let mut seen = HashSet::new();

    for i in (0..(target_line as usize).min(lines.len())).rev() {
        let trimmed = lines[i].trim();
        // Stop at function boundary
        if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
            // Extract parameters
            if let Some(paren_start) = trimmed.find('(') {
                if let Some(paren_end) = trimmed.find(')') {
                    let params = &trimmed[paren_start + 1..paren_end];
                    for param in params.split(',') {
                        let param = param.trim();
                        if let Some(colon) = param.find(':') {
                            let name = param[..colon].trim().to_string();
                            let ty = param[colon + 1..].trim().to_string();
                            if !name.is_empty() && seen.insert(name.clone()) {
                                vars.push(serde_json::json!({
                                    "name": name,
                                    "value": format!("<{ty}>"),
                                    "type": ty,
                                    "variablesReference": 0,
                                }));
                            }
                        }
                    }
                }
            }
            break;
        }

        // Parse let bindings
        if trimmed.starts_with("let ") || trimmed.starts_with("let mut ") {
            let rest = if trimmed.starts_with("let mut ") {
                &trimmed["let mut ".len()..]
            } else {
                &trimmed["let ".len()..]
            };

            // Extract name (before : or =)
            let name_end = rest
                .find(':')
                .or_else(|| rest.find('='))
                .unwrap_or(rest.len());
            let name = rest[..name_end].trim().to_string();

            if !name.is_empty() && seen.insert(name.clone()) {
                // Try to extract type annotation
                let ty = if let Some(colon_pos) = rest.find(':') {
                    let after_colon = &rest[colon_pos + 1..];
                    let eq_pos = after_colon.find('=').unwrap_or(after_colon.len());
                    after_colon[..eq_pos].trim().to_string()
                } else {
                    "auto".to_string()
                };

                // Try to extract initializer value
                let value = if let Some(eq_pos) = rest.find('=') {
                    rest[eq_pos + 1..]
                        .trim()
                        .trim_end_matches(';')
                        .trim()
                        .to_string()
                } else {
                    format!("<{ty}>")
                };

                vars.push(serde_json::json!({
                    "name": name,
                    "value": value,
                    "type": ty,
                    "variablesReference": 0,
                }));
            }
        }
    }

    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_enclosing_fn() {
        let lines = vec![
            "fn main() {".to_string(),
            "  let x = 1".to_string(),
            "  print(x)".to_string(),
            "}".to_string(),
        ];
        assert_eq!(find_enclosing_fn(&lines, 2), "main");
        assert_eq!(find_enclosing_fn(&lines, 3), "main");
    }

    #[test]
    fn test_find_enclosing_fn_pub() {
        let lines = vec![
            "pub fn greet(name: str) {".to_string(),
            "  print(name)".to_string(),
            "}".to_string(),
        ];
        assert_eq!(find_enclosing_fn(&lines, 2), "greet");
    }

    #[test]
    fn test_extract_variables() {
        let lines = vec![
            "fn add(a: i32, b: i32) -> i32 {".to_string(),
            "  let sum = a + b".to_string(),
            "  let msg: str = \"hello\"".to_string(),
            "  print(sum)".to_string(),
            "}".to_string(),
        ];
        let vars = extract_visible_variables(&lines, 4);
        assert_eq!(vars.len(), 4); // sum, msg, a, b
        let names: Vec<&str> = vars.iter().map(|v| v["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"sum"));
        assert!(names.contains(&"msg"));
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }

    #[test]
    fn test_session_executable_lines() {
        let mut sess = DapSession::new();
        sess.source_lines = vec![
            "// comment".to_string(),
            "fn main() {".to_string(),
            "  let x = 1".to_string(),
            "".to_string(),
            "  print(x)".to_string(),
            "}".to_string(),
        ];
        sess.total_lines = 6;
        sess.executable_lines = (1..=6)
            .filter(|&line| {
                let idx = (line - 1) as usize;
                let trimmed = sess.source_lines[idx].trim();
                !trimmed.is_empty()
                    && !trimmed.starts_with("//")
                    && trimmed != "{"
                    && trimmed != "}"
                    && !trimmed.starts_with("import ")
            })
            .collect();

        // Line 1 is comment, line 4 is empty, line 6 is }
        assert!(!sess.executable_lines.contains(&1));
        assert!(sess.executable_lines.contains(&2)); // fn main() {
        assert!(sess.executable_lines.contains(&3)); // let x = 1
        assert!(!sess.executable_lines.contains(&4)); // empty
        assert!(sess.executable_lines.contains(&5)); // print(x)
        assert!(!sess.executable_lines.contains(&6)); // }
    }

    #[test]
    fn test_breakpoint_hit() {
        let mut sess = DapSession::new();
        sess.source_lines = vec![
            "fn main() {".to_string(),
            "  let x = 1".to_string(),
            "  let y = 2".to_string(),
            "  print(x + y)".to_string(),
            "}".to_string(),
        ];
        sess.total_lines = 5;
        sess.executable_lines = [1, 2, 3, 4].iter().copied().collect();
        sess.source_path = Some("test.ark".to_string());
        sess.breakpoints.insert(
            "test.ark".to_string(),
            vec![Breakpoint {
                line: 3,
                verified: true,
            }],
        );
        sess.current_line = 0;

        let stop = sess.advance_to_next_stop(StepMode::Continue);
        assert_eq!(stop, Some((3, "breakpoint".to_string())));
    }

    #[test]
    fn test_step_next() {
        let mut sess = DapSession::new();
        sess.source_lines = vec![
            "fn main() {".to_string(),
            "  let x = 1".to_string(),
            "  print(x)".to_string(),
            "}".to_string(),
        ];
        sess.total_lines = 4;
        sess.executable_lines = [1, 2, 3].iter().copied().collect();
        sess.current_line = 1;

        let stop = sess.advance_to_next_stop(StepMode::Next);
        assert_eq!(stop, Some((2, "step".to_string())));
    }
}
