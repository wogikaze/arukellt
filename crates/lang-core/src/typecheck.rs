use std::collections::HashMap;

use crate::Type;
use crate::ast::{BinaryOp, Expr, Function, MatchArm, Module, Pattern, TypeDecl};
use crate::diagnostics::{CompileResult, Diagnostic, DiagnosticLevel, DiagnosticStage, Span};

#[derive(Clone, Debug, PartialEq)]
pub struct TypedModule {
    pub imports: Vec<String>,
    pub types: Vec<TypeDecl>,
    pub functions: Vec<TypedFunction>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypedFunction {
    pub public: bool,
    pub name: String,
    pub params: Vec<TypedParam>,
    pub return_type: Type,
    pub body: TypedExpr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypedParam {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypedExpr {
    pub kind: TypedExprKind,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypedExprKind {
    Int(i64),
    Bool(bool),
    String(String),
    Ident(String),
    List(Vec<TypedExpr>),
    Tuple(Vec<TypedExpr>),
    Lambda {
        param: String,
        body: Box<TypedExpr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<TypedExpr>,
        right: Box<TypedExpr>,
    },
    If {
        condition: Box<TypedExpr>,
        then_branch: Box<TypedExpr>,
        else_branch: Box<TypedExpr>,
    },
    Match {
        subject: Box<TypedExpr>,
        arms: Vec<TypedMatchArm>,
    },
    Construct {
        variant: String,
        args: Vec<TypedExpr>,
    },
    Call {
        callee: String,
        args: Vec<TypedExpr>,
    },
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypedMatchArm {
    pub pattern: Pattern,
    pub expr: TypedExpr,
}

pub fn typecheck(module: Module, mut diagnostics: Vec<Diagnostic>) -> CompileResult<TypedModule> {
    let capabilities = module
        .imports
        .iter()
        .map(|item| item.name.clone())
        .collect::<Vec<_>>();
    let types = module.types.clone();
    let type_map = types
        .iter()
        .map(|decl| (decl.name.clone(), decl.clone()))
        .collect::<HashMap<_, _>>();
    let constructors = types
        .iter()
        .flat_map(|decl| {
            decl.variants.iter().map(|variant| {
                (
                    variant.name.clone(),
                    (
                        decl.name.clone(),
                        variant
                            .fields
                            .iter()
                            .map(|field| field.ty.clone())
                            .collect::<Vec<_>>(),
                    ),
                )
            })
        })
        .collect::<HashMap<_, _>>();
    let mut functions = Vec::new();
    let signatures = module
        .functions
        .iter()
        .map(|function| {
            (
                function.name.clone(),
                (function.return_type.clone(), function.params.clone()),
            )
        })
        .collect::<HashMap<_, _>>();

    for function in module.functions {
        let mut checker = FunctionChecker::new(
            &function,
            &signatures,
            &capabilities,
            &type_map,
            &constructors,
        );
        let body = checker.infer_expr(&function.body);
        if body.ty != function.return_type && body.ty != Type::Unknown {
            diagnostics.push(make_error(
                "E_RETURN_MISMATCH",
                "Function body type does not match the declared return type",
                function.return_type.to_string(),
                body.ty.to_string(),
                "return_type_mismatch",
                "Adjust the body or return type so they match.",
            ));
        }
        diagnostics.extend(checker.diagnostics);
        functions.push(TypedFunction {
            public: function.public,
            name: function.name,
            params: function
                .params
                .into_iter()
                .map(|param| TypedParam {
                    name: param.name,
                    ty: param.ty,
                })
                .collect(),
            return_type: function.return_type,
            body,
        });
    }

    let typed_module = TypedModule {
        imports: module.imports.into_iter().map(|item| item.name).collect(),
        types,
        functions,
    };

    CompileResult {
        module: if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.level == DiagnosticLevel::Error)
        {
            None
        } else {
            Some(typed_module)
        },
        diagnostics,
    }
}

struct FunctionChecker<'a> {
    env: HashMap<String, Type>,
    signatures: &'a HashMap<String, (Type, Vec<crate::ast::Param>)>,
    capabilities: &'a [String],
    type_map: &'a HashMap<String, TypeDecl>,
    constructors: &'a HashMap<String, (String, Vec<Type>)>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> FunctionChecker<'a> {
    fn new(
        function: &Function,
        signatures: &'a HashMap<String, (Type, Vec<crate::ast::Param>)>,
        capabilities: &'a [String],
        type_map: &'a HashMap<String, TypeDecl>,
        constructors: &'a HashMap<String, (String, Vec<Type>)>,
    ) -> Self {
        let env = function
            .params
            .iter()
            .map(|param| (param.name.clone(), param.ty.clone()))
            .collect();
        Self {
            env,
            signatures,
            capabilities,
            type_map,
            constructors,
            diagnostics: Vec::new(),
        }
    }

    fn infer_expr(&mut self, expr: &Expr) -> TypedExpr {
        match expr {
            Expr::Int(value) => TypedExpr {
                kind: TypedExprKind::Int(*value),
                ty: Type::Int,
            },
            Expr::Bool(value) => TypedExpr {
                kind: TypedExprKind::Bool(*value),
                ty: Type::Bool,
            },
            Expr::String(value) => TypedExpr {
                kind: TypedExprKind::String(value.clone()),
                ty: Type::String,
            },
            Expr::List(items) => {
                let typed_items = items.iter().map(|item| self.infer_expr(item)).collect::<Vec<_>>();
                let element_ty = typed_items
                    .first()
                    .map(|item| item.ty.clone())
                    .unwrap_or(Type::Unknown);
                TypedExpr {
                    kind: TypedExprKind::List(typed_items),
                    ty: Type::List(Box::new(element_ty)),
                }
            }
            Expr::Tuple(items) => {
                let typed_items = items.iter().map(|item| self.infer_expr(item)).collect::<Vec<_>>();
                let item_types = typed_items.iter().map(|item| item.ty.clone()).collect::<Vec<_>>();
                TypedExpr {
                    kind: TypedExprKind::Tuple(typed_items),
                    ty: Type::Tuple(item_types),
                }
            }
            Expr::Lambda { param, body } => {
                let saved = self.env.clone();
                self.env.insert(param.clone(), Type::Unknown);
                let typed_body = self.infer_expr(body);
                self.env = saved;
                TypedExpr {
                    kind: TypedExprKind::Lambda {
                        param: param.clone(),
                        body: Box::new(typed_body),
                    },
                    ty: Type::Named("Fn".to_owned()),
                }
            }
            Expr::Ident(name) => TypedExpr {
                kind: if self
                    .constructors
                    .get(name)
                    .is_some_and(|(_, field_types)| field_types.is_empty())
                {
                    TypedExprKind::Construct {
                        variant: name.clone(),
                        args: Vec::new(),
                    }
                } else if self.signatures.contains_key(name) || is_builtin_function(name) {
                    TypedExprKind::Ident(name.clone())
                } else {
                    TypedExprKind::Ident(name.clone())
                },
                ty: if let Some(ty) = self.env.get(name) {
                    ty.clone()
                } else if let Some((owner_type, field_types)) = self.constructors.get(name) {
                    if field_types.is_empty() {
                        Type::Named(owner_type.clone())
                    } else {
                        self.diagnostics.push(make_error(
                            "E_CONSTRUCTOR_ARGUMENTS_REQUIRED",
                            "Constructor arguments are required for this variant",
                            field_types.len().to_string(),
                            "0",
                            "missing_constructor_arguments",
                            "Call the constructor with the declared field values.",
                        ));
                        Type::Named(owner_type.clone())
                    }
                } else if self.signatures.contains_key(name) || is_builtin_function(name) {
                    Type::Named("Fn".to_owned())
                } else {
                    self.diagnostics.push(make_error(
                        "E_UNKNOWN_IDENT",
                        "Unknown identifier",
                        "bound identifier",
                        name.clone(),
                        "unknown_identifier",
                        "Introduce the identifier before using it.",
                    ));
                    Type::Unknown
                },
            },
            Expr::Binary { op, left, right } => {
                let typed_left = self.infer_expr(left);
                let typed_right = self.infer_expr(right);
                let ty = match op {
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Modulo => {
                        self.expect_type("Int", &typed_left.ty, "binary_left_operand");
                        self.expect_type("Int", &typed_right.ty, "binary_right_operand");
                        Type::Int
                    }
                    BinaryOp::Greater | BinaryOp::Less => {
                        self.expect_type("Int", &typed_left.ty, "comparison_left_operand");
                        self.expect_type("Int", &typed_right.ty, "comparison_right_operand");
                        Type::Bool
                    }
                    BinaryOp::Equal => {
                        if typed_left.ty != typed_right.ty {
                            self.diagnostics.push(make_error(
                                "E_EQUAL_MISMATCH",
                                "Equality operands must have the same type",
                                typed_left.ty.to_string(),
                                typed_right.ty.to_string(),
                                "equality_type_mismatch",
                                "Convert one operand so both sides share the same type.",
                            ));
                        }
                        Type::Bool
                    }
                    BinaryOp::Or => {
                        self.expect_type("Bool", &typed_left.ty, "or_left_operand");
                        self.expect_type("Bool", &typed_right.ty, "or_right_operand");
                        Type::Bool
                    }
                };
                TypedExpr {
                    kind: TypedExprKind::Binary {
                        op: *op,
                        left: Box::new(typed_left),
                        right: Box::new(typed_right),
                    },
                    ty,
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let typed_condition = self.infer_expr(condition);
                let typed_then = self.infer_expr(then_branch);
                let typed_else = self.infer_expr(else_branch);
                self.expect_type("Bool", &typed_condition.ty, "if_condition");
                let ty = merge_types(&typed_then.ty, &typed_else.ty);
                if ty == Type::Unknown && typed_then.ty != Type::Unknown && typed_else.ty != Type::Unknown
                {
                    self.diagnostics.push(make_error(
                        "E_IF_BRANCH_MISMATCH",
                        "If branches must produce the same type",
                        typed_then.ty.to_string(),
                        typed_else.ty.to_string(),
                        "if_branch_mismatch",
                        "Align the then/else expressions to the same type.",
                    ));
                }
                TypedExpr {
                    kind: TypedExprKind::If {
                        condition: Box::new(typed_condition),
                        then_branch: Box::new(typed_then),
                        else_branch: Box::new(typed_else),
                    },
                    ty,
                }
            }
            Expr::Match { subject, arms } => self.infer_match(subject, arms),
            Expr::Apply { func, args } => {
                let typed_func = self.infer_expr(func);
                let mut typed_args = vec![typed_func];
                typed_args.extend(args.iter().map(|arg| self.infer_expr(arg)));
                let ty = builtin_return_type("__apply", &typed_args).unwrap_or(Type::Unknown);
                TypedExpr {
                    kind: TypedExprKind::Call {
                        callee: "__apply".to_owned(),
                        args: typed_args,
                    },
                    ty,
                }
            }
            Expr::Range { start, end } => {
                let typed_start = self.infer_expr(start);
                let typed_end = self.infer_expr(end);
                let typed_args = vec![typed_start, typed_end];
                let ty = builtin_return_type("range_inclusive", &typed_args)
                    .unwrap_or(Type::List(Box::new(Type::Int)));
                TypedExpr {
                    kind: TypedExprKind::Call {
                        callee: "range_inclusive".to_owned(),
                        args: typed_args,
                    },
                    ty,
                }
            }
            Expr::MethodCall {
                receiver,
                method,
                args,
            } => {
                let typed_receiver = self.infer_expr(receiver);
                let mut typed_args = vec![typed_receiver];
                typed_args.extend(args.iter().map(|arg| self.infer_expr(arg)));
                let ty = builtin_return_type(method, &typed_args).unwrap_or_else(|| {
                    if let Some((return_type, _)) = self.signatures.get(method) {
                        return_type.clone()
                    } else {
                        Type::Unknown
                    }
                });
                TypedExpr {
                    kind: TypedExprKind::Call {
                        callee: method.clone(),
                        args: typed_args,
                    },
                    ty,
                }
            }
            Expr::Index { receiver, index } => {
                let typed_receiver = self.infer_expr(receiver);
                let typed_index = self.infer_expr(index);
                let typed_args = vec![typed_receiver, typed_index];
                let ty = builtin_return_type("__index", &typed_args).unwrap_or(Type::Unknown);
                TypedExpr {
                    kind: TypedExprKind::Call {
                        callee: "__index".to_owned(),
                        args: typed_args,
                    },
                    ty,
                }
            }
            Expr::Call { callee, args } => {
                let typed_args = args
                    .iter()
                    .map(|arg| self.infer_expr(arg))
                    .collect::<Vec<_>>();
                if let Some(ty) = builtin_return_type(callee, &typed_args) {
                    return TypedExpr {
                        kind: TypedExprKind::Call {
                            callee: callee.clone(),
                            args: typed_args,
                        },
                        ty,
                    };
                }
                if matches!(callee.as_str(), "Ok" | "Err") {
                    let ty = if callee == "Ok" {
                        Type::Result(
                            Box::new(typed_args.first().map(|arg| arg.ty.clone()).unwrap_or(Type::Unknown)),
                            Box::new(Type::Unknown),
                        )
                    } else {
                        Type::Result(
                            Box::new(Type::Unknown),
                            Box::new(typed_args.first().map(|arg| arg.ty.clone()).unwrap_or(Type::Unknown)),
                        )
                    };
                    return TypedExpr {
                        kind: TypedExprKind::Construct {
                            variant: callee.clone(),
                            args: typed_args,
                        },
                        ty,
                    };
                }
                if let Some((owner_type, field_types)) = self.constructors.get(callee) {
                    if typed_args.len() != field_types.len() {
                        self.diagnostics.push(make_error(
                            "E_CONSTRUCTOR_ARITY_MISMATCH",
                            "Constructor uses the wrong number of arguments",
                            field_types.len().to_string(),
                            typed_args.len().to_string(),
                            "constructor_arity_mismatch",
                            "Pass the same number of fields as the variant declaration.",
                        ));
                    }
                    for (index, (expected, actual)) in
                        field_types.iter().zip(&typed_args).enumerate()
                    {
                        if expected != &actual.ty && actual.ty != Type::Unknown {
                            self.diagnostics.push(make_error(
                                "E_CONSTRUCTOR_ARG_MISMATCH",
                                "Constructor argument type does not match the declared field type",
                                expected.to_string(),
                                actual.ty.to_string(),
                                &format!("constructor_arg_{index}"),
                                "Pass values that match the constructor field types.",
                            ));
                        }
                    }
                    return TypedExpr {
                        kind: TypedExprKind::Construct {
                            variant: callee.clone(),
                            args: typed_args,
                        },
                        ty: Type::Named(owner_type.clone()),
                    };
                }
                let ty = if self
                    .capabilities
                    .iter()
                    .any(|capability| capability == callee)
                {
                    self.diagnostics.push(make_error(
                        "E_EFFECT_LEAK",
                        &format!("pure functions cannot call capability `{callee}` directly"),
                        "pure expression",
                        format!("capability call `{callee}`"),
                        "effect_leak",
                        "Move the capability call to the host boundary or expose it through an explicit effect boundary.",
                    ));
                    Type::Unknown
                } else if let Some((return_type, params)) = self.signatures.get(callee) {
                    if params.len() != typed_args.len() {
                        self.diagnostics.push(make_error(
                            "E_ARITY_MISMATCH",
                            "Call uses the wrong number of arguments",
                            params.len().to_string(),
                            typed_args.len().to_string(),
                            "call_arity_mismatch",
                            "Pass the same number of arguments as the function declaration.",
                        ));
                    }
                    return_type.clone()
                } else {
                    self.diagnostics.push(make_error(
                        "E_UNKNOWN_CALL",
                        "Unknown function call",
                        "known function",
                        callee.clone(),
                        "unknown_call_target",
                        "Declare the function before calling it.",
                    ));
                    Type::Unknown
                };
                TypedExpr {
                    kind: TypedExprKind::Call {
                        callee: callee.clone(),
                        args: typed_args,
                    },
                    ty,
                }
            }
            Expr::Error => TypedExpr {
                kind: TypedExprKind::Error,
                ty: Type::Unknown,
            },
        }
    }

    fn infer_match(&mut self, subject: &Expr, arms: &[MatchArm]) -> TypedExpr {
        let typed_subject = self.infer_expr(subject);
        let mut typed_arms = Vec::new();
        let mut seen_variants = Vec::new();
        let mut has_wildcard = false;
        let mut arm_type = Type::Unknown;

        for arm in arms {
            let mut arm_env = self.env.clone();
            match &arm.pattern {
                Pattern::Wildcard => {
                    has_wildcard = true;
                }
                Pattern::Variant { name, bindings } => {
                    if matches!(name.as_str(), "Ok" | "Err") {
                        let builtin_binding_ty = match (&typed_subject.ty, name.as_str()) {
                            (Type::Result(ok, _), "Ok") => (**ok).clone(),
                            (Type::Result(_, err), "Err") => (**err).clone(),
                            _ => Type::Unknown,
                        };
                        for binding in bindings {
                            arm_env.insert(binding.clone(), builtin_binding_ty.clone());
                        }
                    } else {
                        seen_variants.push(name.clone());
                        if let Some((owner_type, field_types)) = self.constructors.get(name) {
                            if typed_subject.ty != Type::Named(owner_type.clone())
                                && typed_subject.ty != Type::Unknown
                            {
                                self.diagnostics.push(make_error(
                                    "E_PATTERN_TYPE_MISMATCH",
                                    "Pattern variant does not match the match subject type",
                                    owner_type.clone(),
                                    typed_subject.ty.to_string(),
                                    "pattern_type_mismatch",
                                    "Use variants from the same type as the matched value.",
                                ));
                            }
                            if bindings.len() != field_types.len() {
                                self.diagnostics.push(make_error(
                                    "E_PATTERN_ARITY_MISMATCH",
                                    "Pattern binding count does not match the variant fields",
                                    field_types.len().to_string(),
                                    bindings.len().to_string(),
                                    "pattern_arity_mismatch",
                                    "Bind the same number of fields as declared on the variant.",
                                ));
                            }
                            for (binding, ty) in bindings.iter().zip(field_types.iter()) {
                                arm_env.insert(binding.clone(), ty.clone());
                            }
                        } else {
                            self.diagnostics.push(make_error(
                                "E_UNKNOWN_VARIANT",
                                "Unknown variant used in match pattern",
                                "known variant".to_owned(),
                                name.clone(),
                                "unknown_variant_pattern",
                                "Use a variant declared in the matching type.",
                            ));
                        }
                    }
                }
            }

            let saved_env = std::mem::replace(&mut self.env, arm_env);
            let typed_expr = self.infer_expr(&arm.expr);
            self.env = saved_env;
            let merged_arm_type = merge_types(&arm_type, &typed_expr.ty);
            if arm_type == Type::Unknown {
                arm_type = typed_expr.ty.clone();
            } else if merged_arm_type != Type::Unknown {
                arm_type = merged_arm_type;
            } else if typed_expr.ty != Type::Unknown {
                self.diagnostics.push(make_error(
                    "E_MATCH_ARM_MISMATCH",
                    "All match arms must return the same type",
                    arm_type.to_string(),
                    typed_expr.ty.to_string(),
                    "match_arm_type_mismatch",
                    "Rewrite the match arms so they return a common type.",
                ));
                arm_type = Type::Unknown;
            }
            typed_arms.push(TypedMatchArm {
                pattern: arm.pattern.clone(),
                expr: typed_expr,
            });
        }

        if let Type::Named(type_name) = &typed_subject.ty {
            if let Some(type_decl) = self.type_map.get(type_name) {
                let all_variants = type_decl
                    .variants
                    .iter()
                    .map(|variant| variant.name.clone())
                    .collect::<Vec<_>>();
                if !has_wildcard
                    && all_variants
                        .iter()
                        .any(|variant| !seen_variants.contains(variant))
                {
                    self.diagnostics.push(make_error(
                        "E_MATCH_NOT_EXHAUSTIVE",
                        "Match expression is not exhaustive",
                        format!("{all_variants:?}"),
                        format!("{seen_variants:?}"),
                        "match_not_exhaustive",
                        "Cover every variant or add a final wildcard arm.",
                    ));
                }
            }
        }

        TypedExpr {
            kind: TypedExprKind::Match {
                subject: Box::new(typed_subject),
                arms: typed_arms,
            },
            ty: arm_type,
        }
    }

    fn expect_type(&mut self, expected: &str, actual: &Type, cause: &str) {
        if actual == &Type::Unknown || actual.to_string() == expected {
            return;
        }
        self.diagnostics.push(make_error(
            "E_TYPE_MISMATCH",
            "Type mismatch",
            expected.to_owned(),
            actual.to_string(),
            cause,
            "Change the expression so it matches the expected type.",
        ));
    }
}

fn builtin_return_type(callee: &str, args: &[TypedExpr]) -> Option<Type> {
    Some(match callee {
        "__apply" => Type::Unknown,
        "__index" => match args.first().map(|arg| &arg.ty) {
            Some(Type::List(inner)) => (**inner).clone(),
            Some(Type::Tuple(items)) => items.first().cloned().unwrap_or(Type::Unknown),
            _ => Type::Unknown,
        },
        "range_inclusive" => Type::List(Box::new(Type::Int)),
        "string" => Type::String,
        "map" => Type::List(Box::new(Type::Unknown)),
        "filter" => args.first().map(|arg| arg.ty.clone()).unwrap_or(Type::Unknown),
        "sum" => Type::Int,
        "join" => Type::String,
        "take" => Type::List(Box::new(Type::Unknown)),
        "iter.unfold" => Type::Named("Iter".to_owned()),
        "console.println" => Type::Unit,
        "fs.read_text" => Type::Result(Box::new(Type::String), Box::new(Type::Unknown)),
        "Next" => Type::Named("Next".to_owned()),
        "Done" => Type::Named("Done".to_owned()),
        _ if callee.starts_with("console.") => Type::Unit,
        _ => return None,
    })
}

fn merge_types(left: &Type, right: &Type) -> Type {
    if left == right {
        return left.clone();
    }

    match (left, right) {
        (Type::Unknown, other) => other.clone(),
        (other, Type::Unknown) => other.clone(),
        (Type::Result(left_ok, left_err), Type::Result(right_ok, right_err)) => Type::Result(
            Box::new(merge_types(left_ok, right_ok)),
            Box::new(merge_types(left_err, right_err)),
        ),
        (Type::List(left_inner), Type::List(right_inner)) => {
            Type::List(Box::new(merge_types(left_inner, right_inner)))
        }
        (Type::Tuple(left_items), Type::Tuple(right_items))
            if left_items.len() == right_items.len() =>
        {
            Type::Tuple(
                left_items
                    .iter()
                    .zip(right_items.iter())
                    .map(|(left, right)| merge_types(left, right))
                    .collect(),
            )
        }
        _ => Type::Unknown,
    }
}

fn is_builtin_function(name: &str) -> bool {
    matches!(
        name,
        "string"
            | "map"
            | "filter"
            | "sum"
            | "join"
            | "take"
            | "range_inclusive"
            | "console.println"
            | "fs.read_text"
            | "iter.unfold"
            | "Ok"
            | "Err"
            | "Next"
            | "Done"
    )
}

fn make_error(
    code: &str,
    message: &str,
    expected: impl Into<String>,
    actual: impl Into<String>,
    cause: &str,
    suggested_fix: &str,
) -> Diagnostic {
    Diagnostic {
        code: code.to_owned(),
        message: message.to_owned(),
        level: DiagnosticLevel::Error,
        stage: DiagnosticStage::Typecheck,
        range: Span { start: 0, end: 0 },
        expected: expected.into(),
        actual: actual.into(),
        cause: cause.to_owned(),
        related: Vec::new(),
        suggested_fix: suggested_fix.to_owned(),
        alternatives: vec![
            "Add an explicit type annotation.".to_owned(),
            "Rewrite the expression to the expected type.".to_owned(),
        ],
        confidence: 0.86,
    }
}
