use crate::mir::{
    BinOp, BlockId, LocalId, MirFunction, MirModule, MirStmt, Operand, Place, Rvalue, Terminator, UnaryOp,
    push_optimization_trace,
};
use crate::validate::validate_module;

const MAX_OPT_ROUNDS: usize = 3;
const INLINE_SMALL_LEAF_BUDGET: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationPass {
    ConstFold,
    BranchFold,
    CfgSimplify,
    CopyProp,
    ConstProp,
    DeadLocalElim,
    DeadBlockElim,
    UnreachableCleanup,
    InlineSmallLeaf,
    StringConcatOpt,
    AggregateSimplify,
    AlgebraicSimplify,
    StrengthReduction,
    Cse,
}

impl OptimizationPass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConstFold => "const_fold",
            Self::BranchFold => "branch_fold",
            Self::CfgSimplify => "cfg_simplify",
            Self::CopyProp => "copy_prop",
            Self::ConstProp => "const_prop",
            Self::DeadLocalElim => "dead_local_elim",
            Self::DeadBlockElim => "dead_block_elim",
            Self::UnreachableCleanup => "unreachable_cleanup",
            Self::InlineSmallLeaf => "inline_small_leaf",
            Self::StringConcatOpt => "string_concat_opt",
            Self::AggregateSimplify => "aggregate_simplify",
            Self::AlgebraicSimplify => "algebraic_simplify",
            Self::StrengthReduction => "strength_reduction",
            Self::Cse => "cse",
        }
    }
}

pub const DEFAULT_PASS_ORDER: &[OptimizationPass] = &[
    OptimizationPass::ConstFold,
    OptimizationPass::BranchFold,
    OptimizationPass::CfgSimplify,
    OptimizationPass::CopyProp,
    OptimizationPass::ConstProp,
    OptimizationPass::DeadLocalElim,
    OptimizationPass::DeadBlockElim,
    OptimizationPass::UnreachableCleanup,
    OptimizationPass::InlineSmallLeaf,
    OptimizationPass::StringConcatOpt,
    OptimizationPass::AggregateSimplify,
    OptimizationPass::AlgebraicSimplify,
    OptimizationPass::StrengthReduction,
    OptimizationPass::Cse,
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptimizationSummary {
    pub rounds: u32,
    pub const_folded: usize,
    pub branch_folded: usize,
    pub cfg_simplified: usize,
    pub copy_propagated: usize,
    pub const_propagated: usize,
    pub dead_locals_removed: usize,
    pub dead_blocks_removed: usize,
    pub unreachable_cleaned: usize,
    pub inline_small_leaf: usize,
    pub string_concat_normalized: usize,
    pub aggregate_simplified: usize,
    pub algebraic_simplified: usize,
    pub strength_reduced: usize,
    pub cse_eliminated: usize,
}

impl OptimizationSummary {
    fn changed(&self) -> bool {
        self.const_folded > 0
            || self.branch_folded > 0
            || self.cfg_simplified > 0
            || self.copy_propagated > 0
            || self.const_propagated > 0
            || self.dead_locals_removed > 0
            || self.dead_blocks_removed > 0
            || self.unreachable_cleaned > 0
            || self.inline_small_leaf > 0
            || self.string_concat_normalized > 0
            || self.aggregate_simplified > 0
            || self.algebraic_simplified > 0
            || self.strength_reduced > 0
            || self.cse_eliminated > 0
    }

    fn absorb(&mut self, other: OptimizationSummary) {
        self.const_folded += other.const_folded;
        self.branch_folded += other.branch_folded;
        self.cfg_simplified += other.cfg_simplified;
        self.copy_propagated += other.copy_propagated;
        self.const_propagated += other.const_propagated;
        self.dead_locals_removed += other.dead_locals_removed;
        self.dead_blocks_removed += other.dead_blocks_removed;
        self.unreachable_cleaned += other.unreachable_cleaned;
        self.inline_small_leaf += other.inline_small_leaf;
        self.string_concat_normalized += other.string_concat_normalized;
        self.aggregate_simplified += other.aggregate_simplified;
        self.algebraic_simplified += other.algebraic_simplified;
        self.strength_reduced += other.strength_reduced;
        self.cse_eliminated += other.cse_eliminated;
    }
}

pub fn default_pass_order() -> &'static [OptimizationPass] {
    DEFAULT_PASS_ORDER
}

pub fn find_pass(name: &str) -> Option<OptimizationPass> {
    DEFAULT_PASS_ORDER
        .iter()
        .copied()
        .find(|pass| pass.as_str() == name)
}

pub fn optimization_pass_catalog() -> Vec<&'static str> {
    DEFAULT_PASS_ORDER
        .iter()
        .map(|pass| pass.as_str())
        .collect()
}

pub fn optimization_trace_snapshot(module: &MirModule) -> String {
    crate::mir::optimization_trace(module).join(",")
}

pub fn pass_pipeline_snapshot() -> String {
    format!(
        "rounds={} passes={}",
        MAX_OPT_ROUNDS,
        optimization_pass_catalog().join(",")
    )
}

pub fn run_single_pass(
    module: &mut MirModule,
    pass: OptimizationPass,
) -> Result<OptimizationSummary, String> {
    validate_module(module)
        .map_err(|errors| format!("MIR validation failed before {}: {errors:?}", pass.as_str()))?;
    let mut summary = OptimizationSummary::default();
    for function in &mut module.functions {
        summary.absorb(run_pass(function, pass));
    }
    push_optimization_trace(module, pass.as_str());
    validate_module(module)
        .map_err(|errors| format!("MIR validation failed after {}: {errors:?}", pass.as_str()))?;
    Ok(summary)
}

pub fn optimize_module(module: &mut MirModule) -> Result<OptimizationSummary, String> {
    optimize_module_with_passes(module, DEFAULT_PASS_ORDER, MAX_OPT_ROUNDS)
}

pub fn optimize_module_named(
    module: &mut MirModule,
    names: &[&str],
) -> Result<OptimizationSummary, String> {
    let passes = names
        .iter()
        .map(|name| find_pass(name).ok_or_else(|| format!("unknown optimization pass {}", name)))
        .collect::<Result<Vec<_>, _>>()?;
    optimize_module_with_passes(module, &passes, 1)
}

pub fn optimize_module_named_until(
    module: &mut MirModule,
    name: &str,
) -> Result<OptimizationSummary, String> {
    let pass = find_pass(name).ok_or_else(|| format!("unknown optimization pass {}", name))?;
    let index = DEFAULT_PASS_ORDER
        .iter()
        .position(|candidate| *candidate == pass)
        .ok_or_else(|| format!("unknown optimization pass {}", name))?;
    optimize_module_with_passes(module, &DEFAULT_PASS_ORDER[..=index], MAX_OPT_ROUNDS)
}

pub fn optimize_module_named_only(
    module: &mut MirModule,
    name: &str,
) -> Result<OptimizationSummary, String> {
    let pass = find_pass(name).ok_or_else(|| format!("unknown optimization pass {}", name))?;
    optimize_module_with_passes(module, &[pass], 1)
}

pub fn optimize_module_named_without(
    module: &mut MirModule,
    name: &str,
) -> Result<OptimizationSummary, String> {
    let excluded = find_pass(name).ok_or_else(|| format!("unknown optimization pass {}", name))?;
    let passes = DEFAULT_PASS_ORDER
        .iter()
        .copied()
        .filter(|pass| *pass != excluded)
        .collect::<Vec<_>>();
    optimize_module_with_passes(module, &passes, MAX_OPT_ROUNDS)
}

pub fn optimize_module_none(module: &mut MirModule) -> Result<OptimizationSummary, String> {
    validate_module(module)
        .map_err(|errors| format!("MIR validation failed without optimization: {errors:?}"))?;
    Ok(OptimizationSummary::default())
}

fn optimize_module_with_passes(
    module: &mut MirModule,
    passes: &[OptimizationPass],
    max_rounds: usize,
) -> Result<OptimizationSummary, String> {
    validate_module(module)
        .map_err(|errors| format!("MIR validation failed before optimization: {errors:?}"))?;

    let dump = crate::mir::dump_phases_requested();
    let should_dump = dump
        .as_deref()
        .is_some_and(|d| d == "optimized-mir" || d == "all");

    if should_dump {
        crate::mir::dump_mir_phase(module, "pre-opt");
    }

    let mut total = OptimizationSummary::default();
    module.stats.optimization_trace.clear();

    for _round in 0..max_rounds {
        let mut round_summary = OptimizationSummary::default();
        for pass in passes {
            let pass_summary = run_single_pass(module, *pass)?;
            if should_dump && pass_summary.changed() {
                crate::mir::dump_mir_phase(module, &format!("after-{}", pass.as_str()));
            }
            round_summary.absorb(pass_summary);
        }
        round_summary.rounds = 1;
        total.rounds += 1;
        total.absorb(round_summary.clone());
        if !round_summary.changed() {
            break;
        }
    }

    module.stats.optimization_rounds = total.rounds;
    validate_module(module)
        .map_err(|errors| format!("MIR validation failed after optimization: {errors:?}"))?;

    if should_dump {
        crate::mir::dump_mir_phase(module, "post-opt");
    }

    Ok(total)
}

fn run_pass(function: &mut MirFunction, pass: OptimizationPass) -> OptimizationSummary {
    match pass {
        OptimizationPass::ConstFold => const_fold(function),
        OptimizationPass::BranchFold => branch_fold(function),
        OptimizationPass::CfgSimplify => cfg_simplify(function),
        OptimizationPass::CopyProp => copy_prop(function),
        OptimizationPass::ConstProp => const_prop(function),
        OptimizationPass::DeadLocalElim => dead_local_elim(function),
        OptimizationPass::DeadBlockElim => dead_block_elim(function),
        OptimizationPass::UnreachableCleanup => unreachable_cleanup(function),
        OptimizationPass::InlineSmallLeaf => inline_small_leaf(function),
        OptimizationPass::StringConcatOpt => string_concat_opt(function),
        OptimizationPass::AggregateSimplify => aggregate_simplify(function),
        OptimizationPass::AlgebraicSimplify => algebraic_simplify(function),
        OptimizationPass::StrengthReduction => strength_reduction(function),
        OptimizationPass::Cse => cse(function),
    }
}

fn const_fold(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        for stmt in &mut block.stmts {
            if let MirStmt::Assign(_, Rvalue::BinaryOp(op, lhs, rhs)) = stmt {
                if let Some(folded) = fold_binary(*op, lhs, rhs) {
                    let place = extract_assign_target(stmt);
                    *stmt = MirStmt::Assign(place, Rvalue::Use(folded));
                    summary.const_folded += 1;
                }
            }
        }
    }
    summary
}

fn branch_fold(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        if let Terminator::If {
            cond: Operand::ConstBool(value),
            then_block,
            else_block,
        } = &block.terminator
        {
            block.terminator = Terminator::Goto(if *value { *then_block } else { *else_block });
            summary.branch_folded += 1;
        }
    }
    summary
}

fn cfg_simplify(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        if matches!(block.terminator, Terminator::Goto(_)) && block.stmts.is_empty() {
            summary.cfg_simplified += 1;
        }
    }
    summary
}

fn copy_prop(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        let mut replacements = std::collections::HashMap::new();
        for stmt in &mut block.stmts {
            if let MirStmt::Assign(
                Place::Local(dest),
                Rvalue::Use(Operand::Place(Place::Local(src))),
            ) = stmt
            {
                replacements.insert(dest.0, Operand::Place(Place::Local(*src)));
                summary.copy_propagated += 1;
                continue;
            }
            rewrite_stmt_with_replacements(stmt, &replacements);
        }
        rewrite_terminator_with_replacements(&mut block.terminator, &replacements);
    }
    summary
}

fn const_prop(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        let mut constants = std::collections::HashMap::new();
        for stmt in &mut block.stmts {
            if let MirStmt::Assign(Place::Local(dest), Rvalue::Use(value)) = stmt {
                if matches!(
                    value,
                    Operand::ConstI32(_) | Operand::ConstI64(_) | Operand::ConstBool(_)
                ) {
                    constants.insert(dest.0, value.clone());
                }
            }
            if rewrite_stmt_with_replacements(stmt, &constants) {
                summary.const_propagated += 1;
            }
        }
        if rewrite_terminator_with_replacements(&mut block.terminator, &constants) {
            summary.const_propagated += 1;
        }
    }
    summary
}

fn dead_local_elim(function: &mut MirFunction) -> OptimizationSummary {
    let mut used = std::collections::HashSet::new();
    for block in &function.blocks {
        for stmt in &block.stmts {
            collect_stmt_locals(stmt, &mut used);
        }
        collect_terminator_locals(&block.terminator, &mut used);
    }

    let before = function.locals.len();
    function.locals.retain(|local| used.contains(&local.id.0));
    OptimizationSummary {
        dead_locals_removed: before.saturating_sub(function.locals.len()),
        ..OptimizationSummary::default()
    }
}

fn dead_block_elim(function: &mut MirFunction) -> OptimizationSummary {
    let reachable = reachable_blocks(function);
    let before = function.blocks.len();
    function
        .blocks
        .retain(|block| reachable.contains(&block.id));
    OptimizationSummary {
        dead_blocks_removed: before.saturating_sub(function.blocks.len()),
        ..OptimizationSummary::default()
    }
}

fn unreachable_cleanup(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        if let Some(index) = block
            .stmts
            .iter()
            .position(|stmt| matches!(stmt, MirStmt::Return(_)))
        {
            if index + 1 < block.stmts.len() {
                block.stmts.truncate(index + 1);
                summary.unreachable_cleaned += 1;
            }
        }
    }
    summary
}

fn inline_small_leaf(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    if function
        .blocks
        .iter()
        .map(|block| block.stmts.len())
        .sum::<usize>()
        > INLINE_SMALL_LEAF_BUDGET
    {
        return summary;
    }
    for block in &mut function.blocks {
        for stmt in &mut block.stmts {
            if let MirStmt::CallBuiltin { name, args, .. } = stmt {
                if name == "identity" && args.len() == 1 {
                    *stmt = MirStmt::Assign(
                        Place::Local(crate::mir::LocalId(0)),
                        Rvalue::Use(args[0].clone()),
                    );
                    summary.inline_small_leaf += 1;
                }
            }
        }
    }
    summary
}

fn string_concat_opt(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        for stmt in &mut block.stmts {
            if let MirStmt::Assign(_, Rvalue::Use(Operand::Call(name, args))) = stmt {
                if name == "concat" && args.len() == 2 {
                    summary.string_concat_normalized += 1;
                }
            }
        }
    }
    summary
}

fn aggregate_simplify(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        for stmt in &mut block.stmts {
            if let MirStmt::Assign(place, Rvalue::Aggregate(_, operands)) = stmt {
                if operands.len() == 1 {
                    let place = place.clone();
                    let operand = operands[0].clone();
                    *stmt = MirStmt::Assign(place, Rvalue::Use(operand));
                    summary.aggregate_simplified += 1;
                }
            }
        }
    }
    summary
}

/// Algebraic simplification: identity/absorbing element elimination.
///
/// Rules applied:
///   x + 0 → x,  0 + x → x
///   x - 0 → x
///   x * 1 → x,  1 * x → x
///   x * 0 → 0,  0 * x → 0
///   x / 1 → x
///   x & 0 → 0,  0 & x → 0
///   x | 0 → x,  0 | x → x
///   x ^ 0 → x,  0 ^ x → x
///   x && true → x, true && x → x
///   x && false → false
///   x || false → x, false || x → x
///   x || true → true
///   !!x → x (double negation)
///   --x → x (double negation for integers)
fn algebraic_simplify(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        for stmt in &mut block.stmts {
            if let MirStmt::Assign(place, rvalue) = stmt {
                if let Some(simplified) = try_algebraic_simplify_rvalue(rvalue) {
                    *stmt = MirStmt::Assign(place.clone(), Rvalue::Use(simplified));
                    summary.algebraic_simplified += 1;
                }
            }
        }
    }
    summary
}

fn try_algebraic_simplify_rvalue(rvalue: &Rvalue) -> Option<Operand> {
    match rvalue {
        Rvalue::BinaryOp(op, lhs, rhs) => try_algebraic_simplify_binop(*op, lhs, rhs),
        Rvalue::UnaryOp(UnaryOp::Not, Operand::UnaryOp(UnaryOp::Not, inner)) => {
            Some((**inner).clone())
        }
        Rvalue::UnaryOp(UnaryOp::Neg, Operand::UnaryOp(UnaryOp::Neg, inner)) => {
            Some((**inner).clone())
        }
        _ => None,
    }
}

fn try_algebraic_simplify_binop(op: BinOp, lhs: &Operand, rhs: &Operand) -> Option<Operand> {
    let is_zero_i32 = |o: &Operand| matches!(o, Operand::ConstI32(0));
    let is_zero_i64 = |o: &Operand| matches!(o, Operand::ConstI64(0));
    let is_one_i32 = |o: &Operand| matches!(o, Operand::ConstI32(1));
    let is_one_i64 = |o: &Operand| matches!(o, Operand::ConstI64(1));

    match op {
        // x + 0 → x, 0 + x → x
        BinOp::Add => {
            if is_zero_i32(rhs) || is_zero_i64(rhs) {
                Some(lhs.clone())
            } else if is_zero_i32(lhs) || is_zero_i64(lhs) {
                Some(rhs.clone())
            } else {
                None
            }
        }
        // x - 0 → x
        BinOp::Sub => {
            if is_zero_i32(rhs) || is_zero_i64(rhs) {
                Some(lhs.clone())
            } else {
                None
            }
        }
        // x * 1 → x, 1 * x → x, x * 0 → 0, 0 * x → 0
        BinOp::Mul => {
            if is_one_i32(rhs) || is_one_i64(rhs) {
                Some(lhs.clone())
            } else if is_one_i32(lhs) || is_one_i64(lhs) {
                Some(rhs.clone())
            } else if is_zero_i32(rhs) || is_zero_i32(lhs) {
                Some(Operand::ConstI32(0))
            } else if is_zero_i64(rhs) || is_zero_i64(lhs) {
                Some(Operand::ConstI64(0))
            } else {
                None
            }
        }
        // x / 1 → x
        BinOp::Div => {
            if is_one_i32(rhs) || is_one_i64(rhs) {
                Some(lhs.clone())
            } else {
                None
            }
        }
        // x & 0 → 0, 0 & x → 0, x | 0 → x, 0 | x → x
        BinOp::BitAnd => {
            if is_zero_i32(rhs) || is_zero_i32(lhs) {
                Some(Operand::ConstI32(0))
            } else if is_zero_i64(rhs) || is_zero_i64(lhs) {
                Some(Operand::ConstI64(0))
            } else {
                None
            }
        }
        BinOp::BitOr | BinOp::BitXor => {
            if is_zero_i32(rhs) || is_zero_i64(rhs) {
                Some(lhs.clone())
            } else if is_zero_i32(lhs) || is_zero_i64(lhs) {
                Some(rhs.clone())
            } else {
                None
            }
        }
        // x && true → x, true && x → x, x && false → false
        BinOp::And => {
            if matches!(rhs, Operand::ConstBool(true)) {
                Some(lhs.clone())
            } else if matches!(lhs, Operand::ConstBool(true)) {
                Some(rhs.clone())
            } else if matches!(rhs, Operand::ConstBool(false))
                || matches!(lhs, Operand::ConstBool(false))
            {
                Some(Operand::ConstBool(false))
            } else {
                None
            }
        }
        // x || false → x, false || x → x, x || true → true
        BinOp::Or => {
            if matches!(rhs, Operand::ConstBool(false)) {
                Some(lhs.clone())
            } else if matches!(lhs, Operand::ConstBool(false)) {
                Some(rhs.clone())
            } else if matches!(rhs, Operand::ConstBool(true))
                || matches!(lhs, Operand::ConstBool(true))
            {
                Some(Operand::ConstBool(true))
            } else {
                None
            }
        }
        // x << 0 → x, x >> 0 → x
        BinOp::Shl | BinOp::Shr => {
            if is_zero_i32(rhs) || is_zero_i64(rhs) {
                Some(lhs.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Strength reduction: replace expensive operations with cheaper ones.
///
/// Rules:
///   x * 2^n → x << n  (for constant power-of-2 multiplier)
///   x / 2^n → x >> n  (for constant power-of-2 divisor, unsigned semantics)
fn strength_reduction(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        for stmt in &mut block.stmts {
            if let MirStmt::Assign(place, Rvalue::BinaryOp(op, lhs, rhs)) = stmt {
                if let Some(replacement) = try_strength_reduce(*op, lhs, rhs) {
                    *stmt = MirStmt::Assign(place.clone(), replacement);
                    summary.strength_reduced += 1;
                }
            }
        }
    }
    summary
}

fn try_strength_reduce(op: BinOp, lhs: &Operand, rhs: &Operand) -> Option<Rvalue> {
    match op {
        BinOp::Mul => {
            // x * 2^n → x << n
            if let Some(shift) = power_of_two_i32(rhs) {
                Some(Rvalue::BinaryOp(
                    BinOp::Shl,
                    lhs.clone(),
                    Operand::ConstI32(shift),
                ))
            } else if let Some(shift) = power_of_two_i32(lhs) {
                Some(Rvalue::BinaryOp(
                    BinOp::Shl,
                    rhs.clone(),
                    Operand::ConstI32(shift),
                ))
            } else {
                None
            }
        }
        BinOp::Div => {
            // x / 2^n → x >> n (unsigned)
            if let Some(shift) = power_of_two_i32(rhs) {
                Some(Rvalue::BinaryOp(
                    BinOp::Shr,
                    lhs.clone(),
                    Operand::ConstI32(shift),
                ))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn power_of_two_i32(op: &Operand) -> Option<i32> {
    match op {
        Operand::ConstI32(n) if *n > 1 && n.count_ones() == 1 => Some(n.trailing_zeros() as i32),
        _ => None,
    }
}

fn fold_binary(op: BinOp, lhs: &Operand, rhs: &Operand) -> Option<Operand> {
    match (op, lhs, rhs) {
        (BinOp::Add, Operand::ConstI32(a), Operand::ConstI32(b)) => Some(Operand::ConstI32(a + b)),
        (BinOp::Sub, Operand::ConstI32(a), Operand::ConstI32(b)) => Some(Operand::ConstI32(a - b)),
        (BinOp::Mul, Operand::ConstI32(a), Operand::ConstI32(b)) => Some(Operand::ConstI32(a * b)),
        (BinOp::Eq, Operand::ConstI32(a), Operand::ConstI32(b)) => Some(Operand::ConstBool(a == b)),
        (BinOp::Eq, Operand::ConstBool(a), Operand::ConstBool(b)) => {
            Some(Operand::ConstBool(a == b))
        }
        _ => None,
    }
}

fn extract_assign_target(stmt: &MirStmt) -> Place {
    match stmt {
        MirStmt::Assign(place, _) => place.clone(),
        _ => Place::Local(crate::mir::LocalId(0)),
    }
}

fn rewrite_stmt_with_replacements(
    stmt: &mut MirStmt,
    replacements: &std::collections::HashMap<u32, Operand>,
) -> bool {
    let mut changed = false;
    match stmt {
        MirStmt::Assign(_, rvalue) => changed |= rewrite_rvalue(rvalue, replacements),
        MirStmt::Call { args, .. } | MirStmt::CallBuiltin { args, .. } => {
            for arg in args {
                changed |= rewrite_operand(arg, replacements);
            }
        }
        MirStmt::IfStmt {
            cond,
            then_body,
            else_body,
        } => {
            changed |= rewrite_operand(cond, replacements);
            for stmt in then_body {
                changed |= rewrite_stmt_with_replacements(stmt, replacements);
            }
            for stmt in else_body {
                changed |= rewrite_stmt_with_replacements(stmt, replacements);
            }
        }
        MirStmt::WhileStmt { cond, body } => {
            changed |= rewrite_operand(cond, replacements);
            for stmt in body {
                changed |= rewrite_stmt_with_replacements(stmt, replacements);
            }
        }
        MirStmt::Break | MirStmt::Continue => {}
        MirStmt::Return(value) => {
            if let Some(value) = value {
                changed |= rewrite_operand(value, replacements);
            }
        }
    }
    changed
}

fn rewrite_terminator_with_replacements(
    terminator: &mut Terminator,
    replacements: &std::collections::HashMap<u32, Operand>,
) -> bool {
    match terminator {
        Terminator::If { cond, .. } => rewrite_operand(cond, replacements),
        Terminator::Switch { scrutinee, .. } => rewrite_operand(scrutinee, replacements),
        Terminator::Return(value) => value
            .as_mut()
            .is_some_and(|value| rewrite_operand(value, replacements)),
        Terminator::Goto(_) | Terminator::Unreachable => false,
    }
}

fn rewrite_rvalue(
    rvalue: &mut Rvalue,
    replacements: &std::collections::HashMap<u32, Operand>,
) -> bool {
    match rvalue {
        Rvalue::Use(operand) => rewrite_operand(operand, replacements),
        Rvalue::BinaryOp(_, lhs, rhs) => {
            rewrite_operand(lhs, replacements) | rewrite_operand(rhs, replacements)
        }
        Rvalue::UnaryOp(_, operand) => rewrite_operand(operand, replacements),
        Rvalue::Aggregate(_, operands) => operands.iter_mut().fold(false, |changed, operand| {
            rewrite_operand(operand, replacements) || changed
        }),
        Rvalue::Ref(place) => rewrite_place(place, replacements),
    }
}

fn rewrite_place(
    place: &mut Place,
    replacements: &std::collections::HashMap<u32, Operand>,
) -> bool {
    match place {
        Place::Local(_) => false,
        Place::Field(place, _) => rewrite_place(place, replacements),
        Place::Index(place, index) => {
            rewrite_place(place, replacements) | rewrite_operand(index, replacements)
        }
    }
}

fn rewrite_operand(
    operand: &mut Operand,
    replacements: &std::collections::HashMap<u32, Operand>,
) -> bool {
    match operand {
        Operand::Place(Place::Local(local)) => {
            if let Some(replacement) = replacements.get(&local.0) {
                *operand = replacement.clone();
                return true;
            }
            false
        }
        Operand::BinOp(_, lhs, rhs) => {
            rewrite_operand(lhs, replacements) | rewrite_operand(rhs, replacements)
        }
        Operand::UnaryOp(_, operand)
        | Operand::EnumTag(operand)
        | Operand::FieldAccess {
            object: operand, ..
        } => rewrite_operand(operand, replacements),
        Operand::Call(_, args) | Operand::ArrayInit { elements: args } => {
            args.iter_mut().fold(false, |changed, operand| {
                rewrite_operand(operand, replacements) || changed
            })
        }
        Operand::IfExpr {
            cond,
            then_body,
            then_result,
            else_body,
            else_result,
        } => {
            let mut changed = rewrite_operand(cond, replacements);
            for stmt in then_body {
                changed |= rewrite_stmt_with_replacements(stmt, replacements);
            }
            if let Some(result) = then_result {
                changed |= rewrite_operand(result, replacements);
            }
            for stmt in else_body {
                changed |= rewrite_stmt_with_replacements(stmt, replacements);
            }
            if let Some(result) = else_result {
                changed |= rewrite_operand(result, replacements);
            }
            changed
        }
        Operand::StructInit { fields, .. } => {
            fields.iter_mut().fold(false, |changed, (_, operand)| {
                rewrite_operand(operand, replacements) || changed
            })
        }
        Operand::EnumInit { payload, .. } => payload.iter_mut().fold(false, |changed, operand| {
            rewrite_operand(operand, replacements) || changed
        }),
        Operand::EnumPayload { object, .. } => rewrite_operand(object, replacements),
        Operand::LoopExpr { init, body, result } => {
            let mut changed = rewrite_operand(init, replacements);
            for stmt in body {
                changed |= rewrite_stmt_with_replacements(stmt, replacements);
            }
            changed |= rewrite_operand(result, replacements);
            changed
        }
        Operand::TryExpr { expr, .. } => rewrite_operand(expr, replacements),
        Operand::CallIndirect { callee, args } => {
            let mut changed = rewrite_operand(callee, replacements);
            for arg in args {
                changed |= rewrite_operand(arg, replacements);
            }
            changed
        }
        Operand::IndexAccess { object, index } => {
            rewrite_operand(object, replacements) | rewrite_operand(index, replacements)
        }
        Operand::Place(Place::Field(place, _)) => rewrite_place(place, replacements),
        Operand::Place(Place::Index(place, index)) => {
            rewrite_place(place, replacements) | rewrite_operand(index, replacements)
        }
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
        | Operand::FnRef(_) => false,
    }
}

fn collect_stmt_locals(stmt: &MirStmt, used: &mut std::collections::HashSet<u32>) {
    match stmt {
        MirStmt::Assign(place, rvalue) => {
            collect_place_locals(place, used);
            collect_rvalue_locals(rvalue, used);
        }
        MirStmt::Call { args, .. } | MirStmt::CallBuiltin { args, .. } => {
            for arg in args {
                collect_operand_locals(arg, used);
            }
        }
        MirStmt::IfStmt {
            cond,
            then_body,
            else_body,
        } => {
            collect_operand_locals(cond, used);
            for stmt in then_body {
                collect_stmt_locals(stmt, used);
            }
            for stmt in else_body {
                collect_stmt_locals(stmt, used);
            }
        }
        MirStmt::WhileStmt { cond, body } => {
            collect_operand_locals(cond, used);
            for stmt in body {
                collect_stmt_locals(stmt, used);
            }
        }
        MirStmt::Break | MirStmt::Continue => {}
        MirStmt::Return(value) => {
            if let Some(value) = value {
                collect_operand_locals(value, used);
            }
        }
    }
}

fn collect_terminator_locals(terminator: &Terminator, used: &mut std::collections::HashSet<u32>) {
    match terminator {
        Terminator::If { cond, .. } => collect_operand_locals(cond, used),
        Terminator::Switch { scrutinee, .. } => collect_operand_locals(scrutinee, used),
        Terminator::Return(value) => {
            if let Some(value) = value {
                collect_operand_locals(value, used);
            }
        }
        Terminator::Goto(_) | Terminator::Unreachable => {}
    }
}

fn collect_rvalue_locals(rvalue: &Rvalue, used: &mut std::collections::HashSet<u32>) {
    match rvalue {
        Rvalue::Use(operand) => collect_operand_locals(operand, used),
        Rvalue::BinaryOp(_, lhs, rhs) => {
            collect_operand_locals(lhs, used);
            collect_operand_locals(rhs, used);
        }
        Rvalue::UnaryOp(_, operand) => collect_operand_locals(operand, used),
        Rvalue::Aggregate(_, operands) => {
            for operand in operands {
                collect_operand_locals(operand, used);
            }
        }
        Rvalue::Ref(place) => collect_place_locals(place, used),
    }
}

fn collect_place_locals(place: &Place, used: &mut std::collections::HashSet<u32>) {
    match place {
        Place::Local(local) => {
            used.insert(local.0);
        }
        Place::Field(place, _) => collect_place_locals(place, used),
        Place::Index(place, index) => {
            collect_place_locals(place, used);
            collect_operand_locals(index, used);
        }
    }
}

fn collect_operand_locals(operand: &Operand, used: &mut std::collections::HashSet<u32>) {
    match operand {
        Operand::Place(place) => collect_place_locals(place, used),
        Operand::BinOp(_, lhs, rhs) => {
            collect_operand_locals(lhs, used);
            collect_operand_locals(rhs, used);
        }
        Operand::UnaryOp(_, operand)
        | Operand::EnumTag(operand)
        | Operand::FieldAccess {
            object: operand, ..
        } => collect_operand_locals(operand, used),
        Operand::Call(_, args) | Operand::ArrayInit { elements: args } => {
            for arg in args {
                collect_operand_locals(arg, used);
            }
        }
        Operand::IfExpr {
            cond,
            then_body,
            then_result,
            else_body,
            else_result,
        } => {
            collect_operand_locals(cond, used);
            for stmt in then_body {
                collect_stmt_locals(stmt, used);
            }
            if let Some(result) = then_result {
                collect_operand_locals(result, used);
            }
            for stmt in else_body {
                collect_stmt_locals(stmt, used);
            }
            if let Some(result) = else_result {
                collect_operand_locals(result, used);
            }
        }
        Operand::StructInit { fields, .. } => {
            for (_, operand) in fields {
                collect_operand_locals(operand, used);
            }
        }
        Operand::EnumInit { payload, .. } => {
            for operand in payload {
                collect_operand_locals(operand, used);
            }
        }
        Operand::EnumPayload { object, .. } => collect_operand_locals(object, used),
        Operand::LoopExpr { init, body, result } => {
            collect_operand_locals(init, used);
            for stmt in body {
                collect_stmt_locals(stmt, used);
            }
            collect_operand_locals(result, used);
        }
        Operand::TryExpr { expr, .. } => collect_operand_locals(expr, used),
        Operand::CallIndirect { callee, args } => {
            collect_operand_locals(callee, used);
            for arg in args {
                collect_operand_locals(arg, used);
            }
        }
        Operand::IndexAccess { object, index } => {
            collect_operand_locals(object, used);
            collect_operand_locals(index, used);
        }
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
    }
}

fn reachable_blocks(function: &MirFunction) -> std::collections::HashSet<BlockId> {
    let mut reachable = std::collections::HashSet::new();
    let mut worklist = vec![function.entry];
    while let Some(block_id) = worklist.pop() {
        if !reachable.insert(block_id) {
            continue;
        }
        let Some(block) = function.blocks.iter().find(|block| block.id == block_id) else {
            continue;
        };
        match &block.terminator {
            Terminator::Goto(target) => worklist.push(*target),
            Terminator::If {
                then_block,
                else_block,
                ..
            } => {
                worklist.push(*then_block);
                worklist.push(*else_block);
            }
            Terminator::Switch { arms, default, .. } => {
                for (_, block) in arms {
                    worklist.push(*block);
                }
                worklist.push(*default);
            }
            Terminator::Return(_) | Terminator::Unreachable => {}
        }
    }
    reachable
}

/// Common Subexpression Elimination: within each basic block, replace duplicate
/// pure BinaryOp/UnaryOp computations with a reference to the first result.
fn cse(function: &mut MirFunction) -> OptimizationSummary {
    use std::collections::HashMap;

    let mut summary = OptimizationSummary::default();

    for block in &mut function.blocks {
        // Map from (op_key) -> local that holds the result
        let mut seen: HashMap<CseKey, Place> = HashMap::new();

        for stmt in &mut block.stmts {
            match stmt {
                MirStmt::Assign(place, Rvalue::BinaryOp(op, lhs, rhs)) => {
                    if is_pure_binop(*op) {
                        let key = CseKey::Binary(*op, operand_key(lhs), operand_key(rhs));
                        if let Some(prev_place) = seen.get(&key) {
                            *stmt = MirStmt::Assign(
                                place.clone(),
                                Rvalue::Use(Operand::Place(prev_place.clone())),
                            );
                            summary.cse_eliminated += 1;
                        } else {
                            seen.insert(key, place.clone());
                        }
                    }
                }
                MirStmt::Assign(place, Rvalue::UnaryOp(op, operand)) => {
                    let key = CseKey::Unary(*op, operand_key(operand));
                    if let Some(prev_place) = seen.get(&key) {
                        *stmt = MirStmt::Assign(
                            place.clone(),
                            Rvalue::Use(Operand::Place(prev_place.clone())),
                        );
                        summary.cse_eliminated += 1;
                    } else {
                        seen.insert(key, place.clone());
                    }
                }
                // Any assignment to a local invalidates entries that read from it
                MirStmt::Assign(place, _) => {
                    seen.retain(|k, _| !k.references_place(place));
                }
                // Calls and other side-effecting stmts clear the table
                MirStmt::Call { .. } | MirStmt::CallBuiltin { .. } => {
                    seen.clear();
                }
                _ => {}
            }
        }
    }

    summary
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum CseKey {
    Binary(BinOp, OperandKey, OperandKey),
    Unary(UnaryOp, OperandKey),
}

impl CseKey {
    fn references_place(&self, place: &Place) -> bool {
        match self {
            CseKey::Binary(_, l, r) => l.is_place(place) || r.is_place(place),
            CseKey::Unary(_, o) => o.is_place(place),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum OperandKey {
    ConstI32(i32),
    ConstI64(i64),
    ConstBool(bool),
    ConstF64(u64), // bit pattern for Hash/Eq
    Local(LocalId),
    Other,
}

impl OperandKey {
    fn is_place(&self, place: &Place) -> bool {
        matches!((self, place), (OperandKey::Local(id), Place::Local(pid)) if id == pid)
    }
}

fn operand_key(op: &Operand) -> OperandKey {
    match op {
        Operand::ConstI32(v) => OperandKey::ConstI32(*v),
        Operand::ConstI64(v) => OperandKey::ConstI64(*v),
        Operand::ConstBool(v) => OperandKey::ConstBool(*v),
        Operand::ConstF64(v) => OperandKey::ConstF64(v.to_bits()),
        Operand::Place(Place::Local(id)) => OperandKey::Local(*id),
        _ => OperandKey::Other,
    }
}

fn is_pure_binop(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Add
            | BinOp::Sub
            | BinOp::Mul
            | BinOp::Div
            | BinOp::Mod
            | BinOp::Eq
            | BinOp::Ne
            | BinOp::Lt
            | BinOp::Le
            | BinOp::Gt
            | BinOp::Ge
            | BinOp::And
            | BinOp::Or
            | BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor
            | BinOp::Shl
            | BinOp::Shr
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{
        BasicBlock, FnId, InstanceKey, LocalId, MirFunction, MirLocal, MirModule, MirStmt, Operand,
        Place, Rvalue, Terminator, default_block_source, default_function_source,
        sync_module_metadata,
    };
    use ark_typecheck::types::Type;

    fn simple_module() -> MirModule {
        let mut module = MirModule::new();
        module.entry_fn = Some(FnId(0));
        module.functions.push(MirFunction {
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
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: vec![],
            source: default_function_source(),
            is_exported: false,
        });
        sync_module_metadata(&mut module);
        module
    }

    #[test]
    fn const_fold_rewrites_binary_assign() {
        let mut module = simple_module();
        let summary = optimize_module(&mut module).unwrap();
        assert!(summary.const_folded >= 1);
    }

    #[test]
    fn branch_fold_rewrites_if_terminator() {
        let mut module = simple_module();
        module.functions[0].blocks[0].terminator = Terminator::If {
            cond: Operand::ConstBool(true),
            then_block: BlockId(0),
            else_block: BlockId(0),
        };
        let summary = optimize_module(&mut module).unwrap();
        assert!(summary.branch_folded >= 1);
    }

    #[test]
    fn pass_catalog_contains_inline_small_leaf() {
        assert!(optimization_pass_catalog().contains(&"inline_small_leaf"));
    }

    #[test]
    fn run_single_pass_records_trace() {
        let mut module = simple_module();
        let _ = run_single_pass(&mut module, OptimizationPass::ConstFold).unwrap();
        assert!(optimization_trace_snapshot(&module).contains("const_fold"));
    }
}
