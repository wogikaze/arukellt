//! TCP connect host implementation for `arukellt_host::sockets_connect`.

use crate::{read_string_from_mem, write_error};
use wasmtime::*;
use wasmtime_wasi::preview1::WasiP1Ctx;

pub fn register_sockets_host_fns(linker: &mut Linker<WasiP1Ctx>) -> Result<(), String> {
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

    Ok(())
}

fn tcp_connect_impl(host: &str, port: u16) -> Result<i32, String> {
    use std::net::TcpStream;
    use std::time::Duration;

    let socket_addr = to_socket_addr_for_connect(host, port)?;
    match TcpStream::connect_timeout(&socket_addr, Duration::from_secs(5)) {
        Ok(_stream) => Ok(3),
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
