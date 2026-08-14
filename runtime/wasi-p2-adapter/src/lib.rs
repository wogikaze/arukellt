use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Mutex, OnceLock};

use wasip2::http::outgoing_handler;
use wasip2::http::types::{Fields, IncomingBody, Method, OutgoingBody, OutgoingRequest, Scheme};
use wasip2::io::streams::StreamError;

wit_bindgen::generate!({
    path: "wit",
    world: "runtime-adapter",
});

struct RuntimeAdapter;

enum HandleValue {
    Buffer(Vec<u8>),
    Stream(TcpStream),
    Listener(TcpListener),
}

#[derive(Default)]
struct RuntimeState {
    handles: Vec<Option<HandleValue>>,
}

impl RuntimeState {
    fn insert(&mut self, value: HandleValue) -> u32 {
        for (index, slot) in self.handles.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(value);
                return (index + 1) as u32;
            }
        }
        self.handles.push(Some(value));
        self.handles.len() as u32
    }

    fn get(&self, handle: u32) -> Option<&HandleValue> {
        handle.checked_sub(1).and_then(|index| self.handles.get(index as usize))?.as_ref()
    }

    fn get_mut(&mut self, handle: u32) -> Option<&mut HandleValue> {
        handle
            .checked_sub(1)
            .and_then(|index| self.handles.get_mut(index as usize))?
            .as_mut()
    }

    fn remove(&mut self, handle: u32) {
        if let Some(slot) = handle
            .checked_sub(1)
            .and_then(|index| self.handles.get_mut(index as usize))
        {
            *slot = None;
        }
    }
}

fn state() -> &'static Mutex<RuntimeState> {
    static STATE: OnceLock<Mutex<RuntimeState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(RuntimeState::default()))
}

fn store_buffer(bytes: Vec<u8>) -> i32 {
    state().lock().expect("runtime state poisoned").insert(HandleValue::Buffer(bytes)) as i32
}

fn store_error(message: impl Into<String>) -> i32 {
    let handle = store_buffer(message.into().into_bytes());
    -handle
}

fn store_io<T>(result: std::io::Result<T>, ok: impl FnOnce(T) -> i32) -> i32 {
    match result {
        Ok(value) => ok(value),
        Err(error) => store_error(error.to_string()),
    }
}

fn decode_http_url(url: &str) -> Result<(&str, &str), String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| "only plaintext http:// URLs are supported".to_string())?;
    let split = rest.find('/').unwrap_or(rest.len());
    let authority = &rest[..split];
    if authority.is_empty() {
        return Err("HTTP URL is missing an authority".to_string());
    }
    let path = if split == rest.len() { "/" } else { &rest[split..] };
    Ok((authority, path))
}

fn method_from_string(method: &str) -> Method {
    match method {
        "GET" => Method::Get,
        "HEAD" => Method::Head,
        "POST" => Method::Post,
        "PUT" => Method::Put,
        "DELETE" => Method::Delete,
        "CONNECT" => Method::Connect,
        "OPTIONS" => Method::Options,
        "TRACE" => Method::Trace,
        "PATCH" => Method::Patch,
        other => Method::Other(other.to_string()),
    }
}

fn http_request(method: &str, url: &str, request_body: &[u8]) -> Result<Vec<u8>, String> {
    let (authority, path) = decode_http_url(url)?;
    let headers = Fields::new();
    let request = OutgoingRequest::new(headers);
    let method = method_from_string(method);
    request
        .set_method(&method)
        .map_err(|_| "invalid HTTP method".to_string())?;
    request
        .set_scheme(Some(&Scheme::Http))
        .map_err(|_| "invalid HTTP scheme".to_string())?;
    request
        .set_authority(Some(authority))
        .map_err(|_| "invalid HTTP authority".to_string())?;
    request
        .set_path_with_query(Some(path))
        .map_err(|_| "invalid HTTP path/query".to_string())?;

    if !request_body.is_empty() {
        let body = request
            .body()
            .map_err(|_| "HTTP request body is unavailable".to_string())?;
        {
            let stream = body
                .write()
                .map_err(|_| "HTTP request body stream is unavailable".to_string())?;
            stream
                .blocking_write_and_flush(request_body)
                .map_err(|error| format!("HTTP request body write failed: {error:?}"))?;
        }
        OutgoingBody::finish(body, None)
            .map_err(|error| format!("HTTP request body finish failed: {error:?}"))?;
    }

    let future = outgoing_handler::handle(request, None)
        .map_err(|error| format!("HTTP request rejected: {error:?}"))?;
    loop {
        if let Some(result) = future.get() {
            let response = result
                .map_err(|_| "HTTP response future already consumed".to_string())?
                .map_err(|error| format!("HTTP response failed: {error:?}"))?;
            let body = response
                .consume()
                .map_err(|_| "HTTP response body is unavailable".to_string())?;
            let mut bytes = Vec::new();
            {
                let stream = body
                    .stream()
                    .map_err(|_| "HTTP response stream is unavailable".to_string())?;
                loop {
                    match stream.blocking_read(64 * 1024) {
                        Ok(chunk) => {
                            if chunk.is_empty() {
                                continue;
                            }
                            bytes.extend_from_slice(&chunk);
                        }
                        Err(StreamError::Closed) => break,
                        Err(error) => return Err(format!("HTTP response read failed: {error:?}")),
                    }
                }
            }
            let _ = IncomingBody::finish(body);
            return Ok(bytes);
        }
        let pollable = future.subscribe();
        pollable.block();
    }
}

fn socket_connect(host: &str, port: u16) -> Result<TcpStream, String> {
    TcpStream::connect((host, port)).map_err(|error| format!("socket connect failed: {error}"))
}

fn http_serve_once(port: u16, body: &str) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|error| format!("HTTP listen failed: {error}"))?;
    let (mut stream, _) = listener
        .accept()
        .map_err(|error| format!("HTTP accept failed: {error}"))?;
    let mut request = [0u8; 4096];
    let _ = stream.read(&mut request);
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(header.as_bytes())
        .and_then(|_| stream.write_all(body.as_bytes()))
        .and_then(|_| stream.flush())
        .map_err(|error| format!("HTTP response write failed: {error}"))
}

impl exports::arukellt::runtime::host::Guest for RuntimeAdapter {
    fn buffer_len(handle: u32) -> u32 {
        let guard = state().lock().expect("runtime state poisoned");
        match guard.get(handle) {
            Some(HandleValue::Buffer(bytes)) => bytes.len() as u32,
            _ => 0,
        }
    }

    fn buffer_byte(handle: u32, index: u32) -> u8 {
        let guard = state().lock().expect("runtime state poisoned");
        match guard.get(handle) {
            Some(HandleValue::Buffer(bytes)) => bytes.get(index as usize).copied().unwrap_or(0),
            _ => 0,
        }
    }

    fn buffer_close(handle: u32) {
        state().lock().expect("runtime state poisoned").remove(handle);
    }

    fn http_get(url: String) -> i32 {
        match http_request("GET", &url, &[]) {
            Ok(bytes) => store_buffer(bytes),
            Err(error) => store_error(error),
        }
    }

    fn http_request(method: String, url: String, body: String) -> i32 {
        match http_request(&method, &url, body.as_bytes()) {
            Ok(bytes) => store_buffer(bytes),
            Err(error) => store_error(error),
        }
    }

    fn http_serve(port: u16, body: String) -> i32 {
        match http_serve_once(port, &body) {
            Ok(()) => 0,
            Err(error) => store_error(error),
        }
    }

    fn socket_connect(host: String, port: u16) -> i32 {
        match socket_connect(&host, port) {
            Ok(stream) => state()
                .lock()
                .expect("runtime state poisoned")
                .insert(HandleValue::Stream(stream)) as i32,
            Err(error) => store_error(error),
        }
    }

    fn socket_read(socket: u32, max_len: u32) -> i32 {
        let mut guard = state().lock().expect("runtime state poisoned");
        let Some(HandleValue::Stream(stream)) = guard.get_mut(socket) else {
            drop(guard);
            return store_error("invalid socket handle");
        };
        let mut bytes = vec![0u8; max_len as usize];
        match stream.read(&mut bytes) {
            Ok(read) => {
                bytes.truncate(read);
                guard.insert(HandleValue::Buffer(bytes)) as i32
            }
            Err(error) => {
                drop(guard);
                store_error(format!("socket read failed: {error}"))
            }
        }
    }

    fn socket_write(socket: u32, bytes: Vec<u8>) -> i32 {
        let mut guard = state().lock().expect("runtime state poisoned");
        let Some(HandleValue::Stream(stream)) = guard.get_mut(socket) else {
            drop(guard);
            return store_error("invalid socket handle");
        };
        match stream.write(&bytes) {
            Ok(written) => written as i32,
            Err(error) => {
                drop(guard);
                store_error(format!("socket write failed: {error}"))
            }
        }
    }

    fn socket_listen(host: String, port: u16) -> i32 {
        match TcpListener::bind((host.as_str(), port)) {
            Ok(listener) => state()
                .lock()
                .expect("runtime state poisoned")
                .insert(HandleValue::Listener(listener)) as i32,
            Err(error) => store_error(format!("socket listen failed: {error}")),
        }
    }

    fn socket_accept(listener: u32) -> i32 {
        let mut guard = state().lock().expect("runtime state poisoned");
        let Some(HandleValue::Listener(listener)) = guard.get_mut(listener) else {
            drop(guard);
            return store_error("invalid listener handle");
        };
        match listener.accept() {
            Ok((stream, _)) => guard.insert(HandleValue::Stream(stream)) as i32,
            Err(error) => {
                drop(guard);
                store_error(format!("socket accept failed: {error}"))
            }
        }
    }

    fn socket_close(handle: u32) {
        state().lock().expect("runtime state poisoned").remove(handle);
    }

    fn fs_read_file(path: String) -> i32 {
        store_io(fs::read(path), |bytes| store_buffer(bytes))
    }

    fn fs_write_file(path: String, contents: String) -> i32 {
        store_io(fs::write(path, contents.as_bytes()), |_| 0)
    }

    fn fs_write_bytes(path: String, contents: Vec<u8>) -> i32 {
        store_io(fs::write(path, contents), |_| 0)
    }

    fn fs_read_dir(path: String) -> i32 {
        match fs::read_dir(path) {
            Ok(entries) => {
                let mut names = Vec::new();
                for entry in entries {
                    match entry {
                        Ok(entry) => names.push(entry.file_name().to_string_lossy().into_owned()),
                        Err(error) => return store_error(error.to_string()),
                    }
                }
                names.sort();
                store_buffer(names.join("\n").into_bytes())
            }
            Err(error) => store_error(error.to_string()),
        }
    }

    fn fs_metadata(path: String) -> i32 {
        match fs::metadata(path) {
            Ok(metadata) => store_buffer(
                format!(
                    "{}\t{}\t{}",
                    metadata.len(),
                    if metadata.is_file() { 1 } else { 0 },
                    if metadata.is_dir() { 1 } else { 0 }
                )
                .into_bytes(),
            ),
            Err(error) => store_error(error.to_string()),
        }
    }

    fn fs_remove_file(path: String) -> i32 {
        store_io(fs::remove_file(path), |_| 0)
    }

    fn fs_create_dir_all(path: String) -> i32 {
        store_io(fs::create_dir_all(path), |_| 0)
    }

    fn env_vars() -> i32 {
        let mut values: Vec<String> = std::env::vars()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        values.sort();
        store_buffer(values.join("\0").into_bytes())
    }

    fn env_current_dir() -> i32 {
        match std::env::current_dir() {
            Ok(path) => store_buffer(path.to_string_lossy().as_bytes().to_vec()),
            Err(error) => store_error(error.to_string()),
        }
    }

    fn env_var(name: String) -> i32 {
        match std::env::var(name) {
            Ok(value) => store_buffer(value.into_bytes()),
            Err(error) => store_error(error.to_string()),
        }
    }
}

export!(RuntimeAdapter);
