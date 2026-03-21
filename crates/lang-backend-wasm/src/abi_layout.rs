use super::*;

fn usize_to_u32(value: usize, what: &str) -> Result<u32> {
    u32::try_from(value).map_err(|_| anyhow::anyhow!("{what} exceeds u32 range in wasm backend"))
}

#[derive(Clone, Debug)]
pub(crate) struct ClosureCapture {
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) offset: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WasmTypeRepr {
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

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ClosureSignatureKey {
    pub(crate) arg: Type,
    pub(crate) result: Type,
}

#[derive(Clone, Debug)]
pub(crate) struct ClosureSignature {
    pub(crate) index: usize,
    pub(crate) key: ClosureSignatureKey,
}

#[derive(Clone, Debug)]
pub(crate) struct ClosureThunk {
    pub(crate) expr_id: usize,
    pub(crate) func_name: String,
    pub(crate) alloc_name: String,
    pub(crate) signature_index: usize,
    pub(crate) table_index: u32,
    pub(crate) param_name: String,
    pub(crate) body: HighExpr,
    pub(crate) captures: Vec<ClosureCapture>,
    pub(crate) env_size: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct NamedCallbackThunk {
    pub(crate) func_name: String,
    pub(crate) alloc_name: String,
    pub(crate) signature_index: usize,
    pub(crate) table_index: u32,
    pub(crate) target: NamedCallbackTarget,
}

#[derive(Clone, Debug)]
pub(crate) enum NamedCallbackTarget {
    Function(String),
    BuiltinString,
}

#[derive(Debug)]
pub(crate) struct WasmAbi {
    pub(crate) target: WasmTarget,
    pub(crate) string_table: StringTable,
    pub(crate) named_types: HashMap<String, NamedTypeAbi>,
    pub(crate) variants: HashMap<String, VariantLayout>,
    pub(crate) uses_console_println: bool,
    pub(crate) uses_string_builtin: bool,
    pub(crate) uses_len_builtin: bool,
    pub(crate) uses_ends_with_at_builtin: bool,
    pub(crate) uses_string_eq: bool,
    pub(crate) uses_fs_read_text: bool,
    pub(crate) uses_stdin_read_text: bool,
    pub(crate) uses_stdin_read_line: bool,
    pub(crate) uses_split_whitespace_nth: bool,
    pub(crate) uses_parse_i64: bool,
    pub(crate) uses_parse_bool: bool,
    pub(crate) uses_list_runtime: bool,
    pub(crate) uses_adt_runtime: bool,
    pub(crate) uses_option_runtime: bool,
    pub(crate) uses_gc_option_i32: bool,
    pub(crate) uses_list_index_builtin: bool,
    pub(crate) uses_range_inclusive: bool,
    pub(crate) uses_iter_runtime: bool,
    pub(crate) uses_take_builtin: bool,
    pub(crate) uses_map_builtin: bool,
    pub(crate) uses_option_map_builtin: bool,
    pub(crate) uses_any_builtin: bool,
    pub(crate) uses_filter_builtin: bool,
    pub(crate) uses_sum_builtin: bool,
    pub(crate) uses_join_builtin: bool,
    pub(crate) scratch_base: u32,
    pub(crate) closure_signatures: Vec<ClosureSignature>,
    pub(crate) closure_signature_ids: HashMap<ClosureSignatureKey, usize>,
    pub(crate) closure_thunks: Vec<ClosureThunk>,
    pub(crate) closure_expr_map: HashMap<usize, usize>,
    pub(crate) named_callback_thunks: Vec<NamedCallbackThunk>,
    pub(crate) named_callback_map: HashMap<String, usize>,
}

impl WasmAbi {
    pub(crate) fn from_module(
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
                        tag: usize_to_u32(tag, "variant tag")?,
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

    pub(crate) fn needs_scratch(&self) -> bool {
        self.uses_string_builtin
            || (self.target == WasmTarget::Wasi
                && (self.uses_console_println
                    || self.uses_fs_read_text
                    || self.uses_stdin_read_text
                    || self.uses_stdin_read_line))
    }

    pub(crate) fn iovec_base(&self) -> u32 {
        self.scratch_base
    }

    pub(crate) fn nwritten_base(&self) -> u32 {
        self.scratch_base + 8
    }

    pub(crate) fn newline_base(&self) -> u32 {
        self.scratch_base + 12
    }

    pub(crate) fn str_buf_end(&self) -> u32 {
        self.scratch_base + 28
    }

    pub(crate) fn fs_opened_fd_base(&self) -> u32 {
        self.scratch_base + 28
    }

    pub(crate) fn fs_iovec_base(&self) -> u32 {
        self.scratch_base + 32
    }

    pub(crate) fn fs_nread_base(&self) -> u32 {
        self.scratch_base + 40
    }

    pub(crate) fn fs_read_buffer_base(&self) -> u32 {
        self.scratch_base + 44
    }

    pub(crate) fn fs_read_buffer_len(&self) -> u32 {
        4096
    }

    pub(crate) fn heap_base(&self) -> u32 {
        let base = if self.needs_scratch() {
            self.fs_read_buffer_base() + self.fs_read_buffer_len()
        } else {
            self.string_table.next_offset
        };
        (base + 3) & !3
    }

    pub(crate) fn needs_heap(&self) -> bool {
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

    pub(crate) fn memory_end(&self) -> u32 {
        if self.needs_heap() {
            self.heap_base()
        } else if self.needs_scratch() {
            self.fs_read_buffer_base() + self.fs_read_buffer_len()
        } else {
            self.string_table.next_offset
        }
    }

    pub(crate) fn needs_memory(&self) -> bool {
        !self.string_table.is_empty() || self.needs_scratch() || self.needs_heap()
    }

    pub(crate) fn variant_layout(&self, name: &str) -> Option<&VariantLayout> {
        self.variants.get(name)
    }

    pub(crate) fn required_fieldless_variant_tag(&self, name: &str) -> Result<u32> {
        let layout = self
            .variant_layout(name)
            .ok_or_else(|| anyhow!("missing variant `{name}` required by wasm backend helper"))?;
        if layout.field_count != 0 {
            bail!("variant `{name}` must stay fieldless for wasm backend helper support");
        }
        Ok(layout.tag)
    }

    pub(crate) fn variant_layout_for_construct(
        &self,
        variant: &str,
        expr_ty: &Type,
    ) -> Result<VariantLayout> {
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

    pub(crate) fn variant_layout_for_match(
        &self,
        variant: &str,
        subject_ty: &Type,
    ) -> Result<VariantLayout> {
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

    pub(crate) fn match_subject_is_heap(&self, ty: &Type) -> bool {
        match ty {
            Type::Result(_, _) => true,
            Type::Option(_) => true,
            Type::Named(name) => matches!(self.named_types.get(name), Some(NamedTypeAbi::HeapEnum)),
            _ => false,
        }
    }

    pub(crate) fn closure_for_expr(&self, expr: &HighExpr) -> Option<&ClosureThunk> {
        let expr_id = expr as *const HighExpr as usize;
        self.closure_expr_map
            .get(&expr_id)
            .and_then(|index| self.closure_thunks.get(*index))
    }

    pub(crate) fn named_callback_for(&self, name: &str) -> Option<&NamedCallbackThunk> {
        self.named_callback_map
            .get(name)
            .and_then(|index| self.named_callback_thunks.get(*index))
    }

    pub(crate) fn closure_signature_for_apply(
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

    pub(crate) fn closure_type_name(&self, index: usize) -> String {
        format!("__closure_type_{index}")
    }

    pub(crate) fn apply_helper_name(&self, index: usize) -> String {
        format!("__apply_closure_{index}")
    }

    pub(crate) fn map_helper_name(&self, index: usize) -> String {
        format!("__list_map_{index}")
    }

    pub(crate) fn option_map_helper_name(&self, index: usize) -> String {
        format!("__option_map_{index}")
    }

    pub(crate) fn any_helper_name(&self, index: usize) -> String {
        format!("__list_any_{index}")
    }

    pub(crate) fn filter_helper_name(&self, index: usize) -> String {
        format!("__list_filter_{index}")
    }

    pub(crate) fn sum_helper_name(&self) -> &'static str {
        "__list_sum_i64"
    }

    pub(crate) fn wasm_type(&self, ty: &Type) -> Result<Option<WasmTypeRepr>> {
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NamedTypeAbi {
    FieldlessEnum,
    HeapEnum,
    Unsupported,
}

#[derive(Clone, Debug)]
pub(crate) struct VariantLayout {
    pub(crate) owner_type: String,
    pub(crate) tag: u32,
    pub(crate) field_count: usize,
}

pub(crate) fn result_variant_layout(name: &str) -> Option<VariantLayout> {
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

pub(crate) fn option_variant_layout(name: &str) -> Option<VariantLayout> {
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
pub(crate) struct StringTable {
    pub(crate) offsets: HashMap<String, u32>,
    pub(crate) next_offset: u32,
}

impl StringTable {
    pub(crate) fn collect(module: &HighModule) -> Self {
        let mut table = Self::default();
        for function in &module.functions {
            table.collect_expr(&function.body);
        }
        table
    }

    pub(crate) fn collect_expr(&mut self, expr: &HighExpr) {
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

    pub(crate) fn intern(&mut self, text: &str) -> u32 {
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

    pub(crate) fn offset_for(&self, text: &str) -> Option<u32> {
        self.offsets.get(text).copied()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }
}
