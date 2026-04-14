//! GC type registration for the T3 Wasm GC emitter.
//!
//! Registers all Wasm GC types (strings, vectors, structs, enums) in the
//! type section based on MIR type tables.
//!
//! User-defined struct fields are analyzed for mutability: fields that are
//! never written after construction (`struct.new`) are declared immutable,
//! eliminating GC write barriers in the runtime.

use ark_mir::mir::*;
use std::collections::{HashMap, HashSet};
use wasm_encoder::{FieldType, StorageType, ValType};

use super::{Ctx, immutable_field, mutable_field, ref_nullable};

/// Scan all MIR functions and collect `(struct_name, field_name)` pairs that
/// are mutated after construction (i.e. appear as `Assign(Place::Field(…), …)`).
/// Fields *not* in this set are safe to declare immutable in the GC type.
fn collect_mutable_struct_fields(mir: &MirModule) -> HashSet<(String, String)> {
    let mut mutable_fields = HashSet::new();

    for func in &mir.functions {
        // Build local → struct_name map from MIR metadata + StructInit inference
        let mut local_structs: HashMap<u32, String> = func.struct_typed_locals.clone();
        for block in &func.blocks {
            infer_struct_locals_stmts(&block.stmts, &mut local_structs);
            infer_struct_locals_terminator(&block.terminator, &mut local_structs);
        }

        for block in &func.blocks {
            find_field_mutations_stmts(&block.stmts, &local_structs, &mut mutable_fields);
            find_field_mutations_terminator(&block.terminator, &local_structs, &mut mutable_fields);
        }
    }

    mutable_fields
}

// ── StructInit inference (also recurses into operands) ──────────────

fn infer_struct_locals_stmts(stmts: &[MirStmt], out: &mut HashMap<u32, String>) {
    for stmt in stmts {
        match stmt {
            MirStmt::Assign(Place::Local(id), rv) => {
                if let Rvalue::Use(Operand::StructInit { name, .. })
                | Rvalue::Aggregate(AggregateKind::Struct(name), _) = rv
                {
                    out.entry(id.0).or_insert_with(|| name.clone());
                }
                infer_struct_locals_rvalue(rv, out);
            }
            MirStmt::Assign(_, rv) => infer_struct_locals_rvalue(rv, out),
            MirStmt::IfStmt {
                then_body,
                else_body,
                ..
            } => {
                infer_struct_locals_stmts(then_body, out);
                infer_struct_locals_stmts(else_body, out);
            }
            MirStmt::WhileStmt { body, .. } => infer_struct_locals_stmts(body, out),
            MirStmt::Call { args, .. } => {
                for a in args {
                    infer_struct_locals_op(a, out);
                }
            }
            MirStmt::CallBuiltin { args, .. } => {
                for a in args {
                    infer_struct_locals_op(a, out);
                }
            }
            _ => {}
        }
    }
}

fn infer_struct_locals_terminator(term: &Terminator, out: &mut HashMap<u32, String>) {
    match term {
        Terminator::Return(Some(op)) => infer_struct_locals_op(op, out),
        Terminator::If { cond, .. } => infer_struct_locals_op(cond, out),
        Terminator::Switch { scrutinee, .. } => infer_struct_locals_op(scrutinee, out),
        _ => {}
    }
}

fn infer_struct_locals_rvalue(rv: &Rvalue, out: &mut HashMap<u32, String>) {
    match rv {
        Rvalue::Use(op) => infer_struct_locals_op(op, out),
        Rvalue::BinaryOp(_, a, b) => {
            infer_struct_locals_op(a, out);
            infer_struct_locals_op(b, out);
        }
        Rvalue::UnaryOp(_, a) => infer_struct_locals_op(a, out),
        Rvalue::Aggregate(_, ops) => {
            for o in ops {
                infer_struct_locals_op(o, out);
            }
        }
        Rvalue::Ref(_) => {}
    }
}

fn infer_struct_locals_op(op: &Operand, out: &mut HashMap<u32, String>) {
    match op {
        Operand::IfExpr {
            cond,
            then_body,
            then_result,
            else_body,
            else_result,
        } => {
            infer_struct_locals_op(cond, out);
            infer_struct_locals_stmts(then_body, out);
            if let Some(r) = then_result {
                infer_struct_locals_op(r, out);
            }
            infer_struct_locals_stmts(else_body, out);
            if let Some(r) = else_result {
                infer_struct_locals_op(r, out);
            }
        }
        Operand::LoopExpr { init, body, result } => {
            infer_struct_locals_op(init, out);
            infer_struct_locals_stmts(body, out);
            infer_struct_locals_op(result, out);
        }
        Operand::BinOp(_, a, b) => {
            infer_struct_locals_op(a, out);
            infer_struct_locals_op(b, out);
        }
        Operand::UnaryOp(_, a)
        | Operand::FieldAccess { object: a, .. }
        | Operand::EnumTag(a)
        | Operand::EnumPayload { object: a, .. }
        | Operand::TryExpr { expr: a, .. } => infer_struct_locals_op(a, out),
        Operand::Call(_, args) => {
            for a in args {
                infer_struct_locals_op(a, out);
            }
        }
        Operand::CallIndirect { callee, args } => {
            infer_struct_locals_op(callee, out);
            for a in args {
                infer_struct_locals_op(a, out);
            }
        }
        Operand::StructInit { fields, .. } => {
            for (_, v) in fields {
                infer_struct_locals_op(v, out);
            }
        }
        Operand::EnumInit { payload, .. } | Operand::ArrayInit { elements: payload } => {
            for p in payload {
                infer_struct_locals_op(p, out);
            }
        }
        Operand::IndexAccess { object, index } => {
            infer_struct_locals_op(object, out);
            infer_struct_locals_op(index, out);
        }
        _ => {}
    }
}

// ── Field mutation scanning (recurses into operands) ────────────────

fn find_field_mutations_stmts(
    stmts: &[MirStmt],
    local_structs: &HashMap<u32, String>,
    out: &mut HashSet<(String, String)>,
) {
    for stmt in stmts {
        // Check the statement destination for field writes
        match stmt {
            MirStmt::Assign(Place::Field(inner, field_name), _)
            | MirStmt::Call {
                dest: Some(Place::Field(inner, field_name)),
                ..
            }
            | MirStmt::CallBuiltin {
                dest: Some(Place::Field(inner, field_name)),
                ..
            } => {
                if let Place::Local(id) = inner.as_ref()
                    && let Some(sname) = local_structs.get(&id.0)
                {
                    out.insert((sname.clone(), field_name.clone()));
                }
            }
            _ => {}
        }
        // Recurse into nested statement/operand bodies
        match stmt {
            MirStmt::Assign(_, rv) => find_field_mutations_rvalue(rv, local_structs, out),
            MirStmt::IfStmt {
                then_body,
                else_body,
                ..
            } => {
                find_field_mutations_stmts(then_body, local_structs, out);
                find_field_mutations_stmts(else_body, local_structs, out);
            }
            MirStmt::WhileStmt { body, .. } => {
                find_field_mutations_stmts(body, local_structs, out);
            }
            MirStmt::Call { args, .. } | MirStmt::CallBuiltin { args, .. } => {
                for a in args {
                    find_field_mutations_op(a, local_structs, out);
                }
            }
            _ => {}
        }
    }
}

fn find_field_mutations_terminator(
    term: &Terminator,
    local_structs: &HashMap<u32, String>,
    out: &mut HashSet<(String, String)>,
) {
    match term {
        Terminator::Return(Some(op)) => find_field_mutations_op(op, local_structs, out),
        Terminator::If { cond, .. } => find_field_mutations_op(cond, local_structs, out),
        Terminator::Switch { scrutinee, .. } => {
            find_field_mutations_op(scrutinee, local_structs, out)
        }
        _ => {}
    }
}

fn find_field_mutations_rvalue(
    rv: &Rvalue,
    local_structs: &HashMap<u32, String>,
    out: &mut HashSet<(String, String)>,
) {
    match rv {
        Rvalue::Use(op) => find_field_mutations_op(op, local_structs, out),
        Rvalue::BinaryOp(_, a, b) => {
            find_field_mutations_op(a, local_structs, out);
            find_field_mutations_op(b, local_structs, out);
        }
        Rvalue::UnaryOp(_, a) => find_field_mutations_op(a, local_structs, out),
        Rvalue::Aggregate(_, ops) => {
            for o in ops {
                find_field_mutations_op(o, local_structs, out);
            }
        }
        Rvalue::Ref(_) => {}
    }
}

fn find_field_mutations_op(
    op: &Operand,
    local_structs: &HashMap<u32, String>,
    out: &mut HashSet<(String, String)>,
) {
    match op {
        Operand::IfExpr {
            cond,
            then_body,
            then_result,
            else_body,
            else_result,
        } => {
            find_field_mutations_op(cond, local_structs, out);
            find_field_mutations_stmts(then_body, local_structs, out);
            if let Some(r) = then_result {
                find_field_mutations_op(r, local_structs, out);
            }
            find_field_mutations_stmts(else_body, local_structs, out);
            if let Some(r) = else_result {
                find_field_mutations_op(r, local_structs, out);
            }
        }
        Operand::LoopExpr { init, body, result } => {
            find_field_mutations_op(init, local_structs, out);
            find_field_mutations_stmts(body, local_structs, out);
            find_field_mutations_op(result, local_structs, out);
        }
        Operand::BinOp(_, a, b) => {
            find_field_mutations_op(a, local_structs, out);
            find_field_mutations_op(b, local_structs, out);
        }
        Operand::UnaryOp(_, a)
        | Operand::FieldAccess { object: a, .. }
        | Operand::EnumTag(a)
        | Operand::EnumPayload { object: a, .. }
        | Operand::TryExpr { expr: a, .. } => find_field_mutations_op(a, local_structs, out),
        Operand::Call(_, args) => {
            for a in args {
                find_field_mutations_op(a, local_structs, out);
            }
        }
        Operand::CallIndirect { callee, args } => {
            find_field_mutations_op(callee, local_structs, out);
            for a in args {
                find_field_mutations_op(a, local_structs, out);
            }
        }
        Operand::StructInit { fields, .. } => {
            for (_, v) in fields {
                find_field_mutations_op(v, local_structs, out);
            }
        }
        Operand::EnumInit { payload, .. } | Operand::ArrayInit { elements: payload } => {
            for p in payload {
                find_field_mutations_op(p, local_structs, out);
            }
        }
        Operand::IndexAccess { object, index } => {
            find_field_mutations_op(object, local_structs, out);
            find_field_mutations_op(index, local_structs, out);
        }
        _ => {}
    }
}

impl Ctx {
    pub(super) fn register_gc_types(&mut self, mir: &MirModule) {
        // Determine which user-struct fields are mutated after construction.
        let mutable_fields = collect_mutable_struct_fields(mir);
        // ── String: bare packed i8 array (no wrapper struct) ──
        // (type $string (array (mut i8)))
        self.string_ty = self
            .types
            .add_array("$string", mutable_field(StorageType::I8));

        // ── Vec backing arrays ──
        // (type $arr_i32 (array (mut i32)))
        self.arr_i32_ty = self
            .types
            .add_array("$arr_i32", mutable_field(StorageType::Val(ValType::I32)));
        // (type $arr_i64 (array (mut i64)))
        self.arr_i64_ty = self
            .types
            .add_array("$arr_i64", mutable_field(StorageType::Val(ValType::I64)));
        // (type $arr_f64 (array (mut f64)))
        self.arr_f64_ty = self
            .types
            .add_array("$arr_f64", mutable_field(StorageType::Val(ValType::F64)));
        // (type $arr_string (array (mut (ref null $string))))
        self.arr_string_ty = self.types.add_array(
            "$arr_string",
            mutable_field(StorageType::Val(ref_nullable(self.string_ty))),
        );

        // ── Vec structs: data ref + len (capacity = array.len) ──
        // (type $vec_i32 (struct (field (mut (ref $arr_i32))) (field (mut i32))))
        self.vec_i32_ty = self.types.add_struct(
            "$vec_i32",
            &[
                mutable_field(StorageType::Val(ref_nullable(self.arr_i32_ty))),
                mutable_field(StorageType::Val(ValType::I32)),
            ],
        );
        self.vec_i64_ty = self.types.add_struct(
            "$vec_i64",
            &[
                mutable_field(StorageType::Val(ref_nullable(self.arr_i64_ty))),
                mutable_field(StorageType::Val(ValType::I32)),
            ],
        );
        self.vec_f64_ty = self.types.add_struct(
            "$vec_f64",
            &[
                mutable_field(StorageType::Val(ref_nullable(self.arr_f64_ty))),
                mutable_field(StorageType::Val(ValType::I32)),
            ],
        );
        self.vec_string_ty = self.types.add_struct(
            "$vec_string",
            &[
                mutable_field(StorageType::Val(ref_nullable(self.arr_string_ty))),
                mutable_field(StorageType::Val(ValType::I32)),
            ],
        );

        // HashMap<i32, i32>: struct { keys: ref $arr_i32, values: ref $arr_i32, count: i32 }
        self.hashmap_i32_i32_ty = self.types.add_struct(
            "$hashmap_i32_i32",
            &[
                mutable_field(StorageType::Val(ref_nullable(self.arr_i32_ty))),
                mutable_field(StorageType::Val(ref_nullable(self.arr_i32_ty))),
                mutable_field(StorageType::Val(ValType::I32)),
            ],
        );
        self.struct_gc_types
            .insert("__hashmap_i32_i32".to_string(), self.hashmap_i32_i32_ty);

        // HashMap<String, i32>: struct { keys: ref $arr_string, values: ref $arr_i32, count: i32 }
        self.hashmap_str_i32_ty = self.types.add_struct(
            "$hashmap_str_i32",
            &[
                mutable_field(StorageType::Val(ref_nullable(self.arr_string_ty))),
                mutable_field(StorageType::Val(ref_nullable(self.arr_i32_ty))),
                mutable_field(StorageType::Val(ValType::I32)),
            ],
        );
        self.struct_gc_types
            .insert("__hashmap_str_i32".to_string(), self.hashmap_str_i32_ty);

        // ── User-defined structs ──
        // Topologically sort structs so field-type dependencies are registered first
        let struct_defs = &mir.type_table.struct_defs;
        let mut sorted_structs: Vec<&String> = Vec::new();
        let mut visited: HashSet<&String> = HashSet::new();
        fn topo_visit<'a>(
            name: &'a String,
            defs: &'a HashMap<String, Vec<(String, String)>>,
            visited: &mut HashSet<&'a String>,
            sorted: &mut Vec<&'a String>,
        ) {
            if visited.contains(name) {
                return;
            }
            visited.insert(name);
            if let Some(fields) = defs.get(name) {
                for (_, fty) in fields {
                    if defs.contains_key(fty.as_str()) {
                        topo_visit(fty, defs, visited, sorted);
                    }
                }
            }
            sorted.push(name);
        }
        let mut struct_names_sorted: Vec<&String> = struct_defs.keys().collect();
        struct_names_sorted.sort();
        for sname in struct_names_sorted {
            topo_visit(sname, struct_defs, &mut visited, &mut sorted_structs);
        }
        for sname in &sorted_structs {
            // Use self.struct_layouts (which may be reordered by layout_opt)
            // for field iteration order when available.
            let fields_from_layout;
            let fields: &[(String, String)] = if let Some(l) = self.struct_layouts.get(*sname) {
                fields_from_layout = l.clone();
                &fields_from_layout
            } else {
                &struct_defs[*sname]
            };
            let gc_fields: Vec<FieldType> = fields
                .iter()
                .map(|(fname, ty)| {
                    let st = StorageType::Val(self.field_valtype(ty));
                    if mutable_fields.contains(&((*sname).clone(), fname.clone())) {
                        mutable_field(st)
                    } else {
                        immutable_field(st)
                    }
                })
                .collect();
            let idx = self.types.add_struct(sname, &gc_fields);
            self.struct_gc_types.insert((*sname).clone(), idx);
        }

        // ── Vec<Struct> types: scan MIR for Vec_new_* calls with struct names ──
        {
            let mut vec_struct_names: HashSet<String> = HashSet::new();
            for func in &mir.functions {
                for block in &func.blocks {
                    for stmt in &block.stmts {
                        self.scan_operands_for_vec_struct(stmt, struct_defs, &mut vec_struct_names);
                    }
                }
            }
            let mut vec_struct_sorted: Vec<String> = vec_struct_names.into_iter().collect();
            vec_struct_sorted.sort();
            for sname in &vec_struct_sorted {
                if let Some(&struct_ty_idx) = self.struct_gc_types.get(sname) {
                    let arr_ty = self.types.add_array(
                        &format!("$arr_{}", sname),
                        mutable_field(StorageType::Val(ref_nullable(struct_ty_idx))),
                    );
                    let vec_ty = self.types.add_struct(
                        &format!("$vec_{}", sname),
                        &[
                            mutable_field(StorageType::Val(ref_nullable(arr_ty))),
                            mutable_field(StorageType::Val(ValType::I32)),
                        ],
                    );
                    self.custom_vec_types
                        .insert(sname.clone(), (arr_ty, vec_ty));
                }
            }
        }

        // ── User-defined enums: subtype hierarchy (rec group) ──
        // Each enum is emitted as one rec group so that structurally
        // identical variants (e.g., unit variants) are type-distinct.
        // Topological sort: enums whose variant fields reference other enums must be processed after them.
        let mut enum_names: Vec<String> = self.enum_defs.keys().cloned().collect();
        enum_names.sort();
        let mut enum_order: Vec<String> = Vec::new();
        let mut enum_visited: HashSet<String> = HashSet::new();
        fn enum_topo_visit(
            name: &str,
            enum_defs: &HashMap<String, Vec<(String, Vec<String>)>>,
            visited: &mut HashSet<String>,
            order: &mut Vec<String>,
        ) {
            if visited.contains(name) {
                return;
            }
            visited.insert(name.to_string());
            if let Some(variants) = enum_defs.get(name) {
                for (_, field_types) in variants {
                    for ft in field_types {
                        if enum_defs.contains_key(ft.as_str()) {
                            enum_topo_visit(ft, enum_defs, visited, order);
                        }
                    }
                }
            }
            order.push(name.to_string());
        }
        for ename in &enum_names {
            enum_topo_visit(ename, &self.enum_defs, &mut enum_visited, &mut enum_order);
        }
        for ename in &enum_order {
            let variants = self.enum_defs.get(ename).unwrap();
            let variant_fields: Vec<(String, Vec<FieldType>)> = variants
                .iter()
                .map(|(vname, field_types)| {
                    let gc_fields: Vec<FieldType> = field_types
                        .iter()
                        .map(|ty| mutable_field(StorageType::Val(self.field_valtype(ty))))
                        .collect();
                    (vname.clone(), gc_fields)
                })
                .collect();

            let (base_idx, variant_indices) = self.types.add_enum_rec_group(ename, &variant_fields);
            self.enum_base_types.insert(ename.clone(), base_idx);

            let mut variant_map = HashMap::new();
            for (vname, v_idx) in variant_indices {
                variant_map.insert(vname, v_idx);
            }
            self.enum_variant_types.insert(ename.clone(), variant_map);

            // Store field type names for enum payload type resolution
            for (vname, field_types) in variants {
                self.enum_variant_field_types
                    .insert((ename.clone(), vname.clone()), field_types.clone());
            }
        }
    }
}
