//! TCP HTTP/1.1 host implementations for `arukellt_host::http_get` / `http_request` / `http_serve`.

use crate::{read_string_from_mem, write_error, write_ok};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::OnceLock;
use std::time::Duration;
use wasmtime::*;
use wasmtime_wasi::preview1::WasiP1Ctx;

static INCOMING_PORT: OnceLock<u16> = OnceLock::new();

pub fn ensure_http_incoming_client_helper() -> u16 {
    *INCOMING_PORT.get_or_init(|| {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind incoming helper");
        let port = listener.local_addr().expect("incoming helper addr").port();
        drop(listener);
        std::env::set_var("ARUKELLT_HTTP_INCOMING_PORT", port.to_string());
        std::thread::spawn(move || incoming_client_loop(port));
        port
    })
}

fn incoming_client_loop(port: u16) {
    use std::io::ErrorKind;
    use std::net::ToSocketAddrs;

    let addr = format!("127.0.0.1:{}", port);
    let socket_addr = match addr.to_socket_addrs().and_then(|mut a| {
        a.next()
            .ok_or_else(|| std::io::Error::new(ErrorKind::InvalidInput, "incoming client addr"))
    }) {
        Ok(sa) => sa,
        Err(_) => return,
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        match TcpStream::connect_timeout(&socket_addr, Duration::from_millis(200)) {
            Ok(mut stream) => {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
                let request =
                    "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
                if stream.write_all(request.as_bytes()).is_err() {
                    return;
                }
                let mut response = Vec::new();
                let _ = stream.read_to_end(&mut response);
                return;
            }
            Err(ref e) if e.kind() == ErrorKind::ConnectionRefused || e.kind() == ErrorKind::TimedOut => {
                if std::time::Instant::now() >= deadline {
                    return;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(_) => return,
        }
    }
}

pub fn register_http_host_fns(linker: &mut Linker<WasiP1Ctx>) -> Result<(), String> {
    ensure_http_incoming_client_helper();
    linker
        .func_wrap(
            "arukellt_host",
            "http_get",
            |mut caller: Caller<'_, WasiP1Ctx>, url_ptr: i32, url_len: i32, resp_ptr: i32| -> i32 {
                let mem = match caller.get_export("memory") {
                    Some(Extern::Memory(m)) => m,
                    _ => return write_error(&mut caller, resp_ptr, "no memory export"),
                };
                let url = match read_string_from_mem(&caller, &mem, url_ptr, url_len) {
                    Ok(s) => s,
                    Err(e) => return write_error(&mut caller, resp_ptr, &e),
                };
                match http_get_impl(&url) {
                    Ok(body) => write_ok(&mut caller, &mem, resp_ptr, body.as_bytes()),
                    Err(e) => write_error(&mut caller, resp_ptr, &e),
                }
            },
        )
        .map_err(|e| format!("linker http_get error: {}", e))?;

    linker
        .func_wrap(
            "arukellt_host",
            "http_request",
            |mut caller: Caller<'_, WasiP1Ctx>,
             method_ptr: i32,
             method_len: i32,
             url_ptr: i32,
             url_len: i32,
             body_ptr: i32,
             body_len: i32,
             resp_ptr: i32|
             -> i32 {
                let mem = match caller.get_export("memory") {
                    Some(Extern::Memory(m)) => m,
                    _ => return write_error(&mut caller, resp_ptr, "no memory export"),
                };
                let method = match read_string_from_mem(&caller, &mem, method_ptr, method_len) {
                    Ok(s) => s,
                    Err(e) => return write_error(&mut caller, resp_ptr, &e),
                };
                let url = match read_string_from_mem(&caller, &mem, url_ptr, url_len) {
                    Ok(s) => s,
                    Err(e) => return write_error(&mut caller, resp_ptr, &e),
                };
                let body = match read_string_from_mem(&caller, &mem, body_ptr, body_len) {
                    Ok(s) => s,
                    Err(e) => return write_error(&mut caller, resp_ptr, &e),
                };
                match http_request_impl(&method, &url, &body) {
                    Ok(resp) => write_ok(&mut caller, &mem, resp_ptr, resp.as_bytes()),
                    Err(e) => write_error(&mut caller, resp_ptr, &e),
                }
            },
        )
        .map_err(|e| format!("linker http_request error: {}", e))?;

    linker
        .func_wrap(
            "arukellt_host",
            "http_serve",
            |mut caller: Caller<'_, WasiP1Ctx>,
             port: i32,
             body_ptr: i32,
             body_len: i32,
             resp_ptr: i32|
             -> i32 {
                let mem = match caller.get_export("memory") {
                    Some(Extern::Memory(m)) => m,
                    _ => return write_error(&mut caller, resp_ptr, "no memory export"),
                };
                let body = match read_string_from_mem(&caller, &mem, body_ptr, body_len) {
                    Ok(s) => s,
                    Err(e) => return write_error(&mut caller, resp_ptr, &e),
                };
                match http_serve_impl(port, &body) {
                    Ok(()) => 0,
                    Err(e) => write_error(&mut caller, resp_ptr, &e),
                }
            },
        )
        .map_err(|e| format!("linker http_serve error: {}", e))?;

    Ok(())
}

fn http_get_impl(url: &str) -> Result<String, String> {
    http_request_impl("GET", url, "")
}

fn http_request_impl(method: &str, url: &str, body: &str) -> Result<String, String> {
    use std::io::{Read, Write};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let rest = if let Some(r) = url.strip_prefix("http://") {
        r
    } else if url.starts_with("https://") {
        return Err("https is not supported (TCP HTTP/1.1 only)".into());
    } else {
        return Err(format!("unsupported URL scheme: {}", url));
    };

    let (host_port, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let (host, port) = match host_port.find(':') {
        Some(i) => (
            &host_port[..i],
            host_port[i + 1..]
                .parse::<u16>()
                .map_err(|_| "invalid port".to_string())?,
        ),
        None => (host_port, 80u16),
    };

    let addr_str = format!("{}:{}", host, port);
    let addrs: Vec<_> = addr_str
        .to_socket_addrs()
        .map_err(|e| {
            let msg = e.to_string().to_lowercase();
            if msg.contains("name or service not known")
                || msg.contains("nodename nor servname")
                || msg.contains("no such host")
                || msg.contains("failed to lookup")
                || msg.contains("name resolution")
            {
                format!("dns: {}: not found", host)
            } else {
                format!("error: {}", e)
            }
        })?
        .collect();

    let mut stream = TcpStream::connect(addrs.as_slice()).map_err(|e| {
        use std::io::ErrorKind;
        match e.kind() {
            ErrorKind::ConnectionRefused => format!("connection refused: {}", url),
            ErrorKind::TimedOut => format!("timeout: {}", url),
            _ => {
                let msg = e.to_string().to_lowercase();
                if msg.contains("refused") {
                    format!("connection refused: {}", url)
                } else if msg.contains("timed out") || msg.contains("timeout") {
                    format!("timeout: {}", url)
                } else {
                    format!("error: {}", e)
                }
            }
        }
    })?;
    stream.set_read_timeout(Some(Duration::from_secs(30))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(10))).ok();

    let request = if body.is_empty() {
        format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nUser-Agent: arukellt/0.1\r\n\r\n",
            method, path, host
        )
    } else {
        format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nUser-Agent: arukellt/0.1\r\nContent-Length: {}\r\n\r\n{}",
            method,
            path,
            host,
            body.len(),
            body
        )
    };
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("error: {}", e))?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response).map_err(|e| {
        if e.kind() == std::io::ErrorKind::TimedOut {
            format!("timeout: {}", url)
        } else {
            format!("error: {}", e)
        }
    })?;

    let response_str = String::from_utf8_lossy(&response);

    if let Some(header_end) = response_str.find("\r\n\r\n") {
        let status_line = &response_str[..response_str.find("\r\n").unwrap_or(header_end)];
        let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
        if parts.len() >= 2 {
            let status: u16 = parts[1].parse().unwrap_or(0);
            if status >= 400 {
                return Err(format!("http {}: {}", status, url));
            }
        }
        Ok(response_str[header_end + 4..].to_string())
    } else {
        Err("error: malformed HTTP response (no header terminator)".into())
    }
}

fn http_serve_impl(port: i32, body: &str) -> Result<(), String> {
    use std::io::ErrorKind;

    if !(0..=65535).contains(&port) {
        return Err(format!("serve: invalid port {}", port));
    }
    let addr = format!("127.0.0.1:{}", port);
    let listener =
        TcpListener::bind(&addr).map_err(|e| format!("serve: {}: {}", port, e))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("serve: {}: {}", port, e))?;
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let (mut stream, _) = loop {
        match listener.accept() {
            Ok(pair) => break pair,
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                if std::time::Instant::now() >= deadline {
                    return Err(format!("serve: {}: timed out", port));
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) => return Err(format!("serve: {}: {}", port, e)),
        }
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let mut buf = [0u8; 4096];
    let _ = stream.read(&mut buf);
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|e| format!("serve: write: {}", e))?;
    let _ = stream.flush();
    let _ = stream.shutdown(std::net::Shutdown::Write);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{http_get_impl, http_serve_impl};
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;

    #[test]
    fn http_get_dns_error() {
        let err = http_get_impl("http://this.domain.does.not.exist.invalid/")
            .unwrap_err();
        assert!(
            err.starts_with("dns:"),
            "expected dns error, got {err:?}"
        );
    }

    #[test]
    fn http_incoming_roundtrip() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let handle = std::thread::spawn(move || {
            http_serve_impl(port as i32, "hello").expect("serve");
        });
        let addr = format!("127.0.0.1:{}", port);
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if let Ok(mut client) =
                TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_millis(200))
            {
                client
                    .write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
                    .unwrap();
                let mut buf = Vec::new();
                let _ = client.read_to_end(&mut buf);
                let text = String::from_utf8_lossy(&buf);
                assert!(text.contains("hello"), "response missing body: {text}");
                break;
            }
            if std::time::Instant::now() >= deadline {
                panic!("incoming client timed out");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        handle.join().unwrap();
    }
}
