//! Wasm runtime execution (wasmtime-based).

pub(crate) struct DirGrant {
    pub host_path: String,
    pub guest_path: String,
    pub read_only: bool,
}

pub(crate) struct RuntimeCaps {
    pub dirs: Vec<DirGrant>,
    pub deny_fs: bool,
    pub deny_clock: bool,
    pub deny_random: bool,
}

impl RuntimeCaps {
    pub fn from_cli(dirs: &[String], deny_fs: bool, deny_clock: bool, deny_random: bool) -> Self {
        let dir_grants = dirs.iter().map(|s| DirGrant::parse(s)).collect();
        RuntimeCaps {
            dirs: dir_grants,
            deny_fs,
            deny_clock,
            deny_random,
        }
    }
}

impl DirGrant {
    fn parse(s: &str) -> Self {
        if let Some(path) = s.strip_suffix(":ro") {
            DirGrant {
                host_path: path.to_string(),
                guest_path: path.to_string(),
                read_only: true,
            }
        } else if let Some(path) = s.strip_suffix(":rw") {
            DirGrant {
                host_path: path.to_string(),
                guest_path: path.to_string(),
                read_only: false,
            }
        } else {
            DirGrant {
                host_path: s.to_string(),
                guest_path: s.to_string(),
                read_only: false,
            }
        }
    }
}

pub(crate) fn run_wasm_p1(wasm_bytes: &[u8], caps: &RuntimeCaps) -> Result<(), String> {
    use wasmtime::*;
    use wasmtime_wasi::preview1::WasiP1Ctx;
    use wasmtime_wasi::{DirPerms, FilePerms, WasiCtxBuilder};

    // Disable Cranelift optimization as a precaution for T1's fixed linear-memory
    // scratch registers (SCRATCH=16, NWRITTEN=8). The primary flakiness root cause
    // was non-deterministic compilation (HashMap ordering in ark-resolve::analyze)
    // fixed in analyze.rs, but OptLevel::None is kept until the T1 emitter is
    // updated to use proper WASM locals instead of fixed absolute addresses.
    let mut config = Config::new();
    config.cranelift_opt_level(OptLevel::None);
    let engine = Engine::new(&config).map_err(|e| format!("engine creation error: {:?}", e))?;
    let module = wasmtime::Module::new(&engine, wasm_bytes)
        .map_err(|e| format!("wasm compile error: {:?}", e))?;

    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |cx| cx)
        .map_err(|e| format!("wasi link error: {}", e))?;

    // Register arukellt_host HTTP functions (conditional — only if the module imports them)
    let needs_http = module.imports().any(|imp| imp.module() == "arukellt_host");
    if needs_http {
        register_http_host_fns(&mut linker)?;
    }
    // Note: sockets_connect is T3-only and is never emitted for T1 modules.

    let mut builder = WasiCtxBuilder::new();
    builder.inherit_stdio();
    builder.inherit_env();
    builder.arg("arukellt-run");

    // deny_clock and deny_random are accepted but not yet enforced;
    // callers reject these flags before reaching this function.
    let _ = caps.deny_clock;
    let _ = caps.deny_random;

    if !caps.deny_fs {
        for grant in &caps.dirs {
            let (dp, fp) = if grant.read_only {
                (DirPerms::READ, FilePerms::READ)
            } else {
                (DirPerms::all(), FilePerms::all())
            };
            builder
                .preopened_dir(&grant.host_path, &grant.guest_path, dp, fp)
                .map_err(|e| format!("preopened dir error for '{}': {}", grant.host_path, e))?;
        }
    }
    let wasi_ctx = builder.build_p1();

    let mut store = Store::new(&engine, wasi_ctx);

    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("wasm instantiation error: {}", e))?;

    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|e| format!("missing _start: {}", e))?;

    start
        .call(&mut store, ())
        .map_err(|e| {
            // WASI proc_exit is reported as I32Exit — treat it as a clean exit
            if e.downcast_ref::<wasmtime_wasi::I32Exit>().is_some() {
                return String::new(); // signal "handled"
            }
            format!("runtime error: {}", e)
        })
        .or_else(|e| if e.is_empty() { Ok(()) } else { Err(e) })?;

    Ok(())
}

/// Run a Wasm GC module (T3 target) with wasmtime GC support enabled.
pub(crate) fn run_wasm_gc(wasm_bytes: &[u8], caps: &RuntimeCaps) -> Result<(), String> {
    use wasmtime::*;
    use wasmtime_wasi::preview1::WasiP1Ctx;
    use wasmtime_wasi::{DirPerms, FilePerms, WasiCtxBuilder};

    let mut config = Config::new();
    config.wasm_gc(true);
    // Enable the function-references proposal: required for ref.func and
    // return_call_ref instructions used by the T3 indirect tail-call path.
    config.wasm_function_references(true);
    // Use the null (non-collecting) GC to work around a wasmtime 29.x DRC bug
    // where `struct.get` results pushed onto the Wasm value stack are not
    // registered in the VMGcRefActivationsTable, causing a panic on the next
    // GC cycle.  The null collector never frees objects; this is acceptable for
    // short-lived program runs.  Track: upgrade wasmtime once ≥30 is tested.
    config.collector(Collector::Null);

    let engine =
        Engine::new(&config).map_err(|e| format!("engine creation error (GC): {:?}", e))?;
    let module = wasmtime::Module::new(&engine, wasm_bytes)
        .map_err(|e| format!("wasm compile error (GC): {:?}", e))?;

    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |cx| cx)
        .map_err(|e| format!("wasi link error: {}", e))?;

    // Register arukellt_host HTTP functions (conditional — only if the module imports them)
    let needs_http = module.imports().any(|imp| imp.module() == "arukellt_host");
    if needs_http {
        register_http_host_fns(&mut linker)?;
    }
    // Register arukellt_host sockets_connect (T3 only: TCP socket connect)
    let needs_sockets = module
        .imports()
        .any(|imp| imp.module() == "arukellt_host" && imp.name() == "sockets_connect");
    if needs_sockets {
        register_sockets_host_fns(&mut linker)?;
    }

    let mut builder = WasiCtxBuilder::new();
    builder.inherit_stdio();
    builder.inherit_env();
    builder.arg("arukellt-run");

    let _ = caps.deny_clock;
    let _ = caps.deny_random;

    if !caps.deny_fs {
        for grant in &caps.dirs {
            let (dp, fp) = if grant.read_only {
                (DirPerms::READ, FilePerms::READ)
            } else {
                (DirPerms::all(), FilePerms::all())
            };
            builder
                .preopened_dir(&grant.host_path, &grant.guest_path, dp, fp)
                .map_err(|e| format!("preopened dir error for '{}': {}", grant.host_path, e))?;
        }
    }
    let wasi_ctx = builder.build_p1();

    let mut store = Store::new(&engine, wasi_ctx);

    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("wasm instantiation error (GC): {}", e))?;

    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|e| format!("missing _start: {}", e))?;

    start
        .call(&mut store, ())
        .map_err(|e| {
            // WASI proc_exit is reported as I32Exit — treat it as a clean exit
            if e.downcast_ref::<wasmtime_wasi::I32Exit>().is_some() {
                return String::new(); // signal "handled"
            }
            format!("runtime error: {}", e)
        })
        .or_else(|e| if e.is_empty() { Ok(()) } else { Err(e) })?;

    Ok(())
}

/// Register `arukellt_host::http_get` and `arukellt_host::http_request` in the linker.
fn register_http_host_fns(
    linker: &mut wasmtime::Linker<wasmtime_wasi::preview1::WasiP1Ctx>,
) -> Result<(), String> {
    use wasmtime::*;

    // http_get(url_ptr: i32, url_len: i32, resp_ptr: i32) -> i32
    // Returns >= 0: Ok, response body length (bytes at resp_ptr)
    // Returns < 0: Err, error message length = abs(return) (bytes at resp_ptr)
    linker
        .func_wrap(
            "arukellt_host",
            "http_get",
            |mut caller: Caller<'_, wasmtime_wasi::preview1::WasiP1Ctx>,
             url_ptr: i32,
             url_len: i32,
             resp_ptr: i32|
             -> i32 {
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

    // http_request(method_ptr, method_len, url_ptr, url_len, body_ptr, body_len, resp_ptr) -> i32
    linker
        .func_wrap(
            "arukellt_host",
            "http_request",
            |mut caller: Caller<'_, wasmtime_wasi::preview1::WasiP1Ctx>,
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

    Ok(())
}

/// Read a UTF-8 string from Wasm linear memory.
fn read_string_from_mem(
    caller: &wasmtime::Caller<'_, wasmtime_wasi::preview1::WasiP1Ctx>,
    mem: &wasmtime::Memory,
    ptr: i32,
    len: i32,
) -> Result<String, String> {
    if len < 0 || ptr < 0 {
        return Err("invalid pointer/length".into());
    }
    let ptr = ptr as usize;
    let len = len as usize;
    let data = mem.data(caller);
    if ptr + len > data.len() {
        return Err("out of bounds memory access".into());
    }
    String::from_utf8(data[ptr..ptr + len].to_vec()).map_err(|_| "invalid UTF-8".into())
}

/// Write an Ok response to linear memory. Returns the positive body length.
fn write_ok(
    caller: &mut wasmtime::Caller<'_, wasmtime_wasi::preview1::WasiP1Ctx>,
    mem: &wasmtime::Memory,
    resp_ptr: i32,
    body: &[u8],
) -> i32 {
    let ptr = resp_ptr as usize;
    let data = mem.data_mut(caller);
    let end = ptr + body.len();
    if end <= data.len() {
        data[ptr..end].copy_from_slice(body);
    }
    body.len() as i32
}

/// Write an error message to linear memory. Returns -(error length).
fn write_error(
    caller: &mut wasmtime::Caller<'_, wasmtime_wasi::preview1::WasiP1Ctx>,
    resp_ptr: i32,
    msg: &str,
) -> i32 {
    let ptr = resp_ptr as usize;
    let bytes = msg.as_bytes();
    let data = caller.get_export("memory").and_then(|e| e.into_memory());
    if let Some(mem) = data {
        let d = mem.data_mut(caller);
        let end = ptr + bytes.len();
        if end <= d.len() {
            d[ptr..end].copy_from_slice(bytes);
        }
    }
    -(bytes.len() as i32)
}

/// Register `arukellt_host::sockets_connect` in the linker (T3 only).
///
/// Host ABI: `sockets_connect(host_ptr: i32, host_len: i32, port: i32, result_ptr: i32) -> i32`
/// - Returns >= 0: Ok, fd = return value (minimum implementation: always 3).
/// - Returns < 0: Err, abs(return) = error message length; message bytes written at result_ptr.
fn register_sockets_host_fns(
    linker: &mut wasmtime::Linker<wasmtime_wasi::preview1::WasiP1Ctx>,
) -> Result<(), String> {
    use wasmtime::*;

    linker
        .func_wrap(
            "arukellt_host",
            "sockets_connect",
            |mut caller: Caller<'_, wasmtime_wasi::preview1::WasiP1Ctx>,
             host_ptr: i32,
             host_len: i32,
             port: i32,
             result_ptr: i32|
             -> i32 {
                let mem = match caller.get_export("memory") {
                    Some(Extern::Memory(m)) => m,
                    _ => return write_error(&mut caller, result_ptr, "no memory export"),
                };
                let host = match read_string_from_mem(&caller, &mem, host_ptr, host_len) {
                    Ok(s) => s,
                    Err(e) => return write_error(&mut caller, result_ptr, &e),
                };
                // Validate port range (Ark i32, convert to u16)
                if !(0..=65535).contains(&port) {
                    let msg = format!("connect: invalid port {}", port);
                    return write_error(&mut caller, result_ptr, &msg);
                }
                match tcp_connect_impl(&host, port as u16) {
                    Ok(fd) => {
                        // Success: fd is returned directly as a positive i32.
                        // TODO(future): real fd management when socket read/write/close are added.
                        fd
                    }
                    Err(msg) => write_error(&mut caller, result_ptr, &msg),
                }
            },
        )
        .map_err(|e| format!("linker sockets_connect error: {}", e))?;

    Ok(())
}

/// TCP connect implementation.
///
/// Returns Ok(3) on success (minimum implementation; fd 3 is a placeholder — full fd
/// management is a future extension).  Returns Err(String) on failure.
///
/// Error format: `"connect: <host>:<port>: <reason>"` to match the error mapping spec
/// in docs/capability-surface.md.
fn tcp_connect_impl(host: &str, port: u16) -> Result<i32, String> {
    use std::net::TcpStream;
    use std::time::Duration;

    let socket_addr = to_socket_addr_for_connect(host, port)?;
    match TcpStream::connect_timeout(&socket_addr, Duration::from_secs(5)) {
        Ok(_stream) => {
            // TODO(future): store _stream in a fd table; return real fd.
            Ok(3)
        }
        Err(e) => {
            use std::io::ErrorKind;
            let reason = match e.kind() {
                ErrorKind::ConnectionRefused => "connection refused".to_string(),
                ErrorKind::TimedOut => "timed out".to_string(),
                _ => {
                    let msg = e.to_string().to_lowercase();
                    if msg.contains("refused") {
                        "connection refused".to_string()
                    } else if msg.contains("timed out") || msg.contains("timeout") {
                        "timed out".to_string()
                    } else {
                        e.to_string()
                    }
                }
            };
            Err(format!("connect: {}:{}: {}", host, port, reason))
        }
    }
}

/// Resolve a host:port pair to a single `SocketAddr`, returning a clean error on DNS failure.
fn to_socket_addr_for_connect(host: &str, port: u16) -> Result<std::net::SocketAddr, String> {
    use std::net::ToSocketAddrs;
    let addr_str = format!("{}:{}", host, port);
    let mut addrs = addr_str.to_socket_addrs().map_err(|e| {
        let msg = e.to_string().to_lowercase();
        if msg.contains("name or service not known")
            || msg.contains("nodename nor servname")
            || msg.contains("no such host")
            || msg.contains("failed to lookup")
            || msg.contains("name resolution")
        {
            format!("connect: {}:{}: dns not found", host, port)
        } else {
            format!("connect: {}:{}: {}", host, port, e)
        }
    })?;
    addrs
        .next()
        .ok_or_else(|| format!("connect: {}:{}: dns not found", host, port))
}

/// TCP-based HTTP/1.1 GET implementation.
fn http_get_impl(url: &str) -> Result<String, String> {
    http_request_impl("GET", url, "")
}

/// TCP-based HTTP/1.1 request implementation.
///
/// Error mapping (matches the spec in docs/capability-surface.md):
/// - DNS resolution failure  → `"dns: <host>: not found"`
/// - Connection refused      → `"connection refused: <url>"`
/// - Connection/read timeout → `"timeout: <url>"`
/// - HTTP 4xx or 5xx status  → `"http <status>: <url>"`
/// - Any other failure       → `"error: <message>"`
fn http_request_impl(method: &str, url: &str, body: &str) -> Result<String, String> {
    use std::io::{Read, Write};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    // Parse URL: expect http://host[:port]/path
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
                .map_err(|_| "invalid port")?,
        ),
        None => (host_port, 80u16),
    };

    // Resolve DNS first so we can produce a clean "dns: … not found" error.
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

    // Connect
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

    // Send HTTP/1.1 request
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

    // Read response
    let mut response = Vec::new();
    stream.read_to_end(&mut response).map_err(|e| {
        if e.kind() == std::io::ErrorKind::TimedOut {
            format!("timeout: {}", url)
        } else {
            format!("error: {}", e)
        }
    })?;

    let response_str = String::from_utf8_lossy(&response);

    // Parse HTTP response: split headers from body, map 4xx/5xx to errors.
    if let Some(header_end) = response_str.find("\r\n\r\n") {
        let status_line = &response_str[..response_str.find("\r\n").unwrap_or(header_end)];
        let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
        if parts.len() >= 2 {
            let status: u16 = parts[1].parse().unwrap_or(0);
            if status >= 400 {
                // "http <status>: <url>" — URL rather than body keeps the error concise.
                return Err(format!("http {}: {}", status, url));
            }
        }
        Ok(response_str[header_end + 4..].to_string())
    } else {
        Err("error: malformed HTTP response (no header terminator)".into())
    }
}
