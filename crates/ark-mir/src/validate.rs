use std::collections::{HashMap, HashSet};

use crate::mir::{
    BinOp, BlockId, FnId, MirFunction, MirModule, MirStmt, Operand, Place, Rvalue, Terminator,
    UnaryOp, function_name_or_fallback, is_backend_legal_module,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirValidationError {
    pub function: String,
    pub block: Option<BlockId>,
    pub message: String,
}

impl MirValidationError {
    fn new(function: String, block: Option<BlockId>, message: impl Into<String>) -> Self {
        Self {
            function,
            block,
            message: message.into(),
        }
    }
}

pub fn validate_module(module: &MirModule) -> Result<(), Vec<MirValidationError>> {
    let mut errors = structural_errors(module);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(std::mem::take(&mut errors))
    }
}

pub fn validate_backend_legal_module(module: &MirModule) -> Result<(), Vec<MirValidationError>> {
    let mut errors = structural_errors(module);
    if !is_backend_legal_module(module) {
        errors.push(MirValidationError::new(
            "<module>".to_string(),
            None,
            "backend-illegal MIR nodes remain after lowering".to_string(),
        ));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn structural_errors(module: &MirModule) -> Vec<MirValidationError> {
    let mut errors = Vec::new();
    let mut seen_fn_ids = HashSet::new();

    for func in &module.functions {
        if !seen_fn_ids.insert(func.id) {
            errors.push(MirValidationError::new(
                func.name.clone(),
                None,
                format!("duplicate function id {}", func.id.0),
            ));
        }
        validate_function(module, func, &mut errors);
    }

    if module
        .entry_fn
        .is_some_and(|entry| !seen_fn_ids.contains(&entry))
    {
        errors.push(MirValidationError::new(
            "<module>".to_string(),
            None,
            format!(
                "entry function {} does not exist",
                module.entry_fn.unwrap().0
            ),
        ));
    }

    errors
}

fn validate_function(module: &MirModule, func: &MirFunction, errors: &mut Vec<MirValidationError>) {
    let function_name = func.name.clone();
    let mut block_ids = HashSet::new();
    let mut block_map = HashMap::new();

    for block in &func.blocks {
        if !block_ids.insert(block.id) {
            errors.push(MirValidationError::new(
                function_name.clone(),
                Some(block.id),
                format!("duplicate block id {}", block.id.0),
            ));
        }
        block_map.insert(block.id, block);
    }

    if !block_ids.contains(&func.entry) {
        errors.push(MirValidationError::new(
            function_name.clone(),
            None,
            format!("entry block {} does not exist", func.entry.0),
        ));
        return;
    }

    let mut declared = HashSet::new();
    for param in &func.params {
        declared.insert(param.id);
    }
    let mut seen_local_ids = HashSet::new();
    for local in &func.locals {
        if !seen_local_ids.insert(local.id) {
            errors.push(MirValidationError::new(
                function_name.clone(),
                None,
                format!("duplicate local id {}", local.id.0),
            ));
        }
        declared.insert(local.id);
    }

    for param in &func.params {
        if !seen_local_ids.contains(&param.id) {
            declared.insert(param.id);
        }
    }

    let mut reachable = HashSet::new();
    let mut worklist = vec![func.entry];
    while let Some(block_id) = worklist.pop() {
        if !reachable.insert(block_id) {
            continue;
        }
        let Some(block) = block_map.get(&block_id) else {
            errors.push(MirValidationError::new(
                function_name.clone(),
                Some(block_id),
                "reachable block id missing from function".to_string(),
            ));
            continue;
        };
        for succ in terminator_successors(&block.terminator) {
            if !block_ids.contains(&succ) {
                errors.push(MirValidationError::new(
                    function_name.clone(),
                    Some(block.id),
                    format!("terminator references unknown block {}", succ.0),
                ));
            } else {
                worklist.push(succ);
            }
        }
    }

    for block in &func.blocks {
        validate_block(module, func.id, &function_name, block, &declared, errors);
    }
}

fn validate_block(
    module: &MirModule,
    func_id: FnId,
    function_name: &str,
    block: &crate::mir::BasicBlock,
    declared: &HashSet<crate::mir::LocalId>,
    errors: &mut Vec<MirValidationError>,
) {
    for stmt in &block.stmts {
        validate_stmt(
            module,
            func_id,
            function_name,
            block.id,
            stmt,
            declared,
            errors,
        );
    }
    validate_terminator(
        module,
        func_id,
        function_name,
        block.id,
        &block.terminator,
        declared,
        errors,
    );
}

fn validate_stmt(
    module: &MirModule,
    _func_id: FnId,
    function_name: &str,
    block_id: BlockId,
    stmt: &MirStmt,
    declared: &HashSet<crate::mir::LocalId>,
    errors: &mut Vec<MirValidationError>,
) {
    match stmt {
        MirStmt::Assign(place, rvalue) => {
            validate_place(function_name, block_id, place, declared, errors);
            validate_rvalue(function_name, block_id, rvalue, declared, errors);
        }
        MirStmt::Call { dest, func, args } => {
            if module
                .functions
                .iter()
                .all(|candidate| candidate.id != *func)
            {
                errors.push(MirValidationError::new(
                    function_name.to_string(),
                    Some(block_id),
                    format!(
                        "call references unknown function {}",
                        function_name_or_fallback(module, *func)
                    ),
                ));
            }
            if let Some(dest) = dest {
                validate_place(function_name, block_id, dest, declared, errors);
            }
            for arg in args {
                validate_operand(function_name, block_id, arg, declared, errors);
            }
        }
        MirStmt::CallBuiltin { dest, args, .. } => {
            if let Some(dest) = dest {
                validate_place(function_name, block_id, dest, declared, errors);
            }
            for arg in args {
                validate_operand(function_name, block_id, arg, declared, errors);
            }
        }
        MirStmt::IfStmt {
            cond,
            then_body,
            else_body,
        } => {
            validate_operand(function_name, block_id, cond, declared, errors);
            for nested in then_body {
                validate_stmt(
                    module,
                    FnId(0),
                    function_name,
                    block_id,
                    nested,
                    declared,
                    errors,
                );
            }
            for nested in else_body {
                validate_stmt(
                    module,
                    FnId(0),
                    function_name,
                    block_id,
                    nested,
                    declared,
                    errors,
                );
            }
        }
        MirStmt::WhileStmt { cond, body } => {
            validate_operand(function_name, block_id, cond, declared, errors);
            for nested in body {
                validate_stmt(
                    module,
                    FnId(0),
                    function_name,
                    block_id,
                    nested,
                    declared,
                    errors,
                );
            }
        }
        MirStmt::Break | MirStmt::Continue => {}
        MirStmt::Return(value) => {
            if let Some(value) = value {
                validate_operand(function_name, block_id, value, declared, errors);
            }
        }
        MirStmt::GcHint { .. } => {}
    }
}

fn validate_terminator(
    module: &MirModule,
    _func_id: FnId,
    function_name: &str,
    block_id: BlockId,
    terminator: &Terminator,
    declared: &HashSet<crate::mir::LocalId>,
    errors: &mut Vec<MirValidationError>,
) {
    match terminator {
        Terminator::Goto(_) | Terminator::Unreachable => {}
        Terminator::If { cond, .. } => {
            validate_operand(function_name, block_id, cond, declared, errors)
        }
        Terminator::Switch { scrutinee, .. } => {
            validate_operand(function_name, block_id, scrutinee, declared, errors)
        }
        Terminator::Return(value) => {
            if let Some(value) = value {
                validate_operand(function_name, block_id, value, declared, errors);
            }
        }
    }

    if matches!(terminator, Terminator::Unreachable)
        && module
            .entry_fn
            .is_some_and(|entry| entry == FnId(block_id.0))
    {
        errors.push(MirValidationError::new(
            function_name.to_string(),
            Some(block_id),
            "entry block cannot terminate with unreachable".to_string(),
        ));
    }
}

fn validate_rvalue(
    function_name: &str,
    block_id: BlockId,
    rvalue: &Rvalue,
    declared: &HashSet<crate::mir::LocalId>,
    errors: &mut Vec<MirValidationError>,
) {
    match rvalue {
        Rvalue::Use(operand) => {
            validate_operand(function_name, block_id, operand, declared, errors)
        }
        Rvalue::BinaryOp(op, lhs, rhs) => {
            validate_binary_op(function_name, block_id, *op, lhs, rhs, declared, errors)
        }
        Rvalue::UnaryOp(op, operand) => {
            validate_unary_op(function_name, block_id, *op, operand, declared, errors)
        }
        Rvalue::Aggregate(_, operands) => {
            for operand in operands {
                validate_operand(function_name, block_id, operand, declared, errors);
            }
        }
        Rvalue::Ref(place) => validate_place(function_name, block_id, place, declared, errors),
    }
}

fn validate_binary_op(
    function_name: &str,
    block_id: BlockId,
    op: BinOp,
    lhs: &Operand,
    rhs: &Operand,
    declared: &HashSet<crate::mir::LocalId>,
    errors: &mut Vec<MirValidationError>,
) {
    validate_operand(function_name, block_id, lhs, declared, errors);
    validate_operand(function_name, block_id, rhs, declared, errors);

    if matches!(op, BinOp::Div | BinOp::Mod)
        && matches!(rhs, Operand::ConstI32(0) | Operand::ConstI64(0))
    {
        errors.push(MirValidationError::new(
            function_name.to_string(),
            Some(block_id),
            "division or modulo by zero constant in MIR".to_string(),
        ));
    }
}

fn validate_unary_op(
    function_name: &str,
    block_id: BlockId,
    _op: UnaryOp,
    operand: &Operand,
    declared: &HashSet<crate::mir::LocalId>,
    errors: &mut Vec<MirValidationError>,
) {
    validate_operand(function_name, block_id, operand, declared, errors);
}

fn validate_place(
    function_name: &str,
    block_id: BlockId,
    place: &Place,
    declared: &HashSet<crate::mir::LocalId>,
    errors: &mut Vec<MirValidationError>,
) {
    match place {
        Place::Local(local) => {
            if !declared.contains(local) {
                errors.push(MirValidationError::new(
                    function_name.to_string(),
                    Some(block_id),
                    format!("use of undeclared local {}", local.0),
                ));
            }
        }
        Place::Field(place, _) => validate_place(function_name, block_id, place, declared, errors),
        Place::Index(place, index) => {
            validate_place(function_name, block_id, place, declared, errors);
            validate_operand(function_name, block_id, index, declared, errors);
        }
    }
}

fn validate_operand(
    function_name: &str,
    block_id: BlockId,
    operand: &Operand,
    declared: &HashSet<crate::mir::LocalId>,
    errors: &mut Vec<MirValidationError>,
) {
    match operand {
        Operand::Place(place) => validate_place(function_name, block_id, place, declared, errors),
        Operand::ConstI32(_)
        | Operand::ConstI64(_)
        | Operand::ConstF32(_)
        | Operand::ConstF64(_)
        | Operand::ConstU8(_)
        | Operand::ConstU16(_)
        | Operand::ConstU32(_)
        | Operand::ConstU64(_)
        | Operand::ConstI8(_)
        | Operand::ConstI16(_)
        | Operand::ConstBool(_)
        | Operand::ConstChar(_)
        | Operand::ConstString(_)
        | Operand::Unit
        | Operand::FnRef(_) => {}
        Operand::BinOp(_, lhs, rhs) => {
            validate_operand(function_name, block_id, lhs, declared, errors);
            validate_operand(function_name, block_id, rhs, declared, errors);
        }
        Operand::UnaryOp(_, operand) | Operand::EnumTag(operand) => {
            validate_operand(function_name, block_id, operand, declared, errors)
        }
        Operand::Call(_, args) => {
            for arg in args {
                validate_operand(function_name, block_id, arg, declared, errors);
            }
        }
        Operand::IfExpr {
            cond,
            then_body,
            then_result,
            else_body,
            else_result,
        } => {
            validate_operand(function_name, block_id, cond, declared, errors);
            for stmt in then_body {
                validate_stmt(
                    &MirModule::default(),
                    FnId(0),
                    function_name,
                    block_id,
                    stmt,
                    declared,
                    errors,
                );
            }
            if let Some(result) = then_result {
                validate_operand(function_name, block_id, result, declared, errors);
            }
            for stmt in else_body {
                validate_stmt(
                    &MirModule::default(),
                    FnId(0),
                    function_name,
                    block_id,
                    stmt,
                    declared,
                    errors,
                );
            }
            if let Some(result) = else_result {
                validate_operand(function_name, block_id, result, declared, errors);
            }
        }
        Operand::StructInit { fields, .. } => {
            for (_, operand) in fields {
                validate_operand(function_name, block_id, operand, declared, errors);
            }
        }
        Operand::FieldAccess { object, .. } => {
            validate_operand(function_name, block_id, object, declared, errors)
        }
        Operand::EnumInit { payload, .. } => {
            for operand in payload {
                validate_operand(function_name, block_id, operand, declared, errors);
            }
        }
        Operand::EnumPayload { object, .. } => {
            validate_operand(function_name, block_id, object, declared, errors)
        }
        Operand::LoopExpr { init, body, result } => {
            validate_operand(function_name, block_id, init, declared, errors);
            for stmt in body {
                validate_stmt(
                    &MirModule::default(),
                    FnId(0),
                    function_name,
                    block_id,
                    stmt,
                    declared,
                    errors,
                );
            }
            validate_operand(function_name, block_id, result, declared, errors);
        }
        Operand::TryExpr { expr, .. } => {
            validate_operand(function_name, block_id, expr, declared, errors)
        }
        Operand::CallIndirect { callee, args } => {
            validate_operand(function_name, block_id, callee, declared, errors);
            for arg in args {
                validate_operand(function_name, block_id, arg, declared, errors);
            }
        }
        Operand::ArrayInit { elements } => {
            for element in elements {
                validate_operand(function_name, block_id, element, declared, errors);
            }
        }
        Operand::IndexAccess { object, index } => {
            validate_operand(function_name, block_id, object, declared, errors);
            validate_operand(function_name, block_id, index, declared, errors);
        }
    }
}

fn terminator_successors(terminator: &Terminator) -> Vec<BlockId> {
    match terminator {
        Terminator::Goto(block) => vec![*block],
        Terminator::If {
            then_block,
            else_block,
            ..
        } => vec![*then_block, *else_block],
        Terminator::Switch { arms, default, .. } => {
            let mut blocks = arms.iter().map(|(_, block)| *block).collect::<Vec<_>>();
            blocks.push(*default);
            blocks
        }
        Terminator::Return(_) | Terminator::Unreachable => Vec::new(),
    }
}

// ── type_table consistency validation (issue 449 / MIR-01) ───────────────────

/// Validates that `module.struct_defs` and `module.enum_defs` are consistent
/// with `module.type_table` — the canonical backend type source of truth.
///
/// Also checks that every `StructInit` / `EnumInit` / `FieldAccess` operand
/// in the MIR references a name present in `type_table`.
///
/// This function is intentionally separate from `validate_module` so it can be
/// invoked as an explicit audit without blocking the existing compilation path
/// (which may encounter pre-existing MIR inconsistencies in trait impls).
pub fn validate_type_table_consistency(module: &MirModule) -> Vec<MirValidationError> {
    let mut errors = Vec::new();

    // -- 1. struct_defs vs type_table.struct_defs --
    for (name, _) in &module.struct_defs {
        if !module.type_table.struct_defs.contains_key(name.as_str()) {
            errors.push(MirValidationError::new(
                "<module>".to_string(),
                None,
                format!(
                    "type_table mismatch: struct '{}' present in module.struct_defs but absent from type_table.struct_defs",
                    name
                ),
            ));
        }
    }
    for (name, _) in &module.type_table.struct_defs {
        if !module.struct_defs.contains_key(name.as_str()) {
            errors.push(MirValidationError::new(
                "<module>".to_string(),
                None,
                format!(
                    "type_table mismatch: struct '{}' present in type_table.struct_defs but absent from module.struct_defs",
                    name
                ),
            ));
        }
    }

    // -- 2. enum_defs vs type_table.enum_defs --
    for (name, _) in &module.enum_defs {
        if !module.type_table.enum_defs.contains_key(name.as_str()) {
            errors.push(MirValidationError::new(
                "<module>".to_string(),
                None,
                format!(
                    "type_table mismatch: enum '{}' present in module.enum_defs but absent from type_table.enum_defs",
                    name
                ),
            ));
        }
    }
    for (name, _) in &module.type_table.enum_defs {
        if !module.enum_defs.contains_key(name.as_str()) {
            errors.push(MirValidationError::new(
                "<module>".to_string(),
                None,
                format!(
                    "type_table mismatch: enum '{}' present in type_table.enum_defs but absent from module.enum_defs",
                    name
                ),
            ));
        }
    }

    // -- 3. MIR operand references must be resolvable in type_table --
    for func in &module.functions {
        for block in &func.blocks {
            for stmt in &block.stmts {
                check_stmt_type_table_ref(module, &func.name, block.id, stmt, &mut errors);
            }
            check_terminator_type_table_ref(
                module,
                &func.name,
                block.id,
                &block.terminator,
                &mut errors,
            );
        }
    }

    errors
}

fn check_operand_type_table_ref(
    module: &MirModule,
    function_name: &str,
    block_id: BlockId,
    operand: &Operand,
    errors: &mut Vec<MirValidationError>,
) {
    match operand {
        Operand::StructInit { name, fields } => {
            if !module.type_table.struct_defs.contains_key(name.as_str()) {
                errors.push(MirValidationError::new(
                    function_name.to_string(),
                    Some(block_id),
                    format!(
                        "type_table mismatch: StructInit references '{}' which is absent from type_table.struct_defs",
                        name
                    ),
                ));
            }
            for (_, op) in fields {
                check_operand_type_table_ref(module, function_name, block_id, op, errors);
            }
        }
        Operand::FieldAccess { object, struct_name, .. } => {
            if !struct_name.is_empty()
                && !module.type_table.struct_defs.contains_key(struct_name.as_str())
            {
                errors.push(MirValidationError::new(
                    function_name.to_string(),
                    Some(block_id),
                    format!(
                        "type_table mismatch: FieldAccess references struct '{}' which is absent from type_table.struct_defs",
                        struct_name
                    ),
                ));
            }
            check_operand_type_table_ref(module, function_name, block_id, object, errors);
        }
        Operand::EnumInit { enum_name, payload, .. } => {
            if !module.type_table.enum_defs.contains_key(enum_name.as_str()) {
                errors.push(MirValidationError::new(
                    function_name.to_string(),
                    Some(block_id),
                    format!(
                        "type_table mismatch: EnumInit references '{}' which is absent from type_table.enum_defs",
                        enum_name
                    ),
                ));
            }
            for op in payload {
                check_operand_type_table_ref(module, function_name, block_id, op, errors);
            }
        }
        Operand::EnumPayload { object, enum_name, .. } => {
            if !module.type_table.enum_defs.contains_key(enum_name.as_str()) {
                errors.push(MirValidationError::new(
                    function_name.to_string(),
                    Some(block_id),
                    format!(
                        "type_table mismatch: EnumPayload references '{}' which is absent from type_table.enum_defs",
                        enum_name
                    ),
                ));
            }
            check_operand_type_table_ref(module, function_name, block_id, object, errors);
        }
        Operand::BinOp(_, lhs, rhs) => {
            check_operand_type_table_ref(module, function_name, block_id, lhs, errors);
            check_operand_type_table_ref(module, function_name, block_id, rhs, errors);
        }
        Operand::UnaryOp(_, inner) | Operand::EnumTag(inner) => {
            check_operand_type_table_ref(module, function_name, block_id, inner, errors);
        }
        Operand::Call(_, args) => {
            for arg in args {
                check_operand_type_table_ref(module, function_name, block_id, arg, errors);
            }
        }
        Operand::IfExpr { cond, then_body, then_result, else_body, else_result } => {
            check_operand_type_table_ref(module, function_name, block_id, cond, errors);
            for s in then_body {
                check_stmt_type_table_ref(module, function_name, block_id, s, errors);
            }
            if let Some(r) = then_result {
                check_operand_type_table_ref(module, function_name, block_id, r, errors);
            }
            for s in else_body {
                check_stmt_type_table_ref(module, function_name, block_id, s, errors);
            }
            if let Some(r) = else_result {
                check_operand_type_table_ref(module, function_name, block_id, r, errors);
            }
        }
        Operand::LoopExpr { init, body, result } => {
            check_operand_type_table_ref(module, function_name, block_id, init, errors);
            for s in body {
                check_stmt_type_table_ref(module, function_name, block_id, s, errors);
            }
            check_operand_type_table_ref(module, function_name, block_id, result, errors);
        }
        Operand::TryExpr { expr, .. } => {
            check_operand_type_table_ref(module, function_name, block_id, expr, errors);
        }
        Operand::CallIndirect { callee, args } => {
            check_operand_type_table_ref(module, function_name, block_id, callee, errors);
            for arg in args {
                check_operand_type_table_ref(module, function_name, block_id, arg, errors);
            }
        }
        Operand::ArrayInit { elements } => {
            for el in elements {
                check_operand_type_table_ref(module, function_name, block_id, el, errors);
            }
        }
        Operand::IndexAccess { object, index } => {
            check_operand_type_table_ref(module, function_name, block_id, object, errors);
            check_operand_type_table_ref(module, function_name, block_id, index, errors);
        }
        Operand::Place(_)
        | Operand::ConstI32(_)
        | Operand::ConstI64(_)
        | Operand::ConstF32(_)
        | Operand::ConstF64(_)
        | Operand::ConstU8(_)
        | Operand::ConstU16(_)
        | Operand::ConstU32(_)
        | Operand::ConstU64(_)
        | Operand::ConstI8(_)
        | Operand::ConstI16(_)
        | Operand::ConstBool(_)
        | Operand::ConstChar(_)
        | Operand::ConstString(_)
        | Operand::Unit
        | Operand::FnRef(_) => {}
    }
}

fn check_stmt_type_table_ref(
    module: &MirModule,
    function_name: &str,
    block_id: BlockId,
    stmt: &MirStmt,
    errors: &mut Vec<MirValidationError>,
) {
    match stmt {
        MirStmt::Assign(_, rvalue) => match rvalue {
            Rvalue::Use(op) => {
                check_operand_type_table_ref(module, function_name, block_id, op, errors);
            }
            Rvalue::Aggregate(_, ops) => {
                for op in ops {
                    check_operand_type_table_ref(module, function_name, block_id, op, errors);
                }
            }
            Rvalue::BinaryOp(_, lhs, rhs) => {
                check_operand_type_table_ref(module, function_name, block_id, lhs, errors);
                check_operand_type_table_ref(module, function_name, block_id, rhs, errors);
            }
            Rvalue::UnaryOp(_, op) => {
                check_operand_type_table_ref(module, function_name, block_id, op, errors);
            }
            Rvalue::Ref(_) => {}
        },
        MirStmt::Call { args, .. } => {
            for arg in args {
                check_operand_type_table_ref(module, function_name, block_id, arg, errors);
            }
        }
        MirStmt::CallBuiltin { args, .. } => {
            for arg in args {
                check_operand_type_table_ref(module, function_name, block_id, arg, errors);
            }
        }
        MirStmt::IfStmt { cond, then_body, else_body } => {
            check_operand_type_table_ref(module, function_name, block_id, cond, errors);
            for s in then_body {
                check_stmt_type_table_ref(module, function_name, block_id, s, errors);
            }
            for s in else_body {
                check_stmt_type_table_ref(module, function_name, block_id, s, errors);
            }
        }
        MirStmt::WhileStmt { cond, body } => {
            check_operand_type_table_ref(module, function_name, block_id, cond, errors);
            for s in body {
                check_stmt_type_table_ref(module, function_name, block_id, s, errors);
            }
        }
        MirStmt::Return(Some(op)) => {
            check_operand_type_table_ref(module, function_name, block_id, op, errors);
        }
        MirStmt::Return(None)
        | MirStmt::Break
        | MirStmt::Continue
        | MirStmt::GcHint { .. } => {}
    }
}

fn check_terminator_type_table_ref(
    module: &MirModule,
    function_name: &str,
    block_id: BlockId,
    terminator: &Terminator,
    errors: &mut Vec<MirValidationError>,
) {
    match terminator {
        Terminator::Return(Some(op)) => {
            check_operand_type_table_ref(module, function_name, block_id, op, errors);
        }
        Terminator::If { cond, .. } => {
            check_operand_type_table_ref(module, function_name, block_id, cond, errors);
        }
        Terminator::Switch { scrutinee, .. } => {
            check_operand_type_table_ref(module, function_name, block_id, scrutinee, errors);
        }
        Terminator::Goto(_)
        | Terminator::Unreachable
        | Terminator::Return(None) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{
        BasicBlock, BinOp, FnId, InstanceKey, LocalId, MirFunction, MirLocal, MirModule, MirStmt,
        Operand, Place, Rvalue, Terminator, default_block_source, default_function_source,
        sync_module_metadata,
    };
    use ark_typecheck::types::Type;

    fn make_function() -> MirFunction {
        MirFunction {
            id: FnId(0),
            name: "main".to_string(),
            instance: InstanceKey::simple("main"),
            params: Vec::new(),
            return_ty: Type::Unit,
            locals: vec![MirLocal {
                id: LocalId(0),
                name: Some("x".to_string()),
                ty: Type::I32,
            }],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![MirStmt::Assign(
                    Place::Local(LocalId(0)),
                    Rvalue::BinaryOp(BinOp::Add, Operand::ConstI32(1), Operand::ConstI32(2)),
                )],
                terminator: Terminator::Return(None),
                source: default_block_source(),
            }],
            entry: BlockId(0),
            struct_typed_locals: HashMap::new(),
            enum_typed_locals: HashMap::new(),
            type_params: vec![],
            source: default_function_source(),
            is_exported: false,
        }
    }

    #[test]
    fn valid_module_passes() {
        let mut module = MirModule::new();
        module.entry_fn = Some(FnId(0));
        module.functions.push(make_function());
        sync_module_metadata(&mut module);
        assert!(validate_module(&module).is_ok());
    }

    #[test]
    fn undeclared_local_fails() {
        let mut module = MirModule::new();
        module.entry_fn = Some(FnId(0));
        let mut function = make_function();
        function.blocks[0].stmts = vec![MirStmt::Assign(
            Place::Local(LocalId(99)),
            Rvalue::Use(Operand::ConstI32(1)),
        )];
        module.functions.push(function);
        sync_module_metadata(&mut module);
        let errors = validate_module(&module).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("undeclared local"))
        );
    }

    // ── type_table consistency tests (issue 449 / MIR-01) ─────────────────

    #[test]
    fn type_table_consistent_module_passes() {
        let mut module = MirModule::new();
        module.entry_fn = Some(FnId(0));
        module.functions.push(make_function());
        let fields = vec![("x".to_string(), "i32".to_string())];
        module.struct_defs.insert("Point".to_string(), fields.clone());
        module.type_table.struct_defs.insert("Point".to_string(), fields);
        let variants = vec![("Some".to_string(), vec!["i32".to_string()])];
        module.enum_defs.insert("Option".to_string(), variants.clone());
        module.type_table.enum_defs.insert("Option".to_string(), variants);
        sync_module_metadata(&mut module);
        let errors = validate_type_table_consistency(&module);
        assert!(errors.is_empty(), "consistent type_table should produce no errors: {:?}", errors);
    }

    #[test]
    fn struct_missing_from_type_table_detected() {
        let mut module = MirModule::new();
        module.entry_fn = Some(FnId(0));
        module.functions.push(make_function());
        module.struct_defs.insert("Ghost".to_string(), vec![("f".to_string(), "i32".to_string())]);
        sync_module_metadata(&mut module);
        let errors = validate_type_table_consistency(&module);
        assert!(
            errors.iter().any(|e| e.message.contains("type_table mismatch")
                && e.message.contains("struct")
                && e.message.contains("Ghost")),
            "expected mismatch for struct 'Ghost': {:?}", errors
        );
    }

    #[test]
    fn enum_missing_from_type_table_detected() {
        let mut module = MirModule::new();
        module.entry_fn = Some(FnId(0));
        module.functions.push(make_function());
        module.enum_defs.insert("Phantom".to_string(), vec![("A".to_string(), vec![])]);
        sync_module_metadata(&mut module);
        let errors = validate_type_table_consistency(&module);
        assert!(
            errors.iter().any(|e| e.message.contains("type_table mismatch")
                && e.message.contains("enum")
                && e.message.contains("Phantom")),
            "expected mismatch for enum 'Phantom': {:?}", errors
        );
    }

    #[test]
    fn struct_init_absent_from_type_table_detected() {
        let mut module = MirModule::new();
        module.entry_fn = Some(FnId(0));
        let mut func = make_function();
        func.blocks[0].stmts = vec![MirStmt::Assign(
            Place::Local(LocalId(0)),
            Rvalue::Use(Operand::StructInit {
                name: "NoSuchStruct".to_string(),
                fields: vec![],
            }),
        )];
        module.functions.push(func);
        sync_module_metadata(&mut module);
        let errors = validate_type_table_consistency(&module);
        assert!(
            errors.iter().any(|e| e.message.contains("type_table mismatch")
                && e.message.contains("StructInit")
                && e.message.contains("NoSuchStruct")),
            "expected mismatch for StructInit 'NoSuchStruct': {:?}", errors
        );
    }

    #[test]
    fn enum_init_absent_from_type_table_detected() {
        let mut module = MirModule::new();
        module.entry_fn = Some(FnId(0));
        let mut func = make_function();
        func.blocks[0].stmts = vec![MirStmt::Assign(
            Place::Local(LocalId(0)),
            Rvalue::Use(Operand::EnumInit {
                enum_name: "NoSuchEnum".to_string(),
                variant: "A".to_string(),
                tag: 0,
                payload: vec![],
            }),
        )];
        module.functions.push(func);
        sync_module_metadata(&mut module);
        let errors = validate_type_table_consistency(&module);
        assert!(
            errors.iter().any(|e| e.message.contains("type_table mismatch")
                && e.message.contains("EnumInit")
                && e.message.contains("NoSuchEnum")),
            "expected mismatch for EnumInit 'NoSuchEnum': {:?}", errors
        );
    }
}
