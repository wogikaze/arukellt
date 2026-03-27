//! MIR → LLVM IR emitter.
//!
//! Lowers MIR to LLVM IR using inkwell. Phase 1 handles scalar types,
//! arithmetic, control flow, and function calls.

use ark_diagnostics::DiagnosticSink;
use ark_mir::mir::*;
use ark_target::{BackendPlan, RuntimeModel};
use ark_typecheck::types::Type;
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PointerValue};
use std::collections::HashMap;

/// Normalize `__intrinsic_*` names to canonical emit names.
fn normalize_intrinsic_name(name: &str) -> &str {
    match name {
        "__intrinsic_println" => "println",
        "__intrinsic_print" => "print",
        "__intrinsic_eprintln" => "eprintln",
        "__intrinsic_string_from" => "String_from",
        "__intrinsic_string_new" => "String_new",
        "__intrinsic_concat" => "concat",
        "__intrinsic_i32_to_string" => "i32_to_string",
        "__intrinsic_i64_to_string" => "i64_to_string",
        "__intrinsic_f64_to_string" => "f64_to_string",
        "__intrinsic_bool_to_string" => "bool_to_string",
        "__intrinsic_char_to_string" => "char_to_string",
        "__intrinsic_panic" => "panic",
        "__intrinsic_assert" => "assert",
        "__intrinsic_assert_eq" => "assert_eq",
        "__intrinsic_len" => "len",
        "__intrinsic_push" => "push",
        "__intrinsic_get" => "get",
        other => other,
    }
}

pub fn validate_plan(plan: &BackendPlan) -> Result<(), String> {
    if !matches!(plan.runtime_model, RuntimeModel::T4LlvmScaffold) {
        return Err("LLVM backend only supports T4LlvmScaffold plans".to_string());
    }
    Ok(())
}

pub fn emit_with_plan(
    mir: &MirModule,
    plan: &BackendPlan,
    sink: &mut DiagnosticSink,
) -> Result<String, String> {
    validate_plan(plan)?;
    Ok(emit_llvm_ir(mir, sink))
}

/// Emit LLVM IR text from MIR.
pub fn emit_llvm_ir(mir: &MirModule, _sink: &mut DiagnosticSink) -> String {
    let context = Context::create();
    let mut emitter = LlvmEmitter::new(&context, mir);
    emitter.emit_module();
    emitter.module.print_to_string().to_string()
}

/// Emit a native object file from MIR.
pub fn emit_object(mir: &MirModule, _sink: &mut DiagnosticSink) -> Result<Vec<u8>, String> {
    let context = Context::create();
    let mut emitter = LlvmEmitter::new(&context, mir);
    emitter.emit_module();

    Target::initialize_native(&InitializationConfig::default())
        .map_err(|e| format!("failed to initialize native target: {}", e))?;

    let triple = TargetMachine::get_default_triple();
    let target =
        Target::from_triple(&triple).map_err(|e| format!("invalid target triple: {}", e))?;
    let machine = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            OptimizationLevel::Default,
            RelocMode::PIC,
            CodeModel::Default,
        )
        .ok_or_else(|| "failed to create target machine".to_string())?;

    let buf = machine
        .write_to_memory_buffer(&emitter.module, FileType::Object)
        .map_err(|e| format!("failed to emit object: {}", e))?;

    Ok(buf.as_slice().to_vec())
}

#[allow(dead_code)]
struct LlvmEmitter<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    mir: &'ctx MirModule,
    fn_values: HashMap<String, FunctionValue<'ctx>>,
    locals: HashMap<u32, PointerValue<'ctx>>,
    local_types: HashMap<u32, Type>,
    printf_fn: Option<FunctionValue<'ctx>>,
    malloc_fn: Option<FunctionValue<'ctx>>,
    sprintf_fn: Option<FunctionValue<'ctx>>,
    strlen_fn: Option<FunctionValue<'ctx>>,
    memcpy_fn: Option<FunctionValue<'ctx>>,
    struct_layouts: HashMap<String, Vec<(String, String)>>,
    enum_defs: HashMap<String, Vec<(String, Vec<String>)>>,
    // Loop tracking: stack of (loop_header, loop_exit) blocks
    loop_stack: Vec<(
        inkwell::basic_block::BasicBlock<'ctx>,
        inkwell::basic_block::BasicBlock<'ctx>,
    )>,
}

impl<'ctx> LlvmEmitter<'ctx> {
    fn new(context: &'ctx Context, mir: &'ctx MirModule) -> Self {
        let module = context.create_module("arukellt");
        let builder = context.create_builder();
        Self {
            context,
            module,
            builder,
            mir,
            fn_values: HashMap::new(),
            locals: HashMap::new(),
            local_types: HashMap::new(),
            printf_fn: None,
            malloc_fn: None,
            sprintf_fn: None,
            strlen_fn: None,
            memcpy_fn: None,
            struct_layouts: mir.type_table.struct_defs.clone(),
            enum_defs: mir.type_table.enum_defs.clone(),
            loop_stack: Vec::new(),
        }
    }

    fn type_to_llvm(&self, ty: &Type) -> BasicTypeEnum<'ctx> {
        match ty {
            Type::I32 => self.context.i32_type().into(),
            Type::I64 => self.context.i64_type().into(),
            Type::F32 => self.context.f32_type().into(),
            Type::F64 => self.context.f64_type().into(),
            Type::Bool => self.context.bool_type().into(),
            Type::Char => self.context.i32_type().into(),
            // Heap types use i8* (pointer) — length-prefixed data in malloc'd memory
            Type::String
            | Type::Vec(_)
            | Type::Struct(_)
            | Type::Enum(_)
            | Type::Option(_)
            | Type::Result(_, _) => self.context.ptr_type(AddressSpace::default()).into(),
            _ => self.context.i32_type().into(),
        }
    }

    fn declare_printf(&mut self) {
        if self.printf_fn.is_some() {
            return;
        }
        let i32_type = self.context.i32_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let printf_type = i32_type.fn_type(&[ptr_type.into()], true);
        let printf = self.module.add_function("printf", printf_type, None);
        self.printf_fn = Some(printf);
    }

    fn declare_exit(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("exit") {
            return f;
        }
        let void_type = self.context.void_type();
        let i32_type = self.context.i32_type();
        let exit_type = void_type.fn_type(&[i32_type.into()], false);
        self.module.add_function("exit", exit_type, None)
    }

    fn declare_malloc(&mut self) {
        if self.malloc_fn.is_some() {
            return;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
        let malloc = self.module.add_function("malloc", malloc_type, None);
        self.malloc_fn = Some(malloc);
    }

    fn declare_sprintf(&mut self) {
        if self.sprintf_fn.is_some() {
            return;
        }
        let i32_type = self.context.i32_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let sprintf_type = i32_type.fn_type(&[ptr_type.into(), ptr_type.into()], true);
        let sprintf = self.module.add_function("sprintf", sprintf_type, None);
        self.sprintf_fn = Some(sprintf);
    }

    fn declare_strlen(&mut self) {
        if self.strlen_fn.is_some() {
            return;
        }
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let strlen_type = i64_type.fn_type(&[ptr_type.into()], false);
        let strlen = self.module.add_function("strlen", strlen_type, None);
        self.strlen_fn = Some(strlen);
    }

    fn declare_memcpy(&mut self) {
        if self.memcpy_fn.is_some() {
            return;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let memcpy_type =
            ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), i64_type.into()], false);
        let memcpy = self.module.add_function("memcpy", memcpy_type, None);
        self.memcpy_fn = Some(memcpy);
    }

    fn emit_module(&mut self) {
        self.declare_printf();
        self.declare_malloc();
        self.declare_sprintf();
        self.declare_strlen();
        self.declare_memcpy();

        // Forward-declare all functions
        for func in &self.mir.functions {
            let params: Vec<BasicMetadataTypeEnum<'ctx>> = func
                .params
                .iter()
                .map(|p| self.type_to_llvm(&p.ty).into())
                .collect();

            let fn_type = match &func.return_ty {
                Type::Unit | Type::Never => self.context.void_type().fn_type(&params, false),
                ty => self.type_to_llvm(ty).fn_type(&params, false),
            };

            let name = if func.name == "main" {
                "__ark_main"
            } else {
                &func.name
            };
            let fn_val = self.module.add_function(name, fn_type, None);
            self.fn_values.insert(func.name.clone(), fn_val);
        }

        // Emit function bodies
        let functions: Vec<MirFunction> = self
            .mir
            .functions
            .iter()
            .map(|f| MirFunction {
                id: f.id,
                name: f.name.clone(),
                instance: f.instance.clone(),
                params: f.params.clone(),
                return_ty: f.return_ty.clone(),
                locals: f.locals.clone(),
                blocks: f
                    .blocks
                    .iter()
                    .map(|b| BasicBlock {
                        id: b.id,
                        stmts: b.stmts.clone(),
                        terminator: match &b.terminator {
                            Terminator::Return(v) => Terminator::Return(v.clone()),
                            Terminator::Goto(b) => Terminator::Goto(*b),
                            Terminator::Unreachable => Terminator::Unreachable,
                            Terminator::If {
                                cond,
                                then_block,
                                else_block,
                            } => Terminator::If {
                                cond: cond.clone(),
                                then_block: *then_block,
                                else_block: *else_block,
                            },
                            Terminator::Switch {
                                scrutinee,
                                arms,
                                default,
                            } => Terminator::Switch {
                                scrutinee: scrutinee.clone(),
                                arms: arms.clone(),
                                default: *default,
                            },
                        },
                        source: b.source,
                    })
                    .collect(),
                entry: f.entry,
                struct_typed_locals: f.struct_typed_locals.clone(),
                source: f.source,
            })
            .collect();

        for func in &functions {
            self.emit_function(func);
        }

        // Emit C main that calls __ark_main
        self.emit_c_main();
    }

    fn emit_c_main(&mut self) {
        let i32_type = self.context.i32_type();
        let main_type = i32_type.fn_type(&[], false);
        let main_fn = self.module.add_function("main", main_type, None);
        let entry = self.context.append_basic_block(main_fn, "entry");
        self.builder.position_at_end(entry);

        if let Some(ark_main) = self.fn_values.get("main") {
            let ret_type = &self
                .mir
                .functions
                .iter()
                .find(|f| f.name == "main")
                .map(|f| f.return_ty.clone())
                .unwrap_or(Type::Unit);
            match ret_type {
                Type::Unit | Type::Never => {
                    self.builder.build_call(*ark_main, &[], "").unwrap();
                }
                _ => {
                    self.builder.build_call(*ark_main, &[], "call").unwrap();
                }
            }
        }

        let zero = i32_type.const_int(0, false);
        self.builder.build_return(Some(&zero)).unwrap();
    }

    fn emit_function(&mut self, func: &MirFunction) {
        let fn_val = match self.fn_values.get(&func.name) {
            Some(f) => *f,
            None => return,
        };

        let entry_block = self.context.append_basic_block(fn_val, "entry");
        self.builder.position_at_end(entry_block);
        self.locals.clear();
        self.local_types.clear();
        self.loop_stack.clear();

        // Allocate locals for parameters
        for (i, param) in func.params.iter().enumerate() {
            let ty = self.type_to_llvm(&param.ty);
            let alloca = self.builder.build_alloca(ty, &format!("p{}", i)).unwrap();
            let param_val = fn_val.get_nth_param(i as u32).unwrap();
            self.builder.build_store(alloca, param_val).unwrap();
            self.locals.insert(param.id.0, alloca);
            self.local_types.insert(param.id.0, param.ty.clone());
        }

        // Allocate locals for temporaries
        for local in &func.locals {
            let ty = self.type_to_llvm(&local.ty);
            let name = local
                .name
                .as_deref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("t{}", local.id.0));
            let alloca = self.builder.build_alloca(ty, &name).unwrap();
            self.locals.insert(local.id.0, alloca);
            self.local_types.insert(local.id.0, local.ty.clone());
        }

        // Emit statements from entry block
        if let Some(block) = func.blocks.first() {
            for stmt in &block.stmts {
                self.emit_stmt(fn_val, stmt, &func.return_ty);
            }

            // Emit terminator
            match &block.terminator {
                Terminator::Return(Some(op)) => {
                    if let Some(val) = self.emit_operand(op) {
                        self.builder.build_return(Some(&val)).unwrap();
                    } else {
                        self.builder.build_return(None).unwrap();
                    }
                }
                Terminator::Return(None) => match func.return_ty {
                    Type::Unit | Type::Never => {
                        self.builder.build_return(None).unwrap();
                    }
                    _ => {
                        self.builder.build_return(None).unwrap();
                    }
                },
                _ => {
                    // Ensure block is terminated
                    if self
                        .builder
                        .get_insert_block()
                        .unwrap()
                        .get_terminator()
                        .is_none()
                    {
                        match func.return_ty {
                            Type::Unit | Type::Never => {
                                self.builder.build_return(None).unwrap();
                            }
                            _ => {
                                let ty = self.type_to_llvm(&func.return_ty);
                                let zero = ty.const_zero();
                                self.builder.build_return(Some(&zero)).unwrap();
                            }
                        }
                    }
                }
            }
        } else {
            // Empty function
            match func.return_ty {
                Type::Unit | Type::Never => {
                    self.builder.build_return(None).unwrap();
                }
                _ => {
                    let ty = self.type_to_llvm(&func.return_ty);
                    let zero = ty.const_zero();
                    self.builder.build_return(Some(&zero)).unwrap();
                }
            }
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn emit_stmt(&mut self, fn_val: FunctionValue<'ctx>, stmt: &MirStmt, return_ty: &Type) {
        match stmt {
            MirStmt::Assign(place, rvalue) => {
                if let Some(val) = self.emit_rvalue(rvalue) {
                    self.emit_store(place, val);
                }
            }
            MirStmt::CallBuiltin { dest, name, args } => {
                let canonical = normalize_intrinsic_name(name);
                match canonical {
                    "println" => {
                        self.emit_println(args);
                    }
                    "print" => {
                        self.emit_print(args);
                    }
                    "panic" => {
                        self.emit_panic(args);
                    }
                    "assert" | "assert_eq" => {
                        // Skip in native for now
                    }
                    _ => {
                        // Unsupported builtin — store default
                        if let Some(Place::Local(id)) = dest.as_ref() {
                            if let Some(alloca) = self.locals.get(&id.0).copied() {
                                let i32_type = self.context.i32_type();
                                let zero = i32_type.const_int(0, false);
                                self.builder.build_store(alloca, zero).unwrap();
                            }
                        }
                    }
                }
            }
            MirStmt::Call { dest, func, args } => {
                // Find function by index
                let fn_name = self
                    .mir
                    .functions
                    .iter()
                    .find(|f| f.id == *func)
                    .map(|f| f.name.clone());
                if let Some(name) = fn_name {
                    if let Some(callee) = self.fn_values.get(&name).copied() {
                        let arg_vals: Vec<BasicMetadataValueEnum<'ctx>> = args
                            .iter()
                            .filter_map(|a| self.emit_operand(a).map(|v| v.into()))
                            .collect();
                        let result = self.builder.build_call(callee, &arg_vals, "call").unwrap();
                        if let Some(Place::Local(id)) = dest.as_ref() {
                            if let Some(ret_val) = result.try_as_basic_value().basic() {
                                if let Some(alloca) = self.locals.get(&id.0).copied() {
                                    self.builder.build_store(alloca, ret_val).unwrap();
                                }
                            }
                        }
                    }
                }
            }
            MirStmt::IfStmt {
                cond,
                then_body,
                else_body,
            } => {
                let cond_val = self
                    .emit_operand(cond)
                    .unwrap_or_else(|| self.context.bool_type().const_int(0, false).into());
                let cond_i1 = self
                    .builder
                    .build_int_truncate(cond_val.into_int_value(), self.context.bool_type(), "cond")
                    .unwrap();

                let parent = fn_val;
                let then_bb = self.context.append_basic_block(parent, "then");
                let else_bb = self.context.append_basic_block(parent, "else");
                let merge_bb = self.context.append_basic_block(parent, "merge");

                self.builder
                    .build_conditional_branch(cond_i1, then_bb, else_bb)
                    .unwrap();

                // Then
                self.builder.position_at_end(then_bb);
                for s in then_body {
                    self.emit_stmt(fn_val, s, return_ty);
                }
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder.build_unconditional_branch(merge_bb).unwrap();
                }

                // Else
                self.builder.position_at_end(else_bb);
                for s in else_body {
                    self.emit_stmt(fn_val, s, return_ty);
                }
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder.build_unconditional_branch(merge_bb).unwrap();
                }

                self.builder.position_at_end(merge_bb);
            }
            MirStmt::WhileStmt { cond, body } => {
                let parent = fn_val;
                let cond_bb = self.context.append_basic_block(parent, "while.cond");
                let body_bb = self.context.append_basic_block(parent, "while.body");
                let exit_bb = self.context.append_basic_block(parent, "while.exit");

                self.builder.build_unconditional_branch(cond_bb).unwrap();

                // Condition
                self.builder.position_at_end(cond_bb);
                let cond_val = self
                    .emit_operand(cond)
                    .unwrap_or_else(|| self.context.bool_type().const_int(0, false).into());
                let cond_i1 = self
                    .builder
                    .build_int_truncate(
                        cond_val.into_int_value(),
                        self.context.bool_type(),
                        "wcond",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(cond_i1, body_bb, exit_bb)
                    .unwrap();

                // Body
                self.builder.position_at_end(body_bb);
                self.loop_stack.push((cond_bb, exit_bb));
                for s in body {
                    self.emit_stmt(fn_val, s, return_ty);
                }
                self.loop_stack.pop();
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder.build_unconditional_branch(cond_bb).unwrap();
                }

                self.builder.position_at_end(exit_bb);
            }
            MirStmt::Return(Some(op)) => {
                if let Some(val) = self.emit_operand(op) {
                    self.builder.build_return(Some(&val)).unwrap();
                } else {
                    self.builder.build_return(None).unwrap();
                }
            }
            MirStmt::Return(None) => {
                self.builder.build_return(None).unwrap();
            }
            MirStmt::Break => {
                if let Some(&(_, exit_bb)) = self.loop_stack.last() {
                    self.builder.build_unconditional_branch(exit_bb).unwrap();
                }
            }
            MirStmt::Continue => {
                if let Some(&(cond_bb, _)) = self.loop_stack.last() {
                    self.builder.build_unconditional_branch(cond_bb).unwrap();
                }
            }
        }
    }

    fn emit_rvalue(&mut self, rv: &Rvalue) -> Option<BasicValueEnum<'ctx>> {
        match rv {
            Rvalue::Use(op) => self.emit_operand(op),
            Rvalue::BinaryOp(op, lhs, rhs) => {
                let l = self.emit_operand(lhs)?;
                let r = self.emit_operand(rhs)?;
                self.emit_binop(*op, l, r)
            }
            Rvalue::UnaryOp(op, operand) => {
                let v = self.emit_operand(operand)?;
                self.emit_unaryop(*op, v)
            }
            _ => None,
        }
    }

    fn emit_binop(
        &self,
        op: BinOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> Option<BasicValueEnum<'ctx>> {
        if lhs.is_float_value() && rhs.is_float_value() {
            let l = lhs.into_float_value();
            let r = rhs.into_float_value();
            let result = match op {
                BinOp::Add => self.builder.build_float_add(l, r, "fadd").unwrap(),
                BinOp::Sub => self.builder.build_float_sub(l, r, "fsub").unwrap(),
                BinOp::Mul => self.builder.build_float_mul(l, r, "fmul").unwrap(),
                BinOp::Div => self.builder.build_float_div(l, r, "fdiv").unwrap(),
                BinOp::Eq => {
                    let cmp = self
                        .builder
                        .build_float_compare(inkwell::FloatPredicate::OEQ, l, r, "feq")
                        .unwrap();
                    let i32_type = self.context.i32_type();
                    return Some(
                        self.builder
                            .build_int_z_extend(cmp, i32_type, "zext")
                            .unwrap()
                            .into(),
                    );
                }
                BinOp::Ne => {
                    let cmp = self
                        .builder
                        .build_float_compare(inkwell::FloatPredicate::ONE, l, r, "fne")
                        .unwrap();
                    let i32_type = self.context.i32_type();
                    return Some(
                        self.builder
                            .build_int_z_extend(cmp, i32_type, "zext")
                            .unwrap()
                            .into(),
                    );
                }
                BinOp::Lt => {
                    let cmp = self
                        .builder
                        .build_float_compare(inkwell::FloatPredicate::OLT, l, r, "flt")
                        .unwrap();
                    let i32_type = self.context.i32_type();
                    return Some(
                        self.builder
                            .build_int_z_extend(cmp, i32_type, "zext")
                            .unwrap()
                            .into(),
                    );
                }
                BinOp::Le => {
                    let cmp = self
                        .builder
                        .build_float_compare(inkwell::FloatPredicate::OLE, l, r, "fle")
                        .unwrap();
                    let i32_type = self.context.i32_type();
                    return Some(
                        self.builder
                            .build_int_z_extend(cmp, i32_type, "zext")
                            .unwrap()
                            .into(),
                    );
                }
                BinOp::Gt => {
                    let cmp = self
                        .builder
                        .build_float_compare(inkwell::FloatPredicate::OGT, l, r, "fgt")
                        .unwrap();
                    let i32_type = self.context.i32_type();
                    return Some(
                        self.builder
                            .build_int_z_extend(cmp, i32_type, "zext")
                            .unwrap()
                            .into(),
                    );
                }
                BinOp::Ge => {
                    let cmp = self
                        .builder
                        .build_float_compare(inkwell::FloatPredicate::OGE, l, r, "fge")
                        .unwrap();
                    let i32_type = self.context.i32_type();
                    return Some(
                        self.builder
                            .build_int_z_extend(cmp, i32_type, "zext")
                            .unwrap()
                            .into(),
                    );
                }
                _ => return None,
            };
            Some(result.into())
        } else if lhs.is_int_value() && rhs.is_int_value() {
            let l = lhs.into_int_value();
            let r = rhs.into_int_value();
            let result = match op {
                BinOp::Add => self.builder.build_int_add(l, r, "add").unwrap(),
                BinOp::Sub => self.builder.build_int_sub(l, r, "sub").unwrap(),
                BinOp::Mul => self.builder.build_int_mul(l, r, "mul").unwrap(),
                BinOp::Div => self.builder.build_int_signed_div(l, r, "div").unwrap(),
                BinOp::Mod => self.builder.build_int_signed_rem(l, r, "rem").unwrap(),
                BinOp::Eq => {
                    let cmp = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, l, r, "eq")
                        .unwrap();
                    self.builder
                        .build_int_z_extend(cmp, l.get_type(), "zext")
                        .unwrap()
                }
                BinOp::Ne => {
                    let cmp = self
                        .builder
                        .build_int_compare(IntPredicate::NE, l, r, "ne")
                        .unwrap();
                    self.builder
                        .build_int_z_extend(cmp, l.get_type(), "zext")
                        .unwrap()
                }
                BinOp::Lt => {
                    let cmp = self
                        .builder
                        .build_int_compare(IntPredicate::SLT, l, r, "lt")
                        .unwrap();
                    self.builder
                        .build_int_z_extend(cmp, l.get_type(), "zext")
                        .unwrap()
                }
                BinOp::Le => {
                    let cmp = self
                        .builder
                        .build_int_compare(IntPredicate::SLE, l, r, "le")
                        .unwrap();
                    self.builder
                        .build_int_z_extend(cmp, l.get_type(), "zext")
                        .unwrap()
                }
                BinOp::Gt => {
                    let cmp = self
                        .builder
                        .build_int_compare(IntPredicate::SGT, l, r, "gt")
                        .unwrap();
                    self.builder
                        .build_int_z_extend(cmp, l.get_type(), "zext")
                        .unwrap()
                }
                BinOp::Ge => {
                    let cmp = self
                        .builder
                        .build_int_compare(IntPredicate::SGE, l, r, "ge")
                        .unwrap();
                    self.builder
                        .build_int_z_extend(cmp, l.get_type(), "zext")
                        .unwrap()
                }
                BinOp::And => self.builder.build_and(l, r, "and").unwrap(),
                BinOp::Or => self.builder.build_or(l, r, "or").unwrap(),
                BinOp::BitAnd => self.builder.build_and(l, r, "band").unwrap(),
                BinOp::BitOr => self.builder.build_or(l, r, "bor").unwrap(),
                BinOp::BitXor => self.builder.build_xor(l, r, "bxor").unwrap(),
                BinOp::Shl => self.builder.build_left_shift(l, r, "shl").unwrap(),
                BinOp::Shr => self.builder.build_right_shift(l, r, true, "shr").unwrap(),
            };
            Some(result.into())
        } else {
            None
        }
    }

    fn emit_unaryop(&self, op: UnaryOp, val: BasicValueEnum<'ctx>) -> Option<BasicValueEnum<'ctx>> {
        match op {
            UnaryOp::Neg => {
                if val.is_int_value() {
                    Some(
                        self.builder
                            .build_int_neg(val.into_int_value(), "neg")
                            .unwrap()
                            .into(),
                    )
                } else if val.is_float_value() {
                    Some(
                        self.builder
                            .build_float_neg(val.into_float_value(), "fneg")
                            .unwrap()
                            .into(),
                    )
                } else {
                    None
                }
            }
            UnaryOp::Not | UnaryOp::BitNot => {
                if val.is_int_value() {
                    Some(
                        self.builder
                            .build_not(val.into_int_value(), "not")
                            .unwrap()
                            .into(),
                    )
                } else {
                    None
                }
            }
        }
    }

    fn emit_operand(&mut self, op: &Operand) -> Option<BasicValueEnum<'ctx>> {
        match op {
            Operand::ConstI32(v) => Some(self.context.i32_type().const_int(*v as u64, true).into()),
            Operand::ConstI64(v) => Some(self.context.i64_type().const_int(*v as u64, true).into()),
            Operand::ConstF32(v) => Some(self.context.f32_type().const_float(*v as f64).into()),
            Operand::ConstF64(v) => Some(self.context.f64_type().const_float(*v).into()),
            Operand::ConstBool(v) => Some(
                self.context
                    .i32_type()
                    .const_int(if *v { 1 } else { 0 }, false)
                    .into(),
            ),
            Operand::ConstChar(c) => {
                Some(self.context.i32_type().const_int(*c as u64, false).into())
            }
            Operand::ConstString(s) => {
                // Create a global string constant and return a pointer to it
                let str_val = self.builder.build_global_string_ptr(s, "str").unwrap();
                Some(str_val.as_pointer_value().into())
            }
            Operand::Unit => None,
            Operand::Place(Place::Local(id)) => {
                if let Some(alloca) = self.locals.get(&id.0).copied() {
                    let load_ty = self
                        .local_types
                        .get(&id.0)
                        .map(|t| self.type_to_llvm(t))
                        .unwrap_or_else(|| self.context.i32_type().into());
                    let val = self
                        .builder
                        .build_load(load_ty, alloca, &format!("l{}", id.0));
                    val.ok()
                } else {
                    Some(self.context.i32_type().const_int(0, false).into())
                }
            }
            Operand::Place(Place::Field(_, _)) | Operand::Place(Place::Index(_, _)) => {
                Some(self.context.i32_type().const_int(0, false).into())
            }
            Operand::BinOp(op, lhs, rhs) => {
                let l = self.emit_operand(lhs)?;
                let r = self.emit_operand(rhs)?;
                self.emit_binop(*op, l, r)
            }
            Operand::UnaryOp(op, inner) => {
                let v = self.emit_operand(inner)?;
                self.emit_unaryop(*op, v)
            }
            Operand::Call(name, args) => {
                let canonical = normalize_intrinsic_name(name);
                if let Some(callee) = self.fn_values.get(canonical).copied() {
                    let arg_vals: Vec<BasicMetadataValueEnum<'ctx>> = args
                        .iter()
                        .filter_map(|a| self.emit_operand(a).map(|v| v.into()))
                        .collect();
                    let result = self.builder.build_call(callee, &arg_vals, "call").unwrap();
                    result.try_as_basic_value().basic()
                } else {
                    Some(self.context.i32_type().const_int(0, false).into())
                }
            }
            Operand::IfExpr {
                cond,
                then_body: _,
                then_result: _,
                else_body: _,
                else_result: _,
            } => {
                let _cond_val = self.emit_operand(cond)?;
                Some(self.context.i32_type().const_int(0, false).into())
            }
            Operand::StructInit { name: _, fields } => {
                // Allocate struct as a flat i32 array (simplified layout)
                let n_fields = fields.len().max(1);
                let malloc = self.malloc_fn?;
                let size = self
                    .context
                    .i64_type()
                    .const_int((n_fields * 8) as u64, false);
                let ptr = self
                    .builder
                    .build_call(malloc, &[size.into()], "struct_alloc")
                    .unwrap();
                Some(ptr.try_as_basic_value().basic().unwrap_or_else(|| {
                    self.context
                        .ptr_type(AddressSpace::default())
                        .const_null()
                        .into()
                }))
            }
            Operand::FieldAccess {
                object,
                struct_name: _,
                field: _,
            } => {
                // Load the struct pointer, return it as a placeholder
                self.emit_operand(object)
            }
            Operand::EnumInit { tag, payload, .. } => {
                // Allocate enum as [tag:i32, payload0, payload1, ...]
                let n_slots = (payload.len() + 1).max(2);
                let malloc = self.malloc_fn?;
                let size = self
                    .context
                    .i64_type()
                    .const_int((n_slots * 4) as u64, false);
                let ptr = self
                    .builder
                    .build_call(malloc, &[size.into()], "enum_alloc")
                    .unwrap();
                let _ptr_val = ptr.try_as_basic_value().basic()?;
                // Store tag at offset 0
                let tag_ptr = self
                    .builder
                    .build_pointer_cast(
                        _ptr_val.into_pointer_value(),
                        self.context.ptr_type(AddressSpace::default()),
                        "tag_ptr",
                    )
                    .unwrap();
                self.builder
                    .build_store(
                        tag_ptr,
                        self.context.i32_type().const_int(*tag as u64, false),
                    )
                    .unwrap();
                Some(_ptr_val)
            }
            Operand::EnumTag(inner) => {
                // Load tag (i32 at offset 0) from enum pointer
                let ptr = self.emit_operand(inner)?;
                let tag = self
                    .builder
                    .build_load(self.context.i32_type(), ptr.into_pointer_value(), "tag")
                    .ok()?;
                Some(tag)
            }
            Operand::EnumPayload { object, .. } => self.emit_operand(object),
            Operand::ArrayInit { elements } => {
                // Allocate array: [len:i32, elem0, elem1, ...]
                let n = elements.len();
                let malloc = self.malloc_fn?;
                let size = self
                    .context
                    .i64_type()
                    .const_int(((n + 1) * 4) as u64, false);
                let ptr = self
                    .builder
                    .build_call(malloc, &[size.into()], "arr_alloc")
                    .unwrap();
                let _ptr_val = ptr.try_as_basic_value().basic()?;
                // Store length at offset 0
                let base = _ptr_val.into_pointer_value();
                self.builder
                    .build_store(base, self.context.i32_type().const_int(n as u64, false))
                    .unwrap();
                Some(_ptr_val)
            }
            Operand::IndexAccess { object, index } => {
                let _obj = self.emit_operand(object)?;
                let _idx = self.emit_operand(index)?;
                Some(self.context.i32_type().const_int(0, false).into())
            }
            Operand::FnRef(_) => Some(self.context.i32_type().const_int(0, false).into()),
            Operand::CallIndirect { .. } => {
                Some(self.context.i32_type().const_int(0, false).into())
            }
            Operand::LoopExpr { body, result, .. } => {
                for _s in body {
                    // Skip loop body emission for now
                }
                self.emit_operand(result)
            }
            Operand::TryExpr { expr, .. } => self.emit_operand(expr),
        }
    }

    fn emit_store(&mut self, place: &Place, val: BasicValueEnum<'ctx>) {
        match place {
            Place::Local(id) => {
                if let Some(alloca) = self.locals.get(&id.0).copied() {
                    self.builder.build_store(alloca, val).unwrap();
                }
            }
            _ => {} // Field/Index stores not yet supported
        }
    }

    fn emit_println(&mut self, args: &[Operand]) {
        let printf = match self.printf_fn {
            Some(f) => f,
            None => return,
        };

        if args.is_empty() {
            // Just print newline
            let fmt = self.builder.build_global_string_ptr("\n", "nl").unwrap();
            self.builder
                .build_call(printf, &[fmt.as_pointer_value().into()], "")
                .unwrap();
            return;
        }

        let arg = &args[0];
        match arg {
            Operand::ConstString(s) => {
                let msg = format!("{}\n", s);
                let fmt = self.builder.build_global_string_ptr(&msg, "str").unwrap();
                self.builder
                    .build_call(printf, &[fmt.as_pointer_value().into()], "")
                    .unwrap();
            }
            Operand::ConstI32(v) => {
                let fmt = self
                    .builder
                    .build_global_string_ptr("%d\n", "ifmt")
                    .unwrap();
                let val = self.context.i32_type().const_int(*v as u64, true);
                self.builder
                    .build_call(printf, &[fmt.as_pointer_value().into(), val.into()], "")
                    .unwrap();
            }
            Operand::ConstBool(v) => {
                let s = if *v { "true\n" } else { "false\n" };
                let fmt = self.builder.build_global_string_ptr(s, "bfmt").unwrap();
                self.builder
                    .build_call(printf, &[fmt.as_pointer_value().into()], "")
                    .unwrap();
            }
            Operand::Place(Place::Local(id)) => {
                if let Some(alloca) = self.locals.get(&id.0).copied() {
                    let local_ty = self.local_types.get(&id.0).cloned().unwrap_or(Type::I32);
                    let load_ty = self.type_to_llvm(&local_ty);
                    let val = self.builder.build_load(load_ty, alloca, "pv").unwrap();
                    let (fmt_str, fmt_val): (&str, BasicMetadataValueEnum) = match &local_ty {
                        Type::String => ("%s\n", val.into()),
                        Type::F64 => ("%g\n", val.into()),
                        Type::I64 => ("%lld\n", val.into()),
                        Type::Bool => ("%d\n", val.into()),
                        _ => ("%d\n", val.into()),
                    };
                    let fmt = self
                        .builder
                        .build_global_string_ptr(fmt_str, "lfmt")
                        .unwrap();
                    self.builder
                        .build_call(printf, &[fmt.as_pointer_value().into(), fmt_val], "")
                        .unwrap();
                }
            }
            Operand::Call(name, inner_args) => {
                let canonical = normalize_intrinsic_name(name);
                match canonical {
                    "i32_to_string" | "i64_to_string" => {
                        if let Some(inner) = inner_args.first() {
                            if let Some(val) = self.emit_operand(inner) {
                                let fmt = self
                                    .builder
                                    .build_global_string_ptr("%d\n", "ifmt")
                                    .unwrap();
                                self.builder
                                    .build_call(
                                        printf,
                                        &[fmt.as_pointer_value().into(), val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                        }
                    }
                    "f64_to_string" => {
                        if let Some(inner) = inner_args.first() {
                            if let Some(val) = self.emit_operand(inner) {
                                let fmt = self
                                    .builder
                                    .build_global_string_ptr("%g\n", "ffmt")
                                    .unwrap();
                                self.builder
                                    .build_call(
                                        printf,
                                        &[fmt.as_pointer_value().into(), val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                        }
                    }
                    "bool_to_string" => {
                        if let Some(inner) = inner_args.first() {
                            if let Some(val) = self.emit_operand(inner) {
                                // Print "true" or "false"
                                let fmt = self
                                    .builder
                                    .build_global_string_ptr("%d\n", "bfmt")
                                    .unwrap();
                                self.builder
                                    .build_call(
                                        printf,
                                        &[fmt.as_pointer_value().into(), val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                        }
                    }
                    _ => {
                        // Generic: try calling function and printing result
                        let fmt = self
                            .builder
                            .build_global_string_ptr("%d\n", "gfmt")
                            .unwrap();
                        if let Some(val) =
                            self.emit_operand(&Operand::Call(name.clone(), inner_args.clone()))
                        {
                            self.builder
                                .build_call(
                                    printf,
                                    &[fmt.as_pointer_value().into(), val.into()],
                                    "",
                                )
                                .unwrap();
                        }
                    }
                }
            }
            _ => {
                // Default: try to evaluate and print as i32
                if let Some(val) = self.emit_operand(arg) {
                    let fmt = self
                        .builder
                        .build_global_string_ptr("%d\n", "dfmt")
                        .unwrap();
                    self.builder
                        .build_call(printf, &[fmt.as_pointer_value().into(), val.into()], "")
                        .unwrap();
                }
            }
        }
    }

    fn emit_print(&mut self, args: &[Operand]) {
        let printf = match self.printf_fn {
            Some(f) => f,
            None => return,
        };
        if let Some(Operand::ConstString(s)) = args.first() {
            let fmt = self.builder.build_global_string_ptr(s, "pstr").unwrap();
            self.builder
                .build_call(printf, &[fmt.as_pointer_value().into()], "")
                .unwrap();
        }
    }

    fn emit_panic(&mut self, args: &[Operand]) {
        let printf = match self.printf_fn {
            Some(f) => f,
            None => return,
        };

        // Print panic message
        if let Some(Operand::ConstString(s)) = args.first() {
            let msg = format!("panic: {}\n", s);
            let fmt = self.builder.build_global_string_ptr(&msg, "panic").unwrap();
            self.builder
                .build_call(printf, &[fmt.as_pointer_value().into()], "")
                .unwrap();
        } else {
            let fmt = self
                .builder
                .build_global_string_ptr("panic: unexpected error\n", "panic")
                .unwrap();
            self.builder
                .build_call(printf, &[fmt.as_pointer_value().into()], "")
                .unwrap();
        }

        // Call exit(1)
        let exit_fn = self.declare_exit();
        let one = self.context.i32_type().const_int(1, false);
        self.builder.build_call(exit_fn, &[one.into()], "").unwrap();
        self.builder.build_unreachable().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_mir::mir::*;

    fn make_hello_mir() -> MirModule {
        let mut module = MirModule::new();
        let main_fn = MirFunction {
            id: FnId(0),
            name: "main".to_string(),
            params: vec![],
            return_ty: Type::Unit,
            locals: vec![],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![MirStmt::CallBuiltin {
                    dest: None,
                    name: "__intrinsic_println".to_string(),
                    args: vec![Operand::ConstString("Hello from LLVM!".to_string())],
                }],
                terminator: Terminator::Return(None),
            }],
            entry: BlockId(0),
            struct_typed_locals: HashMap::new(),
        };
        module.functions.push(main_fn);
        module.entry_fn = Some(FnId(0));
        module
    }

    #[test]
    fn test_emit_hello_llvm_ir() {
        let mir = make_hello_mir();
        let mut sink = DiagnosticSink::new();
        let ir = emit_llvm_ir(&mir, &mut sink);
        assert!(ir.contains("define"), "should produce LLVM IR");
        assert!(
            ir.contains("Hello from LLVM!"),
            "should contain string literal"
        );
        assert!(ir.contains("printf"), "should call printf");
    }

    #[test]
    fn test_emit_arithmetic_ir() {
        let mut module = MirModule::new();
        let func = MirFunction {
            id: FnId(0),
            name: "add".to_string(),
            params: vec![
                MirLocal {
                    id: LocalId(0),
                    name: Some("a".to_string()),
                    ty: Type::I32,
                },
                MirLocal {
                    id: LocalId(1),
                    name: Some("b".to_string()),
                    ty: Type::I32,
                },
            ],
            return_ty: Type::I32,
            locals: vec![MirLocal {
                id: LocalId(2),
                name: Some("result".to_string()),
                ty: Type::I32,
            }],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![MirStmt::Assign(
                    Place::Local(LocalId(2)),
                    Rvalue::BinaryOp(
                        BinOp::Add,
                        Operand::Place(Place::Local(LocalId(0))),
                        Operand::Place(Place::Local(LocalId(1))),
                    ),
                )],
                terminator: Terminator::Return(Some(Operand::Place(Place::Local(LocalId(2))))),
            }],
            entry: BlockId(0),
            struct_typed_locals: HashMap::new(),
        };
        module.functions.push(func);

        let mut sink = DiagnosticSink::new();
        let ir = emit_llvm_ir(&module, &mut sink);
        assert!(ir.contains("add"), "should contain add function");
    }
}
