//! Type checker implementation with bidirectional inference.

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink};
use ark_parser::ast;
use ark_resolve::ResolvedModule;

use crate::types::{Type, TypeId};
use std::collections::HashMap;

/// Type environment for tracking variable types.
#[derive(Debug)]
pub struct TypeEnv {
    bindings: HashMap<String, Type>,
    parent: Option<Box<TypeEnv>>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self { bindings: HashMap::new(), parent: None }
    }

    pub fn child(&self) -> Self {
        // Cloning is expensive but simplifies the implementation
        Self { bindings: HashMap::new(), parent: Some(Box::new(self.clone())) }
    }

    pub fn bind(&mut self, name: String, ty: Type) {
        self.bindings.insert(name, ty);
    }

    pub fn lookup(&self, name: &str) -> Option<&Type> {
        self.bindings.get(name).or_else(|| {
            self.parent.as_ref().and_then(|p| p.lookup(name))
        })
    }
}

impl Clone for TypeEnv {
    fn clone(&self) -> Self {
        Self {
            bindings: self.bindings.clone(),
            parent: self.parent.clone(),
        }
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a struct definition.
#[derive(Debug, Clone)]
pub struct StructInfo {
    pub name: String,
    pub fields: Vec<(String, Type)>,
    pub type_id: TypeId,
}

/// Information about an enum definition.
#[derive(Debug, Clone)]
pub struct EnumInfo {
    pub name: String,
    pub variants: Vec<VariantInfo>,
    pub type_params: Vec<String>,
    pub type_id: TypeId,
}

/// Variant info.
#[derive(Debug, Clone)]
pub struct VariantInfo {
    pub name: String,
    pub fields: Vec<Type>,
}

/// Information about a function signature.
#[derive(Debug, Clone)]
pub struct FnSig {
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<Type>,
    pub ret: Type,
}

/// The main type checker.
#[derive(Debug)]
pub struct TypeChecker {
    pub struct_defs: HashMap<String, StructInfo>,
    pub enum_defs: HashMap<String, EnumInfo>,
    pub fn_sigs: HashMap<String, FnSig>,
    next_type_id: u32,
    next_type_var: u32,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            fn_sigs: HashMap::new(),
            next_type_id: 0,
            next_type_var: 0,
        }
    }

    fn fresh_type_id(&mut self) -> TypeId {
        let id = TypeId(self.next_type_id);
        self.next_type_id += 1;
        id
    }

    fn fresh_type_var(&mut self) -> Type {
        let id = self.next_type_var;
        self.next_type_var += 1;
        Type::TypeVar(id)
    }

    /// Register builtin types (Option, Result, Vec, String).
    pub fn register_builtins(&mut self) {
        // Option<T> is an enum with None and Some(T)
        let opt_id = self.fresh_type_id();
        self.enum_defs.insert("Option".into(), EnumInfo {
            name: "Option".into(),
            variants: vec![
                VariantInfo { name: "None".into(), fields: vec![] },
                VariantInfo { name: "Some".into(), fields: vec![Type::TypeVar(0)] },
            ],
            type_params: vec!["T".into()],
            type_id: opt_id,
        });

        // Result<T, E>
        let res_id = self.fresh_type_id();
        self.enum_defs.insert("Result".into(), EnumInfo {
            name: "Result".into(),
            variants: vec![
                VariantInfo { name: "Ok".into(), fields: vec![Type::TypeVar(0)] },
                VariantInfo { name: "Err".into(), fields: vec![Type::TypeVar(1)] },
            ],
            type_params: vec!["T".into(), "E".into()],
            type_id: res_id,
        });

        // Builtin I/O functions
        self.fn_sigs.insert("println".into(), FnSig {
            name: "println".into(),
            type_params: vec![],
            params: vec![Type::String],
            ret: Type::Unit,
        });
        self.fn_sigs.insert("print".into(), FnSig {
            name: "print".into(),
            type_params: vec![],
            params: vec![Type::String],
            ret: Type::Unit,
        });
        self.fn_sigs.insert("eprintln".into(), FnSig {
            name: "eprintln".into(),
            type_params: vec![],
            params: vec![Type::String],
            ret: Type::Unit,
        });

        // Conversion functions
        self.fn_sigs.insert("i32_to_string".into(), FnSig {
            name: "i32_to_string".into(),
            type_params: vec![],
            params: vec![Type::I32],
            ret: Type::String,
        });
        self.fn_sigs.insert("i64_to_string".into(), FnSig {
            name: "i64_to_string".into(),
            type_params: vec![],
            params: vec![Type::I64],
            ret: Type::String,
        });
        self.fn_sigs.insert("bool_to_string".into(), FnSig {
            name: "bool_to_string".into(),
            type_params: vec![],
            params: vec![Type::Bool],
            ret: Type::String,
        });
        self.fn_sigs.insert("String_from".into(), FnSig {
            name: "String_from".into(),
            type_params: vec![],
            params: vec![Type::String],
            ret: Type::String,
        });
        self.fn_sigs.insert("char_to_string".into(), FnSig {
            name: "char_to_string".into(),
            type_params: vec![],
            params: vec![Type::Char],
            ret: Type::String,
        });
        self.fn_sigs.insert("f64_to_string".into(), FnSig {
            name: "f64_to_string".into(),
            type_params: vec![],
            params: vec![Type::F64],
            ret: Type::String,
        });
        // String equality
        self.fn_sigs.insert("eq".into(), FnSig {
            name: "eq".into(),
            type_params: vec![],
            params: vec![Type::String, Type::String],
            ret: Type::Bool,
        });
    }

    /// Resolve a type expression to a Type.
    pub fn resolve_type_expr(&self, ty: &ast::TypeExpr) -> Type {
        match ty {
            ast::TypeExpr::Named { name, .. } => match name.as_str() {
                "i32" => Type::I32,
                "i64" => Type::I64,
                "f32" => Type::F32,
                "f64" => Type::F64,
                "bool" => Type::Bool,
                "char" => Type::Char,
                "String" => Type::String,
                _ => {
                    if let Some(info) = self.struct_defs.get(name) {
                        Type::Struct(info.type_id)
                    } else if let Some(info) = self.enum_defs.get(name) {
                        Type::Enum(info.type_id)
                    } else {
                        Type::Error
                    }
                }
            },
            ast::TypeExpr::Generic { name, args, .. } => {
                let resolved_args: Vec<Type> = args.iter().map(|a| self.resolve_type_expr(a)).collect();
                match name.as_str() {
                    "Vec" if resolved_args.len() == 1 => Type::Vec(Box::new(resolved_args[0].clone())),
                    "Option" if resolved_args.len() == 1 => Type::Option(Box::new(resolved_args[0].clone())),
                    "Result" if resolved_args.len() == 2 => {
                        Type::Result(Box::new(resolved_args[0].clone()), Box::new(resolved_args[1].clone()))
                    }
                    _ => Type::Error,
                }
            },
            ast::TypeExpr::Tuple(types, _) => {
                Type::Tuple(types.iter().map(|t| self.resolve_type_expr(t)).collect())
            },
            ast::TypeExpr::Array { elem, size, .. } => {
                Type::Array(Box::new(self.resolve_type_expr(elem)), *size)
            },
            ast::TypeExpr::Slice { elem, .. } => {
                Type::Slice(Box::new(self.resolve_type_expr(elem)))
            },
            ast::TypeExpr::Function { params, ret, .. } => {
                Type::Function {
                    params: params.iter().map(|p| self.resolve_type_expr(p)).collect(),
                    ret: Box::new(self.resolve_type_expr(ret)),
                }
            },
            ast::TypeExpr::Unit(_) => Type::Unit,
            ast::TypeExpr::Qualified { .. } => {
                // e.g., io.Capabilities — treated as opaque for now
                Type::Error
            },
        }
    }

    /// Type check a module.
    pub fn check_module(
        &mut self,
        resolved: &ResolvedModule,
        sink: &mut DiagnosticSink,
    ) {
        // Register user-defined structs and enums
        for item in &resolved.module.items {
            match item {
                ast::Item::StructDef(s) => {
                    let type_id = self.fresh_type_id();
                    let fields: Vec<(String, Type)> = s.fields.iter()
                        .map(|f| (f.name.clone(), self.resolve_type_expr(&f.ty)))
                        .collect();
                    self.struct_defs.insert(s.name.clone(), StructInfo {
                        name: s.name.clone(),
                        fields,
                        type_id,
                    });
                }
                ast::Item::EnumDef(e) => {
                    let type_id = self.fresh_type_id();
                    let variants: Vec<VariantInfo> = e.variants.iter()
                        .map(|v| match v {
                            ast::Variant::Unit { name, .. } => VariantInfo { name: name.clone(), fields: vec![] },
                            ast::Variant::Tuple { name, fields, .. } => VariantInfo {
                                name: name.clone(),
                                fields: fields.iter().map(|t| self.resolve_type_expr(t)).collect(),
                            },
                            ast::Variant::Struct { name, fields, .. } => VariantInfo {
                                name: name.clone(),
                                fields: fields.iter().map(|f| self.resolve_type_expr(&f.ty)).collect(),
                            },
                        })
                        .collect();
                    self.enum_defs.insert(e.name.clone(), EnumInfo {
                        name: e.name.clone(),
                        variants,
                        type_params: e.type_params.clone(),
                        type_id,
                    });
                }
                ast::Item::FnDef(f) => {
                    let params: Vec<Type> = f.params.iter()
                        .map(|p| self.resolve_type_expr(&p.ty))
                        .collect();
                    let ret = f.return_type.as_ref()
                        .map(|t| self.resolve_type_expr(t))
                        .unwrap_or(Type::Unit);
                    self.fn_sigs.insert(f.name.clone(), FnSig {
                        name: f.name.clone(),
                        type_params: f.type_params.clone(),
                        params,
                        ret,
                    });
                }
            }
        }

        // Type check function bodies
        for item in &resolved.module.items {
            if let ast::Item::FnDef(f) = item {
                self.check_function(f, sink);
            }
        }
    }

    fn check_function(&mut self, f: &ast::FnDef, sink: &mut DiagnosticSink) {
        let mut env = TypeEnv::new();

        // Bind parameters
        for param in &f.params {
            let ty = self.resolve_type_expr(&param.ty);
            env.bind(param.name.clone(), ty);
        }

        // Bind type parameters
        for tp in &f.type_params {
            env.bind(tp.clone(), self.fresh_type_var());
        }

        let expected_ret = f.return_type.as_ref()
            .map(|t| self.resolve_type_expr(t))
            .unwrap_or(Type::Unit);

        // Check body block
        let _body_type = self.check_block(&f.body, &mut env, &expected_ret, sink);

        // TODO: verify body_type matches expected_ret
    }

    fn check_block(
        &mut self,
        block: &ast::Block,
        env: &mut TypeEnv,
        _expected: &Type,
        sink: &mut DiagnosticSink,
    ) -> Type {
        for stmt in &block.stmts {
            self.check_stmt(stmt, env, sink);
        }

        if let Some(tail) = &block.tail_expr {
            self.synthesize_expr(tail, env, sink)
        } else {
            Type::Unit
        }
    }

    fn check_stmt(&mut self, stmt: &ast::Stmt, env: &mut TypeEnv, sink: &mut DiagnosticSink) {
        match stmt {
            ast::Stmt::Let { name, ty, init, .. } => {
                let init_type = self.synthesize_expr(init, env, sink);
                if let Some(type_expr) = ty {
                    let declared_type = self.resolve_type_expr(type_expr);
                    if !self.types_compatible(&init_type, &declared_type) {
                        sink.emit(
                            Diagnostic::new(DiagnosticCode::E0200)
                                .with_message(format!(
                                    "expected `{}`, found `{}`",
                                    declared_type, init_type
                                ))
                        );
                    }
                    env.bind(name.clone(), declared_type);
                } else {
                    env.bind(name.clone(), init_type);
                }
            }
            ast::Stmt::Expr(expr) => {
                self.synthesize_expr(expr, env, sink);
            }
            ast::Stmt::While { cond, body, .. } => {
                let cond_ty = self.synthesize_expr(cond, env, sink);
                if cond_ty != Type::Bool && cond_ty != Type::Error {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200)
                            .with_message(format!("while condition must be `bool`, found `{}`", cond_ty))
                    );
                }
                self.check_block(body, env, &Type::Unit, sink);
            }
            ast::Stmt::Loop { body, .. } => {
                self.check_block(body, env, &Type::Unit, sink);
            }
        }
    }

    /// Synthesize the type of an expression.
    fn synthesize_expr(&mut self, expr: &ast::Expr, env: &mut TypeEnv, sink: &mut DiagnosticSink) -> Type {
        match expr {
            ast::Expr::IntLit { .. } => Type::I32,
            ast::Expr::FloatLit { .. } => Type::F64,
            ast::Expr::StringLit { .. } => Type::String,
            ast::Expr::CharLit { .. } => Type::Char,
            ast::Expr::BoolLit { .. } => Type::Bool,
            ast::Expr::Ident { name, span } => {
                if let Some(ty) = env.lookup(name) {
                    ty.clone()
                } else if self.fn_sigs.contains_key(name) {
                    // Function reference
                    let sig = self.fn_sigs[name].clone();
                    Type::Function {
                        params: sig.params,
                        ret: Box::new(sig.ret),
                    }
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0100)
                            .with_label(*span, format!("unresolved name `{}`", name))
                    );
                    Type::Error
                }
            }
            ast::Expr::Binary { left, op, right, .. } => {
                let left_ty = self.synthesize_expr(left, env, sink);
                let right_ty = self.synthesize_expr(right, env, sink);
                self.check_binary_op(op, &left_ty, &right_ty, sink)
            }
            ast::Expr::Unary { op, operand, .. } => {
                let operand_ty = self.synthesize_expr(operand, env, sink);
                self.check_unary_op(op, &operand_ty, sink)
            }
            ast::Expr::Call { callee, args, .. } => {
                let callee_ty = self.synthesize_expr(callee, env, sink);
                match callee_ty {
                    Type::Function { params, ret } => {
                        if args.len() != params.len() {
                            sink.emit(
                                Diagnostic::new(DiagnosticCode::E0202)
                                    .with_message(format!(
                                        "expected {} argument(s), found {}",
                                        params.len(), args.len()
                                    ))
                            );
                        }
                        *ret
                    }
                    Type::Error => Type::Error,
                    _ => {
                        sink.emit(
                            Diagnostic::new(DiagnosticCode::E0200)
                                .with_message(format!("expected function, found `{}`", callee_ty))
                        );
                        Type::Error
                    }
                }
            }
            ast::Expr::If { cond, then_block, else_block, .. } => {
                let cond_ty = self.synthesize_expr(cond, env, sink);
                if cond_ty != Type::Bool && cond_ty != Type::Error {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200)
                            .with_message(format!("if condition must be `bool`, found `{}`", cond_ty))
                    );
                }
                let then_ty = self.check_block(then_block, env, &Type::Unit, sink);
                if let Some(else_blk) = else_block {
                    let else_ty = self.check_block(else_blk, env, &then_ty, sink);
                    if !self.types_compatible(&then_ty, &else_ty) {
                        sink.emit(
                            Diagnostic::new(DiagnosticCode::E0205)
                                .with_message(format!(
                                    "if/else branches have incompatible types: `{}` vs `{}`",
                                    then_ty, else_ty
                                ))
                        );
                    }
                    then_ty
                } else {
                    Type::Unit
                }
            }
            ast::Expr::Block(block) => {
                self.check_block(block, env, &Type::Unit, sink)
            }
            ast::Expr::Tuple { elements, .. } => {
                Type::Tuple(elements.iter().map(|e| self.synthesize_expr(e, env, sink)).collect())
            }
            ast::Expr::Array { elements, .. } => {
                if elements.is_empty() {
                    Type::Error // need type annotation
                } else {
                    let first = self.synthesize_expr(&elements[0], env, sink);
                    Type::Array(Box::new(first), elements.len() as u64)
                }
            }
            ast::Expr::Return { value, .. } => {
                if let Some(val) = value {
                    self.synthesize_expr(val, env, sink);
                }
                Type::Never
            }
            ast::Expr::Break { value, .. } => {
                if let Some(val) = value {
                    self.synthesize_expr(val, env, sink);
                }
                Type::Never
            }
            ast::Expr::Continue { .. } => Type::Never,
            ast::Expr::Assign { target, value, .. } => {
                let val_ty = self.synthesize_expr(value, env, sink);
                if let ast::Expr::Ident { name, span } = target.as_ref() {
                    if let Some(target_ty) = env.lookup(name) {
                        let target_ty = target_ty.clone();
                        if !self.types_compatible(&val_ty, &target_ty) {
                            sink.emit(
                                Diagnostic::new(DiagnosticCode::E0200)
                                    .with_label(*span, format!(
                                        "expected `{}`, found `{}`", target_ty, val_ty
                                    ))
                            );
                        }
                    }
                }
                Type::Unit
            }
            _ => {
                // TODO: handle remaining expression types
                Type::Error
            }
        }
    }

    fn check_binary_op(&self, op: &ast::BinOp, left: &Type, right: &Type, sink: &mut DiagnosticSink) -> Type {
        use ast::BinOp::*;
        if *left == Type::Error || *right == Type::Error {
            return Type::Error;
        }
        match op {
            Add | Sub | Mul | Div | Mod => {
                if left.is_numeric() && left == right {
                    left.clone()
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200)
                            .with_message(format!(
                                "cannot apply arithmetic operator to `{}` and `{}`",
                                left, right
                            ))
                    );
                    Type::Error
                }
            }
            Eq | Ne => {
                if left == right {
                    Type::Bool
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200)
                            .with_message(format!(
                                "cannot compare `{}` with `{}`",
                                left, right
                            ))
                    );
                    Type::Error
                }
            }
            Lt | Le | Gt | Ge => {
                if left.is_numeric() && left == right {
                    Type::Bool
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200)
                            .with_message(format!(
                                "cannot order `{}` and `{}`",
                                left, right
                            ))
                    );
                    Type::Error
                }
            }
            And | Or => {
                if *left == Type::Bool && *right == Type::Bool {
                    Type::Bool
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200)
                            .with_message(format!(
                                "logical operators require `bool`, found `{}` and `{}`",
                                left, right
                            ))
                    );
                    Type::Error
                }
            }
            BitAnd | BitOr | BitXor | Shl | Shr => {
                if left.is_integer() && left == right {
                    left.clone()
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200)
                            .with_message(format!(
                                "bitwise operators require integer types, found `{}` and `{}`",
                                left, right
                            ))
                    );
                    Type::Error
                }
            }
        }
    }

    fn check_unary_op(&self, op: &ast::UnaryOp, operand: &Type, sink: &mut DiagnosticSink) -> Type {
        if *operand == Type::Error {
            return Type::Error;
        }
        match op {
            ast::UnaryOp::Neg => {
                if operand.is_numeric() {
                    operand.clone()
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200)
                            .with_message(format!("cannot negate `{}`", operand))
                    );
                    Type::Error
                }
            }
            ast::UnaryOp::Not => {
                if *operand == Type::Bool {
                    Type::Bool
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200)
                            .with_message(format!("logical NOT requires `bool`, found `{}`", operand))
                    );
                    Type::Error
                }
            }
            ast::UnaryOp::BitNot => {
                if operand.is_integer() {
                    operand.clone()
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200)
                            .with_message(format!("bitwise NOT requires integer, found `{}`", operand))
                    );
                    Type::Error
                }
            }
        }
    }

    fn types_compatible(&self, a: &Type, b: &Type) -> bool {
        if *a == Type::Error || *b == Type::Error || *a == Type::Never || *b == Type::Never {
            return true;
        }
        a == b
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}
