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
}
