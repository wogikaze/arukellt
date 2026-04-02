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

    let engine = Engine::default();
    let module = wasmtime::Module::new(&engine, wasm_bytes)
        .map_err(|e| format!("wasm compile error: {:?}", e))?;

    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |cx| cx)
        .map_err(|e| format!("wasi link error: {}", e))?;

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

/// TCP-based HTTP/1.1 GET implementation.
fn http_get_impl(url: &str) -> Result<String, String> {
    http_request_impl("GET", url, "")
}

/// TCP-based HTTP/1.1 request implementation.
fn http_request_impl(method: &str, url: &str, body: &str) -> Result<String, String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    // Parse URL: expect http://host[:port]/path
    let url_str = url;
    let (scheme, rest) = if let Some(r) = url_str.strip_prefix("http://") {
        ("http", r)
    } else if let Some(_r) = url_str.strip_prefix("https://") {
        return Err("https is not supported (TCP HTTP/1.1 only)".into());
    } else {
        return Err(format!("unsupported URL scheme: {}", url_str));
    };
    let _ = scheme;

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

    // Connect
    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect(&addr).map_err(|e| format!("connection error: {}", e))?;
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
        .map_err(|e| format!("write error: {}", e))?;

    // Read response
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|e| format!("read error: {}", e))?;

    let response_str = String::from_utf8_lossy(&response);

    // Parse HTTP response: find end of headers, return body
    if let Some(header_end) = response_str.find("\r\n\r\n") {
        let status_line = &response_str[..response_str.find("\r\n").unwrap_or(header_end)];
        // Parse status code
        let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
        if parts.len() >= 2 {
            let status: u16 = parts[1].parse().unwrap_or(0);
            if status >= 400 {
                return Err(format!(
                    "HTTP {}: {}",
                    status,
                    &response_str[header_end + 4..]
                ));
            }
        }
        Ok(response_str[header_end + 4..].to_string())
    } else {
        Err("malformed HTTP response: no header terminator".into())
    }
}
