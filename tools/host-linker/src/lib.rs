//! Wasmtime runner with conditional `arukellt_host` HTTP and TCP socket linking.

mod debug_runner;
mod host_http;
mod host_sockets;
mod source_map;
mod wasm_debug_patch;

pub use debug_runner::{run_smoke, run_until_breakpoint, DebugPause, LiveLocal};
pub use source_map::{parse_source_map, SourceMapEntry};
pub use wasm_debug_patch::prepare_debug_wasm;

use wasmtime::*;
use wasmtime_wasi::preview1::WasiP1Ctx;
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtxBuilder};

pub struct DirGrant {
    pub host_path: String,
    pub guest_path: String,
    pub read_only: bool,
}

pub struct RuntimeCaps {
    pub dirs: Vec<DirGrant>,
}

impl RuntimeCaps {
    pub fn from_cli(dirs: &[String]) -> Self {
        RuntimeCaps {
            dirs: dirs.iter().map(|s| DirGrant::parse(s)).collect(),
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

pub fn run_wasm(wasm_bytes: &[u8], caps: &RuntimeCaps) -> Result<(), String> {
    let mut config = Config::new();
    config.cranelift_opt_level(OptLevel::None);
    config.wasm_bulk_memory(true);

    let engine = Engine::new(&config).map_err(|e| format!("engine creation error: {:?}", e))?;
    let module = Module::new(&engine, wasm_bytes)
        .map_err(|e| format!("wasm compile error: {:?}", e))?;

    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |cx| cx)
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

    let needs_http = module
        .imports()
        .any(|imp| imp.module() == "arukellt_host" && matches!(imp.name(), "http_get" | "http_request"));
    if needs_http {
        host_http::register_http_host_fns(&mut linker)?;
    }

    let needs_sockets = module.imports().any(|imp| {
        imp.module() == "arukellt_host"
            && matches!(
                imp.name(),
                "sockets_connect" | "sockets_read" | "sockets_write"
            )
    });
    if needs_sockets {
        host_sockets::register_sockets_host_fns(&mut linker)?;
    }

    let mut builder = WasiCtxBuilder::new();
    builder.inherit_stdio();
    builder.inherit_env();
    builder.arg("arukellt-host-run");

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

pub(crate) fn read_string_from_mem(
    caller: &Caller<'_, WasiP1Ctx>,
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

pub(crate) fn write_ok(
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

pub(crate) fn write_error(
    caller: &mut Caller<'_, WasiP1Ctx>,
    resp_ptr: i32,
    msg: &str,
) -> i32 {
    let ptr = resp_ptr as usize;
    let bytes = msg.as_bytes();
    if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
        let d = mem.data_mut(caller);
        let end = ptr + bytes.len();
        if end <= d.len() {
            d[ptr..end].copy_from_slice(bytes);
        }
    }
    -(bytes.len() as i32)
}
