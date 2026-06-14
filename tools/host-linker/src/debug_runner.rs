//! Wasmtime-backed debug runner with breakpoint hooks and live local inspection.

use crate::source_map::line_to_code_offset;
use crate::wasm_debug_patch::prepare_debug_wasm;
use crate::{run_wasm, RuntimeCaps};
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use wasmtime::*;
use wasmtime_wasi::preview1::WasiP1Ctx;
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtxBuilder};

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
    caps: &RuntimeCaps,
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
    let engine = Engine::new(&config).map_err(|e| format!("engine: {:?}", e))?;
    let module = Module::new(&engine, &prepared).map_err(|e| format!("module: {:?}", e))?;

    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |cx| cx)
        .map_err(|e| format!("wasi: {}", e))?;
    linker.allow_shadowing(true);
    linker
        .func_wrap(
            "arukellt_debug",
            "breakpoint",
            |line: i32, value: i32| -> Result<(), wasmtime::Error> {
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

    let mut builder = WasiCtxBuilder::new();
    builder.inherit_stdio();
    builder.inherit_env();
    builder.arg("arukellt-debug-run");
    for grant in &caps.dirs {
        let (dp, fp) = if grant.read_only {
            (DirPerms::READ, FilePerms::READ)
        } else {
            (DirPerms::all(), FilePerms::all())
        };
        builder
            .preopened_dir(&grant.host_path, &grant.guest_path, dp, fp)
            .map_err(|e| format!("dir: {}", e))?;
    }
    let wasi_ctx = builder.build_p1();
    let mut store = Store::new(&engine, wasi_ctx);
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
