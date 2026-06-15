//! TCP socket host implementations for `arukellt_host::{sockets_connect,sockets_read,sockets_write,sockets_listen,sockets_accept}`.

use crate::{read_string_from_mem, write_error};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use wasmtime::*;
use wasmtime_wasi::preview1::WasiP1Ctx;

const SOCKET_FD: i32 = 3;
const LISTENER_FD: i32 = 4;

static SOCKET_TABLE: Mutex<Option<HashMap<i32, TcpStream>>> = Mutex::new(None);
static LISTENER_TABLE: Mutex<Option<HashMap<i32, TcpListener>>> = Mutex::new(None);
static ECHO_PORT: OnceLock<u16> = OnceLock::new();

pub fn ensure_socket_echo_server() -> u16 {
    *ECHO_PORT.get_or_init(|| {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind echo listener");
        listener
            .set_nonblocking(true)
            .expect("echo listener nonblocking");
        let port = listener.local_addr().expect("echo listener addr").port();
        std::env::set_var("ARUKELLT_SOCKET_ECHO_PORT", port.to_string());
        std::thread::spawn(move || echo_server_loop(listener));
        port
    })
}

fn echo_server_loop(listener: TcpListener) {
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
                let mut buf = [0u8; 4096];
                match stream.read(&mut buf) {
                    Ok(0) => {}
                    Ok(n) => {
                        let _ = stream.write_all(&buf[..n]);
                    }
                    Err(_) => {}
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(_) => break,
        }
    }
}

pub fn register_sockets_host_fns(linker: &mut Linker<WasiP1Ctx>) -> Result<(), String> {
    ensure_socket_echo_server();

    linker
        .func_wrap(
            "arukellt_host",
            "sockets_connect",
            |mut caller: Caller<'_, WasiP1Ctx>,
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
                if !(0..=65535).contains(&port) {
                    let msg = format!("connect: invalid port {}", port);
                    return write_error(&mut caller, result_ptr, &msg);
                }
                match tcp_connect_impl(&host, port as u16) {
                    Ok(fd) => fd,
                    Err(msg) => write_error(&mut caller, result_ptr, &msg),
                }
            },
        )
        .map_err(|e| format!("linker sockets_connect error: {}", e))?;

    linker
        .func_wrap(
            "arukellt_host",
            "sockets_read",
            |mut caller: Caller<'_, WasiP1Ctx>, fd: i32, max_len: i32, result_ptr: i32| -> i32 {
                if max_len < 0 {
                    return write_error(&mut caller, result_ptr, "read: invalid max_len");
                }
                let mem = match caller.get_export("memory") {
                    Some(Extern::Memory(m)) => m,
                    _ => return write_error(&mut caller, result_ptr, "no memory export"),
                };
                let mut stream = match take_socket(fd) {
                    Ok(s) => s,
                    Err(msg) => return write_error(&mut caller, result_ptr, &msg),
                };
                let cap = max_len.min(4096) as usize;
                let mut buf = vec![0u8; cap];
                match stream.read(&mut buf) {
                    Ok(n) => {
                        insert_socket(fd, stream);
                        write_ok_bytes(&mut caller, &mem, result_ptr, &buf[..n])
                    }
                    Err(e) => {
                        insert_socket(fd, stream);
                        write_error(
                            &mut caller,
                            result_ptr,
                            &format!("read: {}: {}", fd, e),
                        )
                    }
                }
            },
        )
        .map_err(|e| format!("linker sockets_read error: {}", e))?;

    linker
        .func_wrap(
            "arukellt_host",
            "sockets_write",
            |mut caller: Caller<'_, WasiP1Ctx>,
             fd: i32,
             buf_ptr: i32,
             buf_len: i32,
             result_ptr: i32|
             -> i32 {
                if buf_len < 0 || buf_ptr < 0 {
                    return write_error(&mut caller, result_ptr, "write: invalid buffer");
                }
                let mem = match caller.get_export("memory") {
                    Some(Extern::Memory(m)) => m,
                    _ => return write_error(&mut caller, result_ptr, "no memory export"),
                };
                let bytes = match read_bytes_from_mem(&caller, &mem, buf_ptr, buf_len) {
                    Ok(b) => b,
                    Err(e) => return write_error(&mut caller, result_ptr, &e),
                };
                let mut stream = match take_socket(fd) {
                    Ok(s) => s,
                    Err(msg) => return write_error(&mut caller, result_ptr, &msg),
                };
                match stream.write_all(&bytes) {
                    Ok(()) => {
                        insert_socket(fd, stream);
                        bytes.len() as i32
                    }
                    Err(e) => {
                        insert_socket(fd, stream);
                        write_error(
                            &mut caller,
                            result_ptr,
                            &format!("write: {}: {}", fd, e),
                        )
                    }
                }
            },
        )
        .map_err(|e| format!("linker sockets_write error: {}", e))?;

    linker
        .func_wrap(
            "arukellt_host",
            "sockets_listen",
            |mut caller: Caller<'_, WasiP1Ctx>,
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
                if !(0..=65535).contains(&port) {
                    let msg = format!("listen: invalid port {}", port);
                    return write_error(&mut caller, result_ptr, &msg);
                }
                match tcp_listen_impl(&host, port as u16) {
                    Ok(fd) => fd,
                    Err(msg) => write_error(&mut caller, result_ptr, &msg),
                }
            },
        )
        .map_err(|e| format!("linker sockets_listen error: {}", e))?;

    linker
        .func_wrap(
            "arukellt_host",
            "sockets_accept",
            |mut caller: Caller<'_, WasiP1Ctx>, listener_fd: i32, result_ptr: i32| -> i32 {
                match tcp_accept_impl(listener_fd) {
                    Ok(client_fd) => client_fd,
                    Err(msg) => write_error(&mut caller, result_ptr, &msg),
                }
            },
        )
        .map_err(|e| format!("linker sockets_accept error: {}", e))?;

    Ok(())
}

fn take_socket(fd: i32) -> Result<TcpStream, String> {
    let mut guard = SOCKET_TABLE
        .lock()
        .map_err(|_| "read: socket table poisoned".to_string())?;
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    let table = guard
        .as_mut()
        .ok_or_else(|| "read: socket table unavailable".to_string())?;
    table
        .remove(&fd)
        .ok_or_else(|| format!("read: unknown socket fd {}", fd))
}

fn insert_socket(fd: i32, stream: TcpStream) {
    if let Ok(mut guard) = SOCKET_TABLE.lock() {
        if guard.is_none() {
            *guard = Some(HashMap::new());
        }
        if let Some(table) = guard.as_mut() {
            table.insert(fd, stream);
        }
    }
}

fn write_ok_bytes(
    caller: &mut Caller<'_, WasiP1Ctx>,
    mem: &Memory,
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

fn read_bytes_from_mem(
    caller: &Caller<'_, WasiP1Ctx>,
    mem: &Memory,
    ptr: i32,
    len: i32,
) -> Result<Vec<u8>, String> {
    if len < 0 || ptr < 0 {
        return Err("invalid pointer/length".into());
    }
    let ptr = ptr as usize;
    let len = len as usize;
    let data = mem.data(caller);
    if ptr + len > data.len() {
        return Err("out of bounds memory access".into());
    }
    Ok(data[ptr..ptr + len].to_vec())
}

fn tcp_connect_impl(host: &str, port: u16) -> Result<i32, String> {
    use std::io::ErrorKind;

    let socket_addr = to_socket_addr_for_connect(host, port)?;
    match TcpStream::connect_timeout(&socket_addr, Duration::from_secs(5)) {
        Ok(stream) => {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
            insert_socket(SOCKET_FD, stream);
            Ok(SOCKET_FD)
        }
        Err(e) => {
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

pub fn ensure_listen_client_helper(port: u16) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        let addr = format!("127.0.0.1:{}", port);
        if let Ok(addr) = addr.parse() {
            let _ = TcpStream::connect_timeout(&addr, Duration::from_secs(5));
        }
    });
}

fn insert_listener(fd: i32, listener: TcpListener) {
    if let Ok(mut guard) = LISTENER_TABLE.lock() {
        if guard.is_none() {
            *guard = Some(HashMap::new());
        }
        if let Some(table) = guard.as_mut() {
            table.insert(fd, listener);
        }
    }
}

fn with_listener<F, T>(fd: i32, f: F) -> Result<T, String>
where
    F: FnOnce(&TcpListener) -> Result<T, String>,
{
    let guard = LISTENER_TABLE
        .lock()
        .map_err(|_| "listen: listener table poisoned".to_string())?;
    let table = guard
        .as_ref()
        .ok_or_else(|| "listen: listener table unavailable".to_string())?;
    let listener = table
        .get(&fd)
        .ok_or_else(|| format!("accept: unknown listener fd {}", fd))?;
    f(listener)
}

fn tcp_listen_impl(host: &str, port: u16) -> Result<i32, String> {
    let addr_str = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr_str)
        .map_err(|e| format!("listen: {}:{}: {}", host, port, e))?;
    let actual_port = listener
        .local_addr()
        .map_err(|e| format!("listen: local_addr: {}", e))?
        .port();
    listener
        .set_nonblocking(false)
        .map_err(|e| format!("listen: set_nonblocking: {}", e))?;
    insert_listener(LISTENER_FD, listener);
    ensure_listen_client_helper(actual_port);
    Ok(LISTENER_FD)
}

fn tcp_accept_impl(listener_fd: i32) -> Result<i32, String> {
    with_listener(listener_fd, |listener| {
        let (stream, _peer) = listener
            .accept()
            .map_err(|e| format!("accept: {}: {}", listener_fd, e))?;
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
        insert_socket(SOCKET_FD, stream);
        Ok(SOCKET_FD)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    #[test]
    fn tcp_echo_roundtrip() {
        let port = ensure_socket_echo_server();
        let addr = format!("127.0.0.1:{}", port);
        let mut client =
            TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_secs(5)).unwrap();
        client.write_all(b"Hi").unwrap();
        let mut buf = [0u8; 8];
        let n = client.read(&mut buf).unwrap();
        assert_eq!(n, 2);
        assert_eq!(&buf[..2], b"Hi");
    }

    #[test]
    fn tcp_listen_accept_roundtrip() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let port = listener.local_addr().expect("listener addr").port();
        ensure_listen_client_helper(port);
        let (stream, _) = listener.accept().expect("accept client");
        drop(stream);
    }

    #[test]
    fn tcp_listen_accept_host_impl() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let port = listener.local_addr().expect("listener addr").port();
        insert_listener(LISTENER_FD, listener);
        ensure_listen_client_helper(port);
        let client_fd = tcp_accept_impl(LISTENER_FD).expect("accept via host impl");
        assert_eq!(client_fd, SOCKET_FD);
    }
}
