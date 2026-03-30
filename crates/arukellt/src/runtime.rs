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
        .map_err(|e| format!("runtime error: {}", e))?;

    Ok(())
}

/// Run a Wasm GC module (T3 target) with wasmtime GC support enabled.
pub(crate) fn run_wasm_gc(wasm_bytes: &[u8], caps: &RuntimeCaps) -> Result<(), String> {
    use wasmtime::*;
    use wasmtime_wasi::preview1::WasiP1Ctx;
    use wasmtime_wasi::{DirPerms, FilePerms, WasiCtxBuilder};

    let mut config = Config::new();
    config.wasm_gc(true);

    let engine =
        Engine::new(&config).map_err(|e| format!("engine creation error (GC): {:?}", e))?;
    let module = wasmtime::Module::new(&engine, wasm_bytes)
        .map_err(|e| format!("wasm compile error (GC): {:?}", e))?;

    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |cx| cx)
        .map_err(|e| format!("wasi link error: {}", e))?;

    let mut builder = WasiCtxBuilder::new();
    builder.inherit_stdio();
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
        .map_err(|e| format!("runtime error: {}", e))?;

    Ok(())
}
