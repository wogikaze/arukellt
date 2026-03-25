//! Lower typed AST to MIR.

use std::collections::{HashMap, HashSet};

use ark_diagnostics::DiagnosticSink;
use ark_parser::ast;
use ark_typecheck::TypeChecker;

use crate::mir::*;

/// Lower a type-checked module to MIR.
pub fn lower_to_mir(
    module: &ast::Module,
    _checker: &TypeChecker,
    _sink: &mut DiagnosticSink,
) -> MirModule {
    let mut mir = MirModule::new();
    let mut next_fn_id = 0u32;

    // Collect enum variant tags: "EnumName::Variant" -> tag index
    let mut enum_tags: HashMap<String, i32> = HashMap::new();
    // Collect enum variant info: enum_name -> [(variant_name, field_count)]
    let mut enum_variants: HashMap<String, Vec<(String, usize)>> = HashMap::new();
    // Reverse lookup: "EnumName::Variant" -> enum_name
    let mut variant_to_enum: HashMap<String, String> = HashMap::new();
    // Bare variant names (for prelude types like Option/Result): name -> (enum, tag, field_count)
    let mut bare_variant_tags: HashMap<String, (String, i32, usize)> = HashMap::new();
    // Collect struct definitions: "StructName" -> field names (ordered)
    let mut struct_defs: HashMap<String, Vec<(String, String)>> = HashMap::new();
    // Collect enum definitions: "EnumName" -> [(variant_name, [payload_type_names])]
    let mut enum_defs: HashMap<String, Vec<(String, Vec<String>)>> = HashMap::new();

    // Inject builtin enum types: Option<T> and Result<T, E>
    let builtin_enums: &[(&str, &[(&str, usize)])] = &[
        ("Option", &[("Some", 1), ("None", 0)]),
        ("Result", &[("Ok", 1), ("Err", 1)]),
    ];
    for &(enum_name, variants) in builtin_enums {
        let mut variants_info = Vec::new();
        let mut variants_defs = Vec::new();
        for (i, &(vname, field_count)) in variants.iter().enumerate() {
            let key = format!("{}::{}", enum_name, vname);
            enum_tags.insert(key.clone(), i as i32);
            variant_to_enum.insert(key, enum_name.to_string());
            variants_info.push((vname.to_string(), field_count));
            bare_variant_tags.insert(vname.to_string(), (enum_name.to_string(), i as i32, field_count));
            // Payload types: assume i32 for each field (generic T → i32 at runtime)
            let payload_types: Vec<String> = (0..field_count).map(|_| "i32".to_string()).collect();
            variants_defs.push((vname.to_string(), payload_types));
        }
        enum_variants.insert(enum_name.to_string(), variants_info);
        enum_defs.insert(enum_name.to_string(), variants_defs);
    }

    for item in &module.items {
        if let ast::Item::EnumDef(e) = item {
            let mut variants_info = Vec::new();
            let mut variants_defs = Vec::new();
            for (i, variant) in e.variants.iter().enumerate() {
                let (vname, field_count, payload_types) = match variant {
                    ast::Variant::Unit { name, .. } => (name.clone(), 0, vec![]),
                    ast::Variant::Tuple { name, fields, .. } => {
                        let types: Vec<String> = fields.iter().map(|f| type_expr_name(f)).collect();
                        (name.clone(), fields.len(), types)
                    }
                    ast::Variant::Struct { name, fields, .. } => {
                        let types: Vec<String> = fields.iter().map(|f| type_expr_name(&f.ty)).collect();
                        (name.clone(), fields.len(), types)
                    }
                };
                let key = format!("{}::{}", e.name, vname);
                enum_tags.insert(key.clone(), i as i32);
                variant_to_enum.insert(key, e.name.clone());
                variants_info.push((vname.clone(), field_count));
                variants_defs.push((vname.clone(), payload_types));
                // Register bare variant name for common prelude types
                bare_variant_tags.insert(vname.clone(), (e.name.clone(), i as i32, field_count));
            }
            enum_variants.insert(e.name.clone(), variants_info);
            enum_defs.insert(e.name.clone(), variants_defs);
        }
        if let ast::Item::StructDef(s) = item {
            let fields: Vec<(String, String)> = s.fields.iter().map(|f| {
                let type_name = type_expr_name(&f.ty);
                (f.name.clone(), type_name)
            }).collect();
            struct_defs.insert(s.name.clone(), fields);
        }
    }

    for item in &module.items {
        if let ast::Item::FnDef(f) = item {
            let fn_id = FnId(next_fn_id);
            next_fn_id += 1;

            let mut ctx = LowerCtx::new(
                enum_tags.clone(),
                struct_defs.clone(),
                enum_variants.clone(),
                variant_to_enum.clone(),
                bare_variant_tags.clone(),
                enum_defs.clone(),
            );

            for param in &f.params {
                let pid = ctx.declare_local(&param.name);
                if is_string_type(&param.ty) {
                    ctx.string_locals.insert(pid.0);
                }
                // Track f64-typed parameters
                if let ast::TypeExpr::Named { name: tname, .. } = &param.ty {
                    if tname == "f64" {
                        ctx.f64_locals.insert(pid.0);
                    }
                }
                // Track struct-typed parameters
                if let ast::TypeExpr::Named { name: tname, .. } = &param.ty {
                    if ctx.struct_defs.contains_key(tname.as_str()) {
                        ctx.struct_typed_locals.insert(pid.0, tname.clone());
                    }
                    if ctx.enum_variants.contains_key(tname.as_str()) {
                        ctx.enum_typed_locals.insert(pid.0, tname.clone());
                    }
                }
                if let ast::TypeExpr::Generic { name: tname, .. } = &param.ty {
                    if ctx.enum_variants.contains_key(tname.as_str()) {
                        ctx.enum_typed_locals.insert(pid.0, tname.clone());
                    }
                }
            }

            let entry = BlockId(0);
            let mut stmts = ctx.lower_block(&f.body);
            
            // Handle tail expression: if it's a void call (println etc.), 
            // lower it as a statement. Otherwise, it's the return value.
            let tail_op = if let Some(tail) = &f.body.tail_expr {
                if is_void_expr(tail) {
                    ctx.lower_expr_stmt(tail, &mut stmts);
                    None
                } else {
                    Some(ctx.lower_expr(tail))
                }
            } else {
                None
            };

            let mir_fn = MirFunction {
                id: fn_id,
                name: f.name.clone(),
                params: f.params.iter().enumerate().map(|(i, p)| MirLocal {
                    id: LocalId(i as u32),
                    name: Some(p.name.clone()),
                    ty: match &p.ty {
                        ty if is_string_type(ty) => ark_typecheck::types::Type::String,
                        ast::TypeExpr::Named { name, .. } if name == "f64" => ark_typecheck::types::Type::F64,
                        ast::TypeExpr::Named { name, .. } if name == "f32" => ark_typecheck::types::Type::F32,
                        ast::TypeExpr::Named { name, .. } if name == "i64" => ark_typecheck::types::Type::I64,
                        ast::TypeExpr::Named { name, .. } if name == "bool" => ark_typecheck::types::Type::Bool,
                        _ => ark_typecheck::types::Type::I32,
                    },
                }).collect(),
                return_ty: match &f.return_type {
                    Some(ty) if is_string_type(ty) => ark_typecheck::types::Type::String,
                    Some(ast::TypeExpr::Named { name, .. }) if name == "f64" => ark_typecheck::types::Type::F64,
                    Some(ast::TypeExpr::Named { name, .. }) if name == "f32" => ark_typecheck::types::Type::F32,
                    Some(ast::TypeExpr::Named { name, .. }) if name == "i64" => ark_typecheck::types::Type::I64,
                    Some(ast::TypeExpr::Named { name, .. }) if name == "bool" => ark_typecheck::types::Type::Bool,
                    Some(_) => ark_typecheck::types::Type::I32,
                    None => ark_typecheck::types::Type::Unit,
                },
                locals: ctx.locals.iter().map(|(name, id)| MirLocal {
                    id: *id,
                    name: Some(name.clone()),
                    ty: if ctx.string_locals.contains(&id.0) {
                        ark_typecheck::types::Type::String
                    } else if ctx.f64_locals.contains(&id.0) {
                        ark_typecheck::types::Type::F64
                    } else if ctx.i64_locals.contains(&id.0) {
                        ark_typecheck::types::Type::I64
                    } else {
                        ark_typecheck::types::Type::I32
                    },
                }).collect(),
                blocks: vec![BasicBlock {
                    id: entry,
                    stmts,
                    terminator: if let Some(op) = tail_op {
                        Terminator::Return(Some(op))
                    } else {
                        Terminator::Return(None)
                    },
                }],
                entry,
            };

            if f.name == "main" {
                mir.entry_fn = Some(fn_id);
            }

            mir.functions.push(mir_fn);
        }
    }

    mir.struct_defs = struct_defs;
    mir.enum_defs = enum_defs;
    mir
}

struct LowerCtx {
    locals: Vec<(String, LocalId)>,
    next_local: u32,
    string_locals: HashSet<u32>,
    f64_locals: HashSet<u32>,
    i64_locals: HashSet<u32>,
    enum_tags: HashMap<String, i32>,
    /// enum name -> variant info: (variant_name, field_count)
    enum_variants: HashMap<String, Vec<(String, usize)>>,
    /// "EnumName::Variant" -> enum name (for reverse lookup)
    variant_to_enum: HashMap<String, String>,
    /// Also support bare names like "Some", "None", "Ok", "Err"
    bare_variant_tags: HashMap<String, (String, i32, usize)>, // name -> (enum, tag, field_count)
    /// struct name -> ordered (field name, field type name)
    struct_defs: HashMap<String, Vec<(String, String)>>,
    /// local id -> struct type name
    struct_typed_locals: HashMap<u32, String>,
    /// local id -> enum type name
    enum_typed_locals: HashMap<u32, String>,
    /// local id -> variant-level payload type info: (variant_idx, field_idx) -> is_string
    /// Maps local_id -> mapping from (variant_name, field_index) -> is_string
    enum_local_payload_strings: HashMap<u32, HashSet<(String, u32)>>,
    /// enum name -> [(variant_name, [payload_type_names])]
    enum_defs: HashMap<String, Vec<(String, Vec<String>)>>,
}

impl LowerCtx {
    fn new(
        enum_tags: HashMap<String, i32>,
        struct_defs: HashMap<String, Vec<(String, String)>>,
        enum_variants: HashMap<String, Vec<(String, usize)>>,
        variant_to_enum: HashMap<String, String>,
        bare_variant_tags: HashMap<String, (String, i32, usize)>,
        enum_defs: HashMap<String, Vec<(String, Vec<String>)>>,
    ) -> Self {
        Self {
            locals: Vec::new(),
            next_local: 0,
            string_locals: HashSet::new(),
            f64_locals: HashSet::new(),
            i64_locals: HashSet::new(),
            enum_tags,
            enum_variants,
            variant_to_enum,
            bare_variant_tags,
            struct_defs,
            struct_typed_locals: HashMap::new(),
            enum_typed_locals: HashMap::new(),
            enum_local_payload_strings: HashMap::new(),
            enum_defs,
        }
    }

    fn declare_local(&mut self, name: &str) -> LocalId {
        let id = LocalId(self.next_local);
        self.next_local += 1;
        self.locals.push((name.to_string(), id));
        id
    }

    fn lookup_local(&self, name: &str) -> Option<LocalId> {
        self.locals.iter().rev().find(|(n, _)| n == name).map(|(_, id)| *id)
    }

    /// Infer the struct type name of an expression (for field access).
    fn infer_struct_type(&self, expr: &ast::Expr) -> Option<String> {
        match expr {
            ast::Expr::Ident { name, .. } => {
                let local_id = self.lookup_local(name)?;
                self.struct_typed_locals.get(&local_id.0).cloned()
            }
            ast::Expr::FieldAccess { object, field, .. } => {
                // Chained field access: get parent struct, look up field type
                let parent_struct = self.infer_struct_type(object)?;
                let fields = self.struct_defs.get(&parent_struct)?;
                let field_type = fields.iter()
                    .find(|(fname, _)| fname == field)
                    .map(|(_, ftype)| ftype.clone())?;
                // The field type is the struct name for the nested struct
                if self.struct_defs.contains_key(&field_type) {
                    Some(field_type)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check if an identifier is a known enum variant constructor.
    fn is_enum_variant_call(&self, name: &str) -> bool {
        self.bare_variant_tags.contains_key(name)
    }

    fn lower_block(&mut self, block: &ast::Block) -> Vec<MirStmt> {
        let mut stmts = Vec::new();
        for stmt in &block.stmts {
            self.lower_stmt(stmt, &mut stmts);
        }
        stmts
    }

    /// Lower a block including its tail expression as a statement.
    fn lower_block_all(&mut self, block: &ast::Block) -> Vec<MirStmt> {
        let mut stmts = self.lower_block(block);
        if let Some(tail) = &block.tail_expr {
            self.lower_expr_stmt(tail, &mut stmts);
        }
        stmts
    }

    fn lower_stmt(&mut self, stmt: &ast::Stmt, out: &mut Vec<MirStmt>) {
        match stmt {
            ast::Stmt::Let { name, init, ty, .. } => {
                if name == "_" {
                    // Wildcard binding: evaluate for side effects only
                    self.lower_expr_stmt(init, out);
                    return;
                }
                let local_id = self.declare_local(name);
                if let Some(type_expr) = ty {
                    if is_string_type(type_expr) {
                        self.string_locals.insert(local_id.0);
                    }
                    // Track f64-typed locals
                    if let ast::TypeExpr::Named { name: tname, .. } = type_expr {
                        if tname == "f64" {
                            self.f64_locals.insert(local_id.0);
                        }
                        if tname == "i64" {
                            self.i64_locals.insert(local_id.0);
                        }
                    }
                    // Track struct-typed locals
                    if let ast::TypeExpr::Named { name: tname, .. } = type_expr {
                        if self.struct_defs.contains_key(tname.as_str()) {
                            self.struct_typed_locals.insert(local_id.0, tname.clone());
                        }
                        if self.enum_variants.contains_key(tname.as_str()) {
                            self.enum_typed_locals.insert(local_id.0, tname.clone());
                        }
                    }
                    // Track generic enum types: Option<i32>, Result<i32, String>
                    if let ast::TypeExpr::Generic { name: tname, args, .. } = type_expr {
                        if self.enum_variants.contains_key(tname.as_str()) {
                            self.enum_typed_locals.insert(local_id.0, tname.clone());
                            // Map generic args to variant payload types
                            // For Option<T>: Some has payload 0 = T
                            // For Result<T, E>: Ok has payload 0 = T, Err has payload 0 = E
                            let mut payload_strings = HashSet::new();
                            if tname == "Option" {
                                if args.first().map_or(false, is_string_type) {
                                    payload_strings.insert(("Some".to_string(), 0u32));
                                }
                            } else if tname == "Result" {
                                if args.first().map_or(false, is_string_type) {
                                    payload_strings.insert(("Ok".to_string(), 0u32));
                                }
                                if args.get(1).map_or(false, is_string_type) {
                                    payload_strings.insert(("Err".to_string(), 0u32));
                                }
                            }
                            if !payload_strings.is_empty() {
                                self.enum_local_payload_strings.insert(local_id.0, payload_strings);
                            }
                        }
                    }
                }
                let op = self.lower_expr(init);
                // Promote integer literals to i64 when type annotation is i64
                let op = if self.i64_locals.contains(&local_id.0) {
                    match op {
                        Operand::ConstI32(v) => Operand::ConstI64(v as i64),
                        other => other,
                    }
                } else {
                    op
                };
                out.push(MirStmt::Assign(Place::Local(local_id), Rvalue::Use(op)));
            }
            ast::Stmt::Expr(expr) => {
                self.lower_expr_stmt(expr, out);
            }
            ast::Stmt::While { cond, body, .. } => {
                let cond_op = self.lower_expr(cond);
                out.push(MirStmt::WhileStmt {
                    cond: cond_op,
                    body: self.lower_block_all(body),
                });
            }
            ast::Stmt::Loop { body, .. } => {
                out.push(MirStmt::WhileStmt {
                    cond: Operand::ConstBool(true),
                    body: self.lower_block_all(body),
                });
            }
        }
    }

    fn lower_expr_stmt(&mut self, expr: &ast::Expr, out: &mut Vec<MirStmt>) {
        match expr {
            ast::Expr::Call { callee, args, .. } => {
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    let mir_args: Vec<Operand> = args.iter().map(|a| self.lower_expr(a)).collect();
                    out.push(MirStmt::CallBuiltin {
                        dest: None,
                        name: name.clone(),
                        args: mir_args,
                    });
                }
            }
            ast::Expr::Assign { target, value, .. } => {
                if let ast::Expr::Ident { name, .. } = target.as_ref() {
                    if let Some(local_id) = self.lookup_local(name) {
                        let op = self.lower_expr(value);
                        out.push(MirStmt::Assign(Place::Local(local_id), Rvalue::Use(op)));
                    }
                }
            }
            ast::Expr::If { cond, then_block, else_block, .. } => {
                let c = self.lower_expr(cond);
                let then_stmts = self.lower_block_all(then_block);
                let else_stmts = else_block.as_ref()
                    .map(|b| self.lower_block_all(b))
                    .unwrap_or_default();
                out.push(MirStmt::IfStmt {
                    cond: c,
                    then_body: then_stmts,
                    else_body: else_stmts,
                });
            }
            ast::Expr::Break { .. } => {
                out.push(MirStmt::Break);
            }
            ast::Expr::Continue { .. } => {
                out.push(MirStmt::Continue);
            }
            ast::Expr::Return { value, .. } => {
                let op = value.as_ref().map(|v| self.lower_expr(v));
                out.push(MirStmt::Return(op));
            }
            ast::Expr::Match { scrutinee, arms, .. } => {
                self.lower_match_stmt(scrutinee, arms, out);
            }
            _ => {}
        }
    }

    /// Lower a match expression used as a statement (result discarded).
    /// Converts to nested if-else chains.
    fn lower_match_stmt(&mut self, scrutinee: &ast::Expr, arms: &[ast::MatchArm], out: &mut Vec<MirStmt>) {
        let scrut = self.lower_expr(scrutinee);
        // Build a chain of if-else from the arms
        let stmt = self.build_match_if_chain(&scrut, arms, 0, true);
        if let Some(s) = stmt {
            out.push(s);
        }
    }

    /// Build a nested if-else chain from match arms starting at `idx`.
    /// `as_stmt` indicates whether arm bodies should be lowered as statements.
    fn build_match_if_chain(&mut self, scrut: &Operand, arms: &[ast::MatchArm], idx: usize, as_stmt: bool) -> Option<MirStmt> {
        if idx >= arms.len() {
            return None;
        }
        let arm = &arms[idx];
        match &arm.pattern {
            ast::Pattern::Wildcard(_) => {
                // Default arm — just emit the body
                let mut body = Vec::new();
                self.lower_expr_stmt(&arm.body, &mut body);
                if body.len() == 1 {
                    Some(body.remove(0))
                } else {
                    // Wrap in an always-true if
                    Some(MirStmt::IfStmt {
                        cond: Operand::ConstBool(true),
                        then_body: body,
                        else_body: vec![],
                    })
                }
            }
            ast::Pattern::IntLit { value, .. } => {
                let cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstI32(*value as i32)),
                );
                let mut then_body = Vec::new();
                self.lower_expr_stmt(&arm.body, &mut then_body);
                let else_body = if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                    vec![next]
                } else {
                    vec![]
                };
                Some(MirStmt::IfStmt { cond, then_body, else_body })
            }
            ast::Pattern::BoolLit { value, .. } => {
                let cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstBool(*value)),
                );
                let mut then_body = Vec::new();
                self.lower_expr_stmt(&arm.body, &mut then_body);
                let else_body = if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                    vec![next]
                } else {
                    vec![]
                };
                Some(MirStmt::IfStmt { cond, then_body, else_body })
            }
            ast::Pattern::StringLit { value, .. } => {
                // String match — for now, treat as literal comparison
                let cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstString(value.clone())),
                );
                let mut then_body = Vec::new();
                self.lower_expr_stmt(&arm.body, &mut then_body);
                let else_body = if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                    vec![next]
                } else {
                    vec![]
                };
                Some(MirStmt::IfStmt { cond, then_body, else_body })
            }
            ast::Pattern::Ident { name, .. } => {
                // Binding pattern — bind the scrutinee to the name
                let local_id = self.declare_local(name);
                let mut then_body = vec![
                    MirStmt::Assign(Place::Local(local_id), Rvalue::Use(scrut.clone()))
                ];
                self.lower_expr_stmt(&arm.body, &mut then_body);
                Some(MirStmt::IfStmt {
                    cond: Operand::ConstBool(true),
                    then_body,
                    else_body: vec![],
                })
            }
            ast::Pattern::Enum { path, variant, fields, .. } => {
                let key = format!("{}::{}", path, variant);
                if let Some(&tag) = self.enum_tags.get(&key) {
                    // Compare tag: enum ptr -> i32.load at offset 0
                    let cond = Operand::BinOp(
                        BinOp::Eq,
                        Box::new(Operand::EnumTag(Box::new(scrut.clone()))),
                        Box::new(Operand::ConstI32(tag)),
                    );
                    let mut then_body = Vec::new();
                    // Determine if scrutinee has known payload string types
                    let payload_strings = if let Operand::Place(Place::Local(lid)) = scrut {
                        self.enum_local_payload_strings.get(&lid.0).cloned()
                    } else {
                        None
                    };
                    // Bind payload fields to local variables
                    for (i, field_pat) in fields.iter().enumerate() {
                        if let ast::Pattern::Ident { name: binding, .. } = field_pat {
                            let local_id = self.declare_local(binding);
                            // Check if this payload field is a string
                            if let Some(ref ps) = payload_strings {
                                if ps.contains(&(variant.clone(), i as u32)) {
                                    self.string_locals.insert(local_id.0);
                                }
                            }
                            // Check if this payload field is f64
                            if let Some(variants) = self.enum_defs.get(path.as_str()) {
                                if let Some((_, types)) = variants.iter().find(|(vn, _)| vn == variant) {
                                    if let Some(t) = types.get(i) {
                                        if t == "f64" {
                                            self.f64_locals.insert(local_id.0);
                                        }
                                    }
                                }
                            }
                            let payload = Operand::EnumPayload {
                                object: Box::new(scrut.clone()),
                                index: i as u32,
                                enum_name: path.clone(),
                                variant_name: variant.clone(),
                            };
                            then_body.push(MirStmt::Assign(
                                Place::Local(local_id),
                                Rvalue::Use(payload),
                            ));
                        }
                    }
                    self.lower_expr_stmt(&arm.body, &mut then_body);
                    let else_body = if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                        vec![next]
                    } else {
                        vec![]
                    };
                    Some(MirStmt::IfStmt { cond, then_body, else_body })
                } else {
                    self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
                }
            }
            _ => {
                // Skip unsupported patterns, try next arm
                self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
            }
        }
    }

    /// Build a nested IfExpr from match arms for value-returning match.
    fn build_match_if_expr(&mut self, scrut: &Operand, arms: &[ast::MatchArm], idx: usize) -> Operand {
        if idx >= arms.len() {
            return Operand::Unit;
        }
        let arm = &arms[idx];
        match &arm.pattern {
            ast::Pattern::Wildcard(_) => {
                // Default arm — just return the body value
                self.lower_expr(&arm.body)
            }
            ast::Pattern::Ident { name, .. } => {
                // Binding pattern — bind scrutinee to name, return body
                let local_id = self.declare_local(name);
                // For value-returning match, we wrap in IfExpr with setup stmts
                let body_val = self.lower_expr(&arm.body);
                Operand::IfExpr {
                    cond: Box::new(Operand::ConstBool(true)),
                    then_body: vec![
                        MirStmt::Assign(Place::Local(local_id), Rvalue::Use(scrut.clone()))
                    ],
                    then_result: Some(Box::new(body_val)),
                    else_body: vec![],
                    else_result: Some(Box::new(Operand::Unit)),
                }
            }
            ast::Pattern::IntLit { value, .. } => {
                let cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstI32(*value as i32)),
                );
                let then_result = self.lower_expr(&arm.body);
                let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                Operand::IfExpr {
                    cond: Box::new(cond),
                    then_body: vec![],
                    then_result: Some(Box::new(then_result)),
                    else_body: vec![],
                    else_result: Some(Box::new(else_result)),
                }
            }
            ast::Pattern::BoolLit { value, .. } => {
                let cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstBool(*value)),
                );
                let then_result = self.lower_expr(&arm.body);
                let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                Operand::IfExpr {
                    cond: Box::new(cond),
                    then_body: vec![],
                    then_result: Some(Box::new(then_result)),
                    else_body: vec![],
                    else_result: Some(Box::new(else_result)),
                }
            }
            ast::Pattern::Enum { path, variant, fields, .. } => {
                let key = format!("{}::{}", path, variant);
                if let Some(&tag) = self.enum_tags.get(&key) {
                    let cond = Operand::BinOp(
                        BinOp::Eq,
                        Box::new(Operand::EnumTag(Box::new(scrut.clone()))),
                        Box::new(Operand::ConstI32(tag)),
                    );
                    // Determine if scrutinee has known payload string types
                    let payload_strings = if let Operand::Place(Place::Local(lid)) = scrut {
                        self.enum_local_payload_strings.get(&lid.0).cloned()
                    } else {
                        None
                    };
                    // Setup: bind payload fields
                    let mut setup_stmts = Vec::new();
                    for (i, field_pat) in fields.iter().enumerate() {
                        if let ast::Pattern::Ident { name: binding, .. } = field_pat {
                            let local_id = self.declare_local(binding);
                            // Check if this payload field is a string
                            if let Some(ref ps) = payload_strings {
                                if ps.contains(&(variant.clone(), i as u32)) {
                                    self.string_locals.insert(local_id.0);
                                }
                            }
                            // Check if this payload field is f64
                            if let Some(variants) = self.enum_defs.get(path.as_str()) {
                                if let Some((_, types)) = variants.iter().find(|(vn, _)| vn == variant) {
                                    if let Some(t) = types.get(i) {
                                        if t == "f64" {
                                            self.f64_locals.insert(local_id.0);
                                        }
                                    }
                                }
                            }
                            let payload = Operand::EnumPayload {
                                object: Box::new(scrut.clone()),
                                index: i as u32,
                                enum_name: path.clone(),
                                variant_name: variant.clone(),
                            };
                            setup_stmts.push(MirStmt::Assign(
                                Place::Local(local_id),
                                Rvalue::Use(payload),
                            ));
                        }
                    }
                    let then_result = self.lower_expr(&arm.body);
                    let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                    Operand::IfExpr {
                        cond: Box::new(cond),
                        then_body: setup_stmts,
                        then_result: Some(Box::new(then_result)),
                        else_body: vec![],
                        else_result: Some(Box::new(else_result)),
                    }
                } else {
                    self.build_match_if_expr(scrut, arms, idx + 1)
                }
            }
            _ => {
                // Skip unsupported patterns
                self.build_match_if_expr(scrut, arms, idx + 1)
            }
        }
    }

    fn lower_expr(&mut self, expr: &ast::Expr) -> Operand {
        match expr {
            ast::Expr::StringLit { value, .. } => Operand::ConstString(value.clone()),
            ast::Expr::IntLit { value, .. } => {
                // Keep full i64 precision; will be promoted in let binding if needed
                if *value > i32::MAX as i64 || *value < i32::MIN as i64 {
                    Operand::ConstI64(*value)
                } else {
                    Operand::ConstI32(*value as i32)
                }
            }
            ast::Expr::FloatLit { value, .. } => Operand::ConstF64(*value),
            ast::Expr::BoolLit { value, .. } => Operand::ConstBool(*value),
            ast::Expr::CharLit { value, .. } => Operand::ConstChar(*value),
            ast::Expr::Ident { name, .. } => {
                // Check if this is a bare enum variant (e.g., None)
                if let Some((enum_name, tag, field_count)) = self.bare_variant_tags.get(name) {
                    if *field_count == 0 {
                        return Operand::EnumInit {
                            enum_name: enum_name.clone(),
                            variant: name.clone(),
                            tag: *tag,
                            payload: vec![],
                        };
                    }
                }
                if let Some(local_id) = self.lookup_local(name) {
                    Operand::Place(Place::Local(local_id))
                } else {
                    Operand::Unit
                }
            }
            ast::Expr::Binary { op, left, right, .. } => {
                match op {
                    // Short-circuit: a && b  =>  if a { b } else { false }
                    ast::BinOp::And => {
                        let l = self.lower_expr(left);
                        let r = self.lower_expr(right);
                        Operand::IfExpr {
                            cond: Box::new(l),
                            then_body: vec![],
                            then_result: Some(Box::new(r)),
                            else_body: vec![],
                            else_result: Some(Box::new(Operand::ConstBool(false))),
                        }
                    }
                    // Short-circuit: a || b  =>  if a { true } else { b }
                    ast::BinOp::Or => {
                        let l = self.lower_expr(left);
                        let r = self.lower_expr(right);
                        Operand::IfExpr {
                            cond: Box::new(l),
                            then_body: vec![],
                            then_result: Some(Box::new(Operand::ConstBool(true))),
                            else_body: vec![],
                            else_result: Some(Box::new(r)),
                        }
                    }
                    _ => {
                        let l = self.lower_expr(left);
                        let r = self.lower_expr(right);
                        Operand::BinOp(lower_binop(op), Box::new(l), Box::new(r))
                    }
                }
            }
            ast::Expr::Unary { op, operand, .. } => {
                let inner = self.lower_expr(operand);
                Operand::UnaryOp(lower_unaryop(op), Box::new(inner))
            }
            ast::Expr::Call { callee, args, .. } => {
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    // Check if this is a bare enum variant constructor (e.g., Some(42), Ok(100))
                    if let Some((enum_name, tag, _field_count)) = self.bare_variant_tags.get(name).cloned() {
                        let payload: Vec<Operand> = args.iter().map(|a| self.lower_expr(a)).collect();
                        return Operand::EnumInit {
                            enum_name,
                            variant: name.clone(),
                            tag,
                            payload,
                        };
                    }
                    // Builtin Option/Result operations
                    match name.as_str() {
                        "unwrap" => {
                            let arg = self.lower_expr(&args[0]);
                            // unwrap: extract payload[0] (tag 0 = Some/Ok)
                            return Operand::EnumPayload {
                                object: Box::new(arg),
                                index: 0,
                                enum_name: "Option".to_string(),
                                variant_name: "Some".to_string(),
                            };
                        }
                        "unwrap_or" => {
                            let arg = self.lower_expr(&args[0]);
                            let default = self.lower_expr(&args[1]);
                            // if is_some(arg) then payload[0] else default
                            let cond = Operand::BinOp(
                                BinOp::Eq,
                                Box::new(Operand::EnumTag(Box::new(arg.clone()))),
                                Box::new(Operand::ConstI32(0)), // Some/Ok tag
                            );
                            return Operand::IfExpr {
                                cond: Box::new(cond),
                                then_body: vec![],
                                then_result: Some(Box::new(Operand::EnumPayload {
                                    object: Box::new(arg),
                                    index: 0,
                                    enum_name: "Option".to_string(),
                                    variant_name: "Some".to_string(),
                                })),
                                else_body: vec![],
                                else_result: Some(Box::new(default)),
                            };
                        }
                        "is_some" | "is_ok" => {
                            let arg = self.lower_expr(&args[0]);
                            // Some/Ok tag = 0
                            return Operand::BinOp(
                                BinOp::Eq,
                                Box::new(Operand::EnumTag(Box::new(arg))),
                                Box::new(Operand::ConstI32(0)),
                            );
                        }
                        "is_none" | "is_err" => {
                            let arg = self.lower_expr(&args[0]);
                            // None/Err tag != 0 (i.e., tag == 1)
                            return Operand::BinOp(
                                BinOp::Eq,
                                Box::new(Operand::EnumTag(Box::new(arg))),
                                Box::new(Operand::ConstI32(1)),
                            );
                        }
                        _ => {}
                    }
                    let mir_args: Vec<Operand> = args.iter().map(|a| self.lower_expr(a)).collect();
                    Operand::Call(name.clone(), mir_args)
                } else if let ast::Expr::QualifiedIdent { module, name, .. } = callee.as_ref() {
                    // Qualified enum variant constructor: Shape::Circle(5.0)
                    let key = format!("{}::{}", module, name);
                    if let Some(&tag) = self.enum_tags.get(&key) {
                        let payload: Vec<Operand> = args.iter().map(|a| self.lower_expr(a)).collect();
                        return Operand::EnumInit {
                            enum_name: module.clone(),
                            variant: name.clone(),
                            tag,
                            payload,
                        };
                    }
                    // Fall through: not an enum variant
                    let mir_args: Vec<Operand> = args.iter().map(|a| self.lower_expr(a)).collect();
                    Operand::Call(format!("{}::{}", module, name), mir_args)
                } else {
                    Operand::Unit
                }
            }
            ast::Expr::If { cond, then_block, else_block, .. } => {
                let c = self.lower_expr(cond);
                let then_stmts = self.lower_block(then_block);
                let then_tail = then_block.tail_expr.as_ref().map(|e| self.lower_expr(e));
                let else_stmts = else_block.as_ref().map(|b| self.lower_block(b)).unwrap_or_default();
                let else_tail = else_block.as_ref()
                    .and_then(|b| b.tail_expr.as_ref().map(|e| self.lower_expr(e)));
                Operand::IfExpr {
                    cond: Box::new(c),
                    then_body: then_stmts,
                    then_result: then_tail.map(Box::new),
                    else_body: else_stmts,
                    else_result: else_tail.map(Box::new),
                }
            }
            ast::Expr::Block(block) => {
                if let Some(tail) = &block.tail_expr {
                    self.lower_expr(tail)
                } else {
                    Operand::Unit
                }
            }
            ast::Expr::Match { scrutinee, arms, .. } => {
                let scrut = self.lower_expr(scrutinee);
                self.build_match_if_expr(&scrut, arms, 0)
            }
            ast::Expr::QualifiedIdent { module, name, .. } => {
                // Enum variant reference: Direction::South -> EnumInit with no payload
                let key = format!("{}::{}", module, name);
                if let Some(&tag) = self.enum_tags.get(&key) {
                    // Check if this variant has fields
                    let has_fields = self.enum_variants.get(module.as_str())
                        .and_then(|vs| vs.iter().find(|(vn, _)| vn == name))
                        .map_or(false, |(_, fc)| *fc > 0);
                    if has_fields {
                        // Variant with payload but called without args — shouldn't happen
                        Operand::ConstI32(tag)
                    } else {
                        // Unit variant — allocate in memory like other enum variants for consistency
                        Operand::EnumInit {
                            enum_name: module.clone(),
                            variant: name.clone(),
                            tag,
                            payload: vec![],
                        }
                    }
                } else {
                    Operand::Unit
                }
            }
            ast::Expr::StructInit { name, fields, .. } => {
                let lowered_fields: Vec<(String, Operand)> = fields
                    .iter()
                    .map(|(fname, fexpr)| (fname.clone(), self.lower_expr(fexpr)))
                    .collect();
                Operand::StructInit { name: name.clone(), fields: lowered_fields }
            }
            ast::Expr::FieldAccess { object, field, .. } => {
                // Try to determine the struct type from the object
                let struct_name = self.infer_struct_type(object);
                let obj = self.lower_expr(object);
                Operand::FieldAccess {
                    object: Box::new(obj),
                    struct_name: struct_name.unwrap_or_default(),
                    field: field.clone(),
                }
            }
            _ => Operand::Unit,
        }
    }
}

fn lower_binop(op: &ast::BinOp) -> BinOp {
    match op {
        ast::BinOp::Add => BinOp::Add,
        ast::BinOp::Sub => BinOp::Sub,
        ast::BinOp::Mul => BinOp::Mul,
        ast::BinOp::Div => BinOp::Div,
        ast::BinOp::Mod => BinOp::Mod,
        ast::BinOp::Eq => BinOp::Eq,
        ast::BinOp::Ne => BinOp::Ne,
        ast::BinOp::Lt => BinOp::Lt,
        ast::BinOp::Gt => BinOp::Gt,
        ast::BinOp::Le => BinOp::Le,
        ast::BinOp::Ge => BinOp::Ge,
        ast::BinOp::And => BinOp::And,
        ast::BinOp::Or => BinOp::Or,
        ast::BinOp::BitAnd => BinOp::BitAnd,
        ast::BinOp::BitOr => BinOp::BitOr,
        ast::BinOp::BitXor => BinOp::BitXor,
        ast::BinOp::Shl => BinOp::Shl,
        ast::BinOp::Shr => BinOp::Shr,
    }
}

fn lower_unaryop(op: &ast::UnaryOp) -> UnaryOp {
    match op {
        ast::UnaryOp::Neg => UnaryOp::Neg,
        ast::UnaryOp::Not => UnaryOp::Not,
        ast::UnaryOp::BitNot => UnaryOp::BitNot,
    }
}

/// Check if an expression is void (should be emitted as statement, not value).
fn is_void_expr(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::Call { callee, .. } => {
            if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                matches!(name.as_str(), "println" | "print" | "eprintln")
            } else {
                false
            }
        }
        ast::Expr::Assign { .. } => true,
        ast::Expr::If { then_block, .. } => {
            match &then_block.tail_expr {
                None => true,
                Some(tail) => is_void_expr(tail),
            }
        }
        ast::Expr::Match { arms, .. } => {
            // Match is void if its first arm body is void
            arms.first().map_or(true, |arm| is_void_expr(&arm.body))
        }
        _ => false,
    }
}

fn is_string_type(ty: &ast::TypeExpr) -> bool {
    matches!(ty, ast::TypeExpr::Named { name, .. } if name == "String")
}

fn type_expr_name(ty: &ast::TypeExpr) -> String {
    match ty {
        ast::TypeExpr::Named { name, .. } => name.clone(),
        ast::TypeExpr::Generic { name, .. } => name.clone(),
        ast::TypeExpr::Unit(_) => "()".to_string(),
        _ => "unknown".to_string(),
    }
}
