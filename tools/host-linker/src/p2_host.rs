//! Bridged WASI Preview2 core imports for Arukellt guests (#834).
//!
//! Guest emit uses Preview1-shaped ABIs under `wasi:*@0.2.0` module names so the
//! selfhost compiler can run under host-linker (plain wasmtime cannot link them).

use crate::RuntimeCaps;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use wasmtime::*;

const ERRNO_SUCCESS: i32 = 0;
const ERRNO_BADF: i32 = 8;
const ERRNO_EXIST: i32 = 20;
const ERRNO_INVAL: i32 = 28;
const ERRNO_IO: i32 = 29;
const ERRNO_NOENT: i32 = 44;
const ERRNO_NOTCAPABLE: i32 = 76;

// WASI path_open oflags bits used by guest emit.
const OFLAG_CREAT: i32 = 1;
const OFLAG_TRUNC: i32 = 8;

pub struct P2HostState {
    args: Vec<String>,
    preopens: Vec<PathBuf>,
    files: HashMap<u32, File>,
    next_fd: u32,
}

impl P2HostState {
    pub fn from_caps(caps: &RuntimeCaps) -> Self {
        let mut args = caps.args.clone();
        if args.is_empty() {
            args.push("arukellt-host-run".to_string());
        }
        let preopens: Vec<PathBuf> = caps
            .dirs
            .iter()
            .map(|g| PathBuf::from(&g.host_path))
            .collect();
        Self {
            args,
            preopens,
            files: HashMap::new(),
            next_fd: 10,
        }
    }

    fn preopen_for_dirfd(&self, dirfd: i32) -> Option<&Path> {
        if dirfd < 3 {
            return None;
        }
        let idx = (dirfd - 3) as usize;
        self.preopens.get(idx).map(|p| p.as_path())
    }

    fn resolve_path(&self, dirfd: i32, rel: &str) -> Option<PathBuf> {
        let rel = rel.trim_start_matches('/');
        if let Some(root) = self.preopen_for_dirfd(dirfd) {
            let candidate = root.join(rel);
            return Some(candidate);
        }
        // Fallback: first existing preopen match (bootstrap flat-src + repo root).
        for root in &self.preopens {
            let candidate = root.join(rel);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        self.preopens.first().map(|root| root.join(rel))
    }
}

/// Shared state behind the linker store so import closures can mutate fds.
pub type P2Store = Arc<Mutex<P2HostState>>;

pub fn register_p2_imports(linker: &mut Linker<P2Store>, module: &Module) -> Result<(), String> {
    for import in module.imports() {
        let mod_name = import.module().to_string();
        let field_name = import.name().to_string();
        let ExternType::Func(ft) = import.ty() else {
            continue;
        };
        if try_register_known(linker, &mod_name, &field_name, &ft).is_err() {
            register_auto_stub(linker, &mod_name, &field_name, &ft)?;
        }
    }
    Ok(())
}

fn try_register_known(
    linker: &mut Linker<P2Store>,
    mod_name: &str,
    field_name: &str,
    ft: &FuncType,
) -> Result<(), String> {
    match (mod_name, field_name) {
        ("wasi:cli/stdout@0.2.0", "get-stdout") => register_get_stream(linker, mod_name, field_name, ft, 1),
        ("wasi:cli/stderr@0.2.0", "get-stderr") => register_get_stream(linker, mod_name, field_name, ft, 2),
        ("wasi:io/streams@0.2.0", "blocking-write-and-flush") => register_bwaf(linker, ft),
        ("wasi:cli/environment@0.2.0", "args-sizes") => register_args_sizes(linker, ft),
        ("wasi:cli/environment@0.2.0", "arguments") => register_arguments(linker, ft),
        ("wasi:cli/environment@0.2.0", "environ-sizes") => register_environ_sizes(linker, ft),
        ("wasi:cli/environment@0.2.0", "environ-get") => register_environ_get(linker, ft),
        ("wasi:cli/stdin@0.2.0", "read") => register_stdin_read(linker, ft),
        ("wasi:cli/exit@0.2.0", "exit") => register_exit(linker, ft),
        ("wasi:filesystem/types@0.2.0", "open-at") => {
            register_open_at(linker, "wasi:filesystem/types@0.2.0", "open-at", ft)
        }
        // Current wasm32-gc / wasi-p2 emit uses the runtime host package
        // instead of wasi:filesystem / arukellt:fs. Same P1-shaped ABI.
        ("arukellt:runtime/host@0.1.0", "runtime_fs_open_at") => {
            register_open_at(linker, "arukellt:runtime/host@0.1.0", "runtime_fs_open_at", ft)
        }
        // Bridged fd_read/fd_write ABI: prefer arukellt:fs (#834 close-gates).
        // Keep wasi:filesystem aliases so the previous pin can still bootstrap.
        ("arukellt:fs@0.1.0", "read") => {
            register_fs_read(linker, "arukellt:fs@0.1.0", "read", ft)
        }
        ("arukellt:fs@0.1.0", "write") => {
            register_fs_write(linker, "arukellt:fs@0.1.0", "write", ft)
        }
        ("wasi:filesystem/types@0.2.0", "read") => {
            register_fs_read(linker, "wasi:filesystem/types@0.2.0", "read", ft)
        }
        ("wasi:filesystem/types@0.2.0", "write") => {
            register_fs_write(linker, "wasi:filesystem/types@0.2.0", "write", ft)
        }
        ("arukellt:runtime/host@0.1.0", "runtime_fs_read") => {
            register_fs_read(linker, "arukellt:runtime/host@0.1.0", "runtime_fs_read", ft)
        }
        ("arukellt:runtime/host@0.1.0", "runtime_fs_write") => {
            register_fs_write(linker, "arukellt:runtime/host@0.1.0", "runtime_fs_write", ft)
        }
        ("wasi:filesystem/types@0.2.0", "close") => {
            register_fs_close(linker, "wasi:filesystem/types@0.2.0", "close", ft)
        }
        ("arukellt:runtime/host@0.1.0", "runtime_fs_close") => {
            register_fs_close(linker, "arukellt:runtime/host@0.1.0", "runtime_fs_close", ft)
        }
        ("wasi:clocks/monotonic-clock@0.2.0", "now") => register_clock_now(linker, mod_name, field_name, ft),
        ("wasi:clocks/wall-clock@0.2.0", "now") => register_clock_now(linker, mod_name, field_name, ft),
        ("wasi:random/random@0.2.0", "get-random-u64") => register_random_u64(linker, ft),
        _ => Err("unknown".into()),
    }
}

fn register_get_stream(
    linker: &mut Linker<P2Store>,
    mod_name: &str,
    field_name: &str,
    ft: &FuncType,
    handle: i32,
) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(mod_name, field_name, ft, move |_: Caller<'_, P2Store>, _p: &[Val], r: &mut [Val]| {
            if !r.is_empty() {
                r[0] = Val::I32(handle);
            }
            Ok(())
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn register_bwaf(linker: &mut Linker<P2Store>, ft: &FuncType) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            "wasi:io/streams@0.2.0",
            "blocking-write-and-flush",
            ft,
            move |mut caller: Caller<'_, P2Store>, p: &[Val], _r: &mut [Val]| {
                if p.len() < 4 {
                    return Ok(());
                }
                let (Val::I32(handle), Val::I32(buf), Val::I32(len), Val::I32(ret)) =
                    (p[0], p[1], p[2], p[3])
                else {
                    return Ok(());
                };
                let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
                    return Ok(());
                };
                let mut data = vec![0u8; len.max(0) as usize];
                let _ = mem.read(&caller, buf as usize, &mut data);
                let n = if handle == 2 {
                    let n = std::io::stderr().write(&data).unwrap_or(0) as i32;
                    let _ = std::io::stderr().flush();
                    n
                } else {
                    let n = std::io::stdout().write(&data).unwrap_or(0) as i32;
                    let _ = std::io::stdout().flush();
                    n
                };
                let _ = mem.write(&mut caller, ret as usize, &n.to_le_bytes());
                Ok(())
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn register_args_sizes(linker: &mut Linker<P2Store>, ft: &FuncType) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            "wasi:cli/environment@0.2.0",
            "args-sizes",
            ft,
            move |mut caller: Caller<'_, P2Store>, p: &[Val], r: &mut [Val]| {
                if p.len() < 2 {
                    if !r.is_empty() {
                        r[0] = Val::I32(ERRNO_INVAL);
                    }
                    return Ok(());
                }
                let (Val::I32(argc_ptr), Val::I32(buf_size_ptr)) = (p[0], p[1]) else {
                    if !r.is_empty() {
                        r[0] = Val::I32(ERRNO_INVAL);
                    }
                    return Ok(());
                };
                let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
                    if !r.is_empty() {
                        r[0] = Val::I32(ERRNO_IO);
                    }
                    return Ok(());
                };
                let state = caller.data().lock().map_err(|_| wasmtime::Error::msg("lock"))?;
                let argc = state.args.len() as i32;
                let buf_size: i32 = state.args.iter().map(|a| a.len() as i32 + 1).sum();
                drop(state);
                let _ = mem.write(&mut caller, argc_ptr as usize, &argc.to_le_bytes());
                let _ = mem.write(&mut caller, buf_size_ptr as usize, &buf_size.to_le_bytes());
                if !r.is_empty() {
                    r[0] = Val::I32(ERRNO_SUCCESS);
                }
                Ok(())
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn register_arguments(linker: &mut Linker<P2Store>, ft: &FuncType) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            "wasi:cli/environment@0.2.0",
            "arguments",
            ft,
            move |mut caller: Caller<'_, P2Store>, p: &[Val], r: &mut [Val]| {
                if p.len() < 2 {
                    if !r.is_empty() {
                        r[0] = Val::I32(ERRNO_INVAL);
                    }
                    return Ok(());
                }
                let (Val::I32(argv_ptr), Val::I32(argv_buf_ptr)) = (p[0], p[1]) else {
                    if !r.is_empty() {
                        r[0] = Val::I32(ERRNO_INVAL);
                    }
                    return Ok(());
                };
                let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
                    if !r.is_empty() {
                        r[0] = Val::I32(ERRNO_IO);
                    }
                    return Ok(());
                };
                let args = {
                    let state = caller.data().lock().map_err(|_| wasmtime::Error::msg("lock"))?;
                    state.args.clone()
                };
                let mut buf_off = argv_buf_ptr as usize;
                for (i, arg) in args.iter().enumerate() {
                    let slot = argv_ptr as usize + i * 4;
                    let ptr_i32 = buf_off as i32;
                    let _ = mem.write(&mut caller, slot, &ptr_i32.to_le_bytes());
                    let mut bytes = arg.as_bytes().to_vec();
                    bytes.push(0);
                    let _ = mem.write(&mut caller, buf_off, &bytes);
                    buf_off += bytes.len();
                }
                if !r.is_empty() {
                    r[0] = Val::I32(ERRNO_SUCCESS);
                }
                Ok(())
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn register_environ_sizes(linker: &mut Linker<P2Store>, ft: &FuncType) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            "wasi:cli/environment@0.2.0",
            "environ-sizes",
            ft,
            move |mut caller: Caller<'_, P2Store>, p: &[Val], r: &mut [Val]| {
                if p.len() >= 2 {
                    if let (Val::I32(a), Val::I32(b)) = (p[0], p[1]) {
                        if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
                            let z = 0i32.to_le_bytes();
                            let _ = mem.write(&mut caller, a as usize, &z);
                            let _ = mem.write(&mut caller, b as usize, &z);
                        }
                    }
                }
                if !r.is_empty() {
                    r[0] = Val::I32(ERRNO_SUCCESS);
                }
                Ok(())
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn register_environ_get(linker: &mut Linker<P2Store>, ft: &FuncType) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            "wasi:cli/environment@0.2.0",
            "environ-get",
            ft,
            move |_: Caller<'_, P2Store>, _p: &[Val], r: &mut [Val]| {
                if !r.is_empty() {
                    r[0] = Val::I32(ERRNO_SUCCESS);
                }
                Ok(())
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn register_stdin_read(linker: &mut Linker<P2Store>, ft: &FuncType) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            "wasi:cli/stdin@0.2.0",
            "read",
            ft,
            move |mut caller: Caller<'_, P2Store>, p: &[Val], r: &mut [Val]| {
                let errno = iov_read_write(&mut caller, p, true, |buf| std::io::stdin().read(buf).unwrap_or(0));
                if !r.is_empty() {
                    r[0] = Val::I32(errno);
                }
                Ok(())
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn register_exit(linker: &mut Linker<P2Store>, ft: &FuncType) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            "wasi:cli/exit@0.2.0",
            "exit",
            ft,
            move |_: Caller<'_, P2Store>, p: &[Val], _r: &mut [Val]| {
                let code = match p.first() {
                    Some(Val::I32(c)) => *c,
                    _ => 0,
                };
                std::process::exit(code);
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn register_open_at(
    linker: &mut Linker<P2Store>,
    module: &'static str,
    field: &'static str,
    ft: &FuncType,
) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            module,
            field,
            ft,
            move |mut caller: Caller<'_, P2Store>, p: &[Val], r: &mut [Val]| {
                // (dirfd, dirflags, path_ptr, path_len, oflags, rights_base:i64,
                //  rights_inheriting:i64, fdflags, fd_out_ptr) -> errno
                if p.len() < 9 {
                    if !r.is_empty() {
                        r[0] = Val::I32(ERRNO_INVAL);
                    }
                    return Ok(());
                }
                let dirfd = match p[0] {
                    Val::I32(v) => v,
                    _ => 0,
                };
                let path_ptr = match p[2] {
                    Val::I32(v) => v,
                    _ => 0,
                };
                let path_len = match p[3] {
                    Val::I32(v) => v,
                    _ => 0,
                };
                let oflags = match p[4] {
                    Val::I32(v) => v,
                    _ => 0,
                };
                let fd_out = match p[8] {
                    Val::I32(v) => v,
                    _ => 0,
                };
                let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
                    if !r.is_empty() {
                        r[0] = Val::I32(ERRNO_IO);
                    }
                    return Ok(());
                };
                let mut path_bytes = vec![0u8; path_len.max(0) as usize];
                if mem.read(&caller, path_ptr as usize, &mut path_bytes).is_err() {
                    if !r.is_empty() {
                        r[0] = Val::I32(ERRNO_IO);
                    }
                    return Ok(());
                }
                let path_str = String::from_utf8_lossy(&path_bytes).to_string();
                let mut state = caller.data().lock().map_err(|_| wasmtime::Error::msg("lock"))?;
                let Some(host_path) = state.resolve_path(dirfd, &path_str) else {
                    drop(state);
                    if !r.is_empty() {
                        r[0] = Val::I32(ERRNO_NOTCAPABLE);
                    }
                    return Ok(());
                };
                let creat = (oflags & OFLAG_CREAT) != 0;
                let trunc = (oflags & OFLAG_TRUNC) != 0;
                let file = if creat || trunc {
                    OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(creat)
                        .truncate(trunc)
                        .open(&host_path)
                } else {
                    OpenOptions::new().read(true).open(&host_path)
                };
                let file = match file {
                    Ok(f) => f,
                    Err(e) => {
                        drop(state);
                        let errno = match e.kind() {
                            std::io::ErrorKind::NotFound => ERRNO_NOENT,
                            std::io::ErrorKind::AlreadyExists => ERRNO_EXIST,
                            _ => ERRNO_IO,
                        };
                        if !r.is_empty() {
                            r[0] = Val::I32(errno);
                        }
                        return Ok(());
                    }
                };
                let fd = state.next_fd;
                state.next_fd += 1;
                state.files.insert(fd, file);
                drop(state);
                let _ = mem.write(&mut caller, fd_out as usize, &fd.to_le_bytes());
                if !r.is_empty() {
                    r[0] = Val::I32(ERRNO_SUCCESS);
                }
                Ok(())
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn register_fs_read(
    linker: &mut Linker<P2Store>,
    module: &'static str,
    field: &'static str,
    ft: &FuncType,
) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            module,
            field,
            ft,
            move |mut caller: Caller<'_, P2Store>, p: &[Val], r: &mut [Val]| {
                let errno = fs_iov(&mut caller, p, true);
                if !r.is_empty() {
                    r[0] = Val::I32(errno);
                }
                Ok(())
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn register_fs_write(
    linker: &mut Linker<P2Store>,
    module: &'static str,
    field: &'static str,
    ft: &FuncType,
) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            module,
            field,
            ft,
            move |mut caller: Caller<'_, P2Store>, p: &[Val], r: &mut [Val]| {
                let errno = fs_iov(&mut caller, p, false);
                if !r.is_empty() {
                    r[0] = Val::I32(errno);
                }
                Ok(())
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn register_fs_close(
    linker: &mut Linker<P2Store>,
    module: &'static str,
    field: &'static str,
    ft: &FuncType,
) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            module,
            field,
            ft,
            move |caller: Caller<'_, P2Store>, p: &[Val], r: &mut [Val]| {
                let fd = match p.first() {
                    Some(Val::I32(v)) => *v as u32,
                    _ => {
                        if !r.is_empty() {
                            r[0] = Val::I32(ERRNO_INVAL);
                        }
                        return Ok(());
                    }
                };
                let mut state = caller.data().lock().map_err(|_| wasmtime::Error::msg("lock"))?;
                let errno = if state.files.remove(&fd).is_some() {
                    ERRNO_SUCCESS
                } else {
                    ERRNO_BADF
                };
                if !r.is_empty() {
                    r[0] = Val::I32(errno);
                }
                Ok(())
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn register_clock_now(
    linker: &mut Linker<P2Store>,
    mod_name: &str,
    field_name: &str,
    ft: &FuncType,
) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(mod_name, field_name, ft, move |_: Caller<'_, P2Store>, _p: &[Val], r: &mut [Val]| {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as i64)
                .unwrap_or(0);
            if !r.is_empty() {
                r[0] = Val::I64(nanos);
            }
            Ok(())
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn register_random_u64(linker: &mut Linker<P2Store>, ft: &FuncType) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            "wasi:random/random@0.2.0",
            "get-random-u64",
            ft,
            move |_: Caller<'_, P2Store>, _p: &[Val], r: &mut [Val]| {
                // Deterministic-ish non-zero for compile paths that touch random.
                let v = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(1);
                if !r.is_empty() {
                    r[0] = Val::I64(v as i64);
                }
                Ok(())
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn register_auto_stub(
    linker: &mut Linker<P2Store>,
    mod_name: &str,
    field_name: &str,
    ft: &FuncType,
) -> Result<(), String> {
    let results: Vec<ValType> = ft.results().collect();
    let ft = ft.clone();
    linker
        .func_new(mod_name, field_name, ft, move |_: Caller<'_, P2Store>, _p: &[Val], r: &mut [Val]| {
            for (i, rt) in results.iter().enumerate() {
                if i < r.len() {
                    r[i] = match rt {
                        ValType::I32 => Val::I32(0),
                        ValType::I64 => Val::I64(0),
                        ValType::F32 => Val::F32(0u32),
                        ValType::F64 => Val::F64(0u64),
                        _ => Val::I32(0),
                    };
                }
            }
            Ok(())
        })
        .map_err(|e| format!("{mod_name}::{field_name}: {e}"))?;
    Ok(())
}

fn fs_iov(caller: &mut Caller<'_, P2Store>, p: &[Val], is_read: bool) -> i32 {
    if p.len() < 4 {
        return ERRNO_INVAL;
    }
    let fd = match p[0] {
        Val::I32(v) => v as u32,
        _ => return ERRNO_INVAL,
    };
    let iovs = match p[1] {
        Val::I32(v) => v as usize,
        _ => return ERRNO_INVAL,
    };
    let iovs_len = match p[2] {
        Val::I32(v) => v as usize,
        _ => return ERRNO_INVAL,
    };
    let n_ptr = match p[3] {
        Val::I32(v) => v as usize,
        _ => return ERRNO_INVAL,
    };
    let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
        return ERRNO_IO;
    };
    {
        let state = match caller.data().lock() {
            Ok(s) => s,
            Err(_) => return ERRNO_IO,
        };
        if !state.files.contains_key(&fd) {
            return ERRNO_BADF;
        }
    }
    let mut total: usize = 0;
    for i in 0..iovs_len {
        let base = iovs + i * 8;
        let mut ptr_bytes = [0u8; 4];
        let mut len_bytes = [0u8; 4];
        if mem.read(&*caller, base, &mut ptr_bytes).is_err()
            || mem.read(&*caller, base + 4, &mut len_bytes).is_err()
        {
            return ERRNO_IO;
        }
        let buf_ptr = i32::from_le_bytes(ptr_bytes) as usize;
        let buf_len = i32::from_le_bytes(len_bytes) as usize;
        if buf_len == 0 {
            continue;
        }
        if is_read {
            let mut buf = vec![0u8; buf_len];
            let n = {
                let mut state = match caller.data().lock() {
                    Ok(s) => s,
                    Err(_) => return ERRNO_IO,
                };
                let Some(file) = state.files.get_mut(&fd) else {
                    return ERRNO_BADF;
                };
                match file.read(&mut buf) {
                    Ok(n) => n,
                    Err(_) => return ERRNO_IO,
                }
            };
            if n == 0 {
                break;
            }
            if mem.write(&mut *caller, buf_ptr, &buf[..n]).is_err() {
                return ERRNO_IO;
            }
            total += n;
            if n < buf_len {
                break;
            }
        } else {
            let mut buf = vec![0u8; buf_len];
            if mem.read(&*caller, buf_ptr, &mut buf).is_err() {
                return ERRNO_IO;
            }
            {
                let mut state = match caller.data().lock() {
                    Ok(s) => s,
                    Err(_) => return ERRNO_IO,
                };
                let Some(file) = state.files.get_mut(&fd) else {
                    return ERRNO_BADF;
                };
                if file.write_all(&buf).is_err() {
                    return ERRNO_IO;
                }
                if i + 1 == iovs_len {
                    let _ = file.flush();
                }
            }
            total += buf_len;
        }
    }
    let n = total as i32;
    let _ = mem.write(&mut *caller, n_ptr, &n.to_le_bytes());
    ERRNO_SUCCESS
}

fn iov_read_write<F>(caller: &mut Caller<'_, P2Store>, p: &[Val], _is_read: bool, mut op: F) -> i32
where
    F: FnMut(&mut [u8]) -> usize,
{
    if p.len() < 4 {
        return ERRNO_INVAL;
    }
    // stdin.read uses (fd, iovs, iovs_len, nread) — fd is usually 0.
    let iovs = match p[1] {
        Val::I32(v) => v as usize,
        _ => return ERRNO_INVAL,
    };
    let iovs_len = match p[2] {
        Val::I32(v) => v as usize,
        _ => return ERRNO_INVAL,
    };
    let n_ptr = match p[3] {
        Val::I32(v) => v as usize,
        _ => return ERRNO_INVAL,
    };
    let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
        return ERRNO_IO;
    };
    let mut total: usize = 0;
    for i in 0..iovs_len {
        let base = iovs + i * 8;
        let mut ptr_bytes = [0u8; 4];
        let mut len_bytes = [0u8; 4];
        if mem.read(&*caller, base, &mut ptr_bytes).is_err()
            || mem.read(&*caller, base + 4, &mut len_bytes).is_err()
        {
            return ERRNO_IO;
        }
        let buf_ptr = i32::from_le_bytes(ptr_bytes) as usize;
        let buf_len = i32::from_le_bytes(len_bytes) as usize;
        if buf_len == 0 {
            continue;
        }
        let mut buf = vec![0u8; buf_len];
        let n = op(&mut buf);
        if n == 0 {
            break;
        }
        if mem.write(&mut *caller, buf_ptr, &buf[..n]).is_err() {
            return ERRNO_IO;
        }
        total += n;
        if n < buf_len {
            break;
        }
    }
    let n = total as i32;
    let _ = mem.write(&mut *caller, n_ptr, &n.to_le_bytes());
    ERRNO_SUCCESS
}
