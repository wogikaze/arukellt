use crate::Type;
use crate::ast::{BinaryOp, Expr, Function, MatchArm, Module, Pattern, TypeDecl};

/// Render a parsed `Module` back to canonical arukellt source.
#[must_use]
pub fn format_module(module: &Module) -> String {
    let mut out = String::new();

    for import in &module.imports {
        out.push_str("import ");
        out.push_str(&import.name);
        out.push('\n');
    }
    if !module.imports.is_empty() && (!module.types.is_empty() || !module.functions.is_empty()) {
        out.push('\n');
    }

    for (i, type_decl) in module.types.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&format_type_decl(type_decl));
    }
    if !module.types.is_empty() && !module.functions.is_empty() {
        out.push('\n');
    }

    for (i, function) in module.functions.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&format_function(function));
    }

    out
}

fn format_type_decl(decl: &TypeDecl) -> String {
    let mut out = format!("type {} =\n", decl.name);
    for variant in &decl.variants {
        out.push_str("  ");
        out.push_str(&variant.name);
        if !variant.fields.is_empty() {
            let fields: Vec<String> = variant
                .fields
                .iter()
                .map(|f| format!("{}: {}", f.name, f.ty))
                .collect();
            out.push('(');
            out.push_str(&fields.join(", "));
            out.push(')');
        }
        out.push('\n');
    }
    out
}

fn format_function(func: &Function) -> String {
    let mut out = String::new();
    if func.public {
        out.push_str("pub ");
    }
    out.push_str("fn ");
    out.push_str(&func.name);
    out.push('(');
    let params: Vec<String> = func
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, p.ty))
        .collect();
    out.push_str(&params.join(", "));
    out.push(')');
    if func.return_type != Type::Unit {
        out.push_str(" -> ");
        out.push_str(&func.return_type.to_string());
    }
    out.push_str(":\n");
    out.push_str(&format_expr_block(&func.body, 1));
    out.push('\n');
    out
}

fn indent_str(level: usize) -> String {
    "  ".repeat(level)
}

/// Render an expression with the given indent level prefix.
fn format_expr_block(expr: &Expr, level: usize) -> String {
    let prefix = indent_str(level);
    match expr {
        Expr::Int(n) => format!("{prefix}{n}"),
        Expr::Bool(b) => format!("{prefix}{b}"),
        Expr::String(s) => format!("{prefix}\"{}\"", escape_string(s)),
        Expr::Ident(name) => format!("{prefix}{name}"),
        Expr::List(items) => {
            let rendered: Vec<String> = items.iter().map(format_inline).collect();
            format!("{prefix}[{}]", rendered.join(", "))
        }
        Expr::Tuple(items) => {
            let rendered: Vec<String> = items.iter().map(format_inline).collect();
            format!("{prefix}({})", rendered.join(", "))
        }
        Expr::Lambda { param, body } => {
            format!("{prefix}{param} -> {}", format_inline(body))
        }
        Expr::Let { name, value, body } => {
            format!(
                "{prefix}let {name} = {}\n{}",
                format_inline(value),
                format_expr_block(body, level)
            )
        }
        Expr::Binary { op, left, right } => {
            format!(
                "{prefix}{} {} {}",
                format_inline(left),
                format_op(*op),
                format_inline(right)
            )
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            format!(
                "{prefix}if {}:\n{}\n{prefix}else:\n{}",
                format_inline(condition),
                format_expr_block(then_branch, level + 1),
                format_expr_block(else_branch, level + 1),
            )
        }
        Expr::Match { subject, arms } => {
            let mut out = format!("{prefix}match {}:\n", format_inline(subject));
            for arm in arms {
                out.push_str(&format_match_arm(arm, level + 1));
            }
            // strip trailing newline (outer context adds its own)
            if out.ends_with('\n') {
                out.pop();
            }
            out
        }
        Expr::Call { callee, args } => {
            // Single-arg dotted call → arg |> callee
            if callee.contains('.') && args.len() == 1 {
                format!("{prefix}{} |> {}", format_inline(&args[0]), callee)
            } else {
                let rendered: Vec<String> = args.iter().map(format_inline).collect();
                format!("{prefix}{}({})", callee, rendered.join(", "))
            }
        }
        Expr::Apply { func, args } => {
            let rendered: Vec<String> = args.iter().map(format_inline).collect();
            format!("{prefix}{}({})", format_inline(func), rendered.join(", "))
        }
        Expr::MethodCall {
            receiver,
            method,
            args,
        } => {
            let rendered: Vec<String> = args.iter().map(format_inline).collect();
            format!(
                "{prefix}{}.{}({})",
                format_inline(receiver),
                method,
                rendered.join(", ")
            )
        }
        Expr::Index { receiver, index } => {
            format!(
                "{prefix}{}[{}]",
                format_inline(receiver),
                format_inline(index)
            )
        }
        Expr::Range { start, end } => {
            format!("{prefix}{}..={}", format_inline(start), format_inline(end))
        }
        Expr::Error => format!("{prefix}<error>"),
    }
}

/// Render an expression as a single-line string (no leading indent).
fn format_inline(expr: &Expr) -> String {
    format_expr_block(expr, 0)
}

fn format_match_arm(arm: &MatchArm, level: usize) -> String {
    let prefix = indent_str(level);
    let pattern = match &arm.pattern {
        Pattern::Wildcard => "_".to_owned(),
        Pattern::Variant { name, bindings } => {
            if bindings.is_empty() {
                name.clone()
            } else {
                format!("{}({})", name, bindings.join(", "))
            }
        }
    };
    format!("{prefix}{pattern} -> {}\n", format_inline(&arm.expr))
}

fn format_op(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
        BinaryOp::Modulo => "%",
        BinaryOp::Greater => ">",
        BinaryOp::Less => "<",
        BinaryOp::Equal => "==",
        BinaryOp::And => "and",
        BinaryOp::Or => "or",
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}
