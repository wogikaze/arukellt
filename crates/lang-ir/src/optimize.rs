use std::collections::{HashMap, HashSet};

use lang_core::Pattern;

use crate::high::{HighExpr, HighExprKind, HighFunction, HighMatchArm, HighModule};

const INLINE_NODE_BUDGET: usize = 8;

#[must_use]
pub fn optimize_high_module(module: &HighModule, roots: &HashSet<String>) -> HighModule {
    let pure_functions = pure_function_names(module);
    let inlineable = inlineable_functions(module, &pure_functions);

    let mut optimized = module.clone();
    optimized.functions = optimized
        .functions
        .into_iter()
        .map(|mut function| {
            function.body = inline_expr(&function.body, &inlineable);
            function
        })
        .collect();
    optimized.functions = retain_reachable_functions(&optimized, roots);
    optimized
}

fn pure_function_names(module: &HighModule) -> HashSet<String> {
    let function_map = module
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function))
        .collect::<HashMap<_, _>>();
    let mut pure = HashSet::new();
    let mut changed = true;
    while changed {
        changed = false;
        for function in &module.functions {
            if pure.contains(&function.name) {
                continue;
            }
            if expr_is_pure(&function.body, &pure, &function_map) {
                changed = pure.insert(function.name.clone()) || changed;
            }
        }
    }
    pure
}

fn inlineable_functions<'a>(
    module: &'a HighModule,
    pure_functions: &HashSet<String>,
) -> HashMap<String, &'a HighFunction> {
    module
        .functions
        .iter()
        .filter(|function| pure_functions.contains(&function.name))
        .filter(|function| !expr_contains_call(&function.body, &function.name))
        .filter(|function| !expr_has_binders(&function.body))
        .filter(|function| expr_node_count(&function.body) <= INLINE_NODE_BUDGET)
        .map(|function| (function.name.clone(), function))
        .collect()
}

fn retain_reachable_functions(module: &HighModule, roots: &HashSet<String>) -> Vec<HighFunction> {
    let function_map = module
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function))
        .collect::<HashMap<_, _>>();
    let mut reachable = HashSet::new();
    let mut pending = roots.iter().cloned().collect::<Vec<_>>();

    while let Some(name) = pending.pop() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        let Some(function) = function_map.get(name.as_str()) else {
            continue;
        };
        collect_called_functions(&function.body, &function_map, &mut pending);
    }

    module
        .functions
        .iter()
        .filter(|function| reachable.contains(&function.name))
        .cloned()
        .collect()
}

fn inline_expr(expr: &HighExpr, inlineable: &HashMap<String, &HighFunction>) -> HighExpr {
    let kind = match &expr.kind {
        HighExprKind::Int(value) => HighExprKind::Int(*value),
        HighExprKind::Bool(value) => HighExprKind::Bool(*value),
        HighExprKind::String(value) => HighExprKind::String(value.clone()),
        HighExprKind::Ident(name) => HighExprKind::Ident(name.clone()),
        HighExprKind::List(items) => HighExprKind::List(
            items
                .iter()
                .map(|item| inline_expr(item, inlineable))
                .collect(),
        ),
        HighExprKind::Tuple(items) => HighExprKind::Tuple(
            items
                .iter()
                .map(|item| inline_expr(item, inlineable))
                .collect(),
        ),
        HighExprKind::Lambda { param, body } => HighExprKind::Lambda {
            param: param.clone(),
            body: Box::new(inline_expr(body, inlineable)),
        },
        HighExprKind::Let { name, value, body } => HighExprKind::Let {
            name: name.clone(),
            value: Box::new(inline_expr(value, inlineable)),
            body: Box::new(inline_expr(body, inlineable)),
        },
        HighExprKind::Binary { op, left, right } => HighExprKind::Binary {
            op: *op,
            left: Box::new(inline_expr(left, inlineable)),
            right: Box::new(inline_expr(right, inlineable)),
        },
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => HighExprKind::If {
            condition: Box::new(inline_expr(condition, inlineable)),
            then_branch: Box::new(inline_expr(then_branch, inlineable)),
            else_branch: Box::new(inline_expr(else_branch, inlineable)),
        },
        HighExprKind::Match { subject, arms } => HighExprKind::Match {
            subject: Box::new(inline_expr(subject, inlineable)),
            arms: arms
                .iter()
                .map(|arm| HighMatchArm {
                    pattern: arm.pattern.clone(),
                    expr: inline_expr(&arm.expr, inlineable),
                })
                .collect(),
        },
        HighExprKind::Construct { variant, args } => HighExprKind::Construct {
            variant: variant.clone(),
            args: args
                .iter()
                .map(|arg| inline_expr(arg, inlineable))
                .collect(),
        },
        HighExprKind::Call { callee, args } => {
            let args = args
                .iter()
                .map(|arg| inline_expr(arg, inlineable))
                .collect::<Vec<_>>();
            if let Some(function) = inlineable.get(callee) {
                let replacements = function
                    .params
                    .iter()
                    .map(|param| param.name.clone())
                    .zip(args)
                    .collect::<HashMap<_, _>>();
                return inline_expr(&substitute_expr(&function.body, &replacements), inlineable);
            }
            HighExprKind::Call {
                callee: callee.clone(),
                args,
            }
        }
        HighExprKind::Error => HighExprKind::Error,
    };
    HighExpr {
        kind,
        ty: expr.ty.clone(),
    }
}

fn substitute_expr(expr: &HighExpr, replacements: &HashMap<String, HighExpr>) -> HighExpr {
    let kind = match &expr.kind {
        HighExprKind::Ident(name) => {
            if let Some(replacement) = replacements.get(name) {
                return replacement.clone();
            }
            HighExprKind::Ident(name.clone())
        }
        HighExprKind::Int(value) => HighExprKind::Int(*value),
        HighExprKind::Bool(value) => HighExprKind::Bool(*value),
        HighExprKind::String(value) => HighExprKind::String(value.clone()),
        HighExprKind::List(items) => HighExprKind::List(
            items
                .iter()
                .map(|item| substitute_expr(item, replacements))
                .collect(),
        ),
        HighExprKind::Tuple(items) => HighExprKind::Tuple(
            items
                .iter()
                .map(|item| substitute_expr(item, replacements))
                .collect(),
        ),
        HighExprKind::Lambda { param, body } => HighExprKind::Lambda {
            param: param.clone(),
            body: Box::new(substitute_expr(
                body,
                &without_binding(replacements, std::slice::from_ref(param)),
            )),
        },
        HighExprKind::Let { name, value, body } => HighExprKind::Let {
            name: name.clone(),
            value: Box::new(substitute_expr(value, replacements)),
            body: Box::new(substitute_expr(
                body,
                &without_binding(replacements, std::slice::from_ref(name)),
            )),
        },
        HighExprKind::Binary { op, left, right } => HighExprKind::Binary {
            op: *op,
            left: Box::new(substitute_expr(left, replacements)),
            right: Box::new(substitute_expr(right, replacements)),
        },
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => HighExprKind::If {
            condition: Box::new(substitute_expr(condition, replacements)),
            then_branch: Box::new(substitute_expr(then_branch, replacements)),
            else_branch: Box::new(substitute_expr(else_branch, replacements)),
        },
        HighExprKind::Match { subject, arms } => HighExprKind::Match {
            subject: Box::new(substitute_expr(subject, replacements)),
            arms: arms
                .iter()
                .map(|arm| {
                    let bound = pattern_bindings(&arm.pattern);
                    HighMatchArm {
                        pattern: arm.pattern.clone(),
                        expr: substitute_expr(&arm.expr, &without_binding(replacements, &bound)),
                    }
                })
                .collect(),
        },
        HighExprKind::Construct { variant, args } => HighExprKind::Construct {
            variant: variant.clone(),
            args: args
                .iter()
                .map(|arg| substitute_expr(arg, replacements))
                .collect(),
        },
        HighExprKind::Call { callee, args } => HighExprKind::Call {
            callee: callee.clone(),
            args: args
                .iter()
                .map(|arg| substitute_expr(arg, replacements))
                .collect(),
        },
        HighExprKind::Error => HighExprKind::Error,
    };
    HighExpr {
        kind,
        ty: expr.ty.clone(),
    }
}

fn without_binding(
    replacements: &HashMap<String, HighExpr>,
    names: &[String],
) -> HashMap<String, HighExpr> {
    let mut filtered = replacements.clone();
    for name in names {
        filtered.remove(name);
    }
    filtered
}

fn expr_is_pure(
    expr: &HighExpr,
    pure_functions: &HashSet<String>,
    function_map: &HashMap<&str, &HighFunction>,
) -> bool {
    match &expr.kind {
        HighExprKind::Int(_)
        | HighExprKind::Bool(_)
        | HighExprKind::String(_)
        | HighExprKind::Ident(_)
        | HighExprKind::Error => true,
        HighExprKind::List(items)
        | HighExprKind::Tuple(items)
        | HighExprKind::Construct { args: items, .. } => items
            .iter()
            .all(|item| expr_is_pure(item, pure_functions, function_map)),
        HighExprKind::Lambda { body, .. } => expr_is_pure(body, pure_functions, function_map),
        HighExprKind::Let { value, body, .. } => {
            expr_is_pure(value, pure_functions, function_map)
                && expr_is_pure(body, pure_functions, function_map)
        }
        HighExprKind::Binary { left, right, .. } => {
            expr_is_pure(left, pure_functions, function_map)
                && expr_is_pure(right, pure_functions, function_map)
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_is_pure(condition, pure_functions, function_map)
                && expr_is_pure(then_branch, pure_functions, function_map)
                && expr_is_pure(else_branch, pure_functions, function_map)
        }
        HighExprKind::Match { subject, arms } => {
            expr_is_pure(subject, pure_functions, function_map)
                && arms
                    .iter()
                    .all(|arm| expr_is_pure(&arm.expr, pure_functions, function_map))
        }
        HighExprKind::Call { callee, args } => {
            args.iter()
                .all(|arg| expr_is_pure(arg, pure_functions, function_map))
                && if function_map.contains_key(callee.as_str()) {
                    pure_functions.contains(callee)
                } else {
                    builtin_is_pure(callee)
                }
        }
    }
}

fn builtin_is_pure(name: &str) -> bool {
    matches!(
        name,
        "__index"
            | "range_inclusive"
            | "string"
            | "len"
            | "ends_with_at"
            | "split_whitespace"
            | "strip_suffix"
            | "parse.i64"
            | "parse.bool"
            | "map"
            | "unwrap_or"
            | "any"
            | "filter"
            | "sum"
            | "join"
            | "take"
            | "iter.unfold"
            | "Some"
            | "None"
            | "Next"
            | "Done"
    )
}

fn collect_called_functions(
    expr: &HighExpr,
    function_map: &HashMap<&str, &HighFunction>,
    pending: &mut Vec<String>,
) {
    match &expr.kind {
        HighExprKind::Call { callee, args } => {
            pending.push(callee.clone());
            for arg in args {
                collect_called_functions(arg, function_map, pending);
            }
        }
        HighExprKind::Ident(name) => {
            if function_map.contains_key(name.as_str()) {
                pending.push(name.clone());
            }
        }
        HighExprKind::Binary { left, right, .. } => {
            collect_called_functions(left, function_map, pending);
            collect_called_functions(right, function_map, pending);
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_called_functions(condition, function_map, pending);
            collect_called_functions(then_branch, function_map, pending);
            collect_called_functions(else_branch, function_map, pending);
        }
        HighExprKind::Match { subject, arms } => {
            collect_called_functions(subject, function_map, pending);
            for arm in arms {
                collect_called_functions(&arm.expr, function_map, pending);
            }
        }
        HighExprKind::Construct { args, .. }
        | HighExprKind::List(args)
        | HighExprKind::Tuple(args) => {
            for arg in args {
                collect_called_functions(arg, function_map, pending);
            }
        }
        HighExprKind::Lambda { body, .. } => collect_called_functions(body, function_map, pending),
        HighExprKind::Let { value, body, .. } => {
            collect_called_functions(value, function_map, pending);
            collect_called_functions(body, function_map, pending);
        }
        HighExprKind::Int(_)
        | HighExprKind::Bool(_)
        | HighExprKind::String(_)
        | HighExprKind::Error => {}
    }
}

fn expr_contains_call(expr: &HighExpr, callee: &str) -> bool {
    match &expr.kind {
        HighExprKind::Call {
            callee: current,
            args,
        } => current == callee || args.iter().any(|arg| expr_contains_call(arg, callee)),
        HighExprKind::Binary { left, right, .. } => {
            expr_contains_call(left, callee) || expr_contains_call(right, callee)
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_contains_call(condition, callee)
                || expr_contains_call(then_branch, callee)
                || expr_contains_call(else_branch, callee)
        }
        HighExprKind::Match { subject, arms } => {
            expr_contains_call(subject, callee)
                || arms.iter().any(|arm| expr_contains_call(&arm.expr, callee))
        }
        HighExprKind::Construct { args, .. }
        | HighExprKind::List(args)
        | HighExprKind::Tuple(args) => args.iter().any(|arg| expr_contains_call(arg, callee)),
        HighExprKind::Lambda { body, .. } => expr_contains_call(body, callee),
        HighExprKind::Let { value, body, .. } => {
            expr_contains_call(value, callee) || expr_contains_call(body, callee)
        }
        HighExprKind::Int(_)
        | HighExprKind::Bool(_)
        | HighExprKind::String(_)
        | HighExprKind::Ident(_)
        | HighExprKind::Error => false,
    }
}

fn expr_has_binders(expr: &HighExpr) -> bool {
    match &expr.kind {
        HighExprKind::Lambda { .. } | HighExprKind::Let { .. } => true,
        HighExprKind::Match { subject, arms } => {
            expr_has_binders(subject)
                || arms.iter().any(|arm| {
                    !pattern_bindings(&arm.pattern).is_empty() || expr_has_binders(&arm.expr)
                })
        }
        HighExprKind::Binary { left, right, .. } => {
            expr_has_binders(left) || expr_has_binders(right)
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_has_binders(condition)
                || expr_has_binders(then_branch)
                || expr_has_binders(else_branch)
        }
        HighExprKind::Construct { args, .. }
        | HighExprKind::List(args)
        | HighExprKind::Tuple(args)
        | HighExprKind::Call { args, .. } => args.iter().any(expr_has_binders),
        HighExprKind::Int(_)
        | HighExprKind::Bool(_)
        | HighExprKind::String(_)
        | HighExprKind::Ident(_)
        | HighExprKind::Error => false,
    }
}

fn expr_node_count(expr: &HighExpr) -> usize {
    match &expr.kind {
        HighExprKind::Int(_)
        | HighExprKind::Bool(_)
        | HighExprKind::String(_)
        | HighExprKind::Ident(_)
        | HighExprKind::Error => 1,
        HighExprKind::List(items)
        | HighExprKind::Tuple(items)
        | HighExprKind::Construct { args: items, .. } => {
            1 + items.iter().map(expr_node_count).sum::<usize>()
        }
        HighExprKind::Lambda { body, .. } => 1 + expr_node_count(body),
        HighExprKind::Let { value, body, .. } => 1 + expr_node_count(value) + expr_node_count(body),
        HighExprKind::Binary { left, right, .. } => {
            1 + expr_node_count(left) + expr_node_count(right)
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            1 + expr_node_count(condition)
                + expr_node_count(then_branch)
                + expr_node_count(else_branch)
        }
        HighExprKind::Match { subject, arms } => {
            1 + expr_node_count(subject)
                + arms
                    .iter()
                    .map(|arm| expr_node_count(&arm.expr))
                    .sum::<usize>()
        }
        HighExprKind::Call { args, .. } => 1 + args.iter().map(expr_node_count).sum::<usize>(),
    }
}

fn pattern_bindings(pattern: &Pattern) -> Vec<String> {
    match pattern {
        Pattern::Wildcard => Vec::new(),
        Pattern::Variant { bindings, .. } => bindings.clone(),
    }
}
