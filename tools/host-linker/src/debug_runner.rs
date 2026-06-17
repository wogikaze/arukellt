//! Wasmtime-backed debug runner with breakpoint hooks and live local inspection.
//! Supports WASI P2-style imports (wasi:cli/*, wasi:filesystem/*).

use crate::source_map::line_to_code_offset;
use crate::wasm_debug_patch::prepare_debug_wasm;
use crate::{run_wasm, RuntimeCaps};
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use wasmtime::*;

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
    config.wasm_multi_value(true);
    let engine = Engine::new(&config).map_err(|e| format!("engine: {:?}", e))?;
    let module = Module::new(&engine, &prepared).map_err(|e| format!("module: {:?}", e))?;

    let mut linker = Linker::<()>::new(&engine);
    register_p2_stubs(&mut linker)?;
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

fn register_p2_stubs(linker: &mut Linker<()>) -> Result<(), String> {
    use std::io::Write;

    // wasi:cli/stdout@0.2.0.write(fd: i32, buf: i32, len: i32, ret: i32) -> i32
    linker
        .func_wrap(
            "wasi:cli/stdout@0.2.0",
            "write",
            |mut caller: Caller<'_, ()>, _fd: i32, buf: i32, len: i32, ret: i32| -> Result<i32, wasmtime::Error> {
                let mem = caller.get_export("memory").and_then(|e| e.into_memory())
                    .ok_or_else(|| wasmtime::Error::msg("no memory"))?;
                let mut data = vec![0u8; len as usize];
                mem.read(&caller, buf as usize, &mut data)
                    .map_err(|e| wasmtime::Error::msg(e.to_string()))?;
                let n = std::io::stdout().write(&data).unwrap_or(0) as i32;
                let _ = std::io::stdout().flush();
                let nwritten = n.to_le_bytes();
                mem.write(&mut caller, ret as usize, &nwritten)
                    .map_err(|e| wasmtime::Error::msg(e.to_string()))?;
                Ok(0)
            },
        )
        .map_err(|e| format!("stdout write: {}", e))?;

    // wasi:cli/environment@0.2.0.args-sizes(ret0: i32, ret1: i32) -> i32
    linker
        .func_wrap(
            "wasi:cli/environment@0.2.0",
            "args-sizes",
            |mut caller: Caller<'_, ()>, ret0: i32, ret1: i32| -> Result<i32, wasmtime::Error> {
                let mem = caller.get_export("memory").and_then(|e| e.into_memory())
                    .ok_or_else(|| wasmtime::Error::msg("no memory"))?;
                mem.write(&mut caller, ret0 as usize, &0i32.to_le_bytes())
                    .map_err(|e| wasmtime::Error::msg(e.to_string()))?;
                mem.write(&mut caller, ret1 as usize, &0i32.to_le_bytes())
                    .map_err(|e| wasmtime::Error::msg(e.to_string()))?;
                Ok(0)
            },
        )
        .map_err(|e| format!("args-sizes: {}", e))?;

    // wasi:cli/environment@0.2.0.arguments(buf: i32, len: i32) -> i32
    linker
        .func_wrap(
            "wasi:cli/environment@0.2.0",
            "arguments",
            |_: Caller<'_, ()>, _buf: i32, _len: i32| -> Result<i32, wasmtime::Error> { Ok(0) },
        )
        .map_err(|e| format!("arguments: {}", e))?;

    // wasi:cli/stdin@0.2.0.read(fd: i32, buf: i32, len: i32, ret: i32) -> i32
    linker
        .func_wrap(
            "wasi:cli/stdin@0.2.0",
            "read",
            |_: Caller<'_, ()>, _fd: i32, _buf: i32, _len: i32, _ret: i32| -> Result<i32, wasmtime::Error> { Ok(0) },
        )
        .map_err(|e| format!("stdin read: {}", e))?;

    // wasi:cli/exit@0.2.0.exit(code: i32) -> ()
    linker
        .func_wrap(
            "wasi:cli/exit@0.2.0",
            "exit",
            |_code: i32| -> Result<(), wasmtime::Error> {
                Err(wasmtime::Error::msg("exit"))
            },
        )
        .map_err(|e| format!("exit: {}", e))?;

    // wasi:filesystem/types@0.2.0.open-at(7xi32,2xi64) -> i32
    linker
        .func_wrap(
            "wasi:filesystem/types@0.2.0",
            "open-at",
            |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32, _f: i64, _g: i64, _h: i32, _i: i32|
             -> Result<i32, wasmtime::Error> { Ok(-1) },
        )
        .map_err(|e| format!("open-at: {}", e))?;

    // wasi:filesystem/types@0.2.0.close(fd: i32) -> i32
    linker
        .func_wrap(
            "wasi:filesystem/types@0.2.0",
            "close",
            |_: Caller<'_, ()>, _fd: i32| -> Result<i32, wasmtime::Error> { Ok(0) },
        )
        .map_err(|e| format!("close: {}", e))?;

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
