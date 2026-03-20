use super::*;

pub(crate) fn expr_uses_split_whitespace_nth(expr: &HighExpr) -> bool {
    match &expr.kind {
        HighExprKind::Call { callee, args } if callee == "__index" => {
            matches_split_whitespace_nth_expr(args.first()).is_some()
                || args.iter().any(expr_uses_split_whitespace_nth)
        }
        HighExprKind::Call { args, .. } | HighExprKind::Construct { args, .. } => {
            args.iter().any(expr_uses_split_whitespace_nth)
        }
        HighExprKind::Binary { left, right, .. } => {
            expr_uses_split_whitespace_nth(left) || expr_uses_split_whitespace_nth(right)
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_uses_split_whitespace_nth(condition)
                || expr_uses_split_whitespace_nth(then_branch)
                || expr_uses_split_whitespace_nth(else_branch)
        }
        HighExprKind::Match { subject, arms } => {
            expr_uses_split_whitespace_nth(subject)
                || arms
                    .iter()
                    .any(|arm| expr_uses_split_whitespace_nth(&arm.expr))
        }
        HighExprKind::List(items) | HighExprKind::Tuple(items) => {
            items.iter().any(expr_uses_split_whitespace_nth)
        }
        HighExprKind::Lambda { body, .. } => expr_uses_split_whitespace_nth(body),
        HighExprKind::Let { value, body, .. } => {
            expr_uses_split_whitespace_nth(value) || expr_uses_split_whitespace_nth(body)
        }
        HighExprKind::Int(_)
        | HighExprKind::Bool(_)
        | HighExprKind::String(_)
        | HighExprKind::Ident(_)
        | HighExprKind::Error => false,
    }
}

pub(crate) fn matches_split_whitespace_nth_expr(receiver: Option<&HighExpr>) -> Option<&HighExpr> {
    let receiver = receiver?;
    let HighExprKind::Call { callee, args } = &receiver.kind else {
        return None;
    };
    if callee != "split_whitespace" || args.len() != 1 {
        return None;
    }
    Some(&args[0])
}

pub(crate) fn module_uses_gc_option_i32(module: &HighModule) -> bool {
    module.functions.iter().any(function_uses_gc_option_i32)
}

pub(crate) fn function_uses_gc_option_i32(function: &lang_ir::HighFunction) -> bool {
    function
        .params
        .iter()
        .any(|param| is_gc_option_i32_type(&param.ty))
        || is_gc_option_i32_type(&function.return_type)
        || expr_uses_gc_option_i32(&function.body)
}

pub(crate) fn is_gc_option_i32_type(ty: &Type) -> bool {
    matches!(ty, Type::Option(inner) if **inner == Type::Int)
}

pub(crate) fn expr_uses_gc_option_i32(expr: &HighExpr) -> bool {
    if is_gc_option_i32_type(&expr.ty) {
        return true;
    }
    match &expr.kind {
        HighExprKind::Call { args, .. } | HighExprKind::Construct { args, .. } => {
            args.iter().any(expr_uses_gc_option_i32)
        }
        HighExprKind::Binary { left, right, .. } => {
            expr_uses_gc_option_i32(left) || expr_uses_gc_option_i32(right)
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_uses_gc_option_i32(condition)
                || expr_uses_gc_option_i32(then_branch)
                || expr_uses_gc_option_i32(else_branch)
        }
        HighExprKind::Match { subject, arms } => {
            expr_uses_gc_option_i32(subject)
                || arms.iter().any(|arm| expr_uses_gc_option_i32(&arm.expr))
        }
        HighExprKind::List(items) | HighExprKind::Tuple(items) => {
            items.iter().any(expr_uses_gc_option_i32)
        }
        HighExprKind::Lambda { body, .. } => expr_uses_gc_option_i32(body),
        HighExprKind::Let { value, body, .. } => {
            expr_uses_gc_option_i32(value) || expr_uses_gc_option_i32(body)
        }
        HighExprKind::Int(_)
        | HighExprKind::Bool(_)
        | HighExprKind::String(_)
        | HighExprKind::Ident(_)
        | HighExprKind::Error => false,
    }
}

pub(crate) fn scan_expr(
    expr: &HighExpr,
    uses_console_println: &mut bool,
    uses_string_builtin: &mut bool,
    uses_len_builtin: &mut bool,
    uses_ends_with_at_builtin: &mut bool,
    uses_string_eq: &mut bool,
    uses_fs_read_text: &mut bool,
    uses_stdin_read_text: &mut bool,
    uses_stdin_read_line: &mut bool,
    uses_split_whitespace: &mut bool,
    uses_split_whitespace_nth: &mut bool,
    uses_parse_i64: &mut bool,
    uses_parse_bool: &mut bool,
    uses_list_runtime: &mut bool,
    uses_adt_runtime: &mut bool,
    uses_option_runtime: &mut bool,
    uses_list_index_builtin: &mut bool,
    uses_range_inclusive: &mut bool,
    uses_iter_runtime: &mut bool,
    uses_take_builtin: &mut bool,
    uses_map_builtin: &mut bool,
    uses_option_map_builtin: &mut bool,
    uses_any_builtin: &mut bool,
    uses_filter_builtin: &mut bool,
    uses_sum_builtin: &mut bool,
    uses_join_builtin: &mut bool,
) {
    match &expr.kind {
        HighExprKind::Call { callee, args } => {
            if callee == "console.println" {
                *uses_console_println = true;
            }
            if callee == "string" {
                *uses_string_builtin = true;
            }
            if callee == "len" {
                *uses_len_builtin = true;
            }
            if callee == "ends_with_at" {
                *uses_ends_with_at_builtin = true;
            }
            if callee == "fs.read_text" {
                *uses_fs_read_text = true;
                *uses_adt_runtime = true;
            }
            if callee == "stdin.read_text" {
                *uses_stdin_read_text = true;
            }
            if callee == "stdin.read_line" {
                *uses_stdin_read_line = true;
            }
            if callee == "split_whitespace" {
                *uses_split_whitespace = true;
                *uses_list_runtime = true;
            }
            if callee == "strip_suffix" {
                *uses_option_runtime = true;
            }
            if callee == "unwrap_or" {
                *uses_option_runtime = true;
            }
            if callee == "parse.i64" {
                *uses_parse_i64 = true;
                *uses_adt_runtime = true;
            }
            if callee == "parse.bool" {
                *uses_parse_bool = true;
                *uses_adt_runtime = true;
            }
            if callee == "range_inclusive" {
                *uses_list_runtime = true;
                *uses_range_inclusive = true;
            }
            if callee == "iter.unfold" {
                *uses_iter_runtime = true;
            }
            if callee == "take" {
                *uses_iter_runtime = true;
                *uses_take_builtin = true;
            }
            if callee == "map" {
                match args.first().map(|arg| &arg.ty) {
                    Some(Type::List(_)) => {
                        *uses_list_runtime = true;
                        *uses_map_builtin = true;
                    }
                    Some(Type::Option(_)) => {
                        *uses_option_runtime = true;
                        *uses_option_map_builtin = true;
                    }
                    _ => {}
                }
            }
            if callee == "any" {
                *uses_list_runtime = true;
                *uses_any_builtin = true;
            }
            if callee == "filter" {
                *uses_list_runtime = true;
                *uses_filter_builtin = true;
            }
            if callee == "sum" {
                *uses_sum_builtin = true;
            }
            if callee == "join" {
                *uses_list_runtime = true;
                *uses_join_builtin = true;
            }
            if callee == "__index" {
                if matches_split_whitespace_nth_expr(args.first()).is_some() {
                    *uses_split_whitespace_nth = true;
                } else if matches!(args.first().map(|arg| &arg.ty), Some(Type::List(_))) {
                    *uses_list_runtime = true;
                    *uses_list_index_builtin = true;
                }
            }
            for arg in args {
                scan_expr(
                    arg,
                    uses_console_println,
                    uses_string_builtin,
                    uses_len_builtin,
                    uses_ends_with_at_builtin,
                    uses_string_eq,
                    uses_fs_read_text,
                    uses_stdin_read_text,
                    uses_stdin_read_line,
                    uses_split_whitespace,
                    uses_split_whitespace_nth,
                    uses_parse_i64,
                    uses_parse_bool,
                    uses_list_runtime,
                    uses_adt_runtime,
                    uses_option_runtime,
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
                );
            }
        }
        HighExprKind::Binary { left, right, op } => {
            if *op == lang_core::BinaryOp::Equal
                && left.ty == Type::String
                && right.ty == Type::String
            {
                *uses_string_eq = true;
            }
            scan_expr(
                left,
                uses_console_println,
                uses_string_builtin,
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
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
            );
            scan_expr(
                right,
                uses_console_println,
                uses_string_builtin,
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
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
            );
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            scan_expr(
                condition,
                uses_console_println,
                uses_string_builtin,
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
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
            );
            scan_expr(
                then_branch,
                uses_console_println,
                uses_string_builtin,
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
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
            );
            scan_expr(
                else_branch,
                uses_console_println,
                uses_string_builtin,
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
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
            );
        }
        HighExprKind::Match { subject, arms } => {
            *uses_adt_runtime = true;
            if matches!(subject.ty, Type::Option(_)) {
                *uses_option_runtime = true;
            }
            scan_expr(
                subject,
                uses_console_println,
                uses_string_builtin,
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
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
            );
            for arm in arms {
                scan_expr(
                    &arm.expr,
                    uses_console_println,
                    uses_string_builtin,
                    uses_len_builtin,
                    uses_ends_with_at_builtin,
                    uses_string_eq,
                    uses_fs_read_text,
                    uses_stdin_read_text,
                    uses_stdin_read_line,
                    uses_split_whitespace,
                    uses_split_whitespace_nth,
                    uses_parse_i64,
                    uses_parse_bool,
                    uses_list_runtime,
                    uses_adt_runtime,
                    uses_option_runtime,
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
                );
            }
        }
        HighExprKind::Construct { args, .. } => {
            *uses_adt_runtime = true;
            if matches!(expr.ty, Type::Option(_)) {
                *uses_option_runtime = true;
            }
            for arg in args {
                scan_expr(
                    arg,
                    uses_console_println,
                    uses_string_builtin,
                    uses_len_builtin,
                    uses_ends_with_at_builtin,
                    uses_string_eq,
                    uses_fs_read_text,
                    uses_stdin_read_text,
                    uses_stdin_read_line,
                    uses_split_whitespace,
                    uses_split_whitespace_nth,
                    uses_parse_i64,
                    uses_parse_bool,
                    uses_list_runtime,
                    uses_adt_runtime,
                    uses_option_runtime,
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
                );
            }
        }
        HighExprKind::List(items) | HighExprKind::Tuple(items) => {
            if matches!(&expr.kind, HighExprKind::List(_)) {
                *uses_list_runtime = true;
            }
            for item in items {
                scan_expr(
                    item,
                    uses_console_println,
                    uses_string_builtin,
                    uses_len_builtin,
                    uses_ends_with_at_builtin,
                    uses_string_eq,
                    uses_fs_read_text,
                    uses_stdin_read_text,
                    uses_stdin_read_line,
                    uses_split_whitespace,
                    uses_split_whitespace_nth,
                    uses_parse_i64,
                    uses_parse_bool,
                    uses_list_runtime,
                    uses_adt_runtime,
                    uses_option_runtime,
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
                );
            }
        }
        HighExprKind::Lambda { body, .. } => {
            scan_expr(
                body,
                uses_console_println,
                uses_string_builtin,
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
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
            );
        }
        HighExprKind::Let { value, body, .. } => {
            scan_expr(
                value,
                uses_console_println,
                uses_string_builtin,
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
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
            );
            scan_expr(
                body,
                uses_console_println,
                uses_string_builtin,
                uses_len_builtin,
                uses_ends_with_at_builtin,
                uses_string_eq,
                uses_fs_read_text,
                uses_stdin_read_text,
                uses_stdin_read_line,
                uses_split_whitespace,
                uses_split_whitespace_nth,
                uses_parse_i64,
                uses_parse_bool,
                uses_list_runtime,
                uses_adt_runtime,
                uses_option_runtime,
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
            );
        }
        HighExprKind::Int(_)
        | HighExprKind::Bool(_)
        | HighExprKind::String(_)
        | HighExprKind::Error => {}
        HighExprKind::Ident(name) => {
            if name == "string" {
                *uses_string_builtin = true;
            }
            if name == "None" {
                *uses_option_runtime = true;
            }
        }
    }
}

pub(crate) fn list_item_type(ty: &Type) -> Option<Type> {
    if let Type::List(item) = ty {
        Some((**item).clone())
    } else {
        None
    }
}

pub(crate) fn option_item_type(ty: &Type) -> Option<Type> {
    if let Type::Option(item) = ty {
        Some((**item).clone())
    } else {
        None
    }
}

pub(crate) fn callback_receiver_item_type(ty: &Type, callee: &str) -> Option<Type> {
    match callee {
        "map" => list_item_type(ty).or_else(|| option_item_type(ty)),
        "filter" | "any" => list_item_type(ty),
        _ => None,
    }
}

pub(crate) fn expr_item_type(ty: &Type) -> Type {
    match ty {
        Type::List(item) | Type::Seq(item) => (**item).clone(),
        _ => Type::Unknown,
    }
}

pub(crate) fn callback_result_type(expr: &HighExpr, scope: &HashMap<String, Type>) -> Option<Type> {
    match &expr.kind {
        HighExprKind::Lambda { body, .. } => Some(body.ty.clone()),
        HighExprKind::Ident(name) if name == "string" => Some(Type::String),
        HighExprKind::Ident(name) => {
            if let Some(Type::Fn(_, result)) = scope.get(name) {
                Some((**result).clone())
            } else {
                None
            }
        }
        _ => None,
    }
}
