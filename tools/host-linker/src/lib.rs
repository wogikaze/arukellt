//! Legacy core-module runner.
//!
//! Product WASI P2 components link the checked runtime adapter and run directly
//! under Wasmtime. This crate remains for bootstrap/core-module execution only;
//! it no longer registers Arukellt HTTP or socket bridge functions.

mod debug_runner;
mod p2_host;
mod source_map;
mod wasm_debug_patch;

pub use debug_runner::{run_smoke, run_until_breakpoint, DebugPause, LiveLocal};
pub use source_map::{parse_source_map, SourceMapEntry};
pub use wasm_debug_patch::prepare_debug_wasm;

use wasmtime::*;
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtxBuilder};

pub struct DirGrant {
    pub host_path: String,
    pub guest_path: String,
    pub read_only: bool,
}

pub struct RuntimeCaps {
    pub dirs: Vec<DirGrant>,
    /// Guest argv (program name + args). Empty → host supplies a default name.
    pub args: Vec<String>,
}

impl RuntimeCaps {
    pub fn from_cli(dirs: &[String]) -> Self {
        RuntimeCaps {
            dirs: dirs.iter().map(|s| DirGrant::parse(s)).collect(),
            args: Vec::new(),
        }
    }

    pub fn from_cli_with_args(dirs: &[String], args: &[String]) -> Self {
        RuntimeCaps {
            dirs: dirs.iter().map(|s| DirGrant::parse(s)).collect(),
            args: args.to_vec(),
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

fn make_run_engine() -> Result<Engine, String> {
    let mut config = Config::new();
    // Selfhost compile is minutes of guest work. OptLevel::None made every
    // phase 4–20× slower than wasmtime CLI (default Speed) on the same wasm.
    // Debug runner keeps None so breakpoint modules stay cheap to compile.
    config.cranelift_opt_level(OptLevel::Speed);
    config.wasm_bulk_memory(true);
    config.wasm_reference_types(true);
    config.wasm_function_references(true);
    config.wasm_gc(true);
    // Compiled-module cache only — not AST / s3 / overlay source cache.
    // First run pays Cranelift; later runs deserialize the same engine key.
    if let Ok(cache) = Cache::new(CacheConfig::new()) {
        config.cache(Some(cache));
    }
    Engine::new(&config).map_err(|e| format!("engine creation error: {:?}", e))
}

pub fn run_wasm(wasm_bytes: &[u8], caps: &RuntimeCaps) -> Result<(), String> {
    let engine = make_run_engine()?;
    let module = Module::new(&engine, wasm_bytes)
        .map_err(|e| format!("wasm compile error: {:?}", e))?;

    let uses_p2 = module.imports().any(|imp| imp.module().starts_with("wasi:"));
    if uses_p2 {
        return run_wasm_p2(&engine, &module, caps);
    }

    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |cx| cx)
        .map_err(|e| format!("wasi link error: {}", e))?;
    linker.allow_shadowing(true);
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "proc_exit",
            |_caller: Caller<'_, WasiP1Ctx>, code: i32| -> Result<(), wasmtime::Error> {
                Err(wasmtime_wasi::I32Exit(code).into())
            },
        )
        .map_err(|e| format!("proc_exit override error: {}", e))?;

    let mut builder = WasiCtxBuilder::new();
    builder.inherit_stdio();
    builder.inherit_env();
    if caps.args.is_empty() {
        builder.arg("arukellt-host-run");
    } else {
        for arg in &caps.args {
            builder.arg(arg);
        }
    }

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
    let wasi_ctx = builder.build_p1();
    let mut store = Store::new(&engine, wasi_ctx);
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("wasm instantiation error: {}", e))?;
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|e| format!("missing _start: {}", e))?;

    match start.call(&mut store, ()) {
        Ok(()) => Ok(()),
        Err(e) => {
            if let Some(exit) = e.downcast_ref::<wasmtime_wasi::I32Exit>() {
                std::process::exit(exit.0);
            }
            Err(format!("runtime error: {}", e))
        }
    }
}

fn run_wasm_p2(engine: &Engine, module: &Module, caps: &RuntimeCaps) -> Result<(), String> {
    // Bootstrap-only core-module compatibility. Product P2 components use the
    // real-WASI runtime adapter and never enter this path.
    let mut linker = Linker::<p2_host::P2Store>::new(engine);
    linker.allow_shadowing(true);
    p2_host::register_p2_imports(&mut linker, module)
        .map_err(|e| format!("p2 imports: {}", e))?;

    let state = std::sync::Arc::new(std::sync::Mutex::new(p2_host::P2HostState::from_caps(caps)));
    let mut store = Store::new(engine, state);
    let instance = linker
        .instantiate(&mut store, module)
        .map_err(|e| format!("wasm instantiation error: {}", e))?;
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|e| format!("missing _start: {}", e))?;
    match start.call(&mut store, ()) {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("runtime error: {}", e)),
    }
}

pub(crate) fn read_string_from_mem<T>(
    caller: &Caller<'_, T>,
    mem: &Memory,
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

pub(crate) fn write_ok<T>(
    caller: &mut Caller<'_, T>,
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

pub(crate) fn write_error<T>(
    caller: &mut Caller<'_, T>,
    resp_ptr: i32,
    msg: &str,
) -> i32 {
    let ptr = resp_ptr as usize;
    let bytes = msg.as_bytes();
    if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
        let data = mem.data_mut(caller);
        let end = ptr + bytes.len();
        if end <= data.len() {
            data[ptr..end].copy_from_slice(bytes);
        }
    }
    -(bytes.len() as i32)
}
