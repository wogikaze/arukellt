use std::collections::{HashMap, HashSet};

use anyhow::{Result, anyhow, bail};
use lang_core::{Pattern, Type, compile_module};
use lang_ir::{HighExpr, HighExprKind, HighFunction, HighMatchArm, HighModule, lower_to_high_ir};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WasmTarget {
    JavaScriptHost,
    Wasi,
}

pub fn build_module_from_source(source: &str, target: WasmTarget) -> Result<Vec<u8>> {
    let result = compile_module(source);
    if result.error_count() > 0 {
        bail!("{}", serde_json::to_string_pretty(&result.to_json()?)?);
    }
    let typed = result
        .module
        .ok_or_else(|| anyhow!("typed module missing"))?;
    let high = lower_to_high_ir(&typed);
    emit_wasm(&high, target)
}

pub fn emit_wasm(module: &HighModule, target: WasmTarget) -> Result<Vec<u8>> {
    let abi = WasmAbi::from_module(module, target)?;
    let mut wat_module = String::from("(module\n");
    let function_names = module
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect::<HashSet<_>>();
    if target == WasmTarget::Wasi && abi.uses_console_println {
        wat_module.push_str(
            "  (import \"wasi_snapshot_preview1\" \"fd_write\" \
             (func $fd_write (param i32 i32 i32 i32) (result i32)))\n",
        );
    }
    emit_all_memory(&abi, &mut wat_module);
    emit_heap_primitives(&abi, &mut wat_module);
    if target == WasmTarget::Wasi {
        if abi.uses_string_builtin {
            emit_string_helper(&abi, &mut wat_module);
        }
    }
    emit_closure_support(&abi, &function_names, &mut wat_module)?;
    for function in &module.functions {
        wat_module.push_str(&emit_function(function, &function_names, &abi)?);
    }
    if target == WasmTarget::Wasi && abi.uses_console_println {
        emit_console_println_helper(&abi, &mut wat_module);
    }
    match target {
        WasmTarget::JavaScriptHost => emit_javascript_exports(module, &mut wat_module),
        WasmTarget::Wasi => emit_wasi_entrypoint(module, &abi, &mut wat_module)?,
    }
    wat_module.push_str(")\n");
    Ok(wat::parse_str(&wat_module)?)
}

fn emit_javascript_exports(module: &HighModule, out: &mut String) {
    for function in &module.functions {
        out.push_str(&format!(
            "  (export \"{}\" (func ${}))\n",
            function.name, function.name
        ));
    }
}

fn emit_heap_primitives(abi: &WasmAbi, out: &mut String) {
    if !abi.needs_heap() {
        return;
    }
    emit_alloc_helper(out);
    emit_strlen_helper(out);
    emit_memcpy_helper(out);
}

fn emit_wasi_entrypoint(module: &HighModule, abi: &WasmAbi, out: &mut String) -> Result<()> {
    let main = module
        .functions
        .iter()
        .find(|function| function.name == "main")
        .ok_or_else(|| anyhow!("wasm-wasi target requires a `main` function"))?;
    if !main.params.is_empty() {
        bail!("wasm-wasi target requires `main` to take no parameters");
    }

    out.push_str("  (func $_start\n");
    out.push_str("    call $main\n");
    if abi.wasm_type(&main.return_type)?.is_some() {
        out.push_str("    drop\n");
    }
    out.push_str("  )\n");
    out.push_str("  (export \"_start\" (func $_start))\n");
    Ok(())
}

/// Emit the `$console.println` WASI helper: computes strlen, writes the string
/// to stdout via fd_write, then writes a trailing newline.
fn emit_console_println_helper(abi: &WasmAbi, out: &mut String) {
    let iovec_ptr = abi.iovec_base();
    let iovec_len = abi.iovec_base() + 4;
    let nwritten = abi.nwritten_base();
    let newline = abi.newline_base();

    out.push_str("  (func $console.println (param $ptr i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $cur i32)\n");
    // Compute strlen: scan for NUL byte
    out.push_str("    local.get $ptr\n");
    out.push_str("    local.set $cur\n");
    out.push_str("    (block $strlen_break\n");
    out.push_str("      (loop $strlen_loop\n");
    out.push_str("        local.get $cur\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.eqz\n");
    out.push_str("        br_if $strlen_break\n");
    out.push_str("        local.get $cur\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $cur\n");
    out.push_str("        br $strlen_loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $cur\n");
    out.push_str("    local.get $ptr\n");
    out.push_str("    i32.sub\n");
    out.push_str("    local.set $len\n");
    // Set iovec: {ptr, len}
    out.push_str(&format!("    i32.const {iovec_ptr}\n"));
    out.push_str("    local.get $ptr\n");
    out.push_str("    i32.store\n");
    out.push_str(&format!("    i32.const {iovec_len}\n"));
    out.push_str("    local.get $len\n");
    out.push_str("    i32.store\n");
    // fd_write(1, iovec_base, 1, nwritten)
    out.push_str("    i32.const 1\n");
    out.push_str(&format!("    i32.const {iovec_ptr}\n"));
    out.push_str("    i32.const 1\n");
    out.push_str(&format!("    i32.const {nwritten}\n"));
    out.push_str("    call $fd_write\n");
    out.push_str("    drop\n");
    // Write newline: iovec = {newline_base, 1}
    out.push_str(&format!("    i32.const {iovec_ptr}\n"));
    out.push_str(&format!("    i32.const {newline}\n"));
    out.push_str("    i32.store\n");
    out.push_str(&format!("    i32.const {iovec_len}\n"));
    out.push_str("    i32.const 1\n");
    out.push_str("    i32.store\n");
    out.push_str("    i32.const 1\n");
    out.push_str(&format!("    i32.const {iovec_ptr}\n"));
    out.push_str("    i32.const 1\n");
    out.push_str(&format!("    i32.const {nwritten}\n"));
    out.push_str("    call $fd_write\n");
    out.push_str("    drop\n");
    out.push_str("  )\n");
}

/// Emit the `$string` helper: converts an i32 to decimal ASCII in the scratch
/// buffer, then copies it into durable heap-backed storage and returns a pointer.
fn emit_string_helper(abi: &WasmAbi, out: &mut String) {
    // str_buf occupies [scratch_base+16, scratch_base+28), written backward.
    // str_buf_end is the exclusive end; we start by placing NUL at str_buf_end-1.
    let nul_pos = abi.str_buf_end() - 1;

    out.push_str("  (func $string (param $n i32) (result i32)\n");
    out.push_str("    (local $abs i32)\n");
    out.push_str("    (local $neg i32)\n");
    out.push_str("    (local $pos i32)\n");
    out.push_str("    (local $src i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $dst i32)\n");
    // Write NUL at nul_pos
    out.push_str(&format!("    i32.const {nul_pos}\n"));
    out.push_str("    local.set $pos\n");
    out.push_str("    local.get $pos\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store8\n");
    // neg = (n < 0)
    out.push_str("    local.get $n\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.lt_s\n");
    out.push_str("    local.set $neg\n");
    // Special case: n == 0 → write '0'
    out.push_str("    local.get $n\n");
    out.push_str("    i32.eqz\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        local.get $pos\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.sub\n");
    out.push_str("        local.set $pos\n");
    out.push_str("        local.get $pos\n");
    out.push_str("        i32.const 48\n");
    out.push_str("        i32.store8\n");
    out.push_str("      )\n");
    out.push_str("      (else\n");
    // abs = neg ? (0 - n wrapping) : n
    // Wrapping subtraction is safe: for INT_MIN, (0 - INT_MIN) wraps to INT_MIN,
    // but i32.rem_u / i32.div_u treat it as unsigned 2147483648, giving correct digits.
    out.push_str("        local.get $neg\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            local.get $n\n");
    out.push_str("            i32.sub\n");
    out.push_str("            local.set $abs\n");
    out.push_str("          )\n");
    out.push_str("          (else\n");
    out.push_str("            local.get $n\n");
    out.push_str("            local.set $abs\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    // Loop: extract digits backward
    out.push_str("        (block $digits_break\n");
    out.push_str("          (loop $digits_loop\n");
    out.push_str("            local.get $abs\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $digits_break\n");
    out.push_str("            local.get $pos\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.sub\n");
    out.push_str("            local.set $pos\n");
    out.push_str("            local.get $pos\n");
    out.push_str("            local.get $abs\n");
    out.push_str("            i32.const 10\n");
    out.push_str("            i32.rem_u\n");
    out.push_str("            i32.const 48\n");
    out.push_str("            i32.add\n");
    out.push_str("            i32.store8\n");
    out.push_str("            local.get $abs\n");
    out.push_str("            i32.const 10\n");
    out.push_str("            i32.div_u\n");
    out.push_str("            local.set $abs\n");
    out.push_str("            br $digits_loop\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    // Write '-' if negative
    out.push_str("    local.get $neg\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        local.get $pos\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.sub\n");
    out.push_str("        local.set $pos\n");
    out.push_str("        local.get $pos\n");
    out.push_str("        i32.const 45\n");
    out.push_str("        i32.store8\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $pos\n");
    out.push_str("    local.set $src\n");
    out.push_str("    local.get $src\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $len\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    i32.add\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $dst\n");
    out.push_str("    local.get $dst\n");
    out.push_str("    local.get $src\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    i32.add\n");
    out.push_str("    call $__memcpy\n");
    out.push_str("    local.get $dst\n");
    out.push_str("  )\n");
}

fn emit_function(
    function: &HighFunction,
    function_names: &HashSet<String>,
    abi: &WasmAbi,
) -> Result<String> {
    let result_type = abi.wasm_type(&function.return_type)?;
    let params = function
        .params
        .iter()
        .filter_map(|param| {
            abi.wasm_type(&param.ty).transpose().map(|wasm_ty| {
                wasm_ty.map(|wasm_ty| format!("(param ${} {})", param.name, wasm_ty))
            })
        })
        .collect::<Result<Vec<_>>>()?
        .join(" ");
    let locals = function
        .params
        .iter()
        .filter_map(|param| {
            abi.wasm_type(&param.ty)
                .ok()
                .flatten()
                .map(|_| param.name.clone())
        })
        .collect::<HashSet<_>>();
    let mut local_bindings = function
        .params
        .iter()
        .filter_map(|param| {
            abi.wasm_type(&param.ty)
                .ok()
                .flatten()
                .map(|_| (param.name.clone(), Some(param.name.clone())))
        })
        .collect::<HashMap<_, _>>();
    let mut collect_local_names = locals.clone();
    let mut let_locals = Vec::new();
    collect_let_locals(
        &function.body,
        abi,
        &mut collect_local_names,
        &mut let_locals,
    )?;
    let mut out = String::new();
    out.push_str(&format!("  (func ${}", function.name));
    if !params.is_empty() {
        out.push(' ');
        out.push_str(&params);
    }
    if let Some(result_type) = result_type {
        out.push_str(&format!(" (result {result_type})"));
    }
    out.push('\n');
    for (local_name, local_ty) in &let_locals {
        out.push_str(&format!("    (local ${local_name} {local_ty})\n"));
    }
    let mut emit_local_names = locals.clone();
    emit_expr(
        &function.body,
        2,
        &mut local_bindings,
        &mut emit_local_names,
        &HashMap::new(),
        &HashMap::new(),
        None,
        function_names,
        abi,
        &mut out,
    )?;
    if abi.wasm_type(&function.return_type)?.is_none()
        && abi.wasm_type(&function.body.ty)?.is_some()
    {
        out.push_str("    drop\n");
    }
    out.push_str("  )\n");
    Ok(out)
}

#[derive(Clone, Debug)]
struct MatchBinding {
    subject: HighExpr,
    offset: u32,
}

fn emit_expr(
    expr: &HighExpr,
    indent: usize,
    locals: &mut HashMap<String, Option<String>>,
    declared_local_names: &mut HashSet<String>,
    match_bindings: &HashMap<String, MatchBinding>,
    captures: &HashMap<String, u32>,
    env_local: Option<&str>,
    function_names: &HashSet<String>,
    abi: &WasmAbi,
    out: &mut String,
) -> Result<()> {
    let pad = "  ".repeat(indent);
    match &expr.kind {
        HighExprKind::Int(value) => {
            out.push_str(&format!("{pad}i32.const {value}\n"));
        }
        HighExprKind::Bool(value) => {
            let numeric = i32::from(*value);
            out.push_str(&format!("{pad}i32.const {numeric}\n"));
        }
        HighExprKind::List(items) => {
            emit_list_literal(
                items,
                &expr.ty,
                indent,
                locals,
                declared_local_names,
                match_bindings,
                captures,
                env_local,
                function_names,
                abi,
                out,
            )?;
        }
        HighExprKind::Tuple(_) => {
            bail!("tuple expressions are not yet supported in wasm backend");
        }
        HighExprKind::Ident(name) => {
            if let Some(binding) = locals.get(name) {
                if let Some(local_name) = binding {
                    out.push_str(&format!("{pad}local.get ${local_name}\n"));
                }
            } else if let Some(binding) = match_bindings.get(name) {
                emit_expr(
                    &binding.subject,
                    indent,
                    locals,
                    declared_local_names,
                    match_bindings,
                    captures,
                    env_local,
                    function_names,
                    abi,
                    out,
                )?;
                out.push_str(&format!("{pad}i32.load offset={}\n", binding.offset));
            } else if let (Some(env_local), Some(offset)) = (env_local, captures.get(name)) {
                out.push_str(&format!("{pad}local.get ${env_local}\n"));
                if *offset == 0 {
                    out.push_str(&format!("{pad}i32.load\n"));
                } else {
                    out.push_str(&format!("{pad}i32.load offset={offset}\n"));
                }
            } else if let Some(callback) = abi.named_callback_for(name) {
                out.push_str(&format!("{pad}call ${}\n", callback.alloc_name));
            } else {
                bail!(
                    "function references and non-local identifiers are not yet supported in wasm backend: {name}"
                );
            }
        }
        HighExprKind::Binary { op, left, right } => {
            if left.ty == Type::String || right.ty == Type::String {
                bail!("string operations are not yet supported in wasm backend");
            }
            emit_expr(
                left,
                indent,
                locals,
                declared_local_names,
                match_bindings,
                captures,
                env_local,
                function_names,
                abi,
                out,
            )?;
            emit_expr(
                right,
                indent,
                locals,
                declared_local_names,
                match_bindings,
                captures,
                env_local,
                function_names,
                abi,
                out,
            )?;
            let opcode = match op {
                lang_core::BinaryOp::Add => "i32.add",
                lang_core::BinaryOp::Subtract => "i32.sub",
                lang_core::BinaryOp::Multiply => "i32.mul",
                lang_core::BinaryOp::Divide => "i32.div_s",
                lang_core::BinaryOp::Modulo => "i32.rem_s",
                lang_core::BinaryOp::Greater => "i32.gt_s",
                lang_core::BinaryOp::Less => "i32.lt_s",
                lang_core::BinaryOp::Equal => "i32.eq",
                lang_core::BinaryOp::And => "i32.and",
                lang_core::BinaryOp::Or => "i32.or",
            };
            out.push_str(&format!("{pad}{opcode}\n"));
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            emit_expr(
                condition,
                indent,
                locals,
                declared_local_names,
                match_bindings,
                captures,
                env_local,
                function_names,
                abi,
                out,
            )?;
            out.push_str(&format!("{pad}(if"));
            if let Some(result_ty) = abi.wasm_type(&expr.ty)? {
                out.push_str(&format!(" (result {result_ty})"));
            }
            out.push('\n');
            out.push_str(&format!("{pad}  (then\n"));
            emit_expr(
                then_branch,
                indent + 2,
                locals,
                declared_local_names,
                match_bindings,
                captures,
                env_local,
                function_names,
                abi,
                out,
            )?;
            out.push_str(&format!("{pad}  )\n"));
            out.push_str(&format!("{pad}  (else\n"));
            emit_expr(
                else_branch,
                indent + 2,
                locals,
                declared_local_names,
                match_bindings,
                captures,
                env_local,
                function_names,
                abi,
                out,
            )?;
            out.push_str(&format!("{pad}  )\n"));
            out.push_str(&format!("{pad})\n"));
        }
        HighExprKind::Call { callee, args } => {
            if callee == "range_inclusive" && abi.target == WasmTarget::Wasi {
                let [start, end] = args.as_slice() else {
                    bail!("`range_inclusive` expects exactly two arguments in wasm backend");
                };
                emit_expr(
                    start,
                    indent,
                    locals,
                    declared_local_names,
                    match_bindings,
                    captures,
                    env_local,
                    function_names,
                    abi,
                    out,
                )?;
                emit_expr(
                    end,
                    indent,
                    locals,
                    declared_local_names,
                    match_bindings,
                    captures,
                    env_local,
                    function_names,
                    abi,
                    out,
                )?;
                out.push_str(&format!("{pad}call $__range_inclusive\n"));
            } else if callee == "map" && abi.target == WasmTarget::Wasi {
                let [receiver, callback] = args.as_slice() else {
                    bail!("`map` expects exactly two arguments in wasm backend");
                };
                let signature = abi
                    .closure_signature_for_apply(callback, callback)
                    .ok_or_else(|| {
                        anyhow!("unsupported callback shape for `map` in wasm backend")
                    })?;
                emit_expr(
                    receiver,
                    indent,
                    locals,
                    declared_local_names,
                    match_bindings,
                    captures,
                    env_local,
                    function_names,
                    abi,
                    out,
                )?;
                emit_expr(
                    callback,
                    indent,
                    locals,
                    declared_local_names,
                    match_bindings,
                    captures,
                    env_local,
                    function_names,
                    abi,
                    out,
                )?;
                out.push_str(&format!(
                    "{pad}call ${}\n",
                    abi.map_helper_name(signature.index)
                ));
            } else if callee == "filter" && abi.target == WasmTarget::Wasi {
                let [receiver, callback] = args.as_slice() else {
                    bail!("`filter` expects exactly two arguments in wasm backend");
                };
                match list_item_type(&receiver.ty) {
                    Some(Type::Int) => {}
                    Some(_) => {
                        bail!("only List<i64>.filter is currently supported in wasm backend")
                    }
                    None => bail!("`filter` expects a list receiver in wasm backend"),
                }
                let signature = abi
                    .closure_signature_for_apply(callback, callback)
                    .ok_or_else(|| {
                        anyhow!("unsupported callback shape for `filter` in wasm backend")
                    })?;
                if signature.key.result != Type::Bool {
                    bail!("`filter` callback must return Bool in wasm backend");
                }
                emit_expr(
                    receiver,
                    indent,
                    locals,
                    declared_local_names,
                    match_bindings,
                    captures,
                    env_local,
                    function_names,
                    abi,
                    out,
                )?;
                emit_expr(
                    callback,
                    indent,
                    locals,
                    declared_local_names,
                    match_bindings,
                    captures,
                    env_local,
                    function_names,
                    abi,
                    out,
                )?;
                out.push_str(&format!(
                    "{pad}call ${}\n",
                    abi.filter_helper_name(signature.index)
                ));
            } else if callee == "sum" && abi.target == WasmTarget::Wasi {
                let [receiver] = args.as_slice() else {
                    bail!("`sum` expects exactly one argument in wasm backend");
                };
                match list_item_type(&receiver.ty) {
                    Some(Type::Int) => {}
                    Some(_) => bail!("only List<i64>.sum is currently supported in wasm backend"),
                    None => bail!("`sum` expects a list receiver in wasm backend"),
                }
                emit_expr(
                    receiver,
                    indent,
                    locals,
                    declared_local_names,
                    match_bindings,
                    captures,
                    env_local,
                    function_names,
                    abi,
                    out,
                )?;
                out.push_str(&format!("{pad}call ${}\n", abi.sum_helper_name()));
            } else if callee == "join" && abi.target == WasmTarget::Wasi {
                let [receiver, separator] = args.as_slice() else {
                    bail!("`join` expects exactly two arguments in wasm backend");
                };
                match list_item_type(&receiver.ty) {
                    Some(Type::String) => {}
                    Some(_) => {
                        bail!("only List<String>.join is currently supported in wasm backend")
                    }
                    None => bail!("`join` expects a list receiver in wasm backend"),
                }
                if !matches!(separator.ty, Type::String) {
                    bail!("`join` expects a String separator in wasm backend");
                }
                emit_expr(
                    receiver,
                    indent,
                    locals,
                    declared_local_names,
                    match_bindings,
                    captures,
                    env_local,
                    function_names,
                    abi,
                    out,
                )?;
                emit_expr(
                    separator,
                    indent,
                    locals,
                    declared_local_names,
                    match_bindings,
                    captures,
                    env_local,
                    function_names,
                    abi,
                    out,
                )?;
                out.push_str(&format!("{pad}call $__list_join_strings\n"));
            } else if function_names.contains(callee) {
                for arg in args {
                    emit_expr(
                        arg,
                        indent,
                        locals,
                        declared_local_names,
                        match_bindings,
                        captures,
                        env_local,
                        function_names,
                        abi,
                        out,
                    )?;
                }
                out.push_str(&format!("{pad}call ${callee}\n"));
            } else if callee == "__apply" {
                emit_apply(
                    expr,
                    args,
                    indent,
                    locals,
                    declared_local_names,
                    match_bindings,
                    captures,
                    env_local,
                    function_names,
                    abi,
                    out,
                )?;
            } else if abi.target == WasmTarget::Wasi && callee == "console.println" {
                for arg in args {
                    emit_expr(
                        arg,
                        indent,
                        locals,
                        declared_local_names,
                        match_bindings,
                        captures,
                        env_local,
                        function_names,
                        abi,
                        out,
                    )?;
                }
                out.push_str(&format!("{pad}call $console.println\n"));
            } else if abi.target == WasmTarget::Wasi && callee == "string" {
                for arg in args {
                    emit_expr(
                        arg,
                        indent,
                        locals,
                        declared_local_names,
                        match_bindings,
                        captures,
                        env_local,
                        function_names,
                        abi,
                        out,
                    )?;
                }
                out.push_str(&format!("{pad}call $string\n"));
            } else {
                bail!("calls to `{callee}` are not yet supported in wasm backend");
            }
        }
        HighExprKind::Lambda { .. } => {
            emit_lambda(
                expr,
                indent,
                locals,
                declared_local_names,
                match_bindings,
                captures,
                env_local,
                function_names,
                abi,
                out,
            )?;
        }
        HighExprKind::Let { name, value, body } => {
            let binding = abi
                .wasm_type(&value.ty)?
                .map(|_| allocate_local_name(name, declared_local_names));
            emit_expr(
                value,
                indent,
                locals,
                declared_local_names,
                match_bindings,
                captures,
                env_local,
                function_names,
                abi,
                out,
            )?;
            if let Some(local_name) = &binding {
                out.push_str(&format!("{pad}local.set ${local_name}\n"));
            }
            let previous = locals.insert(name.clone(), binding);
            emit_expr(
                body,
                indent,
                locals,
                declared_local_names,
                match_bindings,
                captures,
                env_local,
                function_names,
                abi,
                out,
            )?;
            if let Some(previous) = previous {
                locals.insert(name.clone(), previous);
            } else {
                locals.remove(name);
            }
        }
        HighExprKind::Match { subject, arms } => {
            emit_match_arms(
                subject,
                arms,
                &expr.ty,
                indent,
                locals,
                declared_local_names,
                match_bindings,
                captures,
                env_local,
                function_names,
                abi,
                out,
            )?;
        }
        HighExprKind::Construct { variant, args } => {
            emit_construct(
                variant,
                args,
                &expr.ty,
                indent,
                locals,
                declared_local_names,
                match_bindings,
                captures,
                env_local,
                function_names,
                abi,
                out,
            )?;
        }
        HighExprKind::String(text) => {
            let _ = abi.wasm_type(&expr.ty)?;
            let offset = abi
                .string_table
                .offset_for(text)
                .ok_or_else(|| anyhow!("missing string literal in string table: {text}"))?;
            out.push_str(&format!("{pad}i32.const {offset}\n"));
        }
        HighExprKind::Error => {
            bail!("cannot codegen erroneous expression");
        }
    }
    Ok(())
}

fn emit_apply(
    expr: &HighExpr,
    args: &[HighExpr],
    indent: usize,
    locals: &mut HashMap<String, Option<String>>,
    declared_local_names: &mut HashSet<String>,
    match_bindings: &HashMap<String, MatchBinding>,
    captures: &HashMap<String, u32>,
    env_local: Option<&str>,
    function_names: &HashSet<String>,
    abi: &WasmAbi,
    out: &mut String,
) -> Result<()> {
    let [closure, arg] = args else {
        bail!("`__apply` expects exactly two arguments in wasm backend");
    };
    let signature = abi
        .closure_signature_for_apply(closure, expr)
        .ok_or_else(|| anyhow!("unsupported closure application shape in wasm backend"))?;
    emit_expr(
        closure,
        indent,
        locals,
        declared_local_names,
        match_bindings,
        captures,
        env_local,
        function_names,
        abi,
        out,
    )?;
    emit_expr(
        arg,
        indent,
        locals,
        declared_local_names,
        match_bindings,
        captures,
        env_local,
        function_names,
        abi,
        out,
    )?;
    let pad = "  ".repeat(indent);
    out.push_str(&format!(
        "{pad}call ${}\n",
        abi.apply_helper_name(signature.index)
    ));
    Ok(())
}

fn emit_list_literal(
    items: &[HighExpr],
    list_ty: &Type,
    indent: usize,
    locals: &mut HashMap<String, Option<String>>,
    declared_local_names: &mut HashSet<String>,
    match_bindings: &HashMap<String, MatchBinding>,
    captures: &HashMap<String, u32>,
    env_local: Option<&str>,
    function_names: &HashSet<String>,
    abi: &WasmAbi,
    out: &mut String,
) -> Result<()> {
    ensure_wasi_int_list_type(list_ty, abi)?;

    let pad = "  ".repeat(indent);
    let len = u32::try_from(items.len()).expect("list length fits into u32");
    let total_bytes = 8 + len * 4;

    out.push_str(&format!("{pad}global.get $heap_ptr\n"));
    out.push_str(&format!("{pad}i32.const {total_bytes}\n"));
    out.push_str(&format!("{pad}i32.add\n"));
    out.push_str(&format!("{pad}global.set $heap_ptr\n"));

    out.push_str(&format!("{pad}global.get $heap_ptr\n"));
    out.push_str(&format!("{pad}i32.const {total_bytes}\n"));
    out.push_str(&format!("{pad}i32.sub\n"));
    out.push_str(&format!("{pad}i32.const {len}\n"));
    out.push_str(&format!("{pad}i32.store\n"));

    out.push_str(&format!("{pad}global.get $heap_ptr\n"));
    out.push_str(&format!("{pad}i32.const {total_bytes}\n"));
    out.push_str(&format!("{pad}i32.sub\n"));
    out.push_str(&format!("{pad}global.get $heap_ptr\n"));
    out.push_str(&format!("{pad}i32.const {}\n", total_bytes - 8));
    out.push_str(&format!("{pad}i32.sub\n"));
    out.push_str(&format!("{pad}i32.store offset=4\n"));

    for (index, item) in items.iter().enumerate() {
        let item_offset = 8 + u32::try_from(index).expect("list index fits into u32") * 4;
        out.push_str(&format!("{pad}global.get $heap_ptr\n"));
        out.push_str(&format!("{pad}i32.const {}\n", total_bytes - item_offset));
        out.push_str(&format!("{pad}i32.sub\n"));
        emit_expr(
            item,
            indent,
            locals,
            declared_local_names,
            match_bindings,
            captures,
            env_local,
            function_names,
            abi,
            out,
        )?;
        out.push_str(&format!("{pad}i32.store\n"));
    }

    out.push_str(&format!("{pad}global.get $heap_ptr\n"));
    out.push_str(&format!("{pad}i32.const {total_bytes}\n"));
    out.push_str(&format!("{pad}i32.sub\n"));
    Ok(())
}

fn ensure_wasi_int_list_type(list_ty: &Type, abi: &WasmAbi) -> Result<()> {
    if abi.target != WasmTarget::Wasi {
        bail!("list expressions are not yet supported in wasm backend");
    }
    match list_ty {
        Type::List(item) if matches!(item.as_ref(), Type::Int) => Ok(()),
        Type::List(_) => bail!("only List<i64> is currently supported in wasm backend"),
        _ => bail!("expected list type in wasm backend"),
    }
}

fn emit_lambda(
    expr: &HighExpr,
    indent: usize,
    locals: &mut HashMap<String, Option<String>>,
    declared_local_names: &mut HashSet<String>,
    match_bindings: &HashMap<String, MatchBinding>,
    captures: &HashMap<String, u32>,
    env_local: Option<&str>,
    function_names: &HashSet<String>,
    abi: &WasmAbi,
    out: &mut String,
) -> Result<()> {
    let closure = abi
        .closure_for_expr(expr)
        .ok_or_else(|| anyhow!("missing closure abi for lambda expression"))?;
    for capture in &closure.captures {
        emit_expr(
            &HighExpr {
                kind: HighExprKind::Ident(capture.name.clone()),
                ty: capture.ty.clone(),
            },
            indent,
            locals,
            declared_local_names,
            match_bindings,
            captures,
            env_local,
            function_names,
            abi,
            out,
        )?;
    }
    let pad = "  ".repeat(indent);
    out.push_str(&format!("{pad}call ${}\n", closure.alloc_name));
    Ok(())
}

fn emit_construct(
    variant: &str,
    args: &[HighExpr],
    expr_ty: &Type,
    indent: usize,
    locals: &HashMap<String, Option<String>>,
    declared_local_names: &HashSet<String>,
    match_bindings: &HashMap<String, MatchBinding>,
    captures: &HashMap<String, u32>,
    env_local: Option<&str>,
    function_names: &HashSet<String>,
    abi: &WasmAbi,
    out: &mut String,
) -> Result<()> {
    let variant = abi.variant_layout_for_construct(variant, expr_ty)?;
    let pad = "  ".repeat(indent);
    if abi.target == WasmTarget::JavaScriptHost {
        if variant.field_count != 0 || !args.is_empty() {
            bail!("ADT payload fields are not yet supported in wasm backend");
        }
        out.push_str(&format!("{pad}i32.const {}\n", variant.tag));
        let _ = (
            locals,
            declared_local_names,
            match_bindings,
            captures,
            env_local,
            function_names,
        );
        return Ok(());
    }

    if variant.field_count != args.len() {
        bail!("constructor field count does not match payload arity in wasm backend");
    }
    let total_bytes = 4 + u32::try_from(variant.field_count).expect("field count fits") * 4;
    out.push_str(&format!("{pad}i32.const {total_bytes}\n"));
    out.push_str(&format!("{pad}call $__alloc\n"));
    out.push_str(&format!("{pad}drop\n"));
    out.push_str(&format!("{pad}global.get $heap_ptr\n"));
    out.push_str(&format!("{pad}i32.const {total_bytes}\n"));
    out.push_str(&format!("{pad}i32.sub\n"));
    out.push_str(&format!("{pad}i32.const {}\n", variant.tag));
    out.push_str(&format!("{pad}i32.store\n"));
    for (index, arg) in args.iter().enumerate() {
        let offset = 4 + u32::try_from(index).expect("field index fits") * 4;
        out.push_str(&format!("{pad}global.get $heap_ptr\n"));
        out.push_str(&format!("{pad}i32.const {}\n", total_bytes - offset));
        out.push_str(&format!("{pad}i32.sub\n"));
        emit_expr(
            arg,
            indent,
            &mut locals.clone(),
            &mut declared_local_names.clone(),
            match_bindings,
            captures,
            env_local,
            function_names,
            abi,
            out,
        )?;
        out.push_str(&format!("{pad}i32.store\n"));
    }
    out.push_str(&format!("{pad}global.get $heap_ptr\n"));
    out.push_str(&format!("{pad}i32.const {total_bytes}\n"));
    out.push_str(&format!("{pad}i32.sub\n"));
    Ok(())
}

fn emit_match_arms(
    subject: &HighExpr,
    arms: &[HighMatchArm],
    match_ty: &Type,
    indent: usize,
    locals: &mut HashMap<String, Option<String>>,
    declared_local_names: &mut HashSet<String>,
    match_bindings: &HashMap<String, MatchBinding>,
    captures: &HashMap<String, u32>,
    env_local: Option<&str>,
    function_names: &HashSet<String>,
    abi: &WasmAbi,
    out: &mut String,
) -> Result<()> {
    let (arm, rest) = arms
        .split_first()
        .ok_or_else(|| anyhow!("match lowering reached an empty arm list in wasm backend"))?;
    match &arm.pattern {
        Pattern::Wildcard => emit_expr(
            &arm.expr,
            indent,
            locals,
            declared_local_names,
            match_bindings,
            captures,
            env_local,
            function_names,
            abi,
            out,
        ),
        Pattern::Variant { name, bindings } => {
            let variant = abi.variant_layout_for_match(name, &subject.ty)?;
            if variant.field_count != bindings.len() {
                bail!("match pattern binding count does not match payload arity in wasm backend");
            }
            let mut arm_bindings = match_bindings.clone();
            for (index, binding_name) in bindings.iter().enumerate() {
                arm_bindings.insert(
                    binding_name.clone(),
                    MatchBinding {
                        subject: subject.clone(),
                        offset: 4 + u32::try_from(index).expect("field index fits") * 4,
                    },
                );
            }
            if rest.is_empty() {
                return emit_expr(
                    &arm.expr,
                    indent,
                    locals,
                    declared_local_names,
                    &arm_bindings,
                    captures,
                    env_local,
                    function_names,
                    abi,
                    out,
                );
            }

            let pad = "  ".repeat(indent);
            emit_expr(
                subject,
                indent,
                locals,
                declared_local_names,
                match_bindings,
                captures,
                env_local,
                function_names,
                abi,
                out,
            )?;
            if abi.target == WasmTarget::Wasi {
                out.push_str(&format!("{pad}i32.load\n"));
            }
            out.push_str(&format!("{pad}i32.const {}\n", variant.tag));
            out.push_str(&format!("{pad}i32.eq\n"));
            out.push_str(&format!("{pad}(if"));
            if let Some(result_ty) = abi.wasm_type(match_ty)? {
                out.push_str(&format!(" (result {result_ty})"));
            }
            out.push('\n');
            out.push_str(&format!("{pad}  (then\n"));
            emit_expr(
                &arm.expr,
                indent + 2,
                locals,
                declared_local_names,
                &arm_bindings,
                captures,
                env_local,
                function_names,
                abi,
                out,
            )?;
            out.push_str(&format!("{pad}  )\n"));
            out.push_str(&format!("{pad}  (else\n"));
            emit_match_arms(
                subject,
                rest,
                match_ty,
                indent + 2,
                locals,
                declared_local_names,
                match_bindings,
                captures,
                env_local,
                function_names,
                abi,
                out,
            )?;
            out.push_str(&format!("{pad}  )\n"));
            out.push_str(&format!("{pad})\n"));
            Ok(())
        }
    }
}

fn collect_let_locals(
    expr: &HighExpr,
    abi: &WasmAbi,
    declared_local_names: &mut HashSet<String>,
    locals: &mut Vec<(String, &'static str)>,
) -> Result<()> {
    match &expr.kind {
        HighExprKind::Let { name, value, body } => {
            if let Some(local_ty) = abi.wasm_type(&value.ty)? {
                locals.push((allocate_local_name(name, declared_local_names), local_ty));
            }
            collect_let_locals(value, abi, declared_local_names, locals)?;
            collect_let_locals(body, abi, declared_local_names, locals)?;
        }
        HighExprKind::Binary { left, right, .. } => {
            collect_let_locals(left, abi, declared_local_names, locals)?;
            collect_let_locals(right, abi, declared_local_names, locals)?;
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_let_locals(condition, abi, declared_local_names, locals)?;
            collect_let_locals(then_branch, abi, declared_local_names, locals)?;
            collect_let_locals(else_branch, abi, declared_local_names, locals)?;
        }
        HighExprKind::Call { args, .. } | HighExprKind::Construct { args, .. } => {
            for arg in args {
                collect_let_locals(arg, abi, declared_local_names, locals)?;
            }
        }
        HighExprKind::Match { subject, arms } => {
            collect_let_locals(subject, abi, declared_local_names, locals)?;
            for arm in arms {
                collect_let_locals(&arm.expr, abi, declared_local_names, locals)?;
            }
        }
        HighExprKind::List(items) | HighExprKind::Tuple(items) => {
            for item in items {
                collect_let_locals(item, abi, declared_local_names, locals)?;
            }
        }
        HighExprKind::Lambda { .. } => {}
        HighExprKind::Int(_)
        | HighExprKind::Bool(_)
        | HighExprKind::String(_)
        | HighExprKind::Ident(_)
        | HighExprKind::Error => {}
    }
    Ok(())
}

fn allocate_local_name(name: &str, declared_local_names: &mut HashSet<String>) -> String {
    if declared_local_names.insert(name.to_owned()) {
        return name.to_owned();
    }

    let mut suffix = 1usize;
    loop {
        let candidate = format!("{name}__{suffix}");
        if declared_local_names.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}

#[derive(Clone, Debug)]
struct ClosureCapture {
    name: String,
    ty: Type,
    offset: u32,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ClosureSignatureKey {
    arg: Type,
    result: Type,
}

#[derive(Clone, Debug)]
struct ClosureSignature {
    index: usize,
    key: ClosureSignatureKey,
}

#[derive(Clone, Debug)]
struct ClosureThunk {
    expr_id: usize,
    func_name: String,
    alloc_name: String,
    signature_index: usize,
    table_index: u32,
    param_name: String,
    body: HighExpr,
    captures: Vec<ClosureCapture>,
    env_size: u32,
}

#[derive(Clone, Debug)]
struct NamedCallbackThunk {
    func_name: String,
    alloc_name: String,
    signature_index: usize,
    table_index: u32,
    target: NamedCallbackTarget,
}

#[derive(Clone, Debug)]
enum NamedCallbackTarget {
    Function(String),
    BuiltinString,
}

#[derive(Debug)]
struct WasmAbi {
    target: WasmTarget,
    string_table: StringTable,
    named_types: HashMap<String, NamedTypeAbi>,
    variants: HashMap<String, VariantLayout>,
    uses_console_println: bool,
    uses_string_builtin: bool,
    uses_list_runtime: bool,
    uses_adt_runtime: bool,
    uses_range_inclusive: bool,
    uses_map_builtin: bool,
    uses_filter_builtin: bool,
    uses_sum_builtin: bool,
    uses_join_builtin: bool,
    scratch_base: u32,
    closure_signatures: Vec<ClosureSignature>,
    closure_signature_ids: HashMap<ClosureSignatureKey, usize>,
    closure_thunks: Vec<ClosureThunk>,
    closure_expr_map: HashMap<usize, usize>,
    named_callback_thunks: Vec<NamedCallbackThunk>,
    named_callback_map: HashMap<String, usize>,
}

impl WasmAbi {
    fn from_module(module: &HighModule, target: WasmTarget) -> Result<Self> {
        let mut named_types = HashMap::new();
        let mut variants = HashMap::new();

        for type_decl in &module.types {
            named_types.insert(
                type_decl.name.clone(),
                if target == WasmTarget::Wasi {
                    NamedTypeAbi::HeapEnum
                } else if type_decl
                    .variants
                    .iter()
                    .all(|variant| variant.fields.is_empty())
                {
                    NamedTypeAbi::FieldlessEnum
                } else {
                    NamedTypeAbi::Unsupported
                },
            );
            for (tag, variant) in type_decl.variants.iter().enumerate() {
                variants.insert(
                    variant.name.clone(),
                    VariantLayout {
                        owner_type: type_decl.name.clone(),
                        tag: u32::try_from(tag).expect("variant tag fits into u32"),
                        field_count: variant.fields.len(),
                    },
                );
            }
        }

        let string_table = StringTable::collect(module);

        let mut uses_console_println = false;
        let mut uses_string_builtin = false;
        let mut uses_list_runtime = false;
        let mut uses_adt_runtime = false;
        let mut uses_range_inclusive = false;
        let mut uses_map_builtin = false;
        let mut uses_filter_builtin = false;
        let mut uses_sum_builtin = false;
        let mut uses_join_builtin = false;
        if target == WasmTarget::Wasi {
            for function in &module.functions {
                scan_expr(
                    &function.body,
                    &mut uses_console_println,
                    &mut uses_string_builtin,
                    &mut uses_list_runtime,
                    &mut uses_adt_runtime,
                    &mut uses_range_inclusive,
                    &mut uses_map_builtin,
                    &mut uses_filter_builtin,
                    &mut uses_sum_builtin,
                    &mut uses_join_builtin,
                );
            }
        }

        let scratch_base = {
            let base = string_table.next_offset;
            (base + 3) & !3
        };

        let mut abi = Self {
            target,
            string_table,
            named_types,
            variants,
            uses_console_println,
            uses_string_builtin,
            uses_list_runtime,
            uses_adt_runtime,
            uses_range_inclusive,
            uses_map_builtin,
            uses_filter_builtin,
            uses_sum_builtin,
            uses_join_builtin,
            scratch_base,
            closure_signatures: Vec::new(),
            closure_signature_ids: HashMap::new(),
            closure_thunks: Vec::new(),
            closure_expr_map: HashMap::new(),
            named_callback_thunks: Vec::new(),
            named_callback_map: HashMap::new(),
        };
        abi.collect_closures(module)?;
        abi.collect_named_callbacks(module)?;
        Ok(abi)
    }

    fn needs_scratch(&self) -> bool {
        self.target == WasmTarget::Wasi && (self.uses_console_println || self.uses_string_builtin)
    }

    fn iovec_base(&self) -> u32 {
        self.scratch_base
    }

    fn nwritten_base(&self) -> u32 {
        self.scratch_base + 8
    }

    fn newline_base(&self) -> u32 {
        self.scratch_base + 12
    }

    fn str_buf_end(&self) -> u32 {
        self.scratch_base + 28
    }

    fn heap_base(&self) -> u32 {
        let base = if self.needs_scratch() {
            self.scratch_base + 28
        } else {
            self.string_table.next_offset
        };
        (base + 3) & !3
    }

    fn needs_heap(&self) -> bool {
        !self.closure_thunks.is_empty()
            || !self.named_callback_thunks.is_empty()
            || self.uses_list_runtime
            || self.uses_adt_runtime
            || self.uses_range_inclusive
            || self.uses_map_builtin
            || self.uses_filter_builtin
            || self.uses_sum_builtin
            || self.uses_join_builtin
            || self.uses_string_builtin
    }

    fn memory_end(&self) -> u32 {
        if self.needs_heap() {
            self.heap_base()
        } else if self.needs_scratch() {
            self.scratch_base + 28
        } else {
            self.string_table.next_offset
        }
    }

    fn needs_memory(&self) -> bool {
        !self.string_table.is_empty() || self.needs_scratch() || self.needs_heap()
    }

    fn variant_layout(&self, name: &str) -> Option<&VariantLayout> {
        self.variants.get(name)
    }

    fn variant_layout_for_construct(&self, variant: &str, expr_ty: &Type) -> Result<VariantLayout> {
        match expr_ty {
            Type::Result(_, _) => result_variant_layout(variant)
                .ok_or_else(|| anyhow!("unknown Result variant in wasm backend: {variant}")),
            Type::Named(owner_type) => {
                let layout = self
                    .variant_layout(variant)
                    .ok_or_else(|| anyhow!("unknown variant in wasm backend: {variant}"))?;
                if layout.owner_type != *owner_type {
                    bail!(
                        "constructor variant does not match the declared result type in wasm backend"
                    );
                }
                Ok(layout.clone())
            }
            _ => bail!("constructors are not yet supported for this type in wasm backend"),
        }
    }

    fn variant_layout_for_match(&self, variant: &str, subject_ty: &Type) -> Result<VariantLayout> {
        match subject_ty {
            Type::Result(_, _) => result_variant_layout(variant)
                .ok_or_else(|| anyhow!("unknown Result pattern in wasm backend: {variant}")),
            Type::Named(owner_type) => {
                let layout = self
                    .variant_layout(variant)
                    .ok_or_else(|| anyhow!("unknown variant in wasm backend: {variant}"))?;
                if layout.owner_type != *owner_type {
                    bail!("match pattern variant does not match the subject type in wasm backend");
                }
                Ok(layout.clone())
            }
            _ => bail!(
                "match is not yet supported in wasm backend except for Result and user-defined ADTs"
            ),
        }
    }

    fn closure_for_expr(&self, expr: &HighExpr) -> Option<&ClosureThunk> {
        let expr_id = expr as *const HighExpr as usize;
        self.closure_expr_map
            .get(&expr_id)
            .and_then(|index| self.closure_thunks.get(*index))
    }

    fn named_callback_for(&self, name: &str) -> Option<&NamedCallbackThunk> {
        self.named_callback_map
            .get(name)
            .and_then(|index| self.named_callback_thunks.get(*index))
    }

    fn closure_signature_for_apply(
        &self,
        closure: &HighExpr,
        result_expr: &HighExpr,
    ) -> Option<&ClosureSignature> {
        match &closure.kind {
            HighExprKind::Lambda { .. } => {
                let thunk = self.closure_for_expr(closure)?;
                return self.closure_signatures.get(thunk.signature_index);
            }
            HighExprKind::Ident(name) => {
                if let Some(thunk) = self.named_callback_for(name) {
                    return self.closure_signatures.get(thunk.signature_index);
                }
            }
            _ => {}
        }
        let key = match &closure.ty {
            Type::Fn(arg, result) => ClosureSignatureKey {
                arg: (**arg).clone(),
                result: (**result).clone(),
            },
            _ if matches!(result_expr.ty, Type::Unknown) => return None,
            _ => return None,
        };
        self.closure_signature_ids
            .get(&key)
            .and_then(|index| self.closure_signatures.get(*index))
    }

    fn closure_type_name(&self, index: usize) -> String {
        format!("__closure_type_{index}")
    }

    fn apply_helper_name(&self, index: usize) -> String {
        format!("__apply_closure_{index}")
    }

    fn map_helper_name(&self, index: usize) -> String {
        format!("__list_map_{index}")
    }

    fn filter_helper_name(&self, index: usize) -> String {
        format!("__list_filter_{index}")
    }

    fn sum_helper_name(&self) -> &'static str {
        "__list_sum_i64"
    }

    fn wasm_type(&self, ty: &Type) -> Result<Option<&'static str>> {
        match ty {
            Type::Unit => Ok(None),
            Type::Int | Type::Bool | Type::Fn(_, _) => Ok(Some("i32")),
            Type::String if self.target == WasmTarget::JavaScriptHost => Ok(Some("i32")),
            Type::String if self.target == WasmTarget::Wasi => Ok(Some("i32")),
            Type::List(_) if self.target == WasmTarget::Wasi => Ok(Some("i32")),
            Type::Result(_, _) if self.target == WasmTarget::Wasi => Ok(Some("i32")),
            Type::Named(name) => match self.named_types.get(name) {
                Some(NamedTypeAbi::FieldlessEnum) => Ok(Some("i32")),
                Some(NamedTypeAbi::HeapEnum) => Ok(Some("i32")),
                Some(NamedTypeAbi::Unsupported) => {
                    bail!("ADT payload fields are not yet supported in wasm backend")
                }
                None => bail!("unsupported wasm type: {ty}"),
            },
            other => bail!("unsupported wasm type: {other}"),
        }
    }

    fn collect_closures(&mut self, module: &HighModule) -> Result<()> {
        for function in &module.functions {
            let mut scope = HashMap::new();
            for param in &function.params {
                scope.insert(param.name.clone(), param.ty.clone());
            }
            self.collect_closures_in_expr(&function.body, Some(&function.return_type), &scope)?;
        }
        Ok(())
    }

    fn collect_closures_in_expr(
        &mut self,
        expr: &HighExpr,
        expected: Option<&Type>,
        scope: &HashMap<String, Type>,
    ) -> Result<()> {
        match &expr.kind {
            HighExprKind::Lambda { param, body } => {
                let Some((arg_ty, result_ty)) = closure_signature_types(expected, &expr.ty) else {
                    bail!("lambda requires a known Fn<A, B> type in wasm backend")
                };
                let signature_index = self.intern_closure_signature(&arg_ty, &result_ty);
                let capture_names = ordered_captures(body, scope, param);
                let mut captures = Vec::new();
                for (index, name) in capture_names.into_iter().enumerate() {
                    let ty = scope
                        .get(&name)
                        .cloned()
                        .ok_or_else(|| anyhow!("missing capture type for `{name}`"))?;
                    captures.push(ClosureCapture {
                        name,
                        ty,
                        offset: 4 + u32::try_from(index).expect("capture index fits in u32") * 4,
                    });
                }
                for capture in &captures {
                    let _ = self.wasm_type(&capture.ty)?;
                }
                let _ = self.wasm_type(&arg_ty)?;
                let _ = self.wasm_type(&result_ty)?;
                let thunk_index = self.closure_thunks.len();
                let closure = ClosureThunk {
                    expr_id: expr as *const HighExpr as usize,
                    func_name: format!("__closure_thunk_{thunk_index}"),
                    alloc_name: format!("__closure_alloc_{thunk_index}"),
                    signature_index,
                    table_index: u32::try_from(thunk_index).expect("closure table index fits"),
                    param_name: param.clone(),
                    body: (**body).clone(),
                    env_size: 4 + u32::try_from(captures.len()).expect("capture count fits") * 4,
                    captures,
                };
                self.closure_expr_map.insert(closure.expr_id, thunk_index);
                self.closure_thunks.push(closure);

                let mut inner_scope = scope.clone();
                inner_scope.insert(param.clone(), arg_ty.clone());
                self.collect_closures_in_expr(body, Some(&result_ty), &inner_scope)?;
            }
            HighExprKind::Let { name, value, body } => {
                self.collect_closures_in_expr(value, None, scope)?;
                let mut body_scope = scope.clone();
                body_scope.insert(name.clone(), value.ty.clone());
                self.collect_closures_in_expr(body, expected, &body_scope)?;
            }
            HighExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_closures_in_expr(condition, None, scope)?;
                self.collect_closures_in_expr(then_branch, Some(&expr.ty), scope)?;
                self.collect_closures_in_expr(else_branch, Some(&expr.ty), scope)?;
            }
            HighExprKind::Match { subject, arms } => {
                self.collect_closures_in_expr(subject, None, scope)?;
                for arm in arms {
                    self.collect_closures_in_expr(&arm.expr, Some(&expr.ty), scope)?;
                }
            }
            HighExprKind::Binary { left, right, .. } => {
                self.collect_closures_in_expr(left, None, scope)?;
                self.collect_closures_in_expr(right, None, scope)?;
            }
            HighExprKind::Call { callee, args } => {
                if callee == "__apply" {
                    if let [callback, arg] = &args[..] {
                        let expected =
                            Type::Fn(Box::new(arg.ty.clone()), Box::new(expr.ty.clone()));
                        self.collect_closures_in_expr(callback, Some(&expected), scope)?;
                        self.collect_closures_in_expr(arg, None, scope)?;
                    }
                } else if callee == "map" {
                    if let [receiver, callback] = &args[..] {
                        self.collect_closures_in_expr(receiver, None, scope)?;
                        if let Some(input_ty) = list_item_type(&receiver.ty) {
                            let output_ty =
                                callback_result_type(callback, scope).unwrap_or(Type::Unknown);
                            let expected = Type::Fn(Box::new(input_ty), Box::new(output_ty));
                            self.collect_closures_in_expr(callback, Some(&expected), scope)?;
                        } else {
                            self.collect_closures_in_expr(callback, None, scope)?;
                        }
                    }
                } else if callee == "filter" {
                    if let [receiver, callback] = &args[..] {
                        self.collect_closures_in_expr(receiver, None, scope)?;
                        if let Some(input_ty) = list_item_type(&receiver.ty) {
                            let expected = Type::Fn(Box::new(input_ty), Box::new(Type::Bool));
                            self.collect_closures_in_expr(callback, Some(&expected), scope)?;
                        } else {
                            self.collect_closures_in_expr(callback, None, scope)?;
                        }
                    }
                } else {
                    for arg in args {
                        self.collect_closures_in_expr(arg, None, scope)?;
                    }
                }
            }
            HighExprKind::Construct { args, .. } => {
                for arg in args {
                    self.collect_closures_in_expr(arg, None, scope)?;
                }
            }
            HighExprKind::List(items) | HighExprKind::Tuple(items) => {
                for item in items {
                    self.collect_closures_in_expr(item, None, scope)?;
                }
            }
            HighExprKind::Int(_)
            | HighExprKind::Bool(_)
            | HighExprKind::String(_)
            | HighExprKind::Ident(_)
            | HighExprKind::Error => {}
        }
        Ok(())
    }

    fn intern_closure_signature(&mut self, arg: &Type, result: &Type) -> usize {
        let key = ClosureSignatureKey {
            arg: arg.clone(),
            result: result.clone(),
        };
        if let Some(index) = self.closure_signature_ids.get(&key) {
            return *index;
        }
        let index = self.closure_signatures.len();
        self.closure_signatures.push(ClosureSignature {
            index,
            key: key.clone(),
        });
        self.closure_signature_ids.insert(key, index);
        index
    }

    fn collect_named_callbacks(&mut self, module: &HighModule) -> Result<()> {
        for function in &module.functions {
            self.collect_named_callbacks_in_expr(&function.body, module)?;
        }
        Ok(())
    }

    fn collect_named_callbacks_in_expr(
        &mut self,
        expr: &HighExpr,
        module: &HighModule,
    ) -> Result<()> {
        match &expr.kind {
            HighExprKind::Call { callee, args } => {
                if callee == "__apply" {
                    if let [callback, arg] = &args[..] {
                        if let HighExprKind::Ident(name) = &callback.kind {
                            self.intern_named_apply_callback(name, arg, module)?;
                        }
                        self.collect_named_callbacks_in_expr(callback, module)?;
                        self.collect_named_callbacks_in_expr(arg, module)?;
                    }
                } else if callee == "map" || callee == "filter" {
                    if let [receiver, callback] = &args[..] {
                        if let HighExprKind::Ident(name) = &callback.kind {
                            self.intern_named_callback(name, receiver, callee, module)?;
                        }
                        self.collect_named_callbacks_in_expr(receiver, module)?;
                        self.collect_named_callbacks_in_expr(callback, module)?;
                    }
                } else {
                    for arg in args {
                        self.collect_named_callbacks_in_expr(arg, module)?;
                    }
                }
            }
            HighExprKind::Binary { left, right, .. } => {
                self.collect_named_callbacks_in_expr(left, module)?;
                self.collect_named_callbacks_in_expr(right, module)?;
            }
            HighExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_named_callbacks_in_expr(condition, module)?;
                self.collect_named_callbacks_in_expr(then_branch, module)?;
                self.collect_named_callbacks_in_expr(else_branch, module)?;
            }
            HighExprKind::Match { subject, arms } => {
                self.collect_named_callbacks_in_expr(subject, module)?;
                for arm in arms {
                    self.collect_named_callbacks_in_expr(&arm.expr, module)?;
                }
            }
            HighExprKind::Construct { args, .. }
            | HighExprKind::List(args)
            | HighExprKind::Tuple(args) => {
                for arg in args {
                    self.collect_named_callbacks_in_expr(arg, module)?;
                }
            }
            HighExprKind::Lambda { body, .. } => {
                self.collect_named_callbacks_in_expr(body, module)?;
            }
            HighExprKind::Let { value, body, .. } => {
                self.collect_named_callbacks_in_expr(value, module)?;
                self.collect_named_callbacks_in_expr(body, module)?;
            }
            HighExprKind::Int(_)
            | HighExprKind::Bool(_)
            | HighExprKind::String(_)
            | HighExprKind::Ident(_)
            | HighExprKind::Error => {}
        }
        Ok(())
    }

    fn intern_named_callback(
        &mut self,
        name: &str,
        receiver: &HighExpr,
        callee: &str,
        module: &HighModule,
    ) -> Result<()> {
        if self.named_callback_map.contains_key(name) {
            return Ok(());
        }

        let Some(input_ty) = list_item_type(&receiver.ty) else {
            return Ok(());
        };

        let (target, result_ty) = if name == "string" {
            (NamedCallbackTarget::BuiltinString, Type::String)
        } else if let Some(function) = module
            .functions
            .iter()
            .find(|function| function.name == name)
        {
            if function.params.len() != 1 {
                bail!(
                    "only unary named function references are supported in wasm backend callbacks: {name}"
                );
            }
            if matches!(function.return_type, Type::Fn(_, _)) {
                bail!(
                    "higher-order function references are not yet supported in wasm backend callbacks: {name}"
                );
            }
            (
                NamedCallbackTarget::Function(name.to_owned()),
                function.return_type.clone(),
            )
        } else {
            return Ok(());
        };

        let result_ty = if callee == "filter" {
            Type::Bool
        } else {
            result_ty
        };
        let signature_index = self.intern_closure_signature(&input_ty, &result_ty);
        let thunk_index = self.closure_thunks.len() + self.named_callback_thunks.len();
        let thunk = NamedCallbackThunk {
            func_name: format!("__named_callback_thunk_{thunk_index}"),
            alloc_name: format!("__named_callback_alloc_{thunk_index}"),
            signature_index,
            table_index: u32::try_from(thunk_index).expect("callback table index fits"),
            target,
        };
        let index = self.named_callback_thunks.len();
        self.named_callback_map.insert(name.to_owned(), index);
        self.named_callback_thunks.push(thunk);
        Ok(())
    }

    fn intern_named_apply_callback(
        &mut self,
        name: &str,
        arg: &HighExpr,
        module: &HighModule,
    ) -> Result<()> {
        if self.named_callback_map.contains_key(name) {
            return Ok(());
        }

        let (target, result_ty) = if name == "string" {
            (NamedCallbackTarget::BuiltinString, Type::String)
        } else if let Some(function) = module
            .functions
            .iter()
            .find(|function| function.name == name)
        {
            if function.params.len() != 1 {
                bail!(
                    "only unary named function references are supported in wasm backend callbacks: {name}"
                );
            }
            if matches!(function.return_type, Type::Fn(_, _)) {
                bail!(
                    "higher-order function references are not yet supported in wasm backend callbacks: {name}"
                );
            }
            (
                NamedCallbackTarget::Function(name.to_owned()),
                function.return_type.clone(),
            )
        } else {
            return Ok(());
        };

        let signature_index = self.intern_closure_signature(&arg.ty, &result_ty);
        let thunk_index = self.closure_thunks.len() + self.named_callback_thunks.len();
        let thunk = NamedCallbackThunk {
            func_name: format!("__named_callback_thunk_{thunk_index}"),
            alloc_name: format!("__named_callback_alloc_{thunk_index}"),
            signature_index,
            table_index: u32::try_from(thunk_index).expect("callback table index fits"),
            target,
        };
        let index = self.named_callback_thunks.len();
        self.named_callback_map.insert(name.to_owned(), index);
        self.named_callback_thunks.push(thunk);
        Ok(())
    }
}

fn emit_closure_support(
    abi: &WasmAbi,
    function_names: &HashSet<String>,
    out: &mut String,
) -> Result<()> {
    if !abi.needs_heap() {
        return Ok(());
    }

    for signature in &abi.closure_signatures {
        out.push_str(&format!(
            "  (type ${} (func (param i32",
            abi.closure_type_name(signature.index)
        ));
        if let Some(arg_ty) = abi.wasm_type(&signature.key.arg)? {
            out.push_str(&format!(" {arg_ty}"));
        }
        out.push(')');
        if let Some(result_ty) = abi.wasm_type(&signature.key.result)? {
            out.push_str(&format!(" (result {result_ty})"));
        }
        out.push_str("))\n");
    }

    let table_size = abi.closure_thunks.len() + abi.named_callback_thunks.len();
    if table_size > 0 {
        out.push_str(&format!("  (table {} funcref)\n", table_size));
        out.push_str("  (elem (i32.const 0) func");
        for closure in &abi.closure_thunks {
            out.push_str(&format!(" ${}", closure.func_name));
        }
        for callback in &abi.named_callback_thunks {
            out.push_str(&format!(" ${}", callback.func_name));
        }
        out.push_str(")\n");
    }
    out.push_str(&format!(
        "  (global $heap_ptr (mut i32) (i32.const {}))\n",
        abi.heap_base()
    ));

    for closure in &abi.closure_thunks {
        emit_closure_allocator(abi, closure, out)?;
    }
    for callback in &abi.named_callback_thunks {
        emit_named_callback_allocator(out, callback);
    }
    for signature in &abi.closure_signatures {
        emit_apply_helper(abi, signature, out)?;
        if abi.uses_map_builtin {
            emit_map_helper(abi, signature, out)?;
        }
        if abi.uses_filter_builtin {
            emit_filter_helper(abi, signature, out)?;
        }
    }
    for closure in &abi.closure_thunks {
        emit_closure_thunk(abi, closure, function_names, out)?;
    }
    for callback in &abi.named_callback_thunks {
        emit_named_callback_thunk(callback, out);
    }
    if abi.uses_range_inclusive {
        emit_range_inclusive_helper(out);
    }
    if abi.uses_join_builtin {
        emit_join_helper(out);
    }
    if abi.uses_sum_builtin {
        emit_sum_helper(out);
    }

    Ok(())
}

fn emit_alloc_helper(out: &mut String) {
    out.push_str("  (func $__alloc (param $size i32) (result i32)\n");
    out.push_str("    (local $ptr i32)\n");
    out.push_str("    (local $aligned i32)\n");
    out.push_str("    global.get $heap_ptr\n");
    out.push_str("    local.set $ptr\n");
    out.push_str("    local.get $size\n");
    out.push_str("    i32.const 3\n");
    out.push_str("    i32.add\n");
    out.push_str("    i32.const -4\n");
    out.push_str("    i32.and\n");
    out.push_str("    local.set $aligned\n");
    out.push_str("    local.get $ptr\n");
    out.push_str("    local.get $aligned\n");
    out.push_str("    i32.add\n");
    out.push_str("    global.set $heap_ptr\n");
    out.push_str("    local.get $ptr\n");
    out.push_str("  )\n");
}

fn emit_strlen_helper(out: &mut String) {
    out.push_str("  (func $__strlen (param $ptr i32) (result i32)\n");
    out.push_str("    (local $cur i32)\n");
    out.push_str("    local.get $ptr\n");
    out.push_str("    local.set $cur\n");
    out.push_str("    (block $break\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $cur\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.eqz\n");
    out.push_str("        br_if $break\n");
    out.push_str("        local.get $cur\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $cur\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $cur\n");
    out.push_str("    local.get $ptr\n");
    out.push_str("    i32.sub\n");
    out.push_str("  )\n");
}

fn emit_memcpy_helper(out: &mut String) {
    out.push_str("  (func $__memcpy (param $dst i32) (param $src i32) (param $len i32)\n");
    out.push_str("    (local $i i32)\n");
    out.push_str("    (block $break\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $i\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $break\n");
    out.push_str("        local.get $dst\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $src\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.store8\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $i\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("  )\n");
}

fn emit_closure_allocator(abi: &WasmAbi, closure: &ClosureThunk, out: &mut String) -> Result<()> {
    out.push_str(&format!("  (func ${}", closure.alloc_name));
    for capture in &closure.captures {
        let wasm_ty = abi
            .wasm_type(&capture.ty)?
            .ok_or_else(|| anyhow!("unsupported unit capture in wasm backend"))?;
        out.push_str(&format!(" (param ${} {wasm_ty})", capture.name));
    }
    out.push_str(" (result i32)\n");
    out.push_str("    (local $ptr i32)\n");
    out.push_str("    global.get $heap_ptr\n");
    out.push_str("    local.set $ptr\n");
    out.push_str("    local.get $ptr\n");
    out.push_str(&format!("    i32.const {}\n", closure.env_size));
    out.push_str("    i32.add\n");
    out.push_str("    global.set $heap_ptr\n");
    out.push_str("    local.get $ptr\n");
    out.push_str(&format!("    i32.const {}\n", closure.table_index));
    out.push_str("    i32.store\n");
    for capture in &closure.captures {
        out.push_str("    local.get $ptr\n");
        out.push_str(&format!("    local.get ${}\n", capture.name));
        out.push_str(&format!("    i32.store offset={}\n", capture.offset));
    }
    out.push_str("    local.get $ptr\n");
    out.push_str("  )\n");
    Ok(())
}

fn emit_named_callback_allocator(out: &mut String, callback: &NamedCallbackThunk) {
    out.push_str(&format!("  (func ${} (result i32)\n", callback.alloc_name));
    out.push_str("    (local $ptr i32)\n");
    out.push_str("    i32.const 4\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $ptr\n");
    out.push_str("    local.get $ptr\n");
    out.push_str(&format!("    i32.const {}\n", callback.table_index));
    out.push_str("    i32.store\n");
    out.push_str("    local.get $ptr\n");
    out.push_str("  )\n");
}

fn emit_apply_helper(abi: &WasmAbi, signature: &ClosureSignature, out: &mut String) -> Result<()> {
    let arg_ty = abi
        .wasm_type(&signature.key.arg)?
        .ok_or_else(|| anyhow!("unsupported closure argument type in wasm backend"))?;
    out.push_str(&format!(
        "  (func ${} (param $closure i32) (param $arg {arg_ty})",
        abi.apply_helper_name(signature.index)
    ));
    if let Some(result_ty) = abi.wasm_type(&signature.key.result)? {
        out.push_str(&format!(" (result {result_ty})"));
    }
    out.push('\n');
    out.push_str("    local.get $closure\n");
    out.push_str("    local.get $arg\n");
    out.push_str("    local.get $closure\n");
    out.push_str("    i32.load\n");
    out.push_str(&format!(
        "    call_indirect (type ${})\n",
        abi.closure_type_name(signature.index)
    ));
    out.push_str("  )\n");
    Ok(())
}

fn emit_map_helper(abi: &WasmAbi, signature: &ClosureSignature, out: &mut String) -> Result<()> {
    let _ = abi
        .wasm_type(&signature.key.arg)?
        .ok_or_else(|| anyhow!("unsupported map argument type in wasm backend"))?;
    out.push_str(&format!(
        "  (func ${} (param $list i32) (param $callback i32) (result i32)\n",
        abi.map_helper_name(signature.index)
    ));
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $in_items i32)\n");
    out.push_str("    (local $out_items i32)\n");
    out.push_str("    (local $out_list i32)\n");
    out.push_str("    (local $index i32)\n");
    out.push_str("    (local $value i32)\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load\n");
    out.push_str("    local.set $len\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load offset=4\n");
    out.push_str("    local.set $in_items\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.const 4\n");
    out.push_str("    i32.mul\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $out_items\n");
    out.push_str("    (block $break\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $index\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $break\n");
    out.push_str("        local.get $in_items\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load\n");
    out.push_str("        local.set $value\n");
    out.push_str("        local.get $out_items\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $callback\n");
    out.push_str("        local.get $value\n");
    out.push_str(&format!(
        "        call ${}\n",
        abi.apply_helper_name(signature.index)
    ));
    out.push_str("        i32.store\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 8\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $out_list\n");
    out.push_str("    local.get $out_list\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.store\n");
    out.push_str("    local.get $out_list\n");
    out.push_str("    local.get $out_items\n");
    out.push_str("    i32.store offset=4\n");
    out.push_str("    local.get $out_list\n");
    out.push_str("  )\n");
    Ok(())
}

fn emit_filter_helper(abi: &WasmAbi, signature: &ClosureSignature, out: &mut String) -> Result<()> {
    let arg_ty = abi
        .wasm_type(&signature.key.arg)?
        .ok_or_else(|| anyhow!("unsupported filter argument type in wasm backend"))?;
    if arg_ty != "i32" || signature.key.result != Type::Bool {
        return Ok(());
    }

    out.push_str(&format!(
        "  (func ${} (param $list i32) (param $callback i32) (result i32)\n",
        abi.filter_helper_name(signature.index)
    ));
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $in_items i32)\n");
    out.push_str("    (local $out_items i32)\n");
    out.push_str("    (local $out_list i32)\n");
    out.push_str("    (local $index i32)\n");
    out.push_str("    (local $out_len i32)\n");
    out.push_str("    (local $value i32)\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load\n");
    out.push_str("    local.set $len\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load offset=4\n");
    out.push_str("    local.set $in_items\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.const 4\n");
    out.push_str("    i32.mul\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $out_items\n");
    out.push_str("    (block $break\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $index\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $break\n");
    out.push_str("        local.get $in_items\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load\n");
    out.push_str("        local.set $value\n");
    out.push_str("        local.get $callback\n");
    out.push_str("        local.get $value\n");
    out.push_str(&format!(
        "        call ${}\n",
        abi.apply_helper_name(signature.index)
    ));
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            local.get $out_items\n");
    out.push_str("            local.get $out_len\n");
    out.push_str("            i32.const 4\n");
    out.push_str("            i32.mul\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.get $value\n");
    out.push_str("            i32.store\n");
    out.push_str("            local.get $out_len\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $out_len\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 8\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $out_list\n");
    out.push_str("    local.get $out_list\n");
    out.push_str("    local.get $out_len\n");
    out.push_str("    i32.store\n");
    out.push_str("    local.get $out_list\n");
    out.push_str("    local.get $out_items\n");
    out.push_str("    i32.store offset=4\n");
    out.push_str("    local.get $out_list\n");
    out.push_str("  )\n");
    Ok(())
}

fn emit_named_callback_thunk(callback: &NamedCallbackThunk, out: &mut String) {
    out.push_str(&format!(
        "  (func ${} (param $env i32) (param $value i32) (result i32)\n",
        callback.func_name
    ));
    match &callback.target {
        NamedCallbackTarget::Function(name) => {
            out.push_str("    local.get $value\n");
            out.push_str(&format!("    call ${name}\n"));
        }
        NamedCallbackTarget::BuiltinString => {
            out.push_str("    local.get $value\n");
            out.push_str("    call $string\n");
        }
    }
    out.push_str("  )\n");
}

fn emit_sum_helper(out: &mut String) {
    out.push_str("  (func $__list_sum_i64 (param $list i32) (result i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $items i32)\n");
    out.push_str("    (local $index i32)\n");
    out.push_str("    (local $sum i32)\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load\n");
    out.push_str("    local.set $len\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load offset=4\n");
    out.push_str("    local.set $items\n");
    out.push_str("    (block $break\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $index\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $break\n");
    out.push_str("        local.get $sum\n");
    out.push_str("        local.get $items\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $sum\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $sum\n");
    out.push_str("  )\n");
}

fn emit_range_inclusive_helper(out: &mut String) {
    out.push_str("  (func $__range_inclusive (param $start i32) (param $end i32) (result i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $items i32)\n");
    out.push_str("    (local $list i32)\n");
    out.push_str("    (local $index i32)\n");
    out.push_str("    local.get $end\n");
    out.push_str("    local.get $start\n");
    out.push_str("    i32.lt_s\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        local.set $len\n");
    out.push_str("      )\n");
    out.push_str("      (else\n");
    out.push_str("        local.get $end\n");
    out.push_str("        local.get $start\n");
    out.push_str("        i32.sub\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $len\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.const 4\n");
    out.push_str("    i32.mul\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $items\n");
    out.push_str("    (block $break\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $index\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $break\n");
    out.push_str("        local.get $items\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $start\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.store\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 8\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $list\n");
    out.push_str("    local.get $list\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.store\n");
    out.push_str("    local.get $list\n");
    out.push_str("    local.get $items\n");
    out.push_str("    i32.store offset=4\n");
    out.push_str("    local.get $list\n");
    out.push_str("  )\n");
}

fn emit_join_helper(out: &mut String) {
    out.push_str("  (func $__list_join_strings (param $list i32) (param $sep i32) (result i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $items i32)\n");
    out.push_str("    (local $sep_len i32)\n");
    out.push_str("    (local $total i32)\n");
    out.push_str("    (local $index i32)\n");
    out.push_str("    (local $item_ptr i32)\n");
    out.push_str("    (local $item_len i32)\n");
    out.push_str("    (local $out i32)\n");
    out.push_str("    (local $cursor i32)\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load\n");
    out.push_str("    local.set $len\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load offset=4\n");
    out.push_str("    local.set $items\n");
    out.push_str("    local.get $sep\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $sep_len\n");
    out.push_str("    (block $measure_break\n");
    out.push_str("      (loop $measure_loop\n");
    out.push_str("        local.get $index\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $measure_break\n");
    out.push_str("        local.get $items\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load\n");
    out.push_str("        local.set $item_ptr\n");
    out.push_str("        local.get $item_ptr\n");
    out.push_str("        call $__strlen\n");
    out.push_str("        local.set $item_len\n");
    out.push_str("        local.get $total\n");
    out.push_str("        local.get $item_len\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $total\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.lt_u\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            local.get $total\n");
    out.push_str("            local.get $sep_len\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $total\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $measure_loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $total\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    i32.add\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $out\n");
    out.push_str("    local.get $out\n");
    out.push_str("    local.set $cursor\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    local.set $index\n");
    out.push_str("    (block $copy_break\n");
    out.push_str("      (loop $copy_loop\n");
    out.push_str("        local.get $index\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $copy_break\n");
    out.push_str("        local.get $items\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load\n");
    out.push_str("        local.set $item_ptr\n");
    out.push_str("        local.get $item_ptr\n");
    out.push_str("        call $__strlen\n");
    out.push_str("        local.set $item_len\n");
    out.push_str("        local.get $cursor\n");
    out.push_str("        local.get $item_ptr\n");
    out.push_str("        local.get $item_len\n");
    out.push_str("        call $__memcpy\n");
    out.push_str("        local.get $cursor\n");
    out.push_str("        local.get $item_len\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $cursor\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.lt_u\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            local.get $cursor\n");
    out.push_str("            local.get $sep\n");
    out.push_str("            local.get $sep_len\n");
    out.push_str("            call $__memcpy\n");
    out.push_str("            local.get $cursor\n");
    out.push_str("            local.get $sep_len\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $cursor\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $copy_loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $cursor\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store8\n");
    out.push_str("    local.get $out\n");
    out.push_str("  )\n");
}

fn emit_closure_thunk(
    abi: &WasmAbi,
    closure: &ClosureThunk,
    function_names: &HashSet<String>,
    out: &mut String,
) -> Result<()> {
    let signature = abi
        .closure_signatures
        .get(closure.signature_index)
        .ok_or_else(|| anyhow!("missing closure signature {}", closure.signature_index))?;
    let arg_ty = abi
        .wasm_type(&signature.key.arg)?
        .ok_or_else(|| anyhow!("unsupported closure argument type in wasm backend"))?;
    out.push_str(&format!(
        "  (func ${} (param $env i32) (param ${} {arg_ty})",
        closure.func_name, closure.param_name
    ));
    if let Some(result_ty) = abi.wasm_type(&signature.key.result)? {
        out.push_str(&format!(" (result {result_ty})"));
    }
    out.push('\n');

    let mut local_bindings =
        HashMap::from([(closure.param_name.clone(), Some(closure.param_name.clone()))]);
    let mut declared_local_names = HashSet::from([closure.param_name.clone()]);
    let capture_offsets = closure
        .captures
        .iter()
        .map(|capture| (capture.name.clone(), capture.offset))
        .collect::<HashMap<_, _>>();
    let mut let_locals = Vec::new();
    collect_let_locals(
        &closure.body,
        abi,
        &mut declared_local_names,
        &mut let_locals,
    )?;
    for (local_name, local_ty) in &let_locals {
        out.push_str(&format!("    (local ${local_name} {local_ty})\n"));
    }
    emit_expr(
        &closure.body,
        2,
        &mut local_bindings,
        &mut declared_local_names,
        &HashMap::new(),
        &capture_offsets,
        Some("env"),
        function_names,
        abi,
        out,
    )?;
    out.push_str("  )\n");
    Ok(())
}

fn closure_signature_types(expected: Option<&Type>, expr_ty: &Type) -> Option<(Type, Type)> {
    let ty = match expected {
        Some(Type::Fn(arg, result)) => {
            return Some(((**arg).clone(), (**result).clone()));
        }
        _ => expr_ty,
    };
    if let Type::Fn(arg, result) = ty {
        Some(((**arg).clone(), (**result).clone()))
    } else {
        None
    }
}

fn ordered_captures(body: &HighExpr, scope: &HashMap<String, Type>, param: &str) -> Vec<String> {
    let mut ordered = Vec::new();
    let mut seen = HashSet::new();
    let mut bound = HashSet::from([param.to_owned()]);
    collect_ordered_captures(body, scope, &mut bound, &mut seen, &mut ordered);
    ordered
}

fn collect_ordered_captures(
    expr: &HighExpr,
    scope: &HashMap<String, Type>,
    bound: &mut HashSet<String>,
    seen: &mut HashSet<String>,
    ordered: &mut Vec<String>,
) {
    match &expr.kind {
        HighExprKind::Ident(name) => {
            if scope.contains_key(name) && !bound.contains(name) && seen.insert(name.clone()) {
                ordered.push(name.clone());
            }
        }
        HighExprKind::Lambda { param, body } => {
            bound.insert(param.clone());
            collect_ordered_captures(body, scope, bound, seen, ordered);
            bound.remove(param);
        }
        HighExprKind::Let { name, value, body } => {
            collect_ordered_captures(value, scope, bound, seen, ordered);
            bound.insert(name.clone());
            collect_ordered_captures(body, scope, bound, seen, ordered);
            bound.remove(name);
        }
        HighExprKind::Binary { left, right, .. } => {
            collect_ordered_captures(left, scope, bound, seen, ordered);
            collect_ordered_captures(right, scope, bound, seen, ordered);
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_ordered_captures(condition, scope, bound, seen, ordered);
            collect_ordered_captures(then_branch, scope, bound, seen, ordered);
            collect_ordered_captures(else_branch, scope, bound, seen, ordered);
        }
        HighExprKind::Match { subject, arms } => {
            collect_ordered_captures(subject, scope, bound, seen, ordered);
            for arm in arms {
                collect_ordered_captures(&arm.expr, scope, bound, seen, ordered);
            }
        }
        HighExprKind::Call { args, .. } | HighExprKind::Construct { args, .. } => {
            for arg in args {
                collect_ordered_captures(arg, scope, bound, seen, ordered);
            }
        }
        HighExprKind::List(items) | HighExprKind::Tuple(items) => {
            for item in items {
                collect_ordered_captures(item, scope, bound, seen, ordered);
            }
        }
        HighExprKind::Int(_)
        | HighExprKind::Bool(_)
        | HighExprKind::String(_)
        | HighExprKind::Error => {}
    }
}

fn scan_expr(
    expr: &HighExpr,
    uses_console_println: &mut bool,
    uses_string_builtin: &mut bool,
    uses_list_runtime: &mut bool,
    uses_adt_runtime: &mut bool,
    uses_range_inclusive: &mut bool,
    uses_map_builtin: &mut bool,
    uses_filter_builtin: &mut bool,
    uses_sum_builtin: &mut bool,
    uses_join_builtin: &mut bool,
) {
    match &expr.kind {
        HighExprKind::Call { callee, args } => {
            if callee == "console.println" {
                *uses_console_println = true;
            }
            if callee == "string" {
                *uses_string_builtin = true;
            }
            if callee == "range_inclusive" {
                *uses_list_runtime = true;
                *uses_range_inclusive = true;
            }
            if callee == "map" {
                *uses_list_runtime = true;
                *uses_map_builtin = true;
            }
            if callee == "filter" {
                *uses_list_runtime = true;
                *uses_filter_builtin = true;
            }
            if callee == "sum" {
                *uses_sum_builtin = true;
            }
            if callee == "join" {
                *uses_list_runtime = true;
                *uses_join_builtin = true;
            }
            for arg in args {
                scan_expr(
                    arg,
                    uses_console_println,
                    uses_string_builtin,
                    uses_list_runtime,
                    uses_adt_runtime,
                    uses_range_inclusive,
                    uses_map_builtin,
                    uses_filter_builtin,
                    uses_sum_builtin,
                    uses_join_builtin,
                );
            }
        }
        HighExprKind::Binary { left, right, .. } => {
            scan_expr(
                left,
                uses_console_println,
                uses_string_builtin,
                uses_list_runtime,
                uses_adt_runtime,
                uses_range_inclusive,
                uses_map_builtin,
                uses_filter_builtin,
                uses_sum_builtin,
                uses_join_builtin,
            );
            scan_expr(
                right,
                uses_console_println,
                uses_string_builtin,
                uses_list_runtime,
                uses_adt_runtime,
                uses_range_inclusive,
                uses_map_builtin,
                uses_filter_builtin,
                uses_sum_builtin,
                uses_join_builtin,
            );
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            scan_expr(
                condition,
                uses_console_println,
                uses_string_builtin,
                uses_list_runtime,
                uses_adt_runtime,
                uses_range_inclusive,
                uses_map_builtin,
                uses_filter_builtin,
                uses_sum_builtin,
                uses_join_builtin,
            );
            scan_expr(
                then_branch,
                uses_console_println,
                uses_string_builtin,
                uses_list_runtime,
                uses_adt_runtime,
                uses_range_inclusive,
                uses_map_builtin,
                uses_filter_builtin,
                uses_sum_builtin,
                uses_join_builtin,
            );
            scan_expr(
                else_branch,
                uses_console_println,
                uses_string_builtin,
                uses_list_runtime,
                uses_adt_runtime,
                uses_range_inclusive,
                uses_map_builtin,
                uses_filter_builtin,
                uses_sum_builtin,
                uses_join_builtin,
            );
        }
        HighExprKind::Match { subject, arms } => {
            *uses_adt_runtime = true;
            scan_expr(
                subject,
                uses_console_println,
                uses_string_builtin,
                uses_list_runtime,
                uses_adt_runtime,
                uses_range_inclusive,
                uses_map_builtin,
                uses_filter_builtin,
                uses_sum_builtin,
                uses_join_builtin,
            );
            for arm in arms {
                scan_expr(
                    &arm.expr,
                    uses_console_println,
                    uses_string_builtin,
                    uses_list_runtime,
                    uses_adt_runtime,
                    uses_range_inclusive,
                    uses_map_builtin,
                    uses_filter_builtin,
                    uses_sum_builtin,
                    uses_join_builtin,
                );
            }
        }
        HighExprKind::Construct { args, .. } => {
            *uses_adt_runtime = true;
            for arg in args {
                scan_expr(
                    arg,
                    uses_console_println,
                    uses_string_builtin,
                    uses_list_runtime,
                    uses_adt_runtime,
                    uses_range_inclusive,
                    uses_map_builtin,
                    uses_filter_builtin,
                    uses_sum_builtin,
                    uses_join_builtin,
                );
            }
        }
        HighExprKind::List(items) | HighExprKind::Tuple(items) => {
            if matches!(&expr.kind, HighExprKind::List(_)) {
                *uses_list_runtime = true;
            }
            for item in items {
                scan_expr(
                    item,
                    uses_console_println,
                    uses_string_builtin,
                    uses_list_runtime,
                    uses_adt_runtime,
                    uses_range_inclusive,
                    uses_map_builtin,
                    uses_filter_builtin,
                    uses_sum_builtin,
                    uses_join_builtin,
                );
            }
        }
        HighExprKind::Lambda { body, .. } => {
            scan_expr(
                body,
                uses_console_println,
                uses_string_builtin,
                uses_list_runtime,
                uses_adt_runtime,
                uses_range_inclusive,
                uses_map_builtin,
                uses_filter_builtin,
                uses_sum_builtin,
                uses_join_builtin,
            );
        }
        HighExprKind::Let { value, body, .. } => {
            scan_expr(
                value,
                uses_console_println,
                uses_string_builtin,
                uses_list_runtime,
                uses_adt_runtime,
                uses_range_inclusive,
                uses_map_builtin,
                uses_filter_builtin,
                uses_sum_builtin,
                uses_join_builtin,
            );
            scan_expr(
                body,
                uses_console_println,
                uses_string_builtin,
                uses_list_runtime,
                uses_adt_runtime,
                uses_range_inclusive,
                uses_map_builtin,
                uses_filter_builtin,
                uses_sum_builtin,
                uses_join_builtin,
            );
        }
        HighExprKind::Int(_)
        | HighExprKind::Bool(_)
        | HighExprKind::String(_)
        | HighExprKind::Error => {}
        HighExprKind::Ident(name) => {
            if name == "string" {
                *uses_string_builtin = true;
            }
        }
    }
}

fn list_item_type(ty: &Type) -> Option<Type> {
    if let Type::List(item) = ty {
        Some((**item).clone())
    } else {
        None
    }
}

fn callback_result_type(expr: &HighExpr, scope: &HashMap<String, Type>) -> Option<Type> {
    match &expr.kind {
        HighExprKind::Lambda { body, .. } => Some(body.ty.clone()),
        HighExprKind::Ident(name) if name == "string" => Some(Type::String),
        HighExprKind::Ident(name) => {
            if let Some(Type::Fn(_, result)) = scope.get(name) {
                Some((**result).clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NamedTypeAbi {
    FieldlessEnum,
    HeapEnum,
    Unsupported,
}

#[derive(Clone, Debug)]
struct VariantLayout {
    owner_type: String,
    tag: u32,
    field_count: usize,
}

fn result_variant_layout(name: &str) -> Option<VariantLayout> {
    match name {
        "Ok" => Some(VariantLayout {
            owner_type: "__result".to_owned(),
            tag: 0,
            field_count: 1,
        }),
        "Err" => Some(VariantLayout {
            owner_type: "__result".to_owned(),
            tag: 1,
            field_count: 1,
        }),
        _ => None,
    }
}

#[derive(Debug, Default)]
struct StringTable {
    offsets: HashMap<String, u32>,
    next_offset: u32,
}

impl StringTable {
    fn collect(module: &HighModule) -> Self {
        let mut table = Self::default();
        for function in &module.functions {
            table.collect_expr(&function.body);
        }
        table
    }

    fn collect_expr(&mut self, expr: &HighExpr) {
        match &expr.kind {
            HighExprKind::String(text) => {
                self.intern(text);
            }
            HighExprKind::Binary { left, right, .. } => {
                self.collect_expr(left);
                self.collect_expr(right);
            }
            HighExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_expr(condition);
                self.collect_expr(then_branch);
                self.collect_expr(else_branch);
            }
            HighExprKind::Call { args, .. } | HighExprKind::Construct { args, .. } => {
                for arg in args {
                    self.collect_expr(arg);
                }
            }
            HighExprKind::Match { subject, arms } => {
                self.collect_expr(subject);
                for arm in arms {
                    self.collect_expr(&arm.expr);
                }
            }
            HighExprKind::List(items) | HighExprKind::Tuple(items) => {
                for item in items {
                    self.collect_expr(item);
                }
            }
            HighExprKind::Lambda { body, .. } => {
                self.collect_expr(body);
            }
            HighExprKind::Let { value, body, .. } => {
                self.collect_expr(value);
                self.collect_expr(body);
            }
            HighExprKind::Int(_)
            | HighExprKind::Bool(_)
            | HighExprKind::Ident(_)
            | HighExprKind::Error => {}
        }
    }

    fn intern(&mut self, text: &str) -> u32 {
        if let Some(offset) = self.offsets.get(text) {
            return *offset;
        }
        let offset = self.next_offset;
        self.next_offset = self
            .next_offset
            .saturating_add(text.len() as u32)
            .saturating_add(1);
        self.offsets.insert(text.to_owned(), offset);
        offset
    }

    fn offset_for(&self, text: &str) -> Option<u32> {
        self.offsets.get(text).copied()
    }

    fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }
}

fn emit_all_memory(abi: &WasmAbi, out: &mut String) {
    if !abi.needs_memory() {
        return;
    }

    let pages = {
        let bytes = abi.memory_end().max(1);
        bytes.div_ceil(65_536)
    };
    out.push_str(&format!("  (memory (export \"memory\") {pages})\n"));

    // String literals
    let mut entries = abi
        .string_table
        .offsets
        .iter()
        .map(|(text, offset)| (*offset, text.as_str()))
        .collect::<Vec<_>>();
    entries.sort_by_key(|(offset, _)| *offset);
    for (offset, text) in entries {
        out.push_str(&format!(
            "  (data (i32.const {offset}) \"{}\")\n",
            wat_string_literal(text)
        ));
    }

    // Newline byte for console.println
    if abi.needs_scratch() && abi.uses_console_println {
        out.push_str(&format!(
            "  (data (i32.const {}) \"\\0a\")\n",
            abi.newline_base()
        ));
    }
}

fn wat_string_literal(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes().chain(std::iter::once(0)) {
        match byte {
            b'"' => out.push_str("\\22"),
            b'\\' => out.push_str("\\5c"),
            0x20..=0x7e => out.push(byte as char),
            _ => out.push_str(&format!("\\{:02x}", byte)),
        }
    }
    out
}
