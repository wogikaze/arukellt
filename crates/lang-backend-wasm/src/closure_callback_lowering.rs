use super::*;
use crate::wat_emitter::{HEAP_TMP_PTR_LOCAL_COUNT, collect_let_locals, emit_expr};

impl WasmAbi {
    pub(crate) fn collect_closures(&mut self, wasm_module: &WasmModule) -> Result<()> {
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

    pub(crate) fn collect_closures_in_expr(
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

    pub(crate) fn intern_closure_signature(&mut self, arg: &Type, result: &Type) -> usize {
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

    pub(crate) fn collect_named_callbacks(
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

    pub(crate) fn collect_named_callbacks_in_expr(
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

    pub(crate) fn intern_named_callback(
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

    pub(crate) fn intern_named_apply_callback(
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

pub(crate) fn emit_closure_support(
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

pub(crate) fn emit_closure_thunk(
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

pub(crate) fn closure_signature_types(
    expected: Option<&Type>,
    expr_ty: &Type,
) -> Option<(Type, Type)> {
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

pub(crate) fn ordered_captures(
    body: &HighExpr,
    scope: &HashMap<String, Type>,
    param: &str,
) -> Vec<String> {
    let mut ordered = Vec::new();
    let mut seen = HashSet::new();
    let mut bound = HashSet::from([param.to_owned()]);
    collect_ordered_captures(body, scope, &mut bound, &mut seen, &mut ordered);
    ordered
}

pub(crate) fn collect_ordered_captures(
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
