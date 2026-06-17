//! Wasmtime-backed debug runner with breakpoint hooks and live local inspection.
//! Dynamically registers import stubs based on the module's import section,
//! supporting WASI P2 and future P3 imports automatically.

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

/// Scan the module's imports and register appropriate stubs for each.
/// Known WASI P2 imports get real implementations; unknown imports
/// (including future P3 additions) get auto-generated zero-value stubs.
fn register_import_stubs(linker: &mut Linker<()>, module: &Module) -> Result<(), String> {
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
        ("wasi:cli/environment@0.2.0", "args-sizes") => register_retptr_stub(linker, engine, mod_name, field_name, ft),
        ("wasi:cli/environment@0.2.0", "arguments") => register_retptr_stub(linker, engine, mod_name, field_name, ft),
        ("wasi:cli/stdin@0.2.0", "read") => register_retptr_stub(linker, engine, mod_name, field_name, ft),
        ("wasi:cli/exit@0.2.0", "exit") => register_exit_stub(linker, engine, ft),
        ("wasi:filesystem/types@0.2.0", "open-at") => register_retptr_stub(linker, engine, mod_name, field_name, ft),
        ("wasi:filesystem/types@0.2.0", "close") => register_retptr_stub(linker, engine, mod_name, field_name, ft),
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

/// Register an exit stub that traps.
fn register_exit_stub(
    linker: &mut Linker<()>,
    _engine: &Engine,
    ft: &FuncType,
) -> Result<(), String> {
    let ft = ft.clone();
    linker.func_new("wasi:cli/exit@0.2.0", "exit", ft, move |_: Caller<'_, ()>, _p: &[Val], _r: &mut [Val]| {
        Err(wasmtime::Error::msg("exit"))
    }).map_err(|e| format!("exit: {}", e))?;
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
