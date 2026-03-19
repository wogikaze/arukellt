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

pub fn emit_wasm(module: &HighModule, _target: WasmTarget) -> Result<Vec<u8>> {
    let mut wat_module = String::from("(module\n");
    let function_names = module
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect::<HashSet<_>>();
    for function in &module.functions {
        wat_module.push_str(&emit_function(function, &function_names)?);
    }
    for function in &module.functions {
        wat_module.push_str(&format!(
            "  (export \"{}\" (func ${}))\n",
            function.name, function.name
        ));
    }
    wat_module.push_str(")\n");
    Ok(wat::parse_str(&wat_module)?)
}

fn emit_function(function: &HighFunction, function_names: &HashSet<String>) -> Result<String> {
    let result_type = wasm_type(&function.return_type)?;
    let params = function
        .params
        .iter()
        .filter_map(|param| {
            wasm_type(&param.ty)
                .transpose()
                .map(|wasm_ty| wasm_ty.map(|wasm_ty| format!("(param ${} {})", param.name, wasm_ty)))
        })
        .collect::<Result<Vec<_>>>()?
        .join(" ");
    let locals = function
        .params
        .iter()
        .filter_map(|param| wasm_type(&param.ty).ok().flatten().map(|_| param.name.clone()))
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
            emit_placeholder(expr, indent, out)?;
        }
        HighExprKind::Ident(name) => {
            if locals.contains(name) {
                out.push_str(&format!("{pad}local.get ${name}\n"));
            } else {
                emit_placeholder(expr, indent, out)?;
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
                emit_placeholder(expr, indent, out)?;
            }
        }
        HighExprKind::Match { .. } | HighExprKind::Construct { .. } => {
            emit_placeholder(expr, indent, out)?;
        }
        HighExprKind::String(_) => {
            emit_placeholder(expr, indent, out)?;
        }
        HighExprKind::Error => {
            emit_placeholder(expr, indent, out)?;
        }
    }
    Ok(())
}

fn emit_placeholder(expr: &HighExpr, indent: usize, out: &mut String) -> Result<()> {
    let pad = "  ".repeat(indent);
    if wasm_type(&expr.ty)?.is_some() {
        out.push_str(&format!("{pad}i32.const 0\n"));
    }
    Ok(())
}

fn wasm_type(ty: &Type) -> Result<Option<&'static str>> {
    match ty {
        Type::Unit => Ok(None),
        Type::Int | Type::Bool => Ok(Some("i32")),
        Type::String
        | Type::List(_)
        | Type::Seq(_)
        | Type::Option(_)
        | Type::Result(_, _)
        | Type::Fn(_, _)
        | Type::Tuple(_)
        | Type::Record(_)
        | Type::Named(_)
        | Type::Unknown => Ok(Some("i32")),
    }
}
