use lang_core::{
    BinaryOp, TypedExpr, TypedExprKind, TypedFunction, TypedMatchArm, TypedModule, TypedParam,
};

use crate::high::{HighExpr, HighExprKind, HighFunction, HighMatchArm, HighModule, HighParam};
use crate::low::{LowFunction, LowInstruction, LowModule};

#[must_use]
pub fn lower_to_high_ir(module: &TypedModule) -> HighModule {
    HighModule {
        imports: module.imports.clone(),
        types: module.types.clone(),
        functions: module.functions.iter().map(lower_function).collect(),
    }
}

#[must_use]
pub fn lower_to_low_ir(module: &HighModule) -> LowModule {
    LowModule {
        functions: module.functions.iter().map(lower_function_to_low).collect(),
    }
}

fn lower_function(function: &TypedFunction) -> HighFunction {
    HighFunction {
        public: function.public,
        name: function.name.clone(),
        params: function.params.iter().map(lower_param).collect(),
        return_type: function.return_type.clone(),
        body: lower_expr(&function.body),
    }
}

fn lower_param(param: &TypedParam) -> HighParam {
    HighParam {
        name: param.name.clone(),
        ty: param.ty.clone(),
    }
}

fn lower_match_arm(arm: &TypedMatchArm) -> HighMatchArm {
    HighMatchArm {
        pattern: arm.pattern.clone(),
        expr: lower_expr(&arm.expr),
    }
}

fn lower_expr(expr: &TypedExpr) -> HighExpr {
    let kind = match &expr.kind {
        TypedExprKind::Int(value) => HighExprKind::Int(*value),
        TypedExprKind::Bool(value) => HighExprKind::Bool(*value),
        TypedExprKind::String(value) => HighExprKind::String(value.clone()),
        TypedExprKind::Ident(name) => HighExprKind::Ident(name.clone()),
        TypedExprKind::List(items) => {
            HighExprKind::List(items.iter().map(lower_expr).collect())
        }
        TypedExprKind::Tuple(items) => {
            HighExprKind::Tuple(items.iter().map(lower_expr).collect())
        }
        TypedExprKind::Lambda { param, body } => HighExprKind::Lambda {
            param: param.clone(),
            body: Box::new(lower_expr(body)),
        },
        TypedExprKind::Binary { op, left, right } => HighExprKind::Binary {
            op: *op,
            left: Box::new(lower_expr(left)),
            right: Box::new(lower_expr(right)),
        },
        TypedExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => HighExprKind::If {
            condition: Box::new(lower_expr(condition)),
            then_branch: Box::new(lower_expr(then_branch)),
            else_branch: Box::new(lower_expr(else_branch)),
        },
        TypedExprKind::Match { subject, arms } => HighExprKind::Match {
            subject: Box::new(lower_expr(subject)),
            arms: arms.iter().map(lower_match_arm).collect(),
        },
        TypedExprKind::Construct { variant, args } => HighExprKind::Construct {
            variant: variant.clone(),
            args: args.iter().map(lower_expr).collect(),
        },
        TypedExprKind::Call { callee, args } => HighExprKind::Call {
            callee: callee.clone(),
            args: args.iter().map(lower_expr).collect(),
        },
        TypedExprKind::Error => HighExprKind::Error,
    };
    HighExpr {
        kind,
        ty: expr.ty.clone(),
    }
}

fn lower_function_to_low(function: &HighFunction) -> LowFunction {
    let mut instructions = function
        .params
        .iter()
        .map(|param| LowInstruction::Param(param.name.clone()))
        .collect::<Vec<_>>();
    lower_expr_to_low(&function.body, &mut instructions);
    instructions.push(LowInstruction::Return);
    LowFunction {
        name: function.name.clone(),
        instructions,
    }
}

fn lower_expr_to_low(expr: &HighExpr, instructions: &mut Vec<LowInstruction>) {
    match &expr.kind {
        HighExprKind::Int(value) => instructions.push(LowInstruction::ConstInt(*value)),
        HighExprKind::Bool(value) => instructions.push(LowInstruction::ConstBool(*value)),
        HighExprKind::String(value) => {
            instructions.push(LowInstruction::ConstString(value.clone()))
        }
        HighExprKind::Ident(name) => instructions.push(LowInstruction::Load(name.clone())),
        HighExprKind::List(_) | HighExprKind::Tuple(_) | HighExprKind::Lambda { .. } => {
            instructions.push(LowInstruction::Error)
        }
        HighExprKind::Binary { op, left, right } => {
            lower_expr_to_low(left, instructions);
            lower_expr_to_low(right, instructions);
            instructions.push(match op {
                BinaryOp::Add => LowInstruction::Add,
                BinaryOp::Subtract => LowInstruction::Subtract,
                BinaryOp::Multiply => LowInstruction::Multiply,
                BinaryOp::Divide => LowInstruction::Divide,
                BinaryOp::Modulo => LowInstruction::Error,
                BinaryOp::Greater => LowInstruction::Greater,
                BinaryOp::Less => LowInstruction::Error,
                BinaryOp::Equal => LowInstruction::Equal,
                BinaryOp::Or => LowInstruction::Error,
            });
        }
        HighExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            lower_expr_to_low(condition, instructions);
            instructions.push(LowInstruction::IfStart);
            lower_expr_to_low(then_branch, instructions);
            instructions.push(LowInstruction::ElseStart);
            lower_expr_to_low(else_branch, instructions);
            instructions.push(LowInstruction::EndIf);
        }
        HighExprKind::Call { callee, args } => {
            for arg in args {
                lower_expr_to_low(arg, instructions);
            }
            instructions.push(LowInstruction::Call {
                callee: callee.clone(),
                argc: args.len(),
            });
        }
        HighExprKind::Construct { .. } | HighExprKind::Match { .. } | HighExprKind::Error => {
            instructions.push(LowInstruction::Error);
        }
    }
}
