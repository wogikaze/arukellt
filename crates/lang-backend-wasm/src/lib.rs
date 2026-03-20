use std::collections::{HashMap, HashSet};

use anyhow::{Result, anyhow, bail};
use lang_core::{Pattern, Type, compile_module};
use lang_ir::{
    HighExpr, HighExprKind, HighMatchArm, HighModule, WasmFunction, WasmFunctionBody, WasmModule,
    lower_to_high_ir, lower_to_wasm_ir, optimize_high_module,
};

mod postprocess;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WasmTarget {
    JavaScriptHost,
    JavaScriptHostGc,
    Wasi,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WasmTypeRepr {
    I32,
    RefNull(&'static str),
}

impl std::fmt::Display for WasmTypeRepr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::I32 => f.write_str("i32"),
            Self::RefNull(name) => write!(f, "(ref null {name})"),
        }
    }
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

pub fn emit_wat(module: &HighModule, target: WasmTarget) -> Result<String> {
    let optimized_module = optimize_high_module(module, &wasm_entry_roots(module, target));
    let module = &optimized_module;
    let wasm_module = lower_to_wasm_ir(module);
    let abi = WasmAbi::from_module(module, &wasm_module, target)?;
    let helper_usage = &wasm_module.helper_usage;
    let uses_split_whitespace_nth = module
        .functions
        .iter()
        .any(|function| expr_uses_split_whitespace_nth(&function.body));
    let mut wat_module = String::from("(module\n");
    let function_names = wasm_module
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
    if matches!(
        target,
        WasmTarget::JavaScriptHost | WasmTarget::JavaScriptHostGc
    ) && abi.uses_console_println
    {
        wat_module.push_str(
            "  (import \"arukellt_host\" \"console.println\" \
             (func $__host_console_println (param i32 i32)))\n",
        );
    }
    if target == WasmTarget::Wasi
        && (abi.uses_fs_read_text || abi.uses_stdin_read_text || abi.uses_stdin_read_line)
    {
        wat_module.push_str(
            "  (import \"wasi_snapshot_preview1\" \"fd_read\" \
             (func $fd_read (param i32 i32 i32 i32) (result i32)))\n",
        );
    }
    if target == WasmTarget::Wasi && abi.uses_fs_read_text {
        wat_module.push_str(
            "  (import \"wasi_snapshot_preview1\" \"path_open\" \
             (func $path_open (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32)))\n",
        );
        wat_module.push_str(
            "  (import \"wasi_snapshot_preview1\" \"fd_close\" \
             (func $fd_close (param i32) (result i32)))\n",
        );
    }
    emit_gc_type_defs(&abi, &mut wat_module);
    emit_all_memory(&abi, &mut wat_module);
    emit_heap_primitives(&abi, &mut wat_module);
    if abi.uses_string_builtin {
        emit_string_helper(&abi, &mut wat_module);
    }
    if helper_usage.uses_split_whitespace || uses_split_whitespace_nth {
        emit_ascii_whitespace_helper(&mut wat_module);
    }
    if helper_usage.uses_split_whitespace {
        emit_split_whitespace_helper(&mut wat_module);
    }
    if uses_split_whitespace_nth {
        emit_split_whitespace_nth_helper(&mut wat_module);
    }
    if helper_usage.uses_parse_i64 {
        emit_parse_i64_helper(&abi, &mut wat_module);
    }
    if helper_usage.uses_parse_i64_or_zero {
        emit_parse_i64_or_zero_helper(&mut wat_module);
    }
    if helper_usage.uses_parse_bool {
        emit_parse_bool_helper(&abi, &mut wat_module);
    }
    if helper_usage.uses_ends_with_at {
        emit_ends_with_at_helper(&mut wat_module);
    }
    if helper_usage.uses_strip_suffix {
        emit_strip_suffix_helper(&mut wat_module);
    }
    if helper_usage.uses_unwrap_or
        && !(abi.target == WasmTarget::JavaScriptHostGc && abi.uses_gc_option_i32)
    {
        emit_option_unwrap_or_helper(&mut wat_module);
    }
    if target == WasmTarget::Wasi {
        if abi.uses_fs_read_text {
            emit_fs_read_text_helper(&abi, &mut wat_module)?;
        }
        if abi.uses_stdin_read_text || abi.uses_stdin_read_line {
            emit_stdin_read_text_helper(&abi, &mut wat_module);
        }
        if abi.uses_stdin_read_line {
            emit_stdin_read_line_helper(&mut wat_module);
        }
    }
    emit_closure_support(&abi, &function_names, &mut wat_module)?;
    for function in &wasm_module.functions {
        wat_module.push_str(&emit_function(function, &function_names, &abi)?);
    }
    if abi.uses_console_println {
        emit_console_println_helper(&abi, &mut wat_module);
    }
    match target {
        WasmTarget::JavaScriptHost | WasmTarget::JavaScriptHostGc => {
            emit_javascript_exports(&wasm_module, &mut wat_module)
        }
        WasmTarget::Wasi => emit_wasi_entrypoint(&wasm_module, &abi, &mut wat_module)?,
    }
    wat_module.push_str(")\n");
    Ok(wat_module)
}

fn wasm_entry_roots(module: &HighModule, target: WasmTarget) -> HashSet<String> {
    match target {
        WasmTarget::JavaScriptHost | WasmTarget::JavaScriptHostGc => module
            .functions
            .iter()
            .map(|function| function.name.clone())
            .collect(),
        WasmTarget::Wasi => HashSet::from([String::from("main")]),
    }
}

pub fn emit_wasm(module: &HighModule, target: WasmTarget) -> Result<Vec<u8>> {
    let bytes = wat::parse_str(&emit_wat(module, target)?)?;
    postprocess::postprocess_wasm(&bytes)
}

pub fn postprocess_wasm(bytes: &[u8]) -> Result<Vec<u8>> {
    postprocess::postprocess_wasm(bytes)
}

fn expr_uses_split_whitespace_nth(expr: &HighExpr) -> bool {
    match &expr.kind {
        HighExprKind::Call { callee, args } if callee == "__index" => {
            matches_split_whitespace_nth_expr(args.first()).is_some()
                || args.iter().any(expr_uses_split_whitespace_nth)
        }
        HighExprKind::Call { args, .. } | HighExprKind::Construct { args, .. } => {
            args.iter().any(expr_uses_split_whitespace_nth)
        }
        HighExprKind::Binary { left, right, .. } => {
            expr_uses_split_whitespace_nth(left) || expr_uses_split_whitespace_nth(right)
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_uses_split_whitespace_nth(condition)
                || expr_uses_split_whitespace_nth(then_branch)
                || expr_uses_split_whitespace_nth(else_branch)
        }
        HighExprKind::Match { subject, arms } => {
            expr_uses_split_whitespace_nth(subject)
                || arms
                    .iter()
                    .any(|arm| expr_uses_split_whitespace_nth(&arm.expr))
        }
        HighExprKind::List(items) | HighExprKind::Tuple(items) => {
            items.iter().any(expr_uses_split_whitespace_nth)
        }
        HighExprKind::Lambda { body, .. } => expr_uses_split_whitespace_nth(body),
        HighExprKind::Let { value, body, .. } => {
            expr_uses_split_whitespace_nth(value) || expr_uses_split_whitespace_nth(body)
        }
        HighExprKind::Int(_)
        | HighExprKind::Bool(_)
        | HighExprKind::String(_)
        | HighExprKind::Ident(_)
        | HighExprKind::Error => false,
    }
}

fn matches_split_whitespace_nth_expr(receiver: Option<&HighExpr>) -> Option<&HighExpr> {
    let receiver = receiver?;
    let HighExprKind::Call { callee, args } = &receiver.kind else {
        return None;
    };
    if callee != "split_whitespace" || args.len() != 1 {
        return None;
    }
    Some(&args[0])
}

fn module_uses_gc_option_i32(module: &HighModule) -> bool {
    module.functions.iter().any(function_uses_gc_option_i32)
}

fn function_uses_gc_option_i32(function: &lang_ir::HighFunction) -> bool {
    function
        .params
        .iter()
        .any(|param| is_gc_option_i32_type(&param.ty))
        || is_gc_option_i32_type(&function.return_type)
        || expr_uses_gc_option_i32(&function.body)
}

fn is_gc_option_i32_type(ty: &Type) -> bool {
    matches!(ty, Type::Option(inner) if **inner == Type::Int)
}

fn expr_uses_gc_option_i32(expr: &HighExpr) -> bool {
    if is_gc_option_i32_type(&expr.ty) {
        return true;
    }
    match &expr.kind {
        HighExprKind::Call { args, .. } | HighExprKind::Construct { args, .. } => {
            args.iter().any(expr_uses_gc_option_i32)
        }
        HighExprKind::Binary { left, right, .. } => {
            expr_uses_gc_option_i32(left) || expr_uses_gc_option_i32(right)
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_uses_gc_option_i32(condition)
                || expr_uses_gc_option_i32(then_branch)
                || expr_uses_gc_option_i32(else_branch)
        }
        HighExprKind::Match { subject, arms } => {
            expr_uses_gc_option_i32(subject)
                || arms.iter().any(|arm| expr_uses_gc_option_i32(&arm.expr))
        }
        HighExprKind::List(items) | HighExprKind::Tuple(items) => {
            items.iter().any(expr_uses_gc_option_i32)
        }
        HighExprKind::Lambda { body, .. } => expr_uses_gc_option_i32(body),
        HighExprKind::Let { value, body, .. } => {
            expr_uses_gc_option_i32(value) || expr_uses_gc_option_i32(body)
        }
        HighExprKind::Int(_)
        | HighExprKind::Bool(_)
        | HighExprKind::String(_)
        | HighExprKind::Ident(_)
        | HighExprKind::Error => false,
    }
}

const HEAP_TMP_PTR_LOCAL_COUNT: usize = 16;

fn heap_tmp_ptr_local(indent: usize) -> String {
    format!("__tmp_ptr{}", indent.min(HEAP_TMP_PTR_LOCAL_COUNT - 1))
}

fn emit_javascript_exports(module: &WasmModule, out: &mut String) {
    for function in &module.functions {
        out.push_str(&format!(
            "  (export \"{}\" (func ${}))\n",
            function.name, function.name
        ));
    }
}

fn emit_gc_type_defs(abi: &WasmAbi, out: &mut String) {
    if abi.uses_gc_option_i32 {
        out.push_str("  (type $__gc_option_i32 (struct (field (mut i32))))\n");
    }
}

fn emit_heap_primitives(abi: &WasmAbi, out: &mut String) {
    if !abi.needs_heap()
        && !abi.uses_len_builtin
        && !abi.uses_ends_with_at_builtin
        && !(abi.target == WasmTarget::JavaScriptHost && abi.uses_console_println)
    {
        return;
    }
    emit_strlen_helper(out);
    if abi.uses_string_eq || abi.uses_ends_with_at_builtin {
        emit_string_eq_helper(out);
    }
    if abi.needs_heap() {
        emit_alloc_helper(out);
        emit_memcpy_helper(out);
        if abi.uses_list_index_builtin {
            emit_list_get_helper(out);
        }
    }
}

fn emit_wasi_entrypoint(module: &WasmModule, abi: &WasmAbi, out: &mut String) -> Result<()> {
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

/// Emit the `$console.println` helper for the selected target ABI.
fn emit_console_println_helper(abi: &WasmAbi, out: &mut String) {
    if abi.target == WasmTarget::JavaScriptHost {
        out.push_str("  (func $console.println (param $ptr i32)\n");
        out.push_str("    local.get $ptr\n");
        out.push_str("    local.get $ptr\n");
        out.push_str("    call $__strlen\n");
        out.push_str("    call $__host_console_println\n");
        out.push_str("  )\n");
        return;
    }

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

fn emit_fs_read_text_helper(abi: &WasmAbi, out: &mut String) -> Result<()> {
    let opened_fd = abi.fs_opened_fd_base();
    let iovec = abi.fs_iovec_base();
    let iovec_len = abi.fs_iovec_base() + 4;
    let nread = abi.fs_nread_base();
    let buffer = abi.fs_read_buffer_base();
    let buffer_len = abi.fs_read_buffer_len() - 1;
    let file_not_found_tag = abi.required_fieldless_variant_tag("FileNotFound")?;
    let permission_denied_tag = abi.required_fieldless_variant_tag("PermissionDenied")?;
    let unknown_read_error_tag = abi.required_fieldless_variant_tag("UnknownReadError")?;

    out.push_str("  (func $fs.read_text (param $path i32) (result i32)\n");
    out.push_str("    (local $path_len i32)\n");
    out.push_str("    (local $errno i32)\n");
    out.push_str("    (local $fd i32)\n");
    out.push_str("    (local $bytes_read i32)\n");
    out.push_str("    (local $text_ptr i32)\n");
    out.push_str("    (local $error_tag i32)\n");
    out.push_str("    (local $error_ptr i32)\n");
    out.push_str("    (local $result_ptr i32)\n");
    out.push_str("    local.get $path\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $path_len\n");
    out.push_str("    i32.const 3\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    local.get $path\n");
    out.push_str("    local.get $path_len\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i64.const 2\n");
    out.push_str("    i64.const 0\n");
    out.push_str("    i32.const 0\n");
    out.push_str(&format!("    i32.const {opened_fd}\n"));
    out.push_str("    call $path_open\n");
    out.push_str("    local.set $errno\n");
    out.push_str("    local.get $errno\n");
    out.push_str("    i32.eqz\n");
    out.push_str("    (if (result i32)\n");
    out.push_str("      (then\n");
    out.push_str(&format!("        i32.const {opened_fd}\n"));
    out.push_str("        i32.load\n");
    out.push_str("        local.set $fd\n");
    out.push_str(&format!("        i32.const {iovec}\n"));
    out.push_str(&format!("        i32.const {buffer}\n"));
    out.push_str("        i32.store\n");
    out.push_str(&format!("        i32.const {iovec_len}\n"));
    out.push_str(&format!("        i32.const {buffer_len}\n"));
    out.push_str("        i32.store\n");
    out.push_str("        local.get $fd\n");
    out.push_str(&format!("        i32.const {iovec}\n"));
    out.push_str("        i32.const 1\n");
    out.push_str(&format!("        i32.const {nread}\n"));
    out.push_str("        call $fd_read\n");
    out.push_str("        local.set $errno\n");
    out.push_str("        local.get $fd\n");
    out.push_str("        call $fd_close\n");
    out.push_str("        drop\n");
    out.push_str("        local.get $errno\n");
    out.push_str("        i32.eqz\n");
    out.push_str("        (if (result i32)\n");
    out.push_str("          (then\n");
    out.push_str(&format!("            i32.const {nread}\n"));
    out.push_str("            i32.load\n");
    out.push_str("            local.set $bytes_read\n");
    out.push_str("            local.get $bytes_read\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            call $__alloc\n");
    out.push_str("            local.set $text_ptr\n");
    out.push_str("            local.get $text_ptr\n");
    out.push_str(&format!("            i32.const {buffer}\n"));
    out.push_str("            local.get $bytes_read\n");
    out.push_str("            call $__memcpy\n");
    out.push_str("            local.get $text_ptr\n");
    out.push_str("            local.get $bytes_read\n");
    out.push_str("            i32.add\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            i32.store8\n");
    out.push_str("            i32.const 8\n");
    out.push_str("            call $__alloc\n");
    out.push_str("            local.set $result_ptr\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            i32.store\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            local.get $text_ptr\n");
    out.push_str("            i32.store offset=4\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("          )\n");
    out.push_str("          (else\n");
    out.push_str(&format!("            i32.const {unknown_read_error_tag}\n"));
    out.push_str("            local.set $error_tag\n");
    out.push_str("            i32.const 4\n");
    out.push_str("            call $__alloc\n");
    out.push_str("            local.set $error_ptr\n");
    out.push_str("            local.get $error_ptr\n");
    out.push_str("            local.get $error_tag\n");
    out.push_str("            i32.store\n");
    out.push_str("            i32.const 8\n");
    out.push_str("            call $__alloc\n");
    out.push_str("            local.set $result_ptr\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.store\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            local.get $error_ptr\n");
    out.push_str("            i32.store offset=4\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("      )\n");
    out.push_str("      (else\n");
    out.push_str("        local.get $errno\n");
    out.push_str("        i32.const 44\n");
    out.push_str("        i32.eq\n");
    out.push_str("        (if (result i32)\n");
    out.push_str(&format!(
        "          (then i32.const {file_not_found_tag})\n"
    ));
    out.push_str("          (else\n");
    out.push_str("            local.get $errno\n");
    out.push_str("            i32.const 2\n");
    out.push_str("            i32.eq\n");
    out.push_str("            (if (result i32)\n");
    out.push_str(&format!(
        "              (then i32.const {permission_denied_tag})\n"
    ));
    out.push_str(&format!(
        "              (else i32.const {unknown_read_error_tag})\n"
    ));
    out.push_str("            )\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.set $error_tag\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        call $__alloc\n");
    out.push_str("        local.set $error_ptr\n");
    out.push_str("        local.get $error_ptr\n");
    out.push_str("        local.get $error_tag\n");
    out.push_str("        i32.store\n");
    out.push_str("        i32.const 8\n");
    out.push_str("        call $__alloc\n");
    out.push_str("        local.set $result_ptr\n");
    out.push_str("        local.get $result_ptr\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.store\n");
    out.push_str("        local.get $result_ptr\n");
    out.push_str("        local.get $error_ptr\n");
    out.push_str("        i32.store offset=4\n");
    out.push_str("        local.get $result_ptr\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("  )\n");
    Ok(())
}

fn emit_stdin_read_text_helper(abi: &WasmAbi, out: &mut String) {
    let iovec = abi.fs_iovec_base();
    let iovec_len = abi.fs_iovec_base() + 4;
    let nread = abi.fs_nread_base();
    let buffer = abi.fs_read_buffer_base();
    let buffer_len = abi.fs_read_buffer_len() - 1;

    out.push_str("  (func $stdin.read_text (result i32)\n");
    out.push_str("    (local $result_ptr i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $chunk_len i32)\n");
    out.push_str("    (local $new_ptr i32)\n");
    out.push_str("    (local $new_len i32)\n");
    out.push_str("    (local $errno i32)\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $result_ptr\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store8\n");
    out.push_str("    (block $done\n");
    out.push_str("      (loop $loop\n");
    out.push_str(&format!("        i32.const {iovec}\n"));
    out.push_str(&format!("        i32.const {buffer}\n"));
    out.push_str("        i32.store\n");
    out.push_str(&format!("        i32.const {iovec_len}\n"));
    out.push_str(&format!("        i32.const {buffer_len}\n"));
    out.push_str("        i32.store\n");
    out.push_str("        i32.const 0\n");
    out.push_str(&format!("        i32.const {iovec}\n"));
    out.push_str("        i32.const 1\n");
    out.push_str(&format!("        i32.const {nread}\n"));
    out.push_str("        call $fd_read\n");
    out.push_str("        local.set $errno\n");
    out.push_str("        local.get $errno\n");
    out.push_str("        i32.eqz\n");
    out.push_str("        (if\n");
    out.push_str("          (then)\n");
    out.push_str("          (else unreachable)\n");
    out.push_str("        )\n");
    out.push_str(&format!("        i32.const {nread}\n"));
    out.push_str("        i32.load\n");
    out.push_str("        local.set $chunk_len\n");
    out.push_str("        local.get $chunk_len\n");
    out.push_str("        i32.eqz\n");
    out.push_str("        br_if $done\n");
    out.push_str("        local.get $len\n");
    out.push_str("        local.get $chunk_len\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $new_len\n");
    out.push_str("        local.get $new_len\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        call $__alloc\n");
    out.push_str("        local.set $new_ptr\n");
    out.push_str("        local.get $new_ptr\n");
    out.push_str("        local.get $result_ptr\n");
    out.push_str("        local.get $len\n");
    out.push_str("        call $__memcpy\n");
    out.push_str("        local.get $new_ptr\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.add\n");
    out.push_str(&format!("        i32.const {buffer}\n"));
    out.push_str("        local.get $chunk_len\n");
    out.push_str("        call $__memcpy\n");
    out.push_str("        local.get $new_ptr\n");
    out.push_str("        local.get $new_len\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        i32.store8\n");
    out.push_str("        local.get $new_ptr\n");
    out.push_str("        local.set $result_ptr\n");
    out.push_str("        local.get $new_len\n");
    out.push_str("        local.set $len\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("  )\n");
}

fn emit_stdin_read_line_helper(out: &mut String) {
    out.push_str("  (global $__stdin_line_text_ptr (mut i32) (i32.const 0))\n");
    out.push_str("  (global $__stdin_line_pos (mut i32) (i32.const 0))\n");
    out.push_str("  (func $stdin.read_line (result i32)\n");
    out.push_str("    (local $text_ptr i32)\n");
    out.push_str("    (local $scan i32)\n");
    out.push_str("    (local $start i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $copy_len i32)\n");
    out.push_str("    (local $line_ptr i32)\n");
    out.push_str("    global.get $__stdin_line_text_ptr\n");
    out.push_str("    local.tee $text_ptr\n");
    out.push_str("    i32.eqz\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        call $stdin.read_text\n");
    out.push_str("        local.tee $text_ptr\n");
    out.push_str("        global.set $__stdin_line_text_ptr\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        global.set $__stdin_line_pos\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    global.get $__stdin_line_pos\n");
    out.push_str("    local.tee $start\n");
    out.push_str("    local.set $scan\n");
    out.push_str("    (block $scan_done\n");
    out.push_str("      (loop $scan_loop\n");
    out.push_str("        local.get $text_ptr\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.eqz\n");
    out.push_str("        br_if $scan_done\n");
    out.push_str("        local.get $text_ptr\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.const 10\n");
    out.push_str("        i32.eq\n");
    out.push_str("        br_if $scan_done\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $scan\n");
    out.push_str("        br $scan_loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $scan\n");
    out.push_str("    local.get $start\n");
    out.push_str("    i32.sub\n");
    out.push_str("    local.set $len\n");
    out.push_str("    local.get $len\n");
    out.push_str("    local.set $copy_len\n");
    out.push_str("    local.get $copy_len\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.gt_s\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        local.get $text_ptr\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.sub\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.const 13\n");
    out.push_str("        i32.eq\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            local.get $copy_len\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.sub\n");
    out.push_str("            local.set $copy_len\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $copy_len\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    i32.add\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $line_ptr\n");
    out.push_str("    local.get $line_ptr\n");
    out.push_str("    local.get $text_ptr\n");
    out.push_str("    local.get $start\n");
    out.push_str("    i32.add\n");
    out.push_str("    local.get $copy_len\n");
    out.push_str("    call $__memcpy\n");
    out.push_str("    local.get $line_ptr\n");
    out.push_str("    local.get $copy_len\n");
    out.push_str("    i32.add\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store8\n");
    out.push_str("    local.get $text_ptr\n");
    out.push_str("    local.get $scan\n");
    out.push_str("    i32.add\n");
    out.push_str("    i32.load8_u\n");
    out.push_str("    i32.eqz\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        global.set $__stdin_line_pos\n");
    out.push_str("      )\n");
    out.push_str("      (else\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        global.set $__stdin_line_pos\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $line_ptr\n");
    out.push_str("  )\n");
}

fn emit_ascii_whitespace_helper(out: &mut String) {
    out.push_str("  (func $__is_ascii_whitespace (param $byte i32) (result i32)\n");
    out.push_str("    local.get $byte\n");
    out.push_str("    i32.const 32\n");
    out.push_str("    i32.eq\n");
    out.push_str("    local.get $byte\n");
    out.push_str("    i32.const 9\n");
    out.push_str("    i32.eq\n");
    out.push_str("    i32.or\n");
    out.push_str("    local.get $byte\n");
    out.push_str("    i32.const 10\n");
    out.push_str("    i32.eq\n");
    out.push_str("    i32.or\n");
    out.push_str("    local.get $byte\n");
    out.push_str("    i32.const 11\n");
    out.push_str("    i32.eq\n");
    out.push_str("    i32.or\n");
    out.push_str("    local.get $byte\n");
    out.push_str("    i32.const 12\n");
    out.push_str("    i32.eq\n");
    out.push_str("    i32.or\n");
    out.push_str("    local.get $byte\n");
    out.push_str("    i32.const 13\n");
    out.push_str("    i32.eq\n");
    out.push_str("    i32.or\n");
    out.push_str("  )\n");
}

fn emit_split_whitespace_helper(out: &mut String) {
    out.push_str("  (func $split_whitespace (param $text i32) (result i32)\n");
    out.push_str("    (local $scan i32)\n");
    out.push_str("    (local $byte i32)\n");
    out.push_str("    (local $count i32)\n");
    out.push_str("    (local $items_ptr i32)\n");
    out.push_str("    (local $list_ptr i32)\n");
    out.push_str("    (local $index i32)\n");
    out.push_str("    (local $start i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $token_ptr i32)\n");
    out.push_str("    local.get $text\n");
    out.push_str("    local.set $scan\n");
    out.push_str("    (block $count_done\n");
    out.push_str("      (loop $count_loop\n");
    out.push_str("        (block $skip_done\n");
    out.push_str("          (loop $skip_loop\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.load8_u\n");
    out.push_str("            local.tee $byte\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $count_done\n");
    out.push_str("            local.get $byte\n");
    out.push_str("            call $__is_ascii_whitespace\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $skip_done\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $scan\n");
    out.push_str("            br $skip_loop\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $count\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $count\n");
    out.push_str("        (block $token_done\n");
    out.push_str("          (loop $token_loop\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.load8_u\n");
    out.push_str("            local.tee $byte\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $count_done\n");
    out.push_str("            local.get $byte\n");
    out.push_str("            call $__is_ascii_whitespace\n");
    out.push_str("            br_if $token_done\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $scan\n");
    out.push_str("            br $token_loop\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        br $count_loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $count\n");
    out.push_str("    i32.eqz\n");
    out.push_str("    (if (result i32)\n");
    out.push_str("      (then i32.const 4)\n");
    out.push_str("      (else\n");
    out.push_str("        local.get $count\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $items_ptr\n");
    out.push_str("    local.get $text\n");
    out.push_str("    local.set $scan\n");
    out.push_str("    (block $emit_done\n");
    out.push_str("      (loop $emit_loop\n");
    out.push_str("        (block $emit_skip_done\n");
    out.push_str("          (loop $emit_skip_loop\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.load8_u\n");
    out.push_str("            local.tee $byte\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $emit_done\n");
    out.push_str("            local.get $byte\n");
    out.push_str("            call $__is_ascii_whitespace\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $emit_skip_done\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $scan\n");
    out.push_str("            br $emit_skip_loop\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        local.set $start\n");
    out.push_str("        (block $emit_token_done\n");
    out.push_str("          (loop $emit_token_loop\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.load8_u\n");
    out.push_str("            local.tee $byte\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $emit_token_done\n");
    out.push_str("            local.get $byte\n");
    out.push_str("            call $__is_ascii_whitespace\n");
    out.push_str("            br_if $emit_token_done\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $scan\n");
    out.push_str("            br $emit_token_loop\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        local.get $start\n");
    out.push_str("        i32.sub\n");
    out.push_str("        local.set $len\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        call $__alloc\n");
    out.push_str("        local.set $token_ptr\n");
    out.push_str("        local.get $token_ptr\n");
    out.push_str("        local.get $start\n");
    out.push_str("        local.get $len\n");
    out.push_str("        call $__memcpy\n");
    out.push_str("        local.get $token_ptr\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        i32.store8\n");
    out.push_str("        local.get $items_ptr\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $token_ptr\n");
    out.push_str("        i32.store\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $emit_loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 8\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $list_ptr\n");
    out.push_str("    local.get $list_ptr\n");
    out.push_str("    local.get $count\n");
    out.push_str("    i32.store\n");
    out.push_str("    local.get $list_ptr\n");
    out.push_str("    local.get $items_ptr\n");
    out.push_str("    i32.store offset=4\n");
    out.push_str("    local.get $list_ptr\n");
    out.push_str("  )\n");
}

fn emit_split_whitespace_nth_helper(out: &mut String) {
    out.push_str(
        "  (func $__split_whitespace_nth (param $text i32) (param $target i32) (result i32)\n",
    );
    out.push_str("    (local $scan i32)\n");
    out.push_str("    (local $byte i32)\n");
    out.push_str("    (local $index i32)\n");
    out.push_str("    (local $start i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $token_ptr i32)\n");
    out.push_str("    local.get $target\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.lt_s\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        call $__alloc\n");
    out.push_str("        local.tee $token_ptr\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        i32.store8\n");
    out.push_str("        local.get $token_ptr\n");
    out.push_str("        return\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $text\n");
    out.push_str("    local.set $scan\n");
    out.push_str("    (block $done\n");
    out.push_str("      (loop $outer\n");
    out.push_str("        (block $skip_done\n");
    out.push_str("          (loop $skip\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.load8_u\n");
    out.push_str("            local.tee $byte\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $done\n");
    out.push_str("            local.get $byte\n");
    out.push_str("            call $__is_ascii_whitespace\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $skip_done\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $scan\n");
    out.push_str("            br $skip\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        local.set $start\n");
    out.push_str("        (block $token_done\n");
    out.push_str("          (loop $token\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.load8_u\n");
    out.push_str("            local.tee $byte\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $token_done\n");
    out.push_str("            local.get $byte\n");
    out.push_str("            call $__is_ascii_whitespace\n");
    out.push_str("            br_if $token_done\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $scan\n");
    out.push_str("            br $token\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $index\n");
    out.push_str("        local.get $target\n");
    out.push_str("        i32.eq\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            local.get $start\n");
    out.push_str("            i32.sub\n");
    out.push_str("            local.set $len\n");
    out.push_str("            local.get $len\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            call $__alloc\n");
    out.push_str("            local.set $token_ptr\n");
    out.push_str("            local.get $token_ptr\n");
    out.push_str("            local.get $start\n");
    out.push_str("            local.get $len\n");
    out.push_str("            call $__memcpy\n");
    out.push_str("            local.get $token_ptr\n");
    out.push_str("            local.get $len\n");
    out.push_str("            i32.add\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            i32.store8\n");
    out.push_str("            local.get $token_ptr\n");
    out.push_str("            return\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $outer\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.tee $token_ptr\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store8\n");
    out.push_str("    local.get $token_ptr\n");
    out.push_str("  )\n");
}

fn emit_strip_suffix_helper(out: &mut String) {
    out.push_str("  (func $strip_suffix (param $text i32) (param $suffix i32) (result i32)\n");
    out.push_str("    (local $text_len i32)\n");
    out.push_str("    (local $suffix_len i32)\n");
    out.push_str("    (local $rest_len i32)\n");
    out.push_str("    (local $scan i32)\n");
    out.push_str("    (local $result_ptr i32)\n");
    out.push_str("    (local $rest_ptr i32)\n");
    out.push_str("    local.get $text\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $text_len\n");
    out.push_str("    local.get $suffix\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $suffix_len\n");
    out.push_str("    local.get $text_len\n");
    out.push_str("    local.get $suffix_len\n");
    out.push_str("    i32.lt_u\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        call $__alloc\n");
    out.push_str("        local.set $result_ptr\n");
    out.push_str("        local.get $result_ptr\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.store\n");
    out.push_str("        local.get $result_ptr\n");
    out.push_str("        return\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $text_len\n");
    out.push_str("    local.get $suffix_len\n");
    out.push_str("    i32.sub\n");
    out.push_str("    local.set $rest_len\n");
    out.push_str("    (block $mismatch\n");
    out.push_str("      (loop $compare\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        local.get $suffix_len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $mismatch\n");
    out.push_str("        local.get $text\n");
    out.push_str("        local.get $rest_len\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        local.get $suffix\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.ne\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            i32.const 4\n");
    out.push_str("            call $__alloc\n");
    out.push_str("            local.set $result_ptr\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.store\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            return\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $scan\n");
    out.push_str("        br $compare\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $rest_len\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    i32.add\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $rest_ptr\n");
    out.push_str("    local.get $rest_ptr\n");
    out.push_str("    local.get $text\n");
    out.push_str("    local.get $rest_len\n");
    out.push_str("    call $__memcpy\n");
    out.push_str("    local.get $rest_ptr\n");
    out.push_str("    local.get $rest_len\n");
    out.push_str("    i32.add\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store8\n");
    out.push_str("    i32.const 8\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $result_ptr\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("    local.get $rest_ptr\n");
    out.push_str("    i32.store offset=4\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("  )\n");
}

fn emit_option_unwrap_or_helper(out: &mut String) {
    out.push_str(
        "  (func $__option_unwrap_or (param $option i32) (param $fallback i32) (result i32)\n",
    );
    out.push_str("    local.get $option\n");
    out.push_str("    i32.load\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.eq\n");
    out.push_str("    (if (result i32)\n");
    out.push_str("      (then\n");
    out.push_str("        local.get $option\n");
    out.push_str("        i32.load offset=4\n");
    out.push_str("      )\n");
    out.push_str("      (else\n");
    out.push_str("        local.get $fallback\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("  )\n");
}

fn emit_parse_i64_or_zero_helper(out: &mut String) {
    out.push_str("  (func $__parse_i64_or_zero (param $text i32) (result i32)\n");
    out.push_str("    (local $scan i32)\n");
    out.push_str("    (local $byte i32)\n");
    out.push_str("    (local $value i32)\n");
    out.push_str("    (local $sign i32)\n");
    out.push_str("    (local $has_digits i32)\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    local.set $sign\n");
    out.push_str("    local.get $text\n");
    out.push_str("    local.set $scan\n");
    out.push_str("    local.get $scan\n");
    out.push_str("    i32.load8_u\n");
    out.push_str("    i32.const 45\n");
    out.push_str("    i32.eq\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const -1\n");
    out.push_str("        local.set $sign\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $scan\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    (block $invalid\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        local.set $byte\n");
    out.push_str("        local.get $byte\n");
    out.push_str("        i32.eqz\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            local.get $has_digits\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $invalid\n");
    out.push_str("            local.get $sign\n");
    out.push_str("            i32.const -1\n");
    out.push_str("            i32.eq\n");
    out.push_str("            (if (result i32)\n");
    out.push_str("              (then\n");
    out.push_str("                i32.const 0\n");
    out.push_str("                local.get $value\n");
    out.push_str("                i32.sub\n");
    out.push_str("              )\n");
    out.push_str("              (else\n");
    out.push_str("                local.get $value\n");
    out.push_str("              )\n");
    out.push_str("            )\n");
    out.push_str("            return\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $byte\n");
    out.push_str("        i32.const 48\n");
    out.push_str("        i32.lt_u\n");
    out.push_str("        br_if $invalid\n");
    out.push_str("        local.get $byte\n");
    out.push_str("        i32.const 57\n");
    out.push_str("        i32.gt_u\n");
    out.push_str("        br_if $invalid\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        local.set $has_digits\n");
    out.push_str("        local.get $value\n");
    out.push_str("        i32.const 10\n");
    out.push_str("        i32.mul\n");
    out.push_str("        local.get $byte\n");
    out.push_str("        i32.const 48\n");
    out.push_str("        i32.sub\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $value\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $scan\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 0\n");
    out.push_str("  )\n");
}

fn emit_parse_i64_helper(_abi: &WasmAbi, out: &mut String) {
    out.push_str("  (func $parse.i64 (param $text i32) (result i32)\n");
    out.push_str("    (local $scan i32)\n");
    out.push_str("    (local $byte i32)\n");
    out.push_str("    (local $value i32)\n");
    out.push_str("    (local $sign i32)\n");
    out.push_str("    (local $has_digits i32)\n");
    out.push_str("    (local $result_ptr i32)\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    local.set $sign\n");
    out.push_str("    local.get $text\n");
    out.push_str("    local.set $scan\n");
    out.push_str("    local.get $scan\n");
    out.push_str("    i32.load8_u\n");
    out.push_str("    i32.const 45\n");
    out.push_str("    i32.eq\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const -1\n");
    out.push_str("        local.set $sign\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $scan\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    (block $parse_err\n");
    out.push_str("      (block $parse_done\n");
    out.push_str("        (loop $parse_loop\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        local.tee $byte\n");
    out.push_str("        i32.eqz\n");
    out.push_str("        br_if $parse_done\n");
    out.push_str("        local.get $byte\n");
    out.push_str("        i32.const 48\n");
    out.push_str("        i32.lt_u\n");
    out.push_str("        br_if $parse_err\n");
    out.push_str("        local.get $byte\n");
    out.push_str("        i32.const 57\n");
    out.push_str("        i32.gt_u\n");
    out.push_str("        br_if $parse_err\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        local.set $has_digits\n");
    out.push_str("        local.get $value\n");
    out.push_str("        i32.const 10\n");
    out.push_str("        i32.mul\n");
    out.push_str("        local.get $byte\n");
    out.push_str("        i32.const 48\n");
    out.push_str("        i32.sub\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $value\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $scan\n");
    out.push_str("        br $parse_loop\n");
    out.push_str("        )\n");
    out.push_str("      )\n");
    out.push_str("      local.get $has_digits\n");
    out.push_str("      i32.eqz\n");
    out.push_str("      br_if $parse_err\n");
    out.push_str("      i32.const 8\n");
    out.push_str("      call $__alloc\n");
    out.push_str("      local.set $result_ptr\n");
    out.push_str("      local.get $result_ptr\n");
    out.push_str("      i32.const 0\n");
    out.push_str("      i32.store\n");
    out.push_str("      local.get $result_ptr\n");
    out.push_str("      local.get $value\n");
    out.push_str("      local.get $sign\n");
    out.push_str("      i32.mul\n");
    out.push_str("      i32.store offset=4\n");
    out.push_str("      local.get $result_ptr\n");
    out.push_str("      return\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 8\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $result_ptr\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    i32.store\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store offset=4\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("  )\n");
}

fn emit_parse_bool_helper(_abi: &WasmAbi, out: &mut String) {
    out.push_str("  (func $parse.bool (param $text i32) (result i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $result_ptr i32)\n");
    out.push_str("    local.get $text\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $len\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.const 4\n");
    out.push_str("    i32.eq\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.const 116\n");
    out.push_str("        i32.eq\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u offset=1\n");
    out.push_str("        i32.const 114\n");
    out.push_str("        i32.eq\n");
    out.push_str("        i32.and\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u offset=2\n");
    out.push_str("        i32.const 117\n");
    out.push_str("        i32.eq\n");
    out.push_str("        i32.and\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u offset=3\n");
    out.push_str("        i32.const 101\n");
    out.push_str("        i32.eq\n");
    out.push_str("        i32.and\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            i32.const 8\n");
    out.push_str("            call $__alloc\n");
    out.push_str("            local.set $result_ptr\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            i32.store\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.store offset=4\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            return\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.const 5\n");
    out.push_str("    i32.eq\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.const 102\n");
    out.push_str("        i32.eq\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u offset=1\n");
    out.push_str("        i32.const 97\n");
    out.push_str("        i32.eq\n");
    out.push_str("        i32.and\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u offset=2\n");
    out.push_str("        i32.const 108\n");
    out.push_str("        i32.eq\n");
    out.push_str("        i32.and\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u offset=3\n");
    out.push_str("        i32.const 115\n");
    out.push_str("        i32.eq\n");
    out.push_str("        i32.and\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u offset=4\n");
    out.push_str("        i32.const 101\n");
    out.push_str("        i32.eq\n");
    out.push_str("        i32.and\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            i32.const 8\n");
    out.push_str("            call $__alloc\n");
    out.push_str("            local.set $result_ptr\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            i32.store\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            i32.store offset=4\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            return\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 8\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $result_ptr\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    i32.store\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store offset=4\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("  )\n");
}

fn emit_function(
    function: &WasmFunction,
    function_names: &HashSet<String>,
    abi: &WasmAbi,
) -> Result<String> {
    match &function.body {
        WasmFunctionBody::SuffixRecursion(spec) => {
            emit_suffix_recursion_function(function, spec, abi)
        }
        WasmFunctionBody::ParseI64OrZero(spec) => emit_parse_or_zero_function(function, spec),
        WasmFunctionBody::High(body) => emit_high_function(function, body, function_names, abi),
    }
}

fn emit_high_function(
    function: &WasmFunction,
    body: &HighExpr,
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
    collect_let_locals(body, abi, &mut collect_local_names, &mut let_locals)?;
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
    if abi.uses_gc_option_i32 {
        out.push_str("    (local $__gc_option_tmp (ref null $__gc_option_i32))\n");
    }
    if abi.needs_heap() {
        for index in 0..HEAP_TMP_PTR_LOCAL_COUNT {
            out.push_str(&format!("    (local $__tmp_ptr{index} i32)\n"));
        }
    }
    let mut emit_local_names = locals.clone();
    emit_expr(
        body,
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
    if abi.wasm_type(&function.return_type)?.is_none() && abi.wasm_type(&body.ty)?.is_some() {
        out.push_str("    drop\n");
    }
    out.push_str("  )\n");
    Ok(out)
}

fn emit_suffix_recursion_function(
    function: &WasmFunction,
    spec: &lang_ir::SuffixRecursionSpec,
    abi: &WasmAbi,
) -> Result<String> {
    let helper_name = format!("__suffix_rec_{}", function.name);
    let mut out = String::new();
    out.push_str(&format!(
        "  (func ${} (param ${} i32) (result i32)\n",
        function.name, spec.param_name
    ));
    out.push_str(&format!("    local.get ${}\n", spec.param_name));
    out.push_str(&format!("    local.get ${}\n", spec.param_name));
    out.push_str("    call $__strlen\n");
    out.push_str(&format!("    call ${helper_name}\n"));
    out.push_str("  )\n");
    out.push_str(&format!(
        "  (func ${helper_name} (param $text i32) (param $end i32) (result i32)\n"
    ));
    out.push_str("    (block $done\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $end\n");
    out.push_str("        i32.eqz\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            return\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    emit_suffix_iteration_cases(spec, 4, abi, &mut out)?;
    out.push_str("        i32.const 0\n");
    out.push_str("        return\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 0\n");
    out.push_str("  )\n");
    Ok(out)
}

fn emit_parse_or_zero_function(
    function: &WasmFunction,
    spec: &lang_ir::ParseOrZeroSpec,
) -> Result<String> {
    let mut out = String::new();
    out.push_str(&format!(
        "  (func ${} (param ${} i32) (result i32)\n",
        function.name, spec.param_name
    ));
    out.push_str(&format!("    local.get ${}\n", spec.param_name));
    out.push_str("    call $__parse_i64_or_zero\n");
    out.push_str("  )\n");
    Ok(out)
}

fn emit_suffix_iteration_cases(
    spec: &lang_ir::SuffixRecursionSpec,
    indent: usize,
    abi: &WasmAbi,
    out: &mut String,
) -> Result<()> {
    let pad = "  ".repeat(indent);
    for suffix in &spec.suffixes {
        let suffix_offset = abi
            .string_table
            .offset_for(suffix)
            .ok_or_else(|| anyhow!("missing suffix literal in string table: {suffix}"))?;
        out.push_str(&format!("{pad}local.get $text\n"));
        out.push_str(&format!("{pad}i32.const {suffix_offset}\n"));
        out.push_str(&format!("{pad}local.get $end\n"));
        out.push_str(&format!("{pad}call $ends_with_at\n"));
        out.push_str(&format!("{pad}(if\n"));
        out.push_str(&format!("{pad}  (then\n"));
        out.push_str(&format!("{pad}    local.get $end\n"));
        out.push_str(&format!("{pad}    i32.const {}\n", suffix.len()));
        out.push_str(&format!("{pad}    i32.sub\n"));
        out.push_str(&format!("{pad}    local.set $end\n"));
        out.push_str(&format!("{pad}    br $loop\n"));
        out.push_str(&format!("{pad}  )\n"));
        out.push_str(&format!("{pad})\n"));
    }
    Ok(())
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
        HighExprKind::Tuple(items) => {
            emit_tuple_literal(
                items,
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
                if matches!(op, lang_core::BinaryOp::Equal)
                    && left.ty == Type::String
                    && right.ty == Type::String
                {
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
                    out.push_str(&format!("{pad}call $__streq\n"));
                    return Ok(());
                }
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
            if callee == "__index" {
                let [receiver, index] = args.as_slice() else {
                    bail!("`__index` expects exactly two arguments in wasm backend");
                };
                if let Some(text) = matches_split_whitespace_nth_expr(Some(receiver)) {
                    emit_expr(
                        text,
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
                        index,
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
                    out.push_str(&format!("{pad}call $__split_whitespace_nth\n"));
                    return Ok(());
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
                match &receiver.ty {
                    Type::Tuple(_)
                        if matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost) =>
                    {
                        let HighExprKind::Int(index_value) = index.kind else {
                            bail!("wasm backend currently requires constant indexes");
                        };
                        if index_value < 0 {
                            bail!("wasm backend does not support negative indexes");
                        }
                        out.push_str(&format!("{pad}i32.load offset={}\n", index_value * 4));
                    }
                    Type::List(_) => {
                        emit_expr(
                            index,
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
                        out.push_str(&format!("{pad}call $__list_get\n"));
                    }
                    _ => bail!("unsupported index receiver type in wasm backend"),
                }
            } else if matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost)
                && callee == "range_inclusive"
            {
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
            } else if matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost)
                && callee == "iter.unfold"
            {
                let [seed, callback] = args.as_slice() else {
                    bail!("`iter.unfold` expects exactly two arguments in wasm backend");
                };
                emit_expr(
                    seed,
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
                out.push_str(&format!("{pad}call $__iter_unfold_new\n"));
            } else if matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost)
                && callee == "take"
            {
                let [receiver, limit] = args.as_slice() else {
                    bail!("`take` expects exactly two arguments in wasm backend");
                };
                if expr_item_type(&receiver.ty) != Type::Int {
                    bail!("only Seq<i64>.take is currently supported in wasm backend");
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
                    limit,
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
                out.push_str(&format!("{pad}call $__iter_take_i64\n"));
            } else if matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost)
                && callee == "map"
            {
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
                match &receiver.ty {
                    Type::List(_) => out.push_str(&format!(
                        "{pad}call ${}\n",
                        abi.map_helper_name(signature.index)
                    )),
                    Type::Option(_) => out.push_str(&format!(
                        "{pad}call ${}\n",
                        abi.option_map_helper_name(signature.index)
                    )),
                    _ => bail!("`map` expects a List or Option receiver in wasm backend"),
                }
            } else if matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost)
                && callee == "unwrap_or"
            {
                let [option, fallback] = args.as_slice() else {
                    bail!("`unwrap_or` expects exactly two arguments in wasm backend");
                };
                if !matches!(option.ty, Type::Option(_)) {
                    bail!("`unwrap_or` expects an Option receiver in wasm backend");
                }
                emit_expr(
                    option,
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
                    fallback,
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
                out.push_str(&format!("{pad}call $__option_unwrap_or\n"));
            } else if abi.target == WasmTarget::JavaScriptHostGc && callee == "unwrap_or" {
                let [option, fallback] = args.as_slice() else {
                    bail!("`unwrap_or` expects exactly two arguments in wasm-js-gc backend");
                };
                if !is_gc_option_i32_type(&option.ty) {
                    bail!("`unwrap_or` only supports Option<Int> in wasm-js-gc backend");
                }
                emit_expr(
                    option,
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
                out.push_str(&format!("{pad}local.set $__gc_option_tmp\n"));
                out.push_str(&format!("{pad}local.get $__gc_option_tmp\n"));
                out.push_str(&format!("{pad}ref.is_null\n"));
                out.push_str(&format!("{pad}(if"));
                if let Some(result_ty) = abi.wasm_type(&expr.ty)? {
                    out.push_str(&format!(" (result {result_ty})"));
                }
                out.push('\n');
                out.push_str(&format!("{pad}  (then\n"));
                emit_expr(
                    fallback,
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
                out.push_str(&format!("{pad}    local.get $__gc_option_tmp\n"));
                out.push_str(&format!("{pad}    struct.get $__gc_option_i32 0\n"));
                out.push_str(&format!("{pad}  )\n"));
                out.push_str(&format!("{pad})\n"));
            } else if matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost)
                && callee == "any"
            {
                let [receiver, callback] = args.as_slice() else {
                    bail!("`any` expects exactly two arguments in wasm backend");
                };
                let Some(_) = list_item_type(&receiver.ty) else {
                    bail!("`any` expects a list receiver in wasm backend");
                };
                let signature = abi
                    .closure_signature_for_apply(callback, callback)
                    .ok_or_else(|| {
                        anyhow!("unsupported callback shape for `any` in wasm backend")
                    })?;
                if signature.key.result != Type::Bool {
                    bail!("`any` callback must return Bool in wasm backend");
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
                    abi.any_helper_name(signature.index)
                ));
            } else if matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost)
                && callee == "filter"
            {
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
            } else if matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost)
                && callee == "sum"
            {
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
            } else if matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost)
                && callee == "join"
            {
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
            } else if matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost)
                && callee == "console.println"
            {
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
            } else if matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost)
                && callee == "string"
            {
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
            } else if matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost)
                && callee == "len"
            {
                let [value] = args.as_slice() else {
                    bail!("`len` expects exactly one argument in wasm backend");
                };
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
                match &value.ty {
                    Type::String => out.push_str(&format!("{pad}call $__strlen\n")),
                    Type::List(_) => out.push_str(&format!("{pad}i32.load\n")),
                    _ => bail!("`len` expects a String or List receiver in wasm backend"),
                }
            } else if matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost)
                && callee == "ends_with_at"
            {
                let [text, suffix, end] = args.as_slice() else {
                    bail!("`ends_with_at` expects exactly three arguments in wasm backend");
                };
                if !matches!(text.ty, Type::String) || !matches!(suffix.ty, Type::String) {
                    bail!("`ends_with_at` expects String arguments in wasm backend");
                }
                emit_expr(
                    text,
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
                    suffix,
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
                out.push_str(&format!("{pad}call $ends_with_at\n"));
            } else if abi.target == WasmTarget::Wasi && callee == "stdin.read_text" {
                if !args.is_empty() {
                    bail!("`stdin.read_text` expects no arguments in wasm backend");
                }
                out.push_str(&format!("{pad}call $stdin.read_text\n"));
            } else if abi.target == WasmTarget::Wasi && callee == "stdin.read_line" {
                if !args.is_empty() {
                    bail!("`stdin.read_line` expects no arguments in wasm backend");
                }
                out.push_str(&format!("{pad}call $stdin.read_line\n"));
            } else if abi.target == WasmTarget::Wasi && callee == "fs.read_text" {
                let [path] = args.as_slice() else {
                    bail!("`fs.read_text` expects exactly one path argument in wasm backend");
                };
                emit_expr(
                    path,
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
                out.push_str(&format!("{pad}call $fs.read_text\n"));
            } else if callee == "split_whitespace" {
                let [text] = args.as_slice() else {
                    bail!("`split_whitespace` expects exactly one string in wasm backend");
                };
                if !matches!(text.ty, Type::String) {
                    bail!("`split_whitespace` expects a String receiver in wasm backend");
                }
                emit_expr(
                    text,
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
                out.push_str(&format!("{pad}call $split_whitespace\n"));
            } else if callee == "parse.i64" {
                let [text] = args.as_slice() else {
                    bail!("`parse.i64` expects exactly one string in wasm backend");
                };
                emit_expr(
                    text,
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
                out.push_str(&format!("{pad}call $parse.i64\n"));
            } else if callee == "parse.bool" {
                let [text] = args.as_slice() else {
                    bail!("`parse.bool` expects exactly one string in wasm backend");
                };
                emit_expr(
                    text,
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
                out.push_str(&format!("{pad}call $parse.bool\n"));
            } else if matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost)
                && callee == "strip_suffix"
            {
                let [text, suffix] = args.as_slice() else {
                    bail!("`strip_suffix` expects exactly two string arguments in wasm backend");
                };
                if !matches!(text.ty, Type::String | Type::Unknown)
                    || !matches!(suffix.ty, Type::String | Type::Unknown)
                {
                    bail!("`strip_suffix` expects String arguments in wasm backend");
                }
                emit_expr(
                    text,
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
                    suffix,
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
                out.push_str(&format!("{pad}call $strip_suffix\n"));
            } else if matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost)
                && callee == "Next"
            {
                let [item, state] = args.as_slice() else {
                    bail!("`Next` expects exactly two arguments in wasm backend");
                };
                emit_iter_step(
                    0,
                    Some(item),
                    Some(state),
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
            } else if matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost)
                && callee == "Done"
            {
                if !args.is_empty() {
                    bail!("`Done` expects no arguments in wasm backend");
                }
                emit_iter_step(
                    1,
                    None,
                    None,
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
            } else {
                bail!(
                    "calls to `{callee}` are not yet supported in wasm backend for target `{}`; see docs/std.md#target-support-matrix",
                    match abi.target {
                        WasmTarget::JavaScriptHost => "wasm-js",
                        WasmTarget::JavaScriptHostGc => "wasm-js-gc",
                        WasmTarget::Wasi => "wasm-wasi",
                    }
                );
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
    ensure_wasm_list_type(list_ty, abi)?;

    let pad = "  ".repeat(indent);
    let tmp_ptr_local = heap_tmp_ptr_local(indent);
    let nested_indent = indent + 1;
    let len = u32::try_from(items.len()).expect("list length fits into u32");
    let total_bytes = 8 + len * 4;

    out.push_str(&format!("{pad}global.get $heap_ptr\n"));
    out.push_str(&format!("{pad}i32.const {total_bytes}\n"));
    out.push_str(&format!("{pad}i32.add\n"));
    out.push_str(&format!("{pad}global.set $heap_ptr\n"));
    out.push_str(&format!("{pad}global.get $heap_ptr\n"));
    out.push_str(&format!("{pad}i32.const {total_bytes}\n"));
    out.push_str(&format!("{pad}i32.sub\n"));
    out.push_str(&format!("{pad}local.set ${tmp_ptr_local}\n"));
    out.push_str(&format!("{pad}local.get ${tmp_ptr_local}\n"));
    out.push_str(&format!("{pad}i32.const {len}\n"));
    out.push_str(&format!("{pad}i32.store\n"));

    out.push_str(&format!("{pad}local.get ${tmp_ptr_local}\n"));
    out.push_str(&format!("{pad}local.get ${tmp_ptr_local}\n"));
    out.push_str(&format!("{pad}i32.const 8\n"));
    out.push_str(&format!("{pad}i32.add\n"));
    out.push_str(&format!("{pad}i32.store offset=4\n"));

    for (index, item) in items.iter().enumerate() {
        let item_offset = 8 + u32::try_from(index).expect("list index fits into u32") * 4;
        out.push_str(&format!("{pad}local.get ${tmp_ptr_local}\n"));
        out.push_str(&format!("{pad}i32.const {item_offset}\n"));
        out.push_str(&format!("{pad}i32.add\n"));
        emit_expr(
            item,
            nested_indent,
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

    out.push_str(&format!("{pad}local.get ${tmp_ptr_local}\n"));
    Ok(())
}

fn emit_tuple_literal(
    items: &[HighExpr],
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
    if !matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost) {
        bail!("tuple expressions are not yet supported in wasm backend");
    }

    let pad = "  ".repeat(indent);
    let tmp_ptr_local = heap_tmp_ptr_local(indent);
    let nested_indent = indent + 1;
    let total_bytes = u32::try_from(items.len()).expect("tuple length fits") * 4;
    out.push_str(&format!("{pad}i32.const {total_bytes}\n"));
    out.push_str(&format!("{pad}call $__alloc\n"));
    out.push_str(&format!("{pad}local.set ${tmp_ptr_local}\n"));
    for (index, item) in items.iter().enumerate() {
        let offset = u32::try_from(index).expect("tuple index fits") * 4;
        out.push_str(&format!("{pad}local.get ${tmp_ptr_local}\n"));
        out.push_str(&format!("{pad}i32.const {offset}\n"));
        out.push_str(&format!("{pad}i32.add\n"));
        emit_expr(
            item,
            nested_indent,
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
    out.push_str(&format!("{pad}local.get ${tmp_ptr_local}\n"));
    Ok(())
}

fn ensure_wasm_list_type(list_ty: &Type, abi: &WasmAbi) -> Result<()> {
    if !matches!(abi.target, WasmTarget::Wasi | WasmTarget::JavaScriptHost) {
        bail!("list expressions are not yet supported in wasm backend");
    }
    match list_ty {
        Type::List(item) if matches!(item.as_ref(), Type::Int) => Ok(()),
        Type::List(item) if matches!(item.as_ref(), Type::String) => Ok(()),
        Type::List(_) => {
            bail!("only List<i64> and List<String> are currently supported in wasm backend")
        }
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

fn emit_iter_step(
    tag: i32,
    item: Option<&HighExpr>,
    state: Option<&HighExpr>,
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
    let tmp_ptr_local = heap_tmp_ptr_local(indent);
    let nested_indent = indent + 1;
    out.push_str(&format!("{pad}i32.const 12\n"));
    out.push_str(&format!("{pad}call $__alloc\n"));
    out.push_str(&format!("{pad}local.set ${tmp_ptr_local}\n"));
    out.push_str(&format!("{pad}local.get ${tmp_ptr_local}\n"));
    out.push_str(&format!("{pad}i32.const {tag}\n"));
    out.push_str(&format!("{pad}i32.store\n"));
    if let Some(item) = item {
        out.push_str(&format!("{pad}local.get ${tmp_ptr_local}\n"));
        out.push_str(&format!("{pad}i32.const 4\n"));
        out.push_str(&format!("{pad}i32.add\n"));
        emit_expr(
            item,
            nested_indent,
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
    if let Some(state) = state {
        out.push_str(&format!("{pad}local.get ${tmp_ptr_local}\n"));
        out.push_str(&format!("{pad}i32.const 8\n"));
        out.push_str(&format!("{pad}i32.add\n"));
        emit_expr(
            state,
            nested_indent,
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
    out.push_str(&format!("{pad}local.get ${tmp_ptr_local}\n"));
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
    if abi.target == WasmTarget::JavaScriptHostGc
        && matches!(variant, "Some" | "None")
        && (is_gc_option_i32_type(expr_ty)
            || (variant == "None" && args.is_empty())
            || (variant == "Some" && matches!(args, [value] if value.ty == Type::Int)))
    {
        let pad = "  ".repeat(indent);
        match variant {
            "Some" => {
                let [value] = args else {
                    bail!("`Some` expects exactly one payload in wasm-js-gc backend");
                };
                emit_expr(
                    value,
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
                out.push_str(&format!("{pad}struct.new $__gc_option_i32\n"));
                return Ok(());
            }
            "None" => {
                if !args.is_empty() {
                    bail!("`None` must stay fieldless in wasm-js-gc backend");
                }
                out.push_str(&format!("{pad}ref.null $__gc_option_i32\n"));
                return Ok(());
            }
            _ => bail!("unknown Option variant in wasm-js-gc backend: {variant}"),
        }
    }
    let variant = abi.variant_layout_for_construct(variant, expr_ty)?;
    let pad = "  ".repeat(indent);
    let tmp_ptr_local = heap_tmp_ptr_local(indent);
    let nested_indent = indent + 1;
    if abi.target == WasmTarget::JavaScriptHost && !abi.match_subject_is_heap(expr_ty) {
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
    out.push_str(&format!("{pad}local.set ${tmp_ptr_local}\n"));
    out.push_str(&format!("{pad}local.get ${tmp_ptr_local}\n"));
    out.push_str(&format!("{pad}i32.const {}\n", variant.tag));
    out.push_str(&format!("{pad}i32.store\n"));
    for (index, arg) in args.iter().enumerate() {
        let offset = 4 + u32::try_from(index).expect("field index fits") * 4;
        out.push_str(&format!("{pad}local.get ${tmp_ptr_local}\n"));
        out.push_str(&format!("{pad}i32.const {offset}\n"));
        out.push_str(&format!("{pad}i32.add\n"));
        emit_expr(
            arg,
            nested_indent,
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
    out.push_str(&format!("{pad}local.get ${tmp_ptr_local}\n"));
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
            if abi.match_subject_is_heap(&subject.ty) {
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
    locals: &mut Vec<(String, WasmTypeRepr)>,
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
    uses_len_builtin: bool,
    uses_ends_with_at_builtin: bool,
    uses_string_eq: bool,
    uses_fs_read_text: bool,
    uses_stdin_read_text: bool,
    uses_stdin_read_line: bool,
    uses_split_whitespace_nth: bool,
    uses_parse_i64: bool,
    uses_parse_bool: bool,
    uses_list_runtime: bool,
    uses_adt_runtime: bool,
    uses_option_runtime: bool,
    uses_gc_option_i32: bool,
    uses_list_index_builtin: bool,
    uses_range_inclusive: bool,
    uses_iter_runtime: bool,
    uses_take_builtin: bool,
    uses_map_builtin: bool,
    uses_option_map_builtin: bool,
    uses_any_builtin: bool,
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
    fn from_module(
        module: &HighModule,
        wasm_module: &WasmModule,
        target: WasmTarget,
    ) -> Result<Self> {
        let specialized_functions = wasm_module.specialized_function_names();
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
        let uses_gc_option_i32 =
            target == WasmTarget::JavaScriptHostGc && module_uses_gc_option_i32(module);

        let mut uses_console_println = false;
        let mut uses_string_builtin = false;
        let mut uses_len_builtin = false;
        let mut uses_ends_with_at_builtin = false;
        let mut uses_string_eq = false;
        let mut uses_fs_read_text = false;
        let mut uses_stdin_read_text = false;
        let mut uses_stdin_read_line = false;
        let mut uses_split_whitespace = false;
        let mut uses_split_whitespace_nth = false;
        let mut uses_parse_i64 = false;
        let mut uses_parse_bool = false;
        let mut uses_list_runtime = false;
        let mut uses_adt_runtime = false;
        let mut uses_option_runtime = wasm_module.helper_usage.uses_option_runtime;
        let mut uses_list_index_builtin = false;
        let mut uses_range_inclusive = false;
        let mut uses_iter_runtime = false;
        let mut uses_take_builtin = false;
        let mut uses_map_builtin = false;
        let mut uses_option_map_builtin = false;
        let mut uses_any_builtin = false;
        let mut uses_filter_builtin = false;
        let mut uses_sum_builtin = false;
        let mut uses_join_builtin = false;
        for function in &module.functions {
            if specialized_functions.contains(&function.name) {
                continue;
            }
            scan_expr(
                &function.body,
                &mut uses_console_println,
                &mut uses_string_builtin,
                &mut uses_len_builtin,
                &mut uses_ends_with_at_builtin,
                &mut uses_string_eq,
                &mut uses_fs_read_text,
                &mut uses_stdin_read_text,
                &mut uses_stdin_read_line,
                &mut uses_split_whitespace,
                &mut uses_split_whitespace_nth,
                &mut uses_parse_i64,
                &mut uses_parse_bool,
                &mut uses_list_runtime,
                &mut uses_adt_runtime,
                &mut uses_option_runtime,
                &mut uses_list_index_builtin,
                &mut uses_range_inclusive,
                &mut uses_iter_runtime,
                &mut uses_take_builtin,
                &mut uses_map_builtin,
                &mut uses_option_map_builtin,
                &mut uses_any_builtin,
                &mut uses_filter_builtin,
                &mut uses_sum_builtin,
                &mut uses_join_builtin,
            );
        }
        if target == WasmTarget::JavaScriptHost {
            uses_fs_read_text = false;
            uses_stdin_read_text = false;
            uses_stdin_read_line = false;
            uses_adt_runtime = false;
        }
        if uses_gc_option_i32 {
            uses_adt_runtime = false;
            uses_option_runtime = false;
        }
        if wasm_module.helper_usage.uses_ends_with_at {
            uses_ends_with_at_builtin = true;
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
            uses_len_builtin,
            uses_ends_with_at_builtin,
            uses_string_eq,
            uses_fs_read_text,
            uses_stdin_read_text,
            uses_stdin_read_line,
            uses_split_whitespace_nth,
            uses_parse_i64,
            uses_parse_bool,
            uses_list_runtime,
            uses_adt_runtime,
            uses_option_runtime,
            uses_gc_option_i32,
            uses_list_index_builtin,
            uses_range_inclusive,
            uses_iter_runtime,
            uses_take_builtin,
            uses_map_builtin,
            uses_option_map_builtin,
            uses_any_builtin,
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
        abi.collect_closures(wasm_module)?;
        abi.collect_named_callbacks(module, wasm_module)?;
        Ok(abi)
    }

    fn needs_scratch(&self) -> bool {
        self.uses_string_builtin
            || (self.target == WasmTarget::Wasi
                && (self.uses_console_println
                    || self.uses_fs_read_text
                    || self.uses_stdin_read_text
                    || self.uses_stdin_read_line))
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

    fn fs_opened_fd_base(&self) -> u32 {
        self.scratch_base + 28
    }

    fn fs_iovec_base(&self) -> u32 {
        self.scratch_base + 32
    }

    fn fs_nread_base(&self) -> u32 {
        self.scratch_base + 40
    }

    fn fs_read_buffer_base(&self) -> u32 {
        self.scratch_base + 44
    }

    fn fs_read_buffer_len(&self) -> u32 {
        4096
    }

    fn heap_base(&self) -> u32 {
        let base = if self.needs_scratch() {
            self.fs_read_buffer_base() + self.fs_read_buffer_len()
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
            || self.uses_option_runtime
            || self.uses_split_whitespace_nth
            || self.uses_parse_i64
            || self.uses_parse_bool
            || self.uses_range_inclusive
            || self.uses_iter_runtime
            || self.uses_take_builtin
            || self.uses_map_builtin
            || self.uses_option_map_builtin
            || self.uses_filter_builtin
            || self.uses_sum_builtin
            || self.uses_join_builtin
            || self.uses_string_builtin
    }

    fn memory_end(&self) -> u32 {
        if self.needs_heap() {
            self.heap_base()
        } else if self.needs_scratch() {
            self.fs_read_buffer_base() + self.fs_read_buffer_len()
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

    fn required_fieldless_variant_tag(&self, name: &str) -> Result<u32> {
        let layout = self
            .variant_layout(name)
            .ok_or_else(|| anyhow!("missing variant `{name}` required by wasm backend helper"))?;
        if layout.field_count != 0 {
            bail!("variant `{name}` must stay fieldless for wasm backend helper support");
        }
        Ok(layout.tag)
    }

    fn variant_layout_for_construct(&self, variant: &str, expr_ty: &Type) -> Result<VariantLayout> {
        match expr_ty {
            Type::Result(_, _) => result_variant_layout(variant)
                .ok_or_else(|| anyhow!("unknown Result variant in wasm backend: {variant}")),
            Type::Option(_) => option_variant_layout(variant)
                .ok_or_else(|| anyhow!("unknown Option variant in wasm backend: {variant}")),
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
            Type::Option(_) => option_variant_layout(variant)
                .ok_or_else(|| anyhow!("unknown Option pattern in wasm backend: {variant}")),
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

    fn match_subject_is_heap(&self, ty: &Type) -> bool {
        match ty {
            Type::Result(_, _) => true,
            Type::Option(_) => true,
            Type::Named(name) => matches!(self.named_types.get(name), Some(NamedTypeAbi::HeapEnum)),
            _ => false,
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

    fn option_map_helper_name(&self, index: usize) -> String {
        format!("__option_map_{index}")
    }

    fn any_helper_name(&self, index: usize) -> String {
        format!("__list_any_{index}")
    }

    fn filter_helper_name(&self, index: usize) -> String {
        format!("__list_filter_{index}")
    }

    fn sum_helper_name(&self) -> &'static str {
        "__list_sum_i64"
    }

    fn wasm_type(&self, ty: &Type) -> Result<Option<WasmTypeRepr>> {
        match ty {
            Type::Unit => Ok(None),
            Type::Int | Type::Bool | Type::Fn(_, _) => Ok(Some(WasmTypeRepr::I32)),
            Type::String
                if matches!(self.target, WasmTarget::JavaScriptHost | WasmTarget::Wasi) =>
            {
                Ok(Some(WasmTypeRepr::I32))
            }
            Type::List(_)
                if matches!(self.target, WasmTarget::JavaScriptHost | WasmTarget::Wasi) =>
            {
                Ok(Some(WasmTypeRepr::I32))
            }
            Type::Tuple(_)
                if matches!(self.target, WasmTarget::JavaScriptHost | WasmTarget::Wasi) =>
            {
                Ok(Some(WasmTypeRepr::I32))
            }
            Type::Seq(_)
                if matches!(self.target, WasmTarget::JavaScriptHost | WasmTarget::Wasi) =>
            {
                Ok(Some(WasmTypeRepr::I32))
            }
            Type::Result(_, _) | Type::Option(_)
                if matches!(self.target, WasmTarget::JavaScriptHost | WasmTarget::Wasi) =>
            {
                Ok(Some(WasmTypeRepr::I32))
            }
            Type::Option(inner)
                if self.target == WasmTarget::JavaScriptHostGc && **inner == Type::Int =>
            {
                Ok(Some(WasmTypeRepr::RefNull("__gc_option_i32")))
            }
            Type::Named(name) => match self.named_types.get(name) {
                Some(NamedTypeAbi::FieldlessEnum) => Ok(Some(WasmTypeRepr::I32)),
                Some(NamedTypeAbi::HeapEnum) => Ok(Some(WasmTypeRepr::I32)),
                Some(NamedTypeAbi::Unsupported) => {
                    bail!("ADT payload fields are not yet supported in wasm backend")
                }
                None => bail!("unsupported wasm type: {ty}"),
            },
            other => bail!("unsupported wasm type: {other}"),
        }
    }

    fn collect_closures(&mut self, wasm_module: &WasmModule) -> Result<()> {
        for function in &wasm_module.functions {
            let WasmFunctionBody::High(body) = &function.body else {
                continue;
            };
            let mut scope = HashMap::new();
            for param in &function.params {
                scope.insert(param.name.clone(), param.ty.clone());
            }
            self.collect_closures_in_expr(body, Some(&function.return_type), &scope)?;
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
                        if let Some(input_ty) = callback_receiver_item_type(&receiver.ty, callee) {
                            let output_ty =
                                callback_result_type(callback, scope).unwrap_or(Type::Unknown);
                            let expected = Type::Fn(Box::new(input_ty), Box::new(output_ty));
                            self.collect_closures_in_expr(callback, Some(&expected), scope)?;
                        } else {
                            self.collect_closures_in_expr(callback, None, scope)?;
                        }
                    }
                } else if callee == "filter" || callee == "any" {
                    if let [receiver, callback] = &args[..] {
                        self.collect_closures_in_expr(receiver, None, scope)?;
                        if let Some(input_ty) = callback_receiver_item_type(&receiver.ty, callee) {
                            let expected = Type::Fn(Box::new(input_ty), Box::new(Type::Bool));
                            self.collect_closures_in_expr(callback, Some(&expected), scope)?;
                        } else {
                            self.collect_closures_in_expr(callback, None, scope)?;
                        }
                    }
                } else if callee == "iter.unfold" {
                    if let [seed, callback] = &args[..] {
                        self.collect_closures_in_expr(seed, None, scope)?;
                        let expected = Type::Fn(
                            Box::new(seed.ty.clone()),
                            Box::new(Type::Result(
                                Box::new(Type::Tuple(vec![
                                    expr_item_type(&expr.ty),
                                    seed.ty.clone(),
                                ])),
                                Box::new(Type::Unit),
                            )),
                        );
                        self.collect_closures_in_expr(callback, Some(&expected), scope)?;
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

    fn collect_named_callbacks(
        &mut self,
        module: &HighModule,
        wasm_module: &WasmModule,
    ) -> Result<()> {
        for function in &wasm_module.functions {
            let WasmFunctionBody::High(body) = &function.body else {
                continue;
            };
            self.collect_named_callbacks_in_expr(body, module)?;
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
                } else if callee == "map" || callee == "filter" || callee == "any" {
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

        let Some(input_ty) = callback_receiver_item_type(&receiver.ty, callee) else {
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

        let result_ty = if callee == "filter" || callee == "any" {
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
        if abi.uses_option_map_builtin {
            emit_option_map_helper(abi, signature, out)?;
        }
        if abi.uses_any_builtin {
            emit_any_helper(abi, signature, out)?;
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
    if abi.uses_take_builtin {
        emit_apply_dynamic_helper(abi, out)?;
        emit_take_helper(out);
    }
    if abi.uses_range_inclusive {
        emit_range_inclusive_helper(out);
    }
    if abi.uses_iter_runtime {
        emit_iter_unfold_helper(out);
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
    out.push_str("    (local $needed_end i32)\n");
    out.push_str("    (local $current_bytes i32)\n");
    out.push_str("    (local $grow_bytes i32)\n");
    out.push_str("    (local $grow_pages i32)\n");
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
    out.push_str("    local.set $needed_end\n");
    out.push_str("    memory.size\n");
    out.push_str("    i32.const 16\n");
    out.push_str("    i32.shl\n");
    out.push_str("    local.set $current_bytes\n");
    out.push_str("    local.get $needed_end\n");
    out.push_str("    local.get $current_bytes\n");
    out.push_str("    i32.gt_u\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        local.get $needed_end\n");
    out.push_str("        local.get $current_bytes\n");
    out.push_str("        i32.sub\n");
    out.push_str("        local.set $grow_bytes\n");
    out.push_str("        local.get $grow_bytes\n");
    out.push_str("        i32.const 65535\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.const 16\n");
    out.push_str("        i32.shr_u\n");
    out.push_str("        local.set $grow_pages\n");
    out.push_str("        local.get $grow_pages\n");
    out.push_str("        memory.grow\n");
    out.push_str("        i32.const -1\n");
    out.push_str("        i32.eq\n");
    out.push_str("        (if\n");
    out.push_str("          (then unreachable)\n");
    out.push_str("        )\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $needed_end\n");
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

fn emit_string_eq_helper(out: &mut String) {
    out.push_str("  (func $__streq (param $left i32) (param $right i32) (result i32)\n");
    out.push_str("    (local $left_len i32)\n");
    out.push_str("    (local $right_len i32)\n");
    out.push_str("    (local $i i32)\n");
    out.push_str("    (local $equal i32)\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    local.set $equal\n");
    out.push_str("    local.get $left\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $left_len\n");
    out.push_str("    local.get $right\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $right_len\n");
    out.push_str("    local.get $left_len\n");
    out.push_str("    local.get $right_len\n");
    out.push_str("    i32.ne\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        return\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    (block $done\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $i\n");
    out.push_str("        local.get $left_len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $done\n");
    out.push_str("        local.get $left\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        local.get $right\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.ne\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            local.set $equal\n");
    out.push_str("            br $done\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $i\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $equal\n");
    out.push_str("  )\n");
}

fn emit_ends_with_at_helper(out: &mut String) {
    out.push_str("  (func $ends_with_at (param $text i32) (param $suffix i32) (param $end i32) (result i32)\n");
    out.push_str("    (local $suffix_len i32)\n");
    out.push_str("    (local $start i32)\n");
    out.push_str("    (local $i i32)\n");
    out.push_str("    (local $matches i32)\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    local.set $matches\n");
    out.push_str("    local.get $end\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.lt_s\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        return\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $suffix\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $suffix_len\n");
    out.push_str("    local.get $suffix_len\n");
    out.push_str("    local.get $end\n");
    out.push_str("    i32.gt_u\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        return\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $end\n");
    out.push_str("    local.get $suffix_len\n");
    out.push_str("    i32.sub\n");
    out.push_str("    local.set $start\n");
    out.push_str("    (block $done\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $i\n");
    out.push_str("        local.get $suffix_len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $done\n");
    out.push_str("        local.get $text\n");
    out.push_str("        local.get $start\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        local.get $suffix\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.ne\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            local.set $matches\n");
    out.push_str("            br $done\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $i\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $matches\n");
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

fn emit_list_get_helper(out: &mut String) {
    out.push_str("  (func $__list_get (param $list i32) (param $index i32) (result i32)\n");
    out.push_str("    local.get $index\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.lt_s\n");
    out.push_str("    (if (result i32)\n");
    out.push_str("      (then i32.const 0)\n");
    out.push_str("      (else\n");
    out.push_str("        local.get $index\n");
    out.push_str("        local.get $list\n");
    out.push_str("        i32.load\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        (if (result i32)\n");
    out.push_str("          (then i32.const 0)\n");
    out.push_str("          (else\n");
    out.push_str("            local.get $list\n");
    out.push_str("            i32.load offset=4\n");
    out.push_str("            local.get $index\n");
    out.push_str("            i32.const 4\n");
    out.push_str("            i32.mul\n");
    out.push_str("            i32.add\n");
    out.push_str("            i32.load\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
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

fn emit_apply_dynamic_helper(abi: &WasmAbi, out: &mut String) -> Result<()> {
    let signature = abi
        .closure_signatures
        .first()
        .ok_or_else(|| anyhow!("iterator lowering requires at least one closure signature"))?;
    out.push_str(
        "  (func $__apply_closure_dyn (param $closure i32) (param $arg i32) (result i32)\n",
    );
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

fn emit_take_helper(out: &mut String) {
    out.push_str("  (func $__iter_take_i64 (param $iter i32) (param $limit i32) (result i32)\n");
    out.push_str("    (local $state i32)\n");
    out.push_str("    (local $callback i32)\n");
    out.push_str("    (local $items i32)\n");
    out.push_str("    (local $list i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $step i32)\n");
    out.push_str("    local.get $limit\n");
    out.push_str("    i32.const 4\n");
    out.push_str("    i32.mul\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $items\n");
    out.push_str("    local.get $iter\n");
    out.push_str("    i32.load\n");
    out.push_str("    local.set $state\n");
    out.push_str("    local.get $iter\n");
    out.push_str("    i32.load offset=4\n");
    out.push_str("    local.set $callback\n");
    out.push_str("    (block $break\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $len\n");
    out.push_str("        local.get $limit\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $break\n");
    out.push_str("        local.get $callback\n");
    out.push_str("        local.get $state\n");
    out.push_str("        call $__apply_closure_dyn\n");
    out.push_str("        local.set $step\n");
    out.push_str("        local.get $step\n");
    out.push_str("        i32.load\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.eq\n");
    out.push_str("        br_if $break\n");
    out.push_str("        local.get $items\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $step\n");
    out.push_str("        i32.load offset=4\n");
    out.push_str("        i32.store\n");
    out.push_str("        local.get $step\n");
    out.push_str("        i32.load offset=8\n");
    out.push_str("        local.set $state\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $len\n");
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

fn emit_option_map_helper(
    abi: &WasmAbi,
    signature: &ClosureSignature,
    out: &mut String,
) -> Result<()> {
    let _ = abi
        .wasm_type(&signature.key.arg)?
        .ok_or_else(|| anyhow!("unsupported option map argument type in wasm backend"))?;
    let _ = abi
        .wasm_type(&signature.key.result)?
        .ok_or_else(|| anyhow!("unsupported option map result type in wasm backend"))?;
    out.push_str(&format!(
        "  (func ${} (param $option i32) (param $callback i32) (result i32)\n",
        abi.option_map_helper_name(signature.index)
    ));
    out.push_str("    (local $out i32)\n");
    out.push_str("    local.get $option\n");
    out.push_str("    i32.load\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.eq\n");
    out.push_str("    (if (result i32)\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const 8\n");
    out.push_str("        call $__alloc\n");
    out.push_str("        local.set $out\n");
    out.push_str("        local.get $out\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        i32.store\n");
    out.push_str("        local.get $out\n");
    out.push_str("        local.get $callback\n");
    out.push_str("        local.get $option\n");
    out.push_str("        i32.load offset=4\n");
    out.push_str(&format!(
        "        call ${}\n",
        abi.apply_helper_name(signature.index)
    ));
    out.push_str("        i32.store offset=4\n");
    out.push_str("        local.get $out\n");
    out.push_str("      )\n");
    out.push_str("      (else\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        call $__alloc\n");
    out.push_str("        local.set $out\n");
    out.push_str("        local.get $out\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.store\n");
    out.push_str("        local.get $out\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("  )\n");
    Ok(())
}

fn emit_any_helper(abi: &WasmAbi, signature: &ClosureSignature, out: &mut String) -> Result<()> {
    let _ = abi
        .wasm_type(&signature.key.arg)?
        .ok_or_else(|| anyhow!("unsupported any argument type in wasm backend"))?;
    out.push_str(&format!(
        "  (func ${} (param $list i32) (param $callback i32) (result i32)\n",
        abi.any_helper_name(signature.index)
    ));
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $items i32)\n");
    out.push_str("    (local $index i32)\n");
    out.push_str("    (local $value i32)\n");
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
    out.push_str("        local.get $items\n");
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
    out.push_str("        i32.const 0\n");
    out.push_str("        i32.ne\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            return\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 0\n");
    out.push_str("  )\n");
    Ok(())
}

fn emit_filter_helper(abi: &WasmAbi, signature: &ClosureSignature, out: &mut String) -> Result<()> {
    let arg_ty = abi
        .wasm_type(&signature.key.arg)?
        .ok_or_else(|| anyhow!("unsupported filter argument type in wasm backend"))?;
    if arg_ty != WasmTypeRepr::I32 || signature.key.result != Type::Bool {
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

fn emit_iter_unfold_helper(out: &mut String) {
    out.push_str(
        "  (func $__iter_unfold_new (param $state i32) (param $callback i32) (result i32)\n",
    );
    out.push_str("    (local $iter i32)\n");
    out.push_str("    i32.const 8\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $iter\n");
    out.push_str("    local.get $iter\n");
    out.push_str("    local.get $state\n");
    out.push_str("    i32.store\n");
    out.push_str("    local.get $iter\n");
    out.push_str("    local.get $callback\n");
    out.push_str("    i32.store offset=4\n");
    out.push_str("    local.get $iter\n");
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
    if abi.needs_heap() {
        for index in 0..HEAP_TMP_PTR_LOCAL_COUNT {
            out.push_str(&format!("    (local $__tmp_ptr{index} i32)\n"));
        }
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
    uses_len_builtin: &mut bool,
    uses_ends_with_at_builtin: &mut bool,
    uses_string_eq: &mut bool,
    uses_fs_read_text: &mut bool,
    uses_stdin_read_text: &mut bool,
    uses_stdin_read_line: &mut bool,
    uses_split_whitespace: &mut bool,
    uses_split_whitespace_nth: &mut bool,
    uses_parse_i64: &mut bool,
    uses_parse_bool: &mut bool,
    uses_list_runtime: &mut bool,
    uses_adt_runtime: &mut bool,
    uses_option_runtime: &mut bool,
    uses_list_index_builtin: &mut bool,
    uses_range_inclusive: &mut bool,
    uses_iter_runtime: &mut bool,
    uses_take_builtin: &mut bool,
    uses_map_builtin: &mut bool,
    uses_option_map_builtin: &mut bool,
    uses_any_builtin: &mut bool,
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
            if callee == "len" {
                *uses_len_builtin = true;
            }
            if callee == "ends_with_at" {
                *uses_ends_with_at_builtin = true;
            }
            if callee == "fs.read_text" {
                *uses_fs_read_text = true;
                *uses_adt_runtime = true;
            }
            if callee == "stdin.read_text" {
                *uses_stdin_read_text = true;
            }
            if callee == "stdin.read_line" {
                *uses_stdin_read_line = true;
            }
            if callee == "split_whitespace" {
                *uses_split_whitespace = true;
                *uses_list_runtime = true;
            }
            if callee == "strip_suffix" {
                *uses_option_runtime = true;
            }
            if callee == "unwrap_or" {
                *uses_option_runtime = true;
            }
            if callee == "parse.i64" {
                *uses_parse_i64 = true;
                *uses_adt_runtime = true;
            }
            if callee == "parse.bool" {
                *uses_parse_bool = true;
                *uses_adt_runtime = true;
            }
            if callee == "range_inclusive" {
                *uses_list_runtime = true;
                *uses_range_inclusive = true;
            }
            if callee == "iter.unfold" {
                *uses_iter_runtime = true;
            }
            if callee == "take" {
                *uses_iter_runtime = true;
                *uses_take_builtin = true;
            }
            if callee == "map" {
                match args.first().map(|arg| &arg.ty) {
                    Some(Type::List(_)) => {
                        *uses_list_runtime = true;
                        *uses_map_builtin = true;
                    }
                    Some(Type::Option(_)) => {
                        *uses_option_runtime = true;
                        *uses_option_map_builtin = true;
                    }
                    _ => {}
                }
            }
            if callee == "any" {
                *uses_list_runtime = true;
                *uses_any_builtin = true;
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
            if callee == "__index" {
                if matches_split_whitespace_nth_expr(args.first()).is_some() {
                    *uses_split_whitespace_nth = true;
                } else if matches!(args.first().map(|arg| &arg.ty), Some(Type::List(_))) {
                    *uses_list_runtime = true;
                    *uses_list_index_builtin = true;
                }
            }
            for arg in args {
                scan_expr(
                    arg,
                    uses_console_println,
                    uses_string_builtin,
                    uses_len_builtin,
                    uses_ends_with_at_builtin,
                    uses_string_eq,
                    uses_fs_read_text,
                    uses_stdin_read_text,
                    uses_stdin_read_line,
                    uses_split_whitespace,
                    uses_split_whitespace_nth,
                    uses_parse_i64,
                    uses_parse_bool,
                    uses_list_runtime,
                    uses_adt_runtime,
                    uses_option_runtime,
                    uses_list_index_builtin,
                    uses_range_inclusive,
                    uses_iter_runtime,
                    uses_take_builtin,
                    uses_map_builtin,
                    uses_option_map_builtin,
                    uses_any_builtin,
                    uses_filter_builtin,
                    uses_sum_builtin,
                    uses_join_builtin,
                );
            }
        }
        HighExprKind::Binary { left, right, op } => {
            if *op == lang_core::BinaryOp::Equal
                && left.ty == Type::String
                && right.ty == Type::String
            {
                *uses_string_eq = true;
            }
            scan_expr(
                left,
                uses_console_println,
                uses_string_builtin,
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
                uses_list_index_builtin,
                uses_range_inclusive,
                uses_iter_runtime,
                uses_take_builtin,
                uses_map_builtin,
                uses_option_map_builtin,
                uses_any_builtin,
                uses_filter_builtin,
                uses_sum_builtin,
                uses_join_builtin,
            );
            scan_expr(
                right,
                uses_console_println,
                uses_string_builtin,
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
                uses_list_index_builtin,
                uses_range_inclusive,
                uses_iter_runtime,
                uses_take_builtin,
                uses_map_builtin,
                uses_option_map_builtin,
                uses_any_builtin,
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
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
                uses_list_index_builtin,
                uses_range_inclusive,
                uses_iter_runtime,
                uses_take_builtin,
                uses_map_builtin,
                uses_option_map_builtin,
                uses_any_builtin,
                uses_filter_builtin,
                uses_sum_builtin,
                uses_join_builtin,
            );
            scan_expr(
                then_branch,
                uses_console_println,
                uses_string_builtin,
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
                uses_list_index_builtin,
                uses_range_inclusive,
                uses_iter_runtime,
                uses_take_builtin,
                uses_map_builtin,
                uses_option_map_builtin,
                uses_any_builtin,
                uses_filter_builtin,
                uses_sum_builtin,
                uses_join_builtin,
            );
            scan_expr(
                else_branch,
                uses_console_println,
                uses_string_builtin,
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
                uses_list_index_builtin,
                uses_range_inclusive,
                uses_iter_runtime,
                uses_take_builtin,
                uses_map_builtin,
                uses_option_map_builtin,
                uses_any_builtin,
                uses_filter_builtin,
                uses_sum_builtin,
                uses_join_builtin,
            );
        }
        HighExprKind::Match { subject, arms } => {
            *uses_adt_runtime = true;
            if matches!(subject.ty, Type::Option(_)) {
                *uses_option_runtime = true;
            }
            scan_expr(
                subject,
                uses_console_println,
                uses_string_builtin,
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
                uses_list_index_builtin,
                uses_range_inclusive,
                uses_iter_runtime,
                uses_take_builtin,
                uses_map_builtin,
                uses_option_map_builtin,
                uses_any_builtin,
                uses_filter_builtin,
                uses_sum_builtin,
                uses_join_builtin,
            );
            for arm in arms {
                scan_expr(
                    &arm.expr,
                    uses_console_println,
                    uses_string_builtin,
                    uses_len_builtin,
                    uses_ends_with_at_builtin,
                    uses_string_eq,
                    uses_fs_read_text,
                    uses_stdin_read_text,
                    uses_stdin_read_line,
                    uses_split_whitespace,
                    uses_split_whitespace_nth,
                    uses_parse_i64,
                    uses_parse_bool,
                    uses_list_runtime,
                    uses_adt_runtime,
                    uses_option_runtime,
                    uses_list_index_builtin,
                    uses_range_inclusive,
                    uses_iter_runtime,
                    uses_take_builtin,
                    uses_map_builtin,
                    uses_option_map_builtin,
                    uses_any_builtin,
                    uses_filter_builtin,
                    uses_sum_builtin,
                    uses_join_builtin,
                );
            }
        }
        HighExprKind::Construct { args, .. } => {
            *uses_adt_runtime = true;
            if matches!(expr.ty, Type::Option(_)) {
                *uses_option_runtime = true;
            }
            for arg in args {
                scan_expr(
                    arg,
                    uses_console_println,
                    uses_string_builtin,
                    uses_len_builtin,
                    uses_ends_with_at_builtin,
                    uses_string_eq,
                    uses_fs_read_text,
                    uses_stdin_read_text,
                    uses_stdin_read_line,
                    uses_split_whitespace,
                    uses_split_whitespace_nth,
                    uses_parse_i64,
                    uses_parse_bool,
                    uses_list_runtime,
                    uses_adt_runtime,
                    uses_option_runtime,
                    uses_list_index_builtin,
                    uses_range_inclusive,
                    uses_iter_runtime,
                    uses_take_builtin,
                    uses_map_builtin,
                    uses_option_map_builtin,
                    uses_any_builtin,
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
                    uses_len_builtin,
                    uses_ends_with_at_builtin,
                    uses_string_eq,
                    uses_fs_read_text,
                    uses_stdin_read_text,
                    uses_stdin_read_line,
                    uses_split_whitespace,
                    uses_split_whitespace_nth,
                    uses_parse_i64,
                    uses_parse_bool,
                    uses_list_runtime,
                    uses_adt_runtime,
                    uses_option_runtime,
                    uses_list_index_builtin,
                    uses_range_inclusive,
                    uses_iter_runtime,
                    uses_take_builtin,
                    uses_map_builtin,
                    uses_option_map_builtin,
                    uses_any_builtin,
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
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
                uses_list_index_builtin,
                uses_range_inclusive,
                uses_iter_runtime,
                uses_take_builtin,
                uses_map_builtin,
                uses_option_map_builtin,
                uses_any_builtin,
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
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
                uses_list_index_builtin,
                uses_range_inclusive,
                uses_iter_runtime,
                uses_take_builtin,
                uses_map_builtin,
                uses_option_map_builtin,
                uses_any_builtin,
                uses_filter_builtin,
                uses_sum_builtin,
                uses_join_builtin,
            );
            scan_expr(
                body,
                uses_console_println,
                uses_string_builtin,
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
                uses_list_index_builtin,
                uses_range_inclusive,
                uses_iter_runtime,
                uses_take_builtin,
                uses_map_builtin,
                uses_option_map_builtin,
                uses_any_builtin,
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
            if name == "None" {
                *uses_option_runtime = true;
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

fn option_item_type(ty: &Type) -> Option<Type> {
    if let Type::Option(item) = ty {
        Some((**item).clone())
    } else {
        None
    }
}

fn callback_receiver_item_type(ty: &Type, callee: &str) -> Option<Type> {
    match callee {
        "map" => list_item_type(ty).or_else(|| option_item_type(ty)),
        "filter" | "any" => list_item_type(ty),
        _ => None,
    }
}

fn expr_item_type(ty: &Type) -> Type {
    match ty {
        Type::List(item) | Type::Seq(item) => (**item).clone(),
        _ => Type::Unknown,
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

fn option_variant_layout(name: &str) -> Option<VariantLayout> {
    match name {
        "Some" => Some(VariantLayout {
            owner_type: "__option".to_owned(),
            tag: 0,
            field_count: 1,
        }),
        "None" => Some(VariantLayout {
            owner_type: "__option".to_owned(),
            tag: 1,
            field_count: 0,
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
    if abi.target == WasmTarget::Wasi && abi.uses_console_println {
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
