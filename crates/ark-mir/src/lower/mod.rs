//! Lower typed AST and CoreHIR to MIR.

mod ctx;
pub(crate) use ctx::LowerCtx;

mod expr;
mod facade;
mod func;
mod pattern;
mod stmt;
mod types;

use std::collections::HashMap;

use ark_typecheck::types::Type as CheckerType;

use crate::mir::*;
use crate::validate::validate_module;

/// Format a checker Type to a string using struct/enum name maps rather than
/// the default Display which produces "struct#N"/"enum#N".
pub(crate) fn type_to_sig_name(
    ty: &CheckerType,
    struct_id_to_name: &HashMap<u32, String>,
    enum_id_to_name: &HashMap<u32, String>,
) -> String {
    use ark_typecheck::types::Type;
    match ty {
        Type::Struct(id) => struct_id_to_name
            .get(&id.0)
            .cloned()
            .unwrap_or_else(|| format!("{}", ty)),
        Type::Enum(id) => enum_id_to_name
            .get(&id.0)
            .cloned()
            .unwrap_or_else(|| format!("{}", ty)),
        Type::Vec(elem) => format!(
            "Vec<{}>",
            type_to_sig_name(elem, struct_id_to_name, enum_id_to_name)
        ),
        Type::Option(inner) => format!(
            "Option<{}>",
            type_to_sig_name(inner, struct_id_to_name, enum_id_to_name)
        ),
        Type::Result(ok, err) => format!(
            "Result<{}, {}>",
            type_to_sig_name(ok, struct_id_to_name, enum_id_to_name),
            type_to_sig_name(err, struct_id_to_name, enum_id_to_name),
        ),
        Type::Tuple(elems) => format!("__tuple{}", elems.len()),
        _ => format!("{}", ty),
    }
}

fn finalize_module_metadata(mir: &mut MirModule) {
    sync_module_metadata(mir);
    let _ = validate_module(mir);
}

#[allow(dead_code)]
fn clone_with_provenance(module: &MirModule, provenance: MirProvenance) -> MirModule {
    let mut cloned = module.clone();
    set_mir_provenance(&mut cloned, provenance);
    cloned
}

#[allow(dead_code)]
fn function_names(module: &MirModule) -> Vec<String> {
    module
        .functions
        .iter()
        .map(|func| func.name.clone())
        .collect()
}

#[allow(dead_code)]
fn validation_runs(module: &MirModule) -> u32 {
    module.stats.validation_runs
}

fn default_function_instance(name: &str) -> InstanceKey {
    InstanceKey::simple(name.to_string())
}

fn default_function_source() -> SourceInfo {
    SourceInfo::unknown()
}

fn default_block_source() -> SourceInfo {
    SourceInfo::unknown()
}

pub(crate) fn finalize_function(function: &mut MirFunction) {
    if function.instance.item.is_empty() {
        function.instance = default_function_instance(&function.name);
    }
}

pub(crate) fn finalize_block(block: &mut BasicBlock) {
    if block.source.span.is_none() {
        block.source = default_block_source();
    }
}

pub(crate) fn finalize_function_blocks(function: &mut MirFunction) {
    for block in &mut function.blocks {
        finalize_block(block);
    }
}

pub(crate) fn finalize_function_metadata(function: &mut MirFunction) {
    finalize_function(function);
    finalize_function_blocks(function);
    if function.source.span.is_none() {
        function.source = default_function_source();
    }
}

pub(crate) fn push_function(mir: &mut MirModule, mut function: MirFunction) {
    finalize_function_metadata(&mut function);
    register_function_metadata(mir, &function);
    mir.functions.push(function);
}

pub(crate) fn infer_fn_id(name: &str, next_fn_id: u32) -> FnId {
    let inferred = legacy_fn_id(name);
    if inferred.0 == 0 {
        FnId(next_fn_id)
    } else {
        inferred
    }
}

pub(crate) fn fallback_block(
    id: BlockId,
    stmts: Vec<MirStmt>,
    terminator: Terminator,
) -> BasicBlock {
    BasicBlock {
        id,
        stmts,
        terminator,
        source: default_block_source(),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn fallback_function(
    id: FnId,
    name: String,
    params: Vec<MirLocal>,
    return_ty: ark_typecheck::types::Type,
    locals: Vec<MirLocal>,
    blocks: Vec<BasicBlock>,
    entry: BlockId,
    struct_typed_locals: std::collections::HashMap<u32, String>,
    enum_typed_locals: std::collections::HashMap<u32, String>,
    type_params: Vec<String>,
    is_exported: bool,
) -> MirFunction {
    MirFunction {
        id,
        name: name.clone(),
        instance: default_function_instance(&name),
        params,
        return_ty,
        locals,
        blocks,
        entry,
        struct_typed_locals,
        enum_typed_locals,
        type_params,
        source: default_function_source(),
        is_exported,
    }
}

pub(crate) fn finalize_lowered_module(mir: &mut MirModule) {
    finalize_module_metadata(mir);
}

pub use facade::*;

// Re-export the main lowering function from func submodule (deprecated, use CoreHIR path)
#[allow(deprecated)]
pub use func::lower_to_mir;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{
        BasicBlock, BlockId, FnId, InstanceKey, LocalId, MirFunction, MirLocal, MirStmt, Operand,
        Place, Rvalue, Terminator, default_block_source, default_function_source,
        is_backend_legal_module,
    };
    use ark_typecheck::types::Type;

    fn make_if_expr_function() -> MirFunction {
        // fn test() -> i32 { if true { 1 } else { 2 } }
        // Lowered as: Assign(result, Use(IfExpr { cond: true, then: 1, else: 2 }))
        MirFunction {
            id: FnId(1),
            name: "test".to_string(),
            instance: InstanceKey::simple("test"),
            params: vec![],
            return_ty: Type::I32,
            locals: vec![MirLocal {
                id: LocalId(0),
                name: Some("result".to_string()),
                ty: Type::I32,
            }],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![MirStmt::Assign(
                    Place::Local(LocalId(0)),
                    Rvalue::Use(Operand::IfExpr {
                        cond: Box::new(Operand::ConstBool(true)),
                        then_body: vec![],
                        then_result: Some(Box::new(Operand::ConstI32(1))),
                        else_body: vec![],
                        else_result: Some(Box::new(Operand::ConstI32(2))),
                    }),
                )],
                terminator: Terminator::Return(Some(Operand::Place(Place::Local(LocalId(0))))),
                source: default_block_source(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: vec![],
            source: default_function_source(),
            is_exported: false,
        }
    }

    #[test]
    fn lower_if_expr_removes_ifexpr_operand() {
        let mut func = make_if_expr_function();
        // Before: function contains IfExpr (backend-illegal)
        let has_if_expr_before = func.blocks[0]
            .stmts
            .iter()
            .any(|s| matches!(s, MirStmt::Assign(_, Rvalue::Use(Operand::IfExpr { .. }))));
        assert!(has_if_expr_before, "pre-condition: IfExpr must be present");

        lower_if_expr(&mut func);

        // After: no IfExpr operands remain; an IfStmt should be present instead
        let has_if_expr_after = func.blocks[0]
            .stmts
            .iter()
            .any(|s| matches!(s, MirStmt::Assign(_, Rvalue::Use(Operand::IfExpr { .. }))));
        assert!(!has_if_expr_after, "IfExpr must be desugared away");

        let has_if_stmt = func.blocks[0]
            .stmts
            .iter()
            .any(|s| matches!(s, MirStmt::IfStmt { .. }));
        assert!(has_if_stmt, "IfStmt must be present after desugaring");
    }

    #[test]
    fn lower_if_exprs_produces_backend_legal_module() {
        let func = make_if_expr_function();
        let mut module = MirModule::new();
        module.functions.push(func);
        module.entry_fn = Some(FnId(1));

        // Before lowering: backend-illegal due to IfExpr
        assert!(
            !is_backend_legal_module(&module),
            "pre-condition: module must be backend-illegal"
        );

        lower_if_exprs(&mut module);

        // After lowering: backend-legal (IfExpr removed)
        assert!(
            is_backend_legal_module(&module),
            "module must be backend-legal after lower_if_exprs"
        );
    }

    #[test]
    fn lower_if_expr_handles_nested_if() {
        // if true { if false { 1 } else { 2 } } else { 3 }
        let nested = Operand::IfExpr {
            cond: Box::new(Operand::ConstBool(true)),
            then_body: vec![],
            then_result: Some(Box::new(Operand::IfExpr {
                cond: Box::new(Operand::ConstBool(false)),
                then_body: vec![],
                then_result: Some(Box::new(Operand::ConstI32(1))),
                else_body: vec![],
                else_result: Some(Box::new(Operand::ConstI32(2))),
            })),
            else_body: vec![],
            else_result: Some(Box::new(Operand::ConstI32(3))),
        };

        let mut func = MirFunction {
            id: FnId(1),
            name: "nested".to_string(),
            instance: InstanceKey::simple("nested"),
            params: vec![],
            return_ty: Type::I32,
            locals: vec![MirLocal {
                id: LocalId(0),
                name: Some("r".to_string()),
                ty: Type::I32,
            }],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![MirStmt::Assign(
                    Place::Local(LocalId(0)),
                    Rvalue::Use(nested),
                )],
                terminator: Terminator::Return(Some(Operand::Place(Place::Local(LocalId(0))))),
                source: default_block_source(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: vec![],
            source: default_function_source(),
            is_exported: false,
        };

        lower_if_expr(&mut func);

        // No IfExpr operands should remain anywhere in the function
        let mut module = MirModule::new();
        module.functions.push(func);
        module.entry_fn = Some(FnId(1));
        assert!(
            is_backend_legal_module(&module),
            "nested IfExpr must be fully desugared"
        );
    }

    #[test]
    fn lower_if_expr_preserves_loop_and_try() {
        // Ensure LoopExpr and TryExpr are NOT desugared by lower_if_expr
        let mut func = MirFunction {
            id: FnId(1),
            name: "mixed".to_string(),
            instance: InstanceKey::simple("mixed"),
            params: vec![],
            return_ty: Type::I32,
            locals: vec![MirLocal {
                id: LocalId(0),
                name: Some("x".to_string()),
                ty: Type::I32,
            }],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![MirStmt::Assign(
                    Place::Local(LocalId(0)),
                    Rvalue::Use(Operand::LoopExpr {
                        init: Box::new(Operand::ConstI32(0)),
                        body: vec![MirStmt::Break],
                        result: Box::new(Operand::ConstI32(42)),
                    }),
                )],
                terminator: Terminator::Return(Some(Operand::Place(Place::Local(LocalId(0))))),
                source: default_block_source(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: vec![],
            source: default_function_source(),
            is_exported: false,
        };

        lower_if_expr(&mut func);

        // LoopExpr should still be present (not desugared)
        let has_loop = func.blocks[0]
            .stmts
            .iter()
            .any(|s| matches!(s, MirStmt::Assign(_, Rvalue::Use(Operand::LoopExpr { .. }))));
        assert!(has_loop, "LoopExpr must be preserved by lower_if_expr");
    }

    fn make_loop_expr_function() -> MirFunction {
        // fn counter() -> i32 { loop { init=0, body=[while cond { ... break }], result=x } }
        MirFunction {
            id: FnId(2),
            name: "counter".to_string(),
            instance: InstanceKey::simple("counter"),
            params: vec![],
            return_ty: Type::I32,
            locals: vec![MirLocal {
                id: LocalId(0),
                name: Some("result".to_string()),
                ty: Type::I32,
            }],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![MirStmt::Assign(
                    Place::Local(LocalId(0)),
                    Rvalue::Use(Operand::LoopExpr {
                        init: Box::new(Operand::ConstI32(0)),
                        body: vec![MirStmt::WhileStmt {
                            cond: Operand::ConstBool(true),
                            body: vec![MirStmt::Break],
                        }],
                        result: Box::new(Operand::ConstI32(42)),
                    }),
                )],
                terminator: Terminator::Return(Some(Operand::Place(Place::Local(LocalId(0))))),
                source: default_block_source(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: vec![],
            source: default_function_source(),
            is_exported: false,
        }
    }

    #[test]
    fn lower_loop_expr_removes_loopexpr_operand() {
        let mut func = make_loop_expr_function();
        // Before: function contains LoopExpr (backend-illegal)
        let has_loop_before = func.blocks[0]
            .stmts
            .iter()
            .any(|s| matches!(s, MirStmt::Assign(_, Rvalue::Use(Operand::LoopExpr { .. }))));
        assert!(has_loop_before, "pre-condition: LoopExpr must be present");

        lower_loop_expr(&mut func);

        // After: no LoopExpr operands remain
        let has_loop_after = func.blocks[0]
            .stmts
            .iter()
            .any(|s| matches!(s, MirStmt::Assign(_, Rvalue::Use(Operand::LoopExpr { .. }))));
        assert!(!has_loop_after, "LoopExpr must be desugared away");

        // A WhileStmt should be present (from the loop body)
        let has_while = func.blocks[0]
            .stmts
            .iter()
            .any(|s| matches!(s, MirStmt::WhileStmt { .. }));
        assert!(has_while, "WhileStmt must be present after desugaring");
    }

    #[test]
    fn lower_loop_exprs_produces_backend_legal_module() {
        let func = make_loop_expr_function();
        let mut module = MirModule::new();
        module.functions.push(func);
        module.entry_fn = Some(FnId(2));

        // Before lowering: backend-illegal due to LoopExpr
        assert!(
            !is_backend_legal_module(&module),
            "pre-condition: module must be backend-illegal"
        );

        lower_loop_exprs(&mut module);

        // After lowering: backend-legal (LoopExpr removed)
        assert!(
            is_backend_legal_module(&module),
            "module must be backend-legal after lower_loop_exprs"
        );
    }

    #[test]
    fn lower_loop_expr_preserves_if_and_try() {
        // Ensure IfExpr and TryExpr are NOT desugared by lower_loop_expr
        let mut func = MirFunction {
            id: FnId(1),
            name: "mixed_if".to_string(),
            instance: InstanceKey::simple("mixed_if"),
            params: vec![],
            return_ty: Type::I32,
            locals: vec![MirLocal {
                id: LocalId(0),
                name: Some("x".to_string()),
                ty: Type::I32,
            }],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![MirStmt::Assign(
                    Place::Local(LocalId(0)),
                    Rvalue::Use(Operand::IfExpr {
                        cond: Box::new(Operand::ConstBool(true)),
                        then_body: vec![],
                        then_result: Some(Box::new(Operand::ConstI32(1))),
                        else_body: vec![],
                        else_result: Some(Box::new(Operand::ConstI32(2))),
                    }),
                )],
                terminator: Terminator::Return(Some(Operand::Place(Place::Local(LocalId(0))))),
                source: default_block_source(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: vec![],
            source: default_function_source(),
            is_exported: false,
        };

        lower_loop_expr(&mut func);

        // IfExpr should still be present (not desugared by loop pass)
        let has_if = func.blocks[0]
            .stmts
            .iter()
            .any(|s| matches!(s, MirStmt::Assign(_, Rvalue::Use(Operand::IfExpr { .. }))));
        assert!(has_if, "IfExpr must be preserved by lower_loop_expr");
    }

    #[test]
    fn lower_combined_if_then_loop_produces_legal_module() {
        // Module with both IfExpr and LoopExpr — both passes needed
        let mut module = MirModule::new();
        module.functions.push(make_if_expr_function());
        module.functions.push(make_loop_expr_function());
        module.entry_fn = Some(FnId(1));

        assert!(!is_backend_legal_module(&module));

        lower_if_exprs(&mut module);
        lower_loop_exprs(&mut module);

        assert!(
            is_backend_legal_module(&module),
            "combined if+loop lowering must produce backend-legal module"
        );
    }

    fn make_try_expr_function() -> MirFunction {
        MirFunction {
            id: FnId(3),
            name: "try_fn".to_string(),
            instance: InstanceKey::simple("try_fn"),
            params: vec![],
            return_ty: Type::Result(Box::new(Type::I32), Box::new(Type::String)),
            locals: vec![
                MirLocal {
                    id: LocalId(0),
                    name: Some("input".to_string()),
                    ty: Type::Result(Box::new(Type::I32), Box::new(Type::String)),
                },
                MirLocal {
                    id: LocalId(1),
                    name: Some("value".to_string()),
                    ty: Type::I32,
                },
            ],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![MirStmt::Assign(
                    Place::Local(LocalId(1)),
                    Rvalue::Use(Operand::TryExpr {
                        expr: Box::new(Operand::Place(Place::Local(LocalId(0)))),
                        from_fn: None,
                    }),
                )],
                terminator: Terminator::Return(Some(Operand::Place(Place::Local(LocalId(1))))),
                source: default_block_source(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: vec![],
            source: default_function_source(),
            is_exported: false,
        }
    }

    #[test]
    fn lower_try_expr_removes_tryexpr_operand() {
        let mut func = make_try_expr_function();
        // Before: function contains TryExpr (backend-illegal)
        let has_try_before = func.blocks[0]
            .stmts
            .iter()
            .any(|s| matches!(s, MirStmt::Assign(_, Rvalue::Use(Operand::TryExpr { .. }))));
        assert!(has_try_before, "pre-condition: TryExpr must be present");

        lower_try_expr(&mut func);

        // After: no TryExpr operands remain; an IfStmt should be present instead
        let has_try_after = func.blocks[0]
            .stmts
            .iter()
            .any(|s| matches!(s, MirStmt::Assign(_, Rvalue::Use(Operand::TryExpr { .. }))));
        assert!(!has_try_after, "TryExpr must be desugared away");

        let has_if_stmt = func.blocks[0]
            .stmts
            .iter()
            .any(|s| matches!(s, MirStmt::IfStmt { .. }));
        assert!(
            has_if_stmt,
            "IfStmt must be present after TryExpr desugaring"
        );
    }

    #[test]
    fn lower_try_exprs_produces_backend_legal_module() {
        let func = make_try_expr_function();
        let mut module = MirModule::new();
        module.functions.push(func);
        module.entry_fn = Some(FnId(3));

        // Before lowering: backend-illegal due to TryExpr
        assert!(
            !is_backend_legal_module(&module),
            "pre-condition: module must be backend-illegal"
        );

        lower_try_exprs(&mut module);

        // After lowering: backend-legal (TryExpr removed)
        assert!(
            is_backend_legal_module(&module),
            "module must be backend-legal after lower_try_exprs"
        );
    }

    #[test]
    fn lower_try_expr_preserves_if_and_loop() {
        // Ensure IfExpr and LoopExpr are NOT desugared by lower_try_expr
        let mut func = MirFunction {
            id: FnId(1),
            name: "mixed_try".to_string(),
            instance: InstanceKey::simple("mixed_try"),
            params: vec![],
            return_ty: Type::I32,
            locals: vec![MirLocal {
                id: LocalId(0),
                name: Some("x".to_string()),
                ty: Type::I32,
            }],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![MirStmt::Assign(
                    Place::Local(LocalId(0)),
                    Rvalue::Use(Operand::IfExpr {
                        cond: Box::new(Operand::ConstBool(true)),
                        then_body: vec![],
                        then_result: Some(Box::new(Operand::ConstI32(1))),
                        else_body: vec![],
                        else_result: Some(Box::new(Operand::ConstI32(2))),
                    }),
                )],
                terminator: Terminator::Return(Some(Operand::Place(Place::Local(LocalId(0))))),
                source: default_block_source(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: vec![],
            source: default_function_source(),
            is_exported: false,
        };

        lower_try_expr(&mut func);

        // IfExpr should still be present (not desugared by try pass)
        let has_if = func.blocks[0]
            .stmts
            .iter()
            .any(|s| matches!(s, MirStmt::Assign(_, Rvalue::Use(Operand::IfExpr { .. }))));
        assert!(has_if, "IfExpr must be preserved by lower_try_expr");
    }

    #[test]
    fn lower_combined_if_loop_try_produces_legal_module() {
        // Module with IfExpr, LoopExpr, and TryExpr — all three passes needed
        let mut module = MirModule::new();
        module.functions.push(make_if_expr_function());
        module.functions.push(make_loop_expr_function());
        module.functions.push(make_try_expr_function());
        module.entry_fn = Some(FnId(1));

        assert!(!is_backend_legal_module(&module));

        lower_if_exprs(&mut module);
        lower_loop_exprs(&mut module);
        lower_try_exprs(&mut module);

        assert!(
            is_backend_legal_module(&module),
            "combined if+loop+try lowering must produce backend-legal module"
        );
    }
}
