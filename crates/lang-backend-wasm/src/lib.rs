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
    let abi = WasmAbi::from_module(module, target);
    let mut wat_module = String::from("(module\n");
    emit_string_memory(&abi.string_table, &mut wat_module);
    let function_names = module
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect::<HashSet<_>>();
    for function in &module.functions {
        wat_module.push_str(&emit_function(function, &function_names, &abi)?);
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
    emit_expr(&function.body, 2, &locals, function_names, abi, &mut out)?;
    if abi.wasm_type(&function.return_type)?.is_none()
        && abi.wasm_type(&function.body.ty)?.is_some()
    {
        out.push_str("    drop\n");
    }
    out.push_str("  )\n");
    Ok(out)
}

fn emit_expr(
    expr: &HighExpr,
    indent: usize,
    locals: &HashSet<String>,
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
        HighExprKind::List(_)
        | HighExprKind::Tuple(_)
        | HighExprKind::Lambda { .. }
        | HighExprKind::Let { .. } => {
            bail!("list, tuple, lambda, and let expressions are not yet supported in wasm backend");
        }
        HighExprKind::Ident(name) => {
            if locals.contains(name) {
                out.push_str(&format!("{pad}local.get ${name}\n"));
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
            emit_expr(left, indent, locals, function_names, abi, out)?;
            emit_expr(right, indent, locals, function_names, abi, out)?;
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
            emit_expr(condition, indent, locals, function_names, abi, out)?;
            out.push_str(&format!("{pad}(if"));
            if let Some(result_ty) = abi.wasm_type(&expr.ty)? {
                out.push_str(&format!(" (result {result_ty})"));
            }
            out.push('\n');
            out.push_str(&format!("{pad}  (then\n"));
            emit_expr(then_branch, indent + 2, locals, function_names, abi, out)?;
            out.push_str(&format!("{pad}  )\n"));
            out.push_str(&format!("{pad}  (else\n"));
            emit_expr(else_branch, indent + 2, locals, function_names, abi, out)?;
            out.push_str(&format!("{pad}  )\n"));
            out.push_str(&format!("{pad})\n"));
        }
        HighExprKind::Call { callee, args } => {
            if function_names.contains(callee) {
                for arg in args {
                    emit_expr(arg, indent, locals, function_names, abi, out)?;
                }
                out.push_str(&format!("{pad}call ${callee}\n"));
            } else {
                bail!("calls to `{callee}` are not yet supported in wasm backend");
            }
        }
        HighExprKind::Match { subject, arms } => {
            emit_match_arms(
                subject,
                arms,
                &expr.ty,
                indent,
                locals,
                function_names,
                abi,
                out,
            )?;
        }
        HighExprKind::Construct { variant, args } => {
            emit_construct(variant, args, indent, locals, function_names, abi, out)?;
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

fn emit_construct(
    variant: &str,
    args: &[HighExpr],
    indent: usize,
    locals: &HashSet<String>,
    function_names: &HashSet<String>,
    abi: &WasmAbi,
    out: &mut String,
) -> Result<()> {
    let variant = abi
        .variant_layout(variant)
        .ok_or_else(|| anyhow!("unknown variant in wasm backend: {variant}"))?;
    if variant.field_count != 0 || !args.is_empty() {
        bail!("ADT payload fields are not yet supported in wasm backend");
    }
    let pad = "  ".repeat(indent);
    out.push_str(&format!("{pad}i32.const {}\n", variant.tag));

    let _ = (locals, function_names);
    Ok(())
}

fn emit_match_arms(
    subject: &HighExpr,
    arms: &[HighMatchArm],
    match_ty: &Type,
    indent: usize,
    locals: &HashSet<String>,
    function_names: &HashSet<String>,
    abi: &WasmAbi,
    out: &mut String,
) -> Result<()> {
    let (arm, rest) = arms
        .split_first()
        .ok_or_else(|| anyhow!("match lowering reached an empty arm list in wasm backend"))?;
    match &arm.pattern {
        Pattern::Wildcard => emit_expr(&arm.expr, indent, locals, function_names, abi, out),
        Pattern::Variant { name, bindings } => {
            let subject_type = match &subject.ty {
                Type::Named(name) if abi.supports_fieldless_enum(name) => name,
                _ => {
                    bail!(
                        "match is not yet supported in wasm backend except for fieldless user-defined ADTs"
                    )
                }
            };
            let variant = abi
                .variant_layout(name)
                .ok_or_else(|| anyhow!("unknown variant in wasm backend: {name}"))?;
            if variant.owner_type != *subject_type {
                bail!("match pattern variant does not match the subject type in wasm backend");
            }
            if variant.field_count != 0 || !bindings.is_empty() {
                bail!("ADT payload fields are not yet supported in wasm backend");
            }
            if rest.is_empty() {
                return emit_expr(&arm.expr, indent, locals, function_names, abi, out);
            }

            let pad = "  ".repeat(indent);
            emit_expr(subject, indent, locals, function_names, abi, out)?;
            out.push_str(&format!("{pad}i32.const {}\n", variant.tag));
            out.push_str(&format!("{pad}i32.eq\n"));
            out.push_str(&format!("{pad}(if"));
            if let Some(result_ty) = abi.wasm_type(match_ty)? {
                out.push_str(&format!(" (result {result_ty})"));
            }
            out.push('\n');
            out.push_str(&format!("{pad}  (then\n"));
            emit_expr(&arm.expr, indent + 2, locals, function_names, abi, out)?;
            out.push_str(&format!("{pad}  )\n"));
            out.push_str(&format!("{pad}  (else\n"));
            emit_match_arms(
                subject,
                rest,
                match_ty,
                indent + 2,
                locals,
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

#[derive(Debug)]
struct WasmAbi {
    target: WasmTarget,
    string_table: StringTable,
    named_types: HashMap<String, NamedTypeAbi>,
    variants: HashMap<String, VariantLayout>,
}

impl WasmAbi {
    fn from_module(module: &HighModule, target: WasmTarget) -> Self {
        let mut named_types = HashMap::new();
        let mut variants = HashMap::new();

        for type_decl in &module.types {
            let fieldless = type_decl
                .variants
                .iter()
                .all(|variant| variant.fields.is_empty());
            named_types.insert(
                type_decl.name.clone(),
                if fieldless {
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

        Self {
            target,
            string_table: StringTable::collect(module),
            named_types,
            variants,
        }
    }

    fn supports_fieldless_enum(&self, type_name: &str) -> bool {
        matches!(
            self.named_types.get(type_name),
            Some(NamedTypeAbi::FieldlessEnum)
        )
    }

    fn variant_layout(&self, name: &str) -> Option<&VariantLayout> {
        self.variants.get(name)
    }

    fn wasm_type(&self, ty: &Type) -> Result<Option<&'static str>> {
        match ty {
            Type::Unit => Ok(None),
            Type::Int | Type::Bool => Ok(Some("i32")),
            Type::String if self.target == WasmTarget::JavaScriptHost => Ok(Some("i32")),
            Type::Named(name) => match self.named_types.get(name) {
                Some(NamedTypeAbi::FieldlessEnum) => Ok(Some("i32")),
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
enum NamedTypeAbi {
    FieldlessEnum,
    Unsupported,
}

#[derive(Clone, Debug)]
struct VariantLayout {
    owner_type: String,
    tag: u32,
    field_count: usize,
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

    fn pages(&self) -> u32 {
        let bytes = self.next_offset.max(1);
        bytes.div_ceil(65_536)
    }
}

fn emit_string_memory(strings: &StringTable, out: &mut String) {
    if strings.is_empty() {
        return;
    }

    out.push_str(&format!(
        "  (memory (export \"memory\") {})\n",
        strings.pages()
    ));
    let mut entries = strings
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
