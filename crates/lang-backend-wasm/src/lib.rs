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
    for function in &module.functions {
        wat_module.push_str(&emit_function(function)?);
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

fn emit_function(function: &HighFunction) -> Result<String> {
    let result_type = wasm_type(&function.return_type)?;
    let params = function
        .params
        .iter()
        .map(|param| Ok(format!("(param ${} {})", param.name, wasm_type(&param.ty)?)))
        .collect::<Result<Vec<_>>>()?
        .join(" ");
    let mut out = String::new();
    out.push_str(&format!(
        "  (func ${} {} (result {})\n",
        function.name, params, result_type
    ));
    emit_expr(&function.body, 2, &mut out)?;
    out.push_str("  )\n");
    Ok(out)
}

fn emit_expr(expr: &HighExpr, indent: usize, out: &mut String) -> Result<()> {
    let pad = "  ".repeat(indent);
    match &expr.kind {
        HighExprKind::Int(value) => {
            out.push_str(&format!("{pad}i32.const {value}\n"));
        }
        HighExprKind::Bool(value) => {
            let numeric = i32::from(*value);
            out.push_str(&format!("{pad}i32.const {numeric}\n"));
        }
        HighExprKind::List(_) | HighExprKind::Tuple(_) | HighExprKind::Lambda { .. } => {
            bail!("list, tuple, and lambda expressions are not yet supported in wasm backend");
        }
        HighExprKind::Ident(name) => {
            out.push_str(&format!("{pad}local.get ${name}\n"));
        }
        HighExprKind::Binary { op, left, right } => {
            emit_expr(left, indent, out)?;
            emit_expr(right, indent, out)?;
            let opcode = match op {
                lang_core::BinaryOp::Add => "i32.add",
                lang_core::BinaryOp::Subtract => "i32.sub",
                lang_core::BinaryOp::Multiply => "i32.mul",
                lang_core::BinaryOp::Divide => "i32.div_s",
                lang_core::BinaryOp::Modulo => bail!("modulo is not yet supported in wasm backend"),
                lang_core::BinaryOp::Greater => "i32.gt_s",
                lang_core::BinaryOp::Less => bail!("less-than is not yet supported in wasm backend"),
                lang_core::BinaryOp::Equal => "i32.eq",
                lang_core::BinaryOp::Or => bail!("logical or is not yet supported in wasm backend"),
            };
            out.push_str(&format!("{pad}{opcode}\n"));
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            emit_expr(condition, indent, out)?;
            out.push_str(&format!("{pad}(if (result {})\n", wasm_type(&expr.ty)?));
            out.push_str(&format!("{pad}  (then\n"));
            emit_expr(then_branch, indent + 2, out)?;
            out.push_str(&format!("{pad}  )\n"));
            out.push_str(&format!("{pad}  (else\n"));
            emit_expr(else_branch, indent + 2, out)?;
            out.push_str(&format!("{pad}  )\n"));
            out.push_str(&format!("{pad})\n"));
        }
        HighExprKind::Call { callee, args } => {
            for arg in args {
                emit_expr(arg, indent, out)?;
            }
            out.push_str(&format!("{pad}call ${callee}\n"));
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

fn wasm_type(ty: &Type) -> Result<&'static str> {
    match ty {
        Type::Int | Type::Bool => Ok("i32"),
        other => bail!("unsupported wasm type: {other}"),
    }
}
