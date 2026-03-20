use std::collections::HashSet;

use lang_core::{Pattern, Type, TypeDecl};

use crate::high::{HighExpr, HighExprKind, HighFunction, HighModule, HighParam};

#[derive(Clone, Debug, PartialEq)]
pub struct WasmModule {
    pub imports: Vec<String>,
    pub types: Vec<TypeDecl>,
    pub functions: Vec<WasmFunction>,
    pub helper_usage: WasmHelperUsage,
}

impl WasmModule {
    #[must_use]
    pub fn specialized_function_names(&self) -> HashSet<String> {
        self.functions
            .iter()
            .filter(|function| !matches!(function.body, WasmFunctionBody::High(_)))
            .map(|function| function.name.clone())
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WasmFunction {
    pub public: bool,
    pub name: String,
    pub params: Vec<HighParam>,
    pub return_type: Type,
    pub body: WasmFunctionBody,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WasmFunctionBody {
    High(HighExpr),
    SuffixRecursion(SuffixRecursionSpec),
    ParseI64OrZero(ParseOrZeroSpec),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WasmHelperUsage {
    pub uses_ends_with_at: bool,
    pub uses_split_whitespace: bool,
    pub uses_parse_i64: bool,
    pub uses_parse_bool: bool,
    pub uses_strip_suffix: bool,
    pub uses_unwrap_or: bool,
    pub uses_parse_i64_or_zero: bool,
    pub uses_option_runtime: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuffixRecursionSpec {
    pub param_name: String,
    pub suffixes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseOrZeroSpec {
    pub param_name: String,
}

#[must_use]
pub fn lower_to_wasm_ir(module: &HighModule) -> WasmModule {
    let functions = module
        .functions
        .iter()
        .map(lower_function_to_wasm)
        .collect::<Vec<_>>();
    let helper_usage = analyze_helper_usage(&functions);
    WasmModule {
        imports: module.imports.clone(),
        types: module.types.clone(),
        functions,
        helper_usage,
    }
}

fn lower_function_to_wasm(function: &HighFunction) -> WasmFunction {
    let body = if let Some(spec) = match_suffix_recursion(function) {
        WasmFunctionBody::SuffixRecursion(spec)
    } else if let Some(spec) = match_parse_or_zero(function) {
        WasmFunctionBody::ParseI64OrZero(spec)
    } else {
        WasmFunctionBody::High(function.body.clone())
    };
    WasmFunction {
        public: function.public,
        name: function.name.clone(),
        params: function.params.clone(),
        return_type: function.return_type.clone(),
        body,
    }
}

fn analyze_helper_usage(functions: &[WasmFunction]) -> WasmHelperUsage {
    let mut usage = WasmHelperUsage::default();
    for function in functions {
        match &function.body {
            WasmFunctionBody::High(body) => {
                usage.uses_ends_with_at |= expr_uses_call(body, "ends_with_at");
                usage.uses_split_whitespace |= expr_uses_call(body, "split_whitespace");
                usage.uses_parse_i64 |= expr_uses_call(body, "parse.i64");
                usage.uses_parse_bool |= expr_uses_call(body, "parse.bool");
                usage.uses_strip_suffix |= expr_uses_call(body, "strip_suffix");
                usage.uses_unwrap_or |= expr_uses_call(body, "unwrap_or");
                usage.uses_option_runtime |= expr_uses_option_runtime(body);
            }
            WasmFunctionBody::SuffixRecursion(_) => {
                usage.uses_ends_with_at = true;
            }
            WasmFunctionBody::ParseI64OrZero(_) => {
                usage.uses_parse_i64_or_zero = true;
            }
        }
    }
    usage
}

fn expr_uses_call(expr: &HighExpr, callee: &str) -> bool {
    match &expr.kind {
        HighExprKind::Call {
            callee: actual,
            args,
        } => {
            if callee == "split_whitespace"
                && actual == "__index"
                && matches_split_whitespace_nth_expr(args.first()).is_some()
            {
                if let Some(text) = matches_split_whitespace_nth_expr(args.first()) {
                    expr_uses_call(text, callee)
                        || args.iter().skip(1).any(|arg| expr_uses_call(arg, callee))
                } else {
                    false
                }
            } else {
                actual == callee || args.iter().any(|arg| expr_uses_call(arg, callee))
            }
        }
        HighExprKind::Binary { left, right, .. } => {
            expr_uses_call(left, callee) || expr_uses_call(right, callee)
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_uses_call(condition, callee)
                || expr_uses_call(then_branch, callee)
                || expr_uses_call(else_branch, callee)
        }
        HighExprKind::Match { subject, arms } => {
            expr_uses_call(subject, callee)
                || arms.iter().any(|arm| expr_uses_call(&arm.expr, callee))
        }
        HighExprKind::Construct { args, .. }
        | HighExprKind::List(args)
        | HighExprKind::Tuple(args) => args.iter().any(|arg| expr_uses_call(arg, callee)),
        HighExprKind::Lambda { body, .. } => expr_uses_call(body, callee),
        HighExprKind::Let { value, body, .. } => {
            expr_uses_call(value, callee) || expr_uses_call(body, callee)
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

fn expr_uses_option_runtime(expr: &HighExpr) -> bool {
    if matches!(expr.ty, Type::Option(_)) {
        return true;
    }
    match &expr.kind {
        HighExprKind::Call { args, .. } | HighExprKind::Construct { args, .. } => {
            args.iter().any(expr_uses_option_runtime)
        }
        HighExprKind::Binary { left, right, .. } => {
            expr_uses_option_runtime(left) || expr_uses_option_runtime(right)
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_uses_option_runtime(condition)
                || expr_uses_option_runtime(then_branch)
                || expr_uses_option_runtime(else_branch)
        }
        HighExprKind::Match { subject, arms } => {
            expr_uses_option_runtime(subject)
                || arms.iter().any(|arm| expr_uses_option_runtime(&arm.expr))
        }
        HighExprKind::List(items) | HighExprKind::Tuple(items) => {
            items.iter().any(expr_uses_option_runtime)
        }
        HighExprKind::Lambda { body, .. } => expr_uses_option_runtime(body),
        HighExprKind::Let { value, body, .. } => {
            expr_uses_option_runtime(value) || expr_uses_option_runtime(body)
        }
        HighExprKind::Int(_)
        | HighExprKind::Bool(_)
        | HighExprKind::String(_)
        | HighExprKind::Ident(_)
        | HighExprKind::Error => false,
    }
}

fn match_suffix_recursion(function: &HighFunction) -> Option<SuffixRecursionSpec> {
    if function.params.len() != 1
        || function.params[0].ty != Type::String
        || function.return_type != Type::Bool
    {
        return None;
    }
    let param_name = function.params[0].name.clone();
    let HighExprKind::If {
        condition,
        then_branch,
        else_branch,
    } = &function.body.kind
    else {
        return None;
    };
    if !matches!(then_branch.kind, HighExprKind::Bool(true))
        || !is_empty_string_check(condition, &param_name)
    {
        return None;
    }
    let suffixes = match_suffix_any(else_branch, &param_name, &function.name)?;
    Some(SuffixRecursionSpec {
        param_name,
        suffixes,
    })
}

fn match_parse_or_zero(function: &HighFunction) -> Option<ParseOrZeroSpec> {
    if function.params.len() != 1
        || function.params[0].ty != Type::String
        || function.return_type != Type::Int
    {
        return None;
    }
    let param_name = function.params[0].name.clone();
    let body = match &function.body.kind {
        HighExprKind::Let { name, value, body } if name == "parsed" => {
            if !matches_parse_i64_call(value, &param_name) {
                return None;
            }
            body
        }
        _ => &function.body,
    };
    matches_parse_or_zero_match(body, &param_name).then_some(ParseOrZeroSpec { param_name })
}

fn matches_parse_i64_call(expr: &HighExpr, param_name: &str) -> bool {
    let HighExprKind::Call { callee, args } = &expr.kind else {
        return false;
    };
    callee == "parse.i64"
        && args.len() == 1
        && matches!(&args[0].kind, HighExprKind::Ident(name) if name == param_name)
}

fn matches_parse_or_zero_match(expr: &HighExpr, param_name: &str) -> bool {
    let HighExprKind::Match { subject, arms } = &expr.kind else {
        return false;
    };
    let subject_matches = matches_parse_i64_call(subject, param_name)
        || matches!(&subject.kind, HighExprKind::Ident(name) if name == "parsed");
    if !subject_matches || arms.len() != 2 {
        return false;
    }
    let ok_matches = matches!(
        (&arms[0].pattern, &arms[0].expr.kind),
        (
            Pattern::Variant { name, bindings },
            HighExprKind::Ident(bound)
        ) if name == "Ok" && bindings.len() == 1 && bindings[0] == *bound
    ) || matches!(
        (&arms[1].pattern, &arms[1].expr.kind),
        (
            Pattern::Variant { name, bindings },
            HighExprKind::Ident(bound)
        ) if name == "Ok" && bindings.len() == 1 && bindings[0] == *bound
    );
    let err_zero_matches = matches!(
        (&arms[0].pattern, &arms[0].expr.kind),
        (Pattern::Variant { name, .. }, HighExprKind::Int(0)) if name == "Err"
    ) || matches!(
        (&arms[1].pattern, &arms[1].expr.kind),
        (Pattern::Variant { name, .. }, HighExprKind::Int(0)) if name == "Err"
    );
    ok_matches && err_zero_matches
}

fn is_empty_string_check(expr: &HighExpr, param_name: &str) -> bool {
    let HighExprKind::Binary { op, left, right } = &expr.kind else {
        return false;
    };
    if *op != lang_core::BinaryOp::Equal {
        return false;
    }
    matches!(
        (&left.kind, &right.kind),
        (HighExprKind::Ident(name), HighExprKind::String(text))
            if name == param_name && text.is_empty()
    ) || matches!(
        (&left.kind, &right.kind),
        (HighExprKind::String(text), HighExprKind::Ident(name))
            if name == param_name && text.is_empty()
    )
}

fn match_suffix_any(expr: &HighExpr, text_param: &str, function_name: &str) -> Option<Vec<String>> {
    let HighExprKind::Call { callee, args } = &expr.kind else {
        return None;
    };
    if callee != "any" || args.len() != 2 {
        return None;
    }
    let suffixes = match &args[0].kind {
        HighExprKind::List(items) => items
            .iter()
            .map(|item| match &item.kind {
                HighExprKind::String(text) => Some(text.clone()),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?,
        _ => return None,
    };
    let HighExprKind::Lambda { param, body } = &args[1].kind else {
        return None;
    };
    if !matches_suffix_callback(body, text_param, param, function_name) {
        return None;
    }
    Some(suffixes)
}

fn matches_suffix_callback(
    expr: &HighExpr,
    text_param: &str,
    suffix_param: &str,
    function_name: &str,
) -> bool {
    let HighExprKind::Call { callee, args } = &expr.kind else {
        return false;
    };
    if callee != "unwrap_or"
        || args.len() != 2
        || !matches!(args[1].kind, HighExprKind::Bool(false))
    {
        return false;
    }
    let HighExprKind::Call {
        callee: map_callee,
        args: map_args,
    } = &args[0].kind
    else {
        return false;
    };
    if map_callee != "map" || map_args.len() != 2 {
        return false;
    }
    if !matches!(&map_args[1].kind, HighExprKind::Ident(name) if name == function_name) {
        return false;
    }
    let HighExprKind::Call {
        callee: strip_callee,
        args: strip_args,
    } = &map_args[0].kind
    else {
        return false;
    };
    if strip_callee != "strip_suffix" || strip_args.len() != 2 {
        return false;
    }
    matches!(&strip_args[0].kind, HighExprKind::Ident(name) if name == text_param)
        && matches!(&strip_args[1].kind, HighExprKind::Ident(name) if name == suffix_param)
}
