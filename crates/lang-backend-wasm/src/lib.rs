use std::collections::HashSet;

use anyhow::{Result, anyhow, bail};
use lang_core::{Type, compile_module};
use lang_ir::{HighExpr, HighExprKind, HighFunction, HighModule, lower_to_high_ir};

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
    let mut wat_module = String::from("(module\n");
    let function_names = module
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect::<HashSet<_>>();
    for function in &module.functions {
        wat_module.push_str(&emit_function(function, &function_names)?);
    }
    match target {
        WasmTarget::JavaScriptHost => emit_javascript_exports(module, &mut wat_module),
        WasmTarget::Wasi => emit_wasi_entrypoint(module, &mut wat_module)?,
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

fn emit_wasi_entrypoint(module: &HighModule, out: &mut String) -> Result<()> {
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
    if wasm_type(&main.return_type)?.is_some() {
        out.push_str("    drop\n");
    }
    out.push_str("  )\n");
    out.push_str("  (export \"_start\" (func $_start))\n");
    Ok(())
}

fn emit_function(function: &HighFunction, function_names: &HashSet<String>) -> Result<String> {
    let result_type = wasm_type(&function.return_type)?;
    let params = function
        .params
        .iter()
        .filter_map(|param| {
            wasm_type(&param.ty).transpose().map(|wasm_ty| {
                wasm_ty.map(|wasm_ty| format!("(param ${} {})", param.name, wasm_ty))
            })
        })
        .collect::<Result<Vec<_>>>()?
        .join(" ");
    let locals = function
        .params
        .iter()
        .filter_map(|param| {
            wasm_type(&param.ty)
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
    emit_expr(&function.body, 2, &locals, function_names, &mut out)?;
    if wasm_type(&function.return_type)?.is_none() && wasm_type(&function.body.ty)?.is_some() {
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
            emit_expr(left, indent, locals, function_names, out)?;
            emit_expr(right, indent, locals, function_names, out)?;
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
            emit_expr(condition, indent, locals, function_names, out)?;
            out.push_str(&format!("{pad}(if"));
            if let Some(result_ty) = wasm_type(&expr.ty)? {
                out.push_str(&format!(" (result {result_ty})"));
            }
            out.push('\n');
            out.push_str(&format!("{pad}  (then\n"));
            emit_expr(then_branch, indent + 2, locals, function_names, out)?;
            out.push_str(&format!("{pad}  )\n"));
            out.push_str(&format!("{pad}  (else\n"));
            emit_expr(else_branch, indent + 2, locals, function_names, out)?;
            out.push_str(&format!("{pad}  )\n"));
            out.push_str(&format!("{pad})\n"));
        }
        HighExprKind::Call { callee, args } => {
            if function_names.contains(callee) {
                for arg in args {
                    emit_expr(arg, indent, locals, function_names, out)?;
                }
                out.push_str(&format!("{pad}call ${callee}\n"));
            } else {
                bail!("calls to `{callee}` are not yet supported in wasm backend");
            }
        }
        HighExprKind::Match { .. } | HighExprKind::Construct { .. } => {
            bail!("adt and match codegen are not yet supported in wasm backend");
        }
        HighExprKind::String(text) => {
            bail!("string literals are not yet supported in wasm codegen: {text}");
        }
        HighExprKind::Error => {
            bail!("cannot codegen erroneous expression");
        }
    }
    Ok(())
}

fn wasm_type(ty: &Type) -> Result<Option<&'static str>> {
    match ty {
        Type::Unit => Ok(None),
        Type::Int | Type::Bool => Ok(Some("i32")),
        other => bail!("unsupported wasm type: {other}"),
    }
}
