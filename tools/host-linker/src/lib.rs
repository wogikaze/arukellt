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

use std::fs;
use std::path::Path;
use wasmtime::*;
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtxBuilder};

// Keeps the selfhost compiler below the 1,000,000 KiB RSS gate while avoiding
// the repeated GC growth pauses seen with the default empty reservation.
const COMPILER_GC_HEAP_PREGROW_BYTES: u32 = 450 * 1024 * 1024;

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

/// `ARUKELLT_HOST_PROFILE=perfmap|jitdump` registers JIT symbols with `perf`
/// so selfhost overlay hot functions can be attributed by wasm function name.
/// Unset (the default) keeps the plain engine.
fn host_profiling_strategy() -> Result<Option<ProfilingStrategy>, String> {
    match std::env::var("ARUKELLT_HOST_PROFILE") {
        Err(_) => Ok(None),
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) if value == "perfmap" => Ok(Some(ProfilingStrategy::PerfMap)),
        Ok(value) if value == "jitdump" => Ok(Some(ProfilingStrategy::JitDump)),
        Ok(value) => Err(format!(
            "ARUKELLT_HOST_PROFILE={value}: expected perfmap or jitdump"
        )),
    }
}

/// `ARUKELLT_HOST_COLLECTOR=auto|drc|null|copying` picks the GC collector and
/// `ARUKELLT_HOST_GC_HEAP_RESERVATION=<bytes>` its reservation. Diagnostics
/// only: the null collector shows the compute floor without collection work.
/// Unset keeps wasmtime's defaults.
fn host_collector() -> Result<Option<Collector>, String> {
    match std::env::var("ARUKELLT_HOST_COLLECTOR") {
        Err(_) => Ok(None),
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) if value == "auto" => Ok(Some(Collector::Auto)),
        Ok(value) if value == "drc" => Ok(Some(Collector::DeferredReferenceCounting)),
        Ok(value) if value == "null" => Ok(Some(Collector::Null)),
        Ok(value) if value == "copying" => Ok(Some(Collector::Copying)),
        Ok(value) => Err(format!(
            "ARUKELLT_HOST_COLLECTOR={value}: expected auto, drc, null or copying"
        )),
    }
}

fn host_gc_heap_reservation() -> Result<Option<u64>, String> {
    match std::env::var("ARUKELLT_HOST_GC_HEAP_RESERVATION") {
        Err(_) => Ok(None),
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => value.parse::<u64>().map(Some).map_err(|e| {
            format!("ARUKELLT_HOST_GC_HEAP_RESERVATION={value}: expected bytes ({e})")
        }),
    }
}

/// `ARUKELLT_HOST_GC_HEAP_PREGROW=<bytes>` allocates and immediately drops one
/// byte array of that size before `_start`. Wasmtime grows the GC heap only
/// when an allocation does not fit after a collection, so a growing live set
/// otherwise collects every few MB; pre-growing sizes the semi-spaces up front.
/// Selfhost compiler invocations use the measured default below; the
/// environment value is an explicit override for diagnostics and tuning.
fn host_gc_heap_pregrow() -> Result<Option<u32>, String> {
    match std::env::var("ARUKELLT_HOST_GC_HEAP_PREGROW") {
        Err(_) => Ok(None),
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => value.parse::<u32>().map(Some).map_err(|e| {
            format!("ARUKELLT_HOST_GC_HEAP_PREGROW={value}: expected bytes < 4GiB ({e})")
        }),
    }
}

fn pregrow_gc_heap<T>(store: &mut Store<T>, caps: &RuntimeCaps) -> Result<(), String> {
    let configured = host_gc_heap_pregrow()?;
    let is_selfhost_compile = caps.args.iter().any(|arg| {
        arg == "src/compiler/main.ark"
            || arg.ends_with("/src/compiler/main.ark")
    });
    let bytes = match configured {
        Some(value) => Some(value),
        None if is_selfhost_compile && caps.args.iter().any(|arg| arg == "compile") => {
            Some(COMPILER_GC_HEAP_PREGROW_BYTES)
        }
        None => None,
    };
    let Some(bytes) = bytes else {
        return Ok(());
    };
    // Wasmtime initializes array elements one `Val` at a time, so use the
    // widest scalar element to keep the pre-grow itself cheap.
    let i64_array = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, StorageType::ValType(ValType::I64)),
    );
    let pre = ArrayRefPre::new(&mut *store, i64_array);
    let mut scope = RootScope::new(&mut *store);
    ArrayRef::new(&mut scope, &pre, &Val::I64(0), bytes / 8)
        .map_err(|e| format!("gc heap pregrow of {bytes} bytes failed: {e}"))?;
    Ok(())
}

/// True when any diagnostics knob changes engine config, so the shared
/// `.cwasm` (built with the plain config) must not be reused or overwritten.
fn host_engine_is_customized() -> Result<bool, String> {
    Ok(host_profiling_strategy()?.is_some()
        || host_collector()?.is_some()
        || host_gc_heap_reservation()?.is_some())
}

fn make_run_engine() -> Result<Engine, String> {
    let mut config = Config::new();
    if let Some(collector) = host_collector()? {
        config.collector(collector);
    }
    if let Some(bytes) = host_gc_heap_reservation()? {
        config.gc_heap_reservation(bytes);
    }
    // Selfhost compile is minutes of guest work. OptLevel::None made every
    // phase 4–20× slower than wasmtime CLI (default Speed) on the same wasm.
    // Debug runner keeps None so breakpoint modules stay cheap to compile.
    config.cranelift_opt_level(OptLevel::Speed);
    config.wasm_bulk_memory(true);
    config.wasm_reference_types(true);
    config.wasm_function_references(true);
    config.wasm_gc(true);
    if let Some(strategy) = host_profiling_strategy()? {
        config.profiler(strategy);
    }
    // Compiled-module cache only — not AST / s3 / overlay source cache.
    // First run pays Cranelift; later runs deserialize the same engine key.
    if let Ok(cache) = Cache::new(CacheConfig::new()) {
        config.cache(Some(cache));
    }
    Engine::new(&config).map_err(|e| format!("engine creation error: {:?}", e))
}

fn serialized_module_path(wasm_path: &Path) -> std::path::PathBuf {
    wasm_path.with_extension("cwasm")
}

fn load_module(engine: &Engine, wasm_path: &Path) -> Result<Module, String> {
    let cwasm = serialized_module_path(wasm_path);
    // A customized engine (profiler, collector) has a different compile key; it
    // must neither reuse nor overwrite the `.cwasm` that plain runs deserialize.
    let customized = host_engine_is_customized()?;
    if !customized {
        if let (Ok(cw), Ok(w)) = (cwasm.metadata(), wasm_path.metadata()) {
            if cw.modified().ok() >= w.modified().ok() {
                // Engine config in make_run_engine must stay aligned with serialize.
                match unsafe { Module::deserialize_file(engine, &cwasm) } {
                    Ok(module) => return Ok(module),
                    Err(_) => {}
                }
            }
        }
    }
    let wasm_bytes = fs::read(wasm_path)
        .map_err(|e| format!("failed to read {}: {}", wasm_path.display(), e))?;
    let module = Module::new(engine, &wasm_bytes)
        .map_err(|e| format!("wasm compile error: {:?}", e))?;
    if !customized {
        if let Ok(bytes) = module.serialize() {
            let _ = fs::write(&cwasm, bytes);
        }
    }
    Ok(module)
}

pub fn run_wasm_path(wasm_path: &Path, caps: &RuntimeCaps) -> Result<(), String> {
    let engine = make_run_engine()?;
    let module = load_module(&engine, wasm_path)?;
    run_compiled_module(&engine, &module, caps)
}

pub fn run_wasm(wasm_bytes: &[u8], caps: &RuntimeCaps) -> Result<(), String> {
    let engine = make_run_engine()?;
    let module = Module::new(&engine, wasm_bytes)
        .map_err(|e| format!("wasm compile error: {:?}", e))?;
    run_compiled_module(&engine, &module, caps)
}

fn run_compiled_module(engine: &Engine, module: &Module, caps: &RuntimeCaps) -> Result<(), String> {
    let uses_p2 = module.imports().any(|imp| imp.module().starts_with("wasi:"));
    if uses_p2 {
        return run_wasm_p2(engine, module, caps);
    }

    let mut linker = Linker::<WasiP1Ctx>::new(engine);
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
    let mut store = Store::new(engine, wasi_ctx);
    pregrow_gc_heap(&mut store, caps)?;
    let instance = linker
        .instantiate(&mut store, module)
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
    pregrow_gc_heap(&mut store, caps)?;
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
