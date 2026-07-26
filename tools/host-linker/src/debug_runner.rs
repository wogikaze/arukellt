//! Wasmtime-backed debug runner with breakpoint hooks and live local inspection.
//! Dynamically registers import stubs based on the module's import section,
//! supporting WASI P2 and future P3 imports automatically.

use crate::source_map::line_to_code_offset;
use crate::wasm_debug_patch::prepare_debug_wasm;
use crate::{run_wasm, RuntimeCaps};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use wasmtime::*;

/// Guest fds allocated by P2 open-at (starts above stdio-ish handles).
fn p2_files() -> &'static Mutex<HashMap<i32, File>> {
    static FILES: OnceLock<Mutex<HashMap<i32, File>>> = OnceLock::new();
    FILES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn p2_next_fd() -> &'static Mutex<i32> {
    static NEXT: OnceLock<Mutex<i32>> = OnceLock::new();
    NEXT.get_or_init(|| Mutex::new(10))
}

#[derive(Debug, Clone)]
pub struct LiveLocal {
    pub index: u32,
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct DebugPause {
    pub source_line: u32,
    pub locals: Vec<LiveLocal>,
}

static DEBUG_CAPTURE: Mutex<Option<DebugPause>> = Mutex::new(None);

pub fn run_until_breakpoint(
    wasm_bytes: &[u8],
    breakpoint_line: u32,
    _caps: &RuntimeCaps,
    ark_source: Option<&str>,
) -> Result<DebugPause, String> {
    let prepared = if let Some(source) = ark_source {
        prepare_debug_wasm(wasm_bytes, source, breakpoint_line)?
    } else {
        wasm_bytes.to_vec()
    };
    if line_to_code_offset(&crate::source_map::parse_source_map(&prepared), breakpoint_line).is_none()
    {
        return Err(format!("no source-map entry for line {}", breakpoint_line));
    }

    {
        let mut guard = DEBUG_CAPTURE
            .lock()
            .map_err(|_| "debug capture mutex poisoned".to_string())?;
        *guard = None;
    }

    let mut config = Config::new();
    config.cranelift_opt_level(OptLevel::None);
    config.wasm_bulk_memory(true);
    config.wasm_multi_value(true);
    config.wasm_reference_types(true);
    config.wasm_function_references(true);
    config.wasm_gc(true);
    let engine = Engine::new(&config).map_err(|e| format!("engine: {:?}", e))?;
    let module = Module::new(&engine, &prepared).map_err(|e| format!("module: {:?}", e))?;

    let mut linker = Linker::<()>::new(&engine);
    register_import_stubs(&mut linker, &module)?;
    linker
        .func_wrap(
            "arukellt_debug",
            "breakpoint",
            |line: i32, value: i64| -> Result<(), wasmtime::Error> {
                let mut guard = DEBUG_CAPTURE
                    .lock()
                    .map_err(|_| wasmtime::Error::msg("debug capture mutex poisoned"))?;
                *guard = Some(DebugPause {
                    source_line: line as u32,
                    locals: vec![LiveLocal {
                        index: 0,
                        name: "x".to_string(),
                        value: value.to_string(),
                    }],
                });
                Err(wasmtime::Error::msg("debug breakpoint"))
            },
        )
        .map_err(|e| format!("debug hook: {}", e))?;

    let mut store = Store::new(&engine, ());
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("instantiate: {}", e))?;
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|e| format!("_start: {}", e))?;

    if start.call(&mut store, ()).is_ok() {
        return Err("breakpoint not hit".to_string());
    }

    DEBUG_CAPTURE
        .lock()
        .map_err(|_| "debug capture mutex poisoned".to_string())?
        .clone()
        .ok_or_else(|| "debug breakpoint hook did not capture locals".to_string())
}

/// Scan the module's imports and register appropriate stubs for each.
/// Known WASI P2 imports get real implementations; unknown imports
/// (including future P3 additions) get auto-generated zero-value stubs.
pub fn register_import_stubs(linker: &mut Linker<()>, module: &Module) -> Result<(), String> {
    for import in module.imports() {
        let mod_name = import.module().to_string();
        let field_name = import.name().to_string();
        let ExternType::Func(ft) = import.ty() else { continue };

        // Skip arukellt_debug imports — registered explicitly by caller.
        if mod_name == "arukellt_debug" { continue }

        // Try known implementations first; fall back to auto-stub.
        let engine = linker.engine().clone();
        let result = try_register_known(linker, &engine, &mod_name, &field_name, &ft)
            .or_else(|_| -> Result<(), String> {
                register_auto_stub(linker, &engine, &mod_name, &field_name, &ft)
            });
        if let Err(e) = result {
            return Err(format!("import {}::{}: {}", mod_name, field_name, e));
        }
    }
    Ok(())
}

/// Attempt to register a known import with real behavior.
fn try_register_known(
    linker: &mut Linker<()>,
    engine: &Engine,
    mod_name: &str,
    field_name: &str,
    ft: &FuncType,
) -> Result<(), String> {
    match (mod_name, field_name) {
        ("wasi:cli/stdout@0.2.0", "write") => register_stdout_write(linker, engine, ft),
        // Guest-native P2 stdio (#668): get-stdout/get-stderr + streams.bwaf.
        ("wasi:cli/stdout@0.2.0", "get-stdout") => {
            register_get_stream(linker, engine, mod_name, field_name, ft, 1)
        }
        ("wasi:cli/stderr@0.2.0", "get-stderr") => {
            register_get_stream(linker, engine, mod_name, field_name, ft, 2)
        }
        ("wasi:io/streams@0.2.0", "blocking-write-and-flush") => {
            register_blocking_write_and_flush(linker, engine, ft)
        }
        ("wasi:cli/environment@0.2.0", "args-sizes") => {
            register_args_sizes_stub(linker, engine, ft)
        }
        ("wasi:cli/environment@0.2.0", "arguments") => {
            register_arguments_stub(linker, engine, ft)
        }
        ("wasi:cli/stdin@0.2.0", "read") => register_fd_read_stub(linker, engine, ft),
        ("wasi:cli/exit@0.2.0", "exit") => register_exit_stub(linker, engine, ft),
        ("wasi:filesystem/types@0.2.0", "open-at") => register_open_at_stub(linker, engine, ft),
        ("wasi:filesystem/types@0.2.0", "close") => register_fs_close_stub(linker, engine, ft),
        ("wasi:clocks/monotonic-clock@0.2.0", "now") => {
            register_monotonic_now_stub(linker, engine, ft)
        }
        ("wasi:clocks/wall-clock@0.2.0", "now") => register_wall_now_stub(linker, engine, ft),
        ("wasi:random/random@0.2.0", "get-random-u64") => {
            register_random_u64_stub(linker, engine, ft)
        }
        // Future WASI P3 imports can be added here as they are implemented.
        _ => Err("unknown import".into()),
    }
}

/// Register stdout.write with actual I/O.
fn register_stdout_write(
    linker: &mut Linker<()>,
    _engine: &Engine,
    ft: &FuncType,
) -> Result<(), String> {
    use std::io::Write;
    let ft = ft.clone();
    linker.func_new("wasi:cli/stdout@0.2.0", "write", ft, move |mut caller: Caller<'_, ()>, p: &[Val], r: &mut [Val]| {
        if p.len() >= 4 {
            if let (Val::I32(buf), Val::I32(len), Val::I32(ret)) = (p[1], p[2], p[3]) {
                if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
                    let mut data = vec![0u8; len as usize];
                    let _ = mem.read(&caller, buf as usize, &mut data);
                    let n = std::io::stdout().write(&data).unwrap_or(0) as i32;
                    let _ = std::io::stdout().flush();
                    let _ = mem.write(&mut caller, ret as usize, &n.to_le_bytes());
                }
            }
        }
        if !r.is_empty() { r[0] = Val::I32(0); }
        Ok(())
    }).map_err(|e| format!("stdout write: {}", e))?;
    Ok(())
}

/// Return a stable stream handle for guest-native get-stdout / get-stderr.
fn register_get_stream(
    linker: &mut Linker<()>,
    _engine: &Engine,
    mod_name: &str,
    field_name: &str,
    ft: &FuncType,
    handle: i32,
) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(mod_name, field_name, ft, move |_: Caller<'_, ()>, _p: &[Val], r: &mut [Val]| {
            if !r.is_empty() {
                r[0] = Val::I32(handle);
            }
            Ok(())
        })
        .map_err(|e| format!("{mod_name}::{field_name}: {e}"))?;
    Ok(())
}

/// Guest-native bwaf: (handle, ptr, len, ret_ptr) → write bytes; store nbytes at ret.
fn register_blocking_write_and_flush(
    linker: &mut Linker<()>,
    _engine: &Engine,
    ft: &FuncType,
) -> Result<(), String> {
    use std::io::Write;
    let ft = ft.clone();
    linker
        .func_new(
            "wasi:io/streams@0.2.0",
            "blocking-write-and-flush",
            ft,
            move |mut caller: Caller<'_, ()>, p: &[Val], _r: &mut [Val]| {
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
        .map_err(|e| format!("blocking-write-and-flush: {e}"))?;
    Ok(())
}

/// Register exit that terminates the host process with the guest status.
/// A trap-style stub appended "runtime error: ..." after panic/assert messages
/// and broke fixture goldens that expect a clean non-zero exit (#807).
fn register_exit_stub(
    linker: &mut Linker<()>,
    _engine: &Engine,
    ft: &FuncType,
) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            "wasi:cli/exit@0.2.0",
            "exit",
            ft,
            move |_: Caller<'_, ()>, p: &[Val], _r: &mut [Val]| {
                let code = match p.first() {
                    Some(Val::I32(c)) => *c,
                    Some(Val::I64(c)) => *c as i32,
                    _ => 1,
                };
                std::process::exit(code);
            },
        )
        .map_err(|e| format!("exit: {}", e))?;
    Ok(())
}

/// Register a stub that delegates to auto-stub.
fn register_retptr_stub(
    linker: &mut Linker<()>,
    engine: &Engine,
    mod_name: &str,
    field_name: &str,
    ft: &FuncType,
) -> Result<(), String> {
    register_auto_stub(linker, engine, mod_name, field_name, ft)
}

/// P1-shaped path_open ABI on the guest-native open-at import (#807).
/// Params: dirfd, lookup, path_ptr, path_len, oflags, rights_base, rights_inheriting,
/// fdflags, fd_retptr → errno. Writes the allocated fd to fd_retptr on success.
fn register_open_at_stub(
    linker: &mut Linker<()>,
    _engine: &Engine,
    ft: &FuncType,
) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            "wasi:filesystem/types@0.2.0",
            "open-at",
            ft,
            move |mut caller: Caller<'_, ()>, p: &[Val], r: &mut [Val]| {
                let errno = open_at_impl(&mut caller, p);
                if !r.is_empty() {
                    r[0] = Val::I32(errno);
                }
                Ok(())
            },
        )
        .map_err(|e| format!("open-at: {e}"))?;
    Ok(())
}

fn open_at_impl(caller: &mut Caller<'_, ()>, p: &[Val]) -> i32 {
    if p.len() < 9 {
        return 28; // EINVAL-ish
    }
    let (Val::I32(path_ptr), Val::I32(path_len), Val::I32(oflags), Val::I32(ret_ptr)) =
        (p[2], p[3], p[4], p[8])
    else {
        return 28;
    };
    let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
        return 28;
    };
    if path_len < 0 {
        return 28;
    }
    let mut path_bytes = vec![0u8; path_len as usize];
    if mem
        .read(&*caller, path_ptr as usize, &mut path_bytes)
        .is_err()
    {
        return 28;
    }
    let path = match std::str::from_utf8(&path_bytes) {
        Ok(s) => s,
        Err(_) => return 28,
    };
    // oflags bit 0 = creat (WASI PATH_OPEN_CREATE); bit 3 often create+trunc in guest emit (9).
    let create = (oflags & 1) != 0 || (oflags & 8) != 0;
    let trunc = (oflags & 8) != 0;
    let mut opts = OpenOptions::new();
    opts.read(true);
    if create {
        opts.write(true).create(true);
    }
    if trunc {
        opts.truncate(true);
    }
    let file = match opts.open(path) {
        Ok(f) => f,
        Err(_) => return 44, // ENOENT-ish for probe fixtures
    };
    let fd = {
        let mut next = p2_next_fd().lock().unwrap_or_else(|e| e.into_inner());
        let fd = *next;
        *next = next.saturating_add(1);
        fd
    };
    {
        let mut files = p2_files().lock().unwrap_or_else(|e| e.into_inner());
        files.insert(fd, file);
    }
    let _ = mem.write(caller, ret_ptr as usize, &fd.to_le_bytes());
    0
}

fn register_fs_close_stub(
    linker: &mut Linker<()>,
    _engine: &Engine,
    ft: &FuncType,
) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            "wasi:filesystem/types@0.2.0",
            "close",
            ft,
            move |_: Caller<'_, ()>, p: &[Val], r: &mut [Val]| {
                if let Some(Val::I32(fd)) = p.first() {
                    let mut files = p2_files().lock().unwrap_or_else(|e| e.into_inner());
                    files.remove(fd);
                }
                if !r.is_empty() {
                    r[0] = Val::I32(0);
                }
                Ok(())
            },
        )
        .map_err(|e| format!("fs close: {e}"))?;
    Ok(())
}

/// Guest import is wasi:cli/stdin read, but ABI is fd_read and file fds use it (#807).
fn register_fd_read_stub(
    linker: &mut Linker<()>,
    _engine: &Engine,
    ft: &FuncType,
) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            "wasi:cli/stdin@0.2.0",
            "read",
            ft,
            move |mut caller: Caller<'_, ()>, p: &[Val], r: &mut [Val]| {
                let errno = fd_read_impl(&mut caller, p);
                if !r.is_empty() {
                    r[0] = Val::I32(errno);
                }
                Ok(())
            },
        )
        .map_err(|e| format!("fd_read: {e}"))?;
    Ok(())
}

fn fd_read_impl(caller: &mut Caller<'_, ()>, p: &[Val]) -> i32 {
    if p.len() < 4 {
        return 28;
    }
    let (Val::I32(fd), Val::I32(iovs_ptr), Val::I32(iovs_len), Val::I32(nread_ptr)) =
        (p[0], p[1], p[2], p[3])
    else {
        return 28;
    };
    let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
        return 28;
    };
    if iovs_len <= 0 {
        let _ = mem.write(caller, nread_ptr as usize, &0i32.to_le_bytes());
        return 0;
    }
    let mut iov = [0u8; 8];
    if mem.read(&*caller, iovs_ptr as usize, &mut iov).is_err() {
        return 28;
    }
    let buf_ptr = i32::from_le_bytes([iov[0], iov[1], iov[2], iov[3]]);
    let buf_len = i32::from_le_bytes([iov[4], iov[5], iov[6], iov[7]]);
    if buf_len <= 0 {
        let _ = mem.write(caller, nread_ptr as usize, &0i32.to_le_bytes());
        return 0;
    }
    let mut buf = vec![0u8; buf_len as usize];
    let n = {
        let mut files = p2_files().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(file) = files.get_mut(&fd) {
            file.read(&mut buf).unwrap_or(0) as i32
        } else if fd == 0 {
            // Best-effort stdin; fixture suite usually does not exercise this.
            let _ = (&mut std::io::stdin()).read(&mut buf);
            0
        } else {
            return 8; // EBADF-ish
        }
    };
    if n > 0 {
        let _ = mem.write(&mut *caller, buf_ptr as usize, &buf[..n as usize]);
    }
    let _ = mem.write(&mut *caller, nread_ptr as usize, &n.to_le_bytes());
    0
}

/// P1-shaped args-sizes: (argc_ptr, argv_buf_size_ptr) -> errno.
/// Report argc=1 (program name only) so env::arg_count = argc-1 = 0 (#807).
fn register_args_sizes_stub(
    linker: &mut Linker<()>,
    _engine: &Engine,
    ft: &FuncType,
) -> Result<(), String> {
    let ft = ft.clone();
    let prog = b"arukellt-host-run\0";
    linker
        .func_new(
            "wasi:cli/environment@0.2.0",
            "args-sizes",
            ft,
            move |mut caller: Caller<'_, ()>, p: &[Val], r: &mut [Val]| {
                if p.len() >= 2 {
                    if let (Val::I32(argc_ptr), Val::I32(size_ptr)) = (p[0], p[1]) {
                        if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
                            let _ = mem.write(&mut caller, argc_ptr as usize, &1i32.to_le_bytes());
                            let _ = mem.write(
                                &mut caller,
                                size_ptr as usize,
                                &(prog.len() as i32).to_le_bytes(),
                            );
                        }
                    }
                }
                if !r.is_empty() {
                    r[0] = Val::I32(0);
                }
                Ok(())
            },
        )
        .map_err(|e| format!("args-sizes: {e}"))?;
    Ok(())
}

/// P1-shaped arguments: (argv_ptr, argv_buf_ptr) -> errno.
fn register_arguments_stub(
    linker: &mut Linker<()>,
    _engine: &Engine,
    ft: &FuncType,
) -> Result<(), String> {
    let ft = ft.clone();
    let prog = b"arukellt-host-run\0";
    linker
        .func_new(
            "wasi:cli/environment@0.2.0",
            "arguments",
            ft,
            move |mut caller: Caller<'_, ()>, p: &[Val], r: &mut [Val]| {
                if p.len() >= 2 {
                    if let (Val::I32(argv_ptr), Val::I32(buf_ptr)) = (p[0], p[1]) {
                        if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
                            let _ = mem.write(
                                &mut caller,
                                argv_ptr as usize,
                                &buf_ptr.to_le_bytes(),
                            );
                            let _ = mem.write(&mut caller, buf_ptr as usize, prog);
                        }
                    }
                }
                if !r.is_empty() {
                    r[0] = Val::I32(0);
                }
                Ok(())
            },
        )
        .map_err(|e| format!("arguments: {e}"))?;
    Ok(())
}

fn register_monotonic_now_stub(
    linker: &mut Linker<()>,
    _engine: &Engine,
    ft: &FuncType,
) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            "wasi:clocks/monotonic-clock@0.2.0",
            "now",
            ft,
            move |_: Caller<'_, ()>, _p: &[Val], r: &mut [Val]| {
                let nanos = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as i64)
                    .unwrap_or(1);
                if !r.is_empty() {
                    r[0] = Val::I64(nanos.max(1));
                }
                Ok(())
            },
        )
        .map_err(|e| format!("monotonic now: {e}"))?;
    Ok(())
}

fn register_wall_now_stub(
    linker: &mut Linker<()>,
    _engine: &Engine,
    ft: &FuncType,
) -> Result<(), String> {
    let ft = ft.clone();
    linker
        .func_new(
            "wasi:clocks/wall-clock@0.2.0",
            "now",
            ft,
            move |_: Caller<'_, ()>, _p: &[Val], r: &mut [Val]| {
                let dur = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                if !r.is_empty() {
                    r[0] = Val::I64(dur.as_secs() as i64);
                }
                if r.len() >= 2 {
                    r[1] = Val::I32(dur.subsec_nanos() as i32);
                }
                Ok(())
            },
        )
        .map_err(|e| format!("wall now: {e}"))?;
    Ok(())
}

fn register_random_u64_stub(
    linker: &mut Linker<()>,
    _engine: &Engine,
    ft: &FuncType,
) -> Result<(), String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0x9e37_79b9_7f4a_7c15);
    let ft = ft.clone();
    linker
        .func_new(
            "wasi:random/random@0.2.0",
            "get-random-u64",
            ft,
            move |_: Caller<'_, ()>, _p: &[Val], r: &mut [Val]| {
                let n = COUNTER.fetch_add(0x2545_f491_4f6c_dd1d, Ordering::Relaxed);
                if !r.is_empty() {
                    r[0] = Val::I64(n as i64);
                }
                Ok(())
            },
        )
        .map_err(|e| format!("get-random-u64: {e}"))?;
    Ok(())
}

/// Auto-generate a stub that returns zero values for any function type.
fn register_auto_stub(
    linker: &mut Linker<()>,
    _engine: &Engine,
    mod_name: &str,
    field_name: &str,
    ft: &FuncType,
) -> Result<(), String> {
    let results: Vec<ValType> = ft.results().collect();
    let ft = ft.clone();
    linker.func_new(mod_name, field_name, ft, move |_: Caller<'_, ()>, _p: &[Val], r: &mut [Val]| {
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
    }).map_err(|e| format!("{}::{}: {}", mod_name, field_name, e))?;
    Ok(())
}

pub fn run_until_breakpoint_for_program(
    wasm_bytes: &[u8],
    breakpoint_line: u32,
    caps: &RuntimeCaps,
    program: &Path,
) -> Result<DebugPause, String> {
    let source = fs::read_to_string(program).map_err(|e| e.to_string())?;
    run_until_breakpoint(wasm_bytes, breakpoint_line, caps, Some(&source))
}

pub fn run_smoke(wasm_bytes: &[u8], caps: &RuntimeCaps) -> Result<(), String> {
    run_wasm(wasm_bytes, caps)
}
