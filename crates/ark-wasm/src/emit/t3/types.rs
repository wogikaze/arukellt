//! GC type registration for the T3 Wasm GC emitter.
//!
//! Registers all Wasm GC types (strings, vectors, structs, enums) in the
//! type section based on MIR type tables.

use ark_mir::mir::*;
use std::collections::{HashMap, HashSet};
use wasm_encoder::{FieldType, StorageType, ValType};

use super::{mutable_field, ref_nullable, Ctx};

impl Ctx {
    pub(super) fn register_gc_types(&mut self, mir: &MirModule) {
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
        for sname in struct_defs.keys() {
            topo_visit(sname, struct_defs, &mut visited, &mut sorted_structs);
        }
        for sname in &sorted_structs {
            let fields = &struct_defs[*sname];
            let gc_fields: Vec<FieldType> = fields
                .iter()
                .map(|(_, ty)| mutable_field(StorageType::Val(self.field_valtype(ty))))
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
            for sname in &vec_struct_names {
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
        let enum_names: Vec<String> = mir.type_table.enum_defs.keys().cloned().collect();
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
            enum_topo_visit(
                ename,
                &mir.type_table.enum_defs,
                &mut enum_visited,
                &mut enum_order,
            );
        }
        for ename in &enum_order {
            let variants = mir.type_table.enum_defs.get(ename).unwrap();
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
