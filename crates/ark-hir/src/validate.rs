use std::collections::BTreeSet;

use crate::hir::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub message: String,
}

impl ValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub fn validate_program(program: &Program) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    if !program.source_map.program_spans.contains_key(&program.id) {
        errors.push(ValidationError::new("program source span missing"));
    }

    for module in &program.modules {
        if !program.source_map.module_spans.contains_key(&module.id) {
            errors.push(ValidationError::new(format!("module `{}` source span missing", module.name)));
        }
        for item in &module.items {
            if !program.source_map.item_spans.contains_key(&item.id) {
                errors.push(ValidationError::new(format!("item `{}` source span missing", item.name)));
            }
            validate_item(program, item, &mut errors);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_item(program: &Program, item: &Item, errors: &mut Vec<ValidationError>) {
    match &item.kind {
        ItemKind::Function(fun) => validate_body_ref(program, fun.body, errors),
        ItemKind::Impl(imp) => {
            for method in &imp.methods {
                if !program.source_map.item_spans.contains_key(method) {
                    errors.push(ValidationError::new(format!(
                        "impl method item {:?} source span missing",
                        method
                    )));
                }
            }
        }
        ItemKind::Trait(tr) => {
            for method in &tr.methods {
                if method.name.is_empty() {
                    errors.push(ValidationError::new("trait method name must not be empty"));
                }
            }
        }
        ItemKind::Struct(_) | ItemKind::Enum(_) => {}
    }
}

fn validate_body_ref(program: &Program, body_id: crate::ids::BodyId, errors: &mut Vec<ValidationError>) {
    let Some(body) = program.bodies.iter().find(|body| body.id == body_id) else {
        errors.push(ValidationError::new(format!("body {:?} missing", body_id)));
        return;
    };

    if !program.source_map.body_spans.contains_key(&body.id) {
        errors.push(ValidationError::new(format!("body {:?} source span missing", body.id)));
    }

    for stmt in &body.stmts {
        validate_stmt(program, stmt, errors);
    }
    if let Some(tail) = &body.tail {
        validate_expr(program, tail, errors);
    }
}

fn validate_stmt(program: &Program, stmt: &Stmt, errors: &mut Vec<ValidationError>) {
    match stmt {
        Stmt::Let(let_stmt) => {
            validate_expr(program, &let_stmt.init, errors);
            if let Some(pattern) = &let_stmt.pattern {
                validate_pattern(program, pattern, errors);
            }
        }
        Stmt::Expr(expr) => validate_expr(program, expr, errors),
        Stmt::While { cond, body } => {
            validate_expr(program, cond, errors);
            validate_body_ref(program, *body, errors);
        }
        Stmt::Loop { body } => validate_body_ref(program, *body, errors),
        Stmt::ForRange { start, end, body, .. } => {
            validate_expr(program, start, errors);
            validate_expr(program, end, errors);
            validate_body_ref(program, *body, errors);
        }
        Stmt::ForValues { iter, body, .. } => {
            validate_expr(program, iter, errors);
            validate_body_ref(program, *body, errors);
        }
    }
}

fn validate_expr(program: &Program, expr: &Expr, errors: &mut Vec<ValidationError>) {
    if !program.source_map.expr_spans.contains_key(&expr.id) {
        errors.push(ValidationError::new(format!("expr {:?} source span missing", expr.id)));
    }
    if matches!(expr.ty, Ty::Error) {
        errors.push(ValidationError::new(format!("expr {:?} has error type", expr.id)));
    }

    match &expr.kind {
        ExprKind::Const(_) | ExprKind::Local(_) | ExprKind::Global(_) | ExprKind::QualifiedGlobal { .. } | ExprKind::Continue => {}
        ExprKind::Call { target, args } => {
            validate_call_target(program, target, errors);
            for arg in args {
                validate_expr(program, &arg.expr, errors);
            }
        }
        ExprKind::BuiltinBinary { left, right, .. } => {
            validate_expr(program, left, errors);
            validate_expr(program, right, errors);
        }
        ExprKind::BuiltinUnary { operand, .. } => validate_expr(program, operand, errors),
        ExprKind::If { cond, then_body, else_body } => {
            validate_expr(program, cond, errors);
            validate_body_ref(program, *then_body, errors);
            if let Some(else_body) = else_body {
                validate_body_ref(program, *else_body, errors);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            validate_expr(program, scrutinee, errors);
            for arm in arms {
                validate_pattern(program, &arm.pattern, errors);
                if let Some(guard) = &arm.guard {
                    validate_expr(program, guard, errors);
                }
                validate_expr(program, &arm.body, errors);
            }
        }
        ExprKind::Block(body) | ExprKind::Loop(body) => validate_body_ref(program, *body, errors),
        ExprKind::Tuple(exprs) | ExprKind::Array(exprs) | ExprKind::StringConcatMany(exprs) => {
            for expr in exprs {
                validate_expr(program, expr, errors);
            }
        }
        ExprKind::ArrayRepeat { value, count } => {
            validate_expr(program, value, errors);
            validate_expr(program, count, errors);
        }
        ExprKind::StructInit { fields, .. } => {
            for (_, value) in fields {
                validate_expr(program, value, errors);
            }
        }
        ExprKind::FieldAccess { object, .. } => validate_expr(program, object, errors),
        ExprKind::Index { object, index } => {
            validate_expr(program, object, errors);
            validate_expr(program, index, errors);
        }
        ExprKind::Return(value) | ExprKind::Break(value) => {
            if let Some(value) = value {
                validate_expr(program, value, errors);
            }
        }
        ExprKind::Try(expr) => validate_expr(program, expr, errors),
        ExprKind::Assign { target, value, .. } => {
            validate_expr(program, target, errors);
            validate_expr(program, value, errors);
        }
        ExprKind::Closure { body, captures, .. } => {
            validate_body_ref(program, *body, errors);
            for capture in captures {
                if capture.name.is_empty() {
                    errors.push(ValidationError::new("capture name must not be empty"));
                }
            }
        }
    }
}

fn validate_call_target(program: &Program, target: &CallTarget, errors: &mut Vec<ValidationError>) {
    match target {
        CallTarget::Direct { function } => {
            if function.is_empty() {
                errors.push(ValidationError::new("direct call target must not be empty"));
            }
        }
        CallTarget::Selected { function, selection } => {
            if function.is_empty() || selection.resolved_function.is_empty() {
                errors.push(ValidationError::new("selected call target must be resolved"));
            }
        }
        CallTarget::Indirect(expr) => validate_expr(program, expr, errors),
    }
}

fn validate_pattern(program: &Program, pattern: &Pattern, errors: &mut Vec<ValidationError>) {
    if !program.source_map.pattern_spans.contains_key(&pattern.id) {
        errors.push(ValidationError::new(format!("pattern {:?} source span missing", pattern.id)));
    }
    if matches!(pattern.ty, Ty::Error) {
        errors.push(ValidationError::new(format!("pattern {:?} has error type", pattern.id)));
    }

    let mut names = BTreeSet::new();
    collect_pattern_names(pattern, &mut names, errors);
}

fn collect_pattern_names(
    pattern: &Pattern,
    names: &mut BTreeSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Const(_) => {}
        PatternKind::Binding { name, .. } => {
            if !names.insert(name.clone()) {
                errors.push(ValidationError::new(format!("duplicate pattern binding `{}`", name)));
            }
        }
        PatternKind::Tuple(patterns) | PatternKind::Or(patterns) => {
            for pattern in patterns {
                collect_pattern_names(pattern, names, errors);
            }
        }
        PatternKind::Enum { fields, .. } => {
            for field in fields {
                collect_pattern_names(field, names, errors);
            }
        }
        PatternKind::Struct { fields, .. } => {
            for (_, pattern) in fields {
                if let Some(pattern) = pattern {
                    collect_pattern_names(pattern, names, errors);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ark_diagnostics::Span;

    use super::*;
    use crate::ids::{BodyId, ExprId, ItemId, LocalId, ModuleId, PatternId, ProgramId};
    use crate::source_map::SourceMap;

    fn ok_program() -> Program {
        let expr = Expr {
            id: ExprId(0),
            kind: ExprKind::Const(ConstValue::Int(1)),
            ty: Ty::I32,
            selection: None,
        };
        let body = Body {
            id: BodyId(0),
            params: vec![],
            locals: vec![],
            stmts: vec![Stmt::Expr(expr)],
            tail: None,
        };
        let item = Item {
            id: ItemId(0),
            name: "main".into(),
            kind: ItemKind::Function(FunctionItem {
                body: BodyId(0),
                params: vec![],
                return_ty: Ty::Unit,
                type_params: vec![],
                type_param_bounds: vec![],
                effects: EffectFlags::default(),
            }),
        };
        let mut source_map = SourceMap::default();
        source_map.insert_program(ProgramId(0), Span::dummy());
        source_map.insert_module(ModuleId(0), Span::dummy());
        source_map.insert_item(ItemId(0), Span::dummy());
        source_map.insert_body(BodyId(0), Span::dummy());
        source_map.insert_expr(ExprId(0), Span::dummy());
        Program {
            id: ProgramId(0),
            modules: vec![Module {
                id: ModuleId(0),
                name: "main".into(),
                imports: vec![],
                items: vec![item],
            }],
            bodies: vec![body],
            source_map,
        }
    }

    #[test]
    fn validates_well_formed_program() {
        assert!(validate_program(&ok_program()).is_ok());
    }

    #[test]
    fn rejects_missing_expr_span() {
        let mut program = ok_program();
        program.source_map.expr_spans.clear();
        let errors = validate_program(&program).unwrap_err();
        assert!(errors.iter().any(|err| err.message.contains("expr")));
    }

    #[test]
    fn rejects_duplicate_pattern_bindings() {
        let pattern = Pattern {
            id: PatternId(0),
            kind: PatternKind::Tuple(vec![
                Pattern {
                    id: PatternId(1),
                    kind: PatternKind::Binding {
                        local: LocalId(0),
                        name: "x".into(),
                    },
                    ty: Ty::I32,
                },
                Pattern {
                    id: PatternId(2),
                    kind: PatternKind::Binding {
                        local: LocalId(1),
                        name: "x".into(),
                    },
                    ty: Ty::I32,
                },
            ]),
            ty: Ty::Tuple(vec![Ty::I32, Ty::I32]),
        };
        let mut program = ok_program();
        program.bodies[0].stmts = vec![Stmt::Let(LetStmt {
            local: Local {
                id: LocalId(2),
                name: "pair".into(),
                ty: Ty::Tuple(vec![Ty::I32, Ty::I32]),
                value_mode: ValueMode::ValueCopy,
            },
            init: Expr {
                id: ExprId(1),
                kind: ExprKind::Tuple(vec![]),
                ty: Ty::Tuple(vec![Ty::I32, Ty::I32]),
                selection: None,
            },
            pattern: Some(pattern),
            binding_mode: ValueMode::ValueCopy,
        })];
        program.source_map.insert_expr(ExprId(1), Span::dummy());
        program.source_map.insert_pattern(PatternId(0), Span::dummy());
        program.source_map.insert_pattern(PatternId(1), Span::dummy());
        program.source_map.insert_pattern(PatternId(2), Span::dummy());

        let errors = validate_program(&program).unwrap_err();
        assert!(errors.iter().any(|err| err.message.contains("duplicate pattern binding")));
    }
}
