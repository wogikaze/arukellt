use super::*;

fn usize_to_u32(value: usize, what: &str) -> Result<u32> {
    u32::try_from(value).map_err(|_| anyhow!("{what} exceeds u32 range in wasm backend"))
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

pub(crate) const HEAP_TMP_PTR_LOCAL_COUNT: usize = 16;

pub(crate) fn heap_tmp_ptr_local(indent: usize) -> String {
    format!("__tmp_ptr{}", indent.min(HEAP_TMP_PTR_LOCAL_COUNT - 1))
}

pub(crate) fn emit_javascript_exports(module: &WasmModule, out: &mut String) {
    for function in &module.functions {
        out.push_str(&format!(
            "  (export \"{}\" (func ${}))\n",
            function.name, function.name
        ));
    }
}

pub(crate) fn emit_gc_type_defs(abi: &WasmAbi, out: &mut String) {
    if abi.uses_gc_option_i32 {
        out.push_str("  (type $__gc_option_i32 (struct (field (mut i32))))\n");
    }
}

pub(crate) fn emit_heap_primitives(abi: &WasmAbi, out: &mut String) {
    if !(abi.needs_heap()
        || abi.uses_len_builtin
        || abi.uses_ends_with_at_builtin
        || (abi.target == WasmTarget::JavaScriptHost && abi.uses_console_println))
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

pub(crate) fn emit_wasi_entrypoint(
    module: &WasmModule,
    abi: &WasmAbi,
    out: &mut String,
) -> Result<()> {
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

pub(crate) fn emit_function(
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

pub(crate) fn emit_high_function(
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

pub(crate) fn emit_suffix_recursion_function(
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

pub(crate) fn emit_parse_or_zero_function(
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

pub(crate) fn emit_suffix_iteration_cases(
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
pub(crate) struct MatchBinding {
    subject: HighExpr,
    offset: u32,
}

pub(crate) fn emit_expr(
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

pub(crate) fn emit_apply(
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

pub(crate) fn emit_list_literal(
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
    let len = usize_to_u32(items.len(), "list length")?;
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
        let item_offset = 8 + usize_to_u32(index, "list index")? * 4;
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

pub(crate) fn emit_tuple_literal(
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
    let total_bytes = usize_to_u32(items.len(), "tuple length")? * 4;
    out.push_str(&format!("{pad}i32.const {total_bytes}\n"));
    out.push_str(&format!("{pad}call $__alloc\n"));
    out.push_str(&format!("{pad}local.set ${tmp_ptr_local}\n"));
    for (index, item) in items.iter().enumerate() {
        let offset = usize_to_u32(index, "tuple index")? * 4;
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

pub(crate) fn ensure_wasm_list_type(list_ty: &Type, abi: &WasmAbi) -> Result<()> {
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

pub(crate) fn emit_lambda(
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

pub(crate) fn emit_iter_step(
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

pub(crate) fn emit_construct(
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
    let total_bytes = 4 + usize_to_u32(variant.field_count, "field count")? * 4;
    out.push_str(&format!("{pad}i32.const {total_bytes}\n"));
    out.push_str(&format!("{pad}call $__alloc\n"));
    out.push_str(&format!("{pad}local.set ${tmp_ptr_local}\n"));
    out.push_str(&format!("{pad}local.get ${tmp_ptr_local}\n"));
    out.push_str(&format!("{pad}i32.const {}\n", variant.tag));
    out.push_str(&format!("{pad}i32.store\n"));
    for (index, arg) in args.iter().enumerate() {
        let offset = 4 + usize_to_u32(index, "field index")? * 4;
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

pub(crate) fn emit_match_arms(
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
                        offset: 4 + usize_to_u32(index, "field index")? * 4,
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

pub(crate) fn collect_let_locals(
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

pub(crate) fn allocate_local_name(
    name: &str,
    declared_local_names: &mut HashSet<String>,
) -> String {
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

pub(crate) fn emit_all_memory(abi: &WasmAbi, out: &mut String) {
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

pub(crate) fn wat_string_literal(text: &str) -> String {
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
