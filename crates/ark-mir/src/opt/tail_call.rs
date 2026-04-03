//! Tail-call detection pass.
//!
//! Converts `Terminator::Return(Some(Operand::Call(...)))` and
//! `Terminator::Return(Some(Operand::CallIndirect {...}))` to
//! `Terminator::TailCall { .. }` / `Terminator::TailCallIndirect { .. }`.
//!
//! This makes the tail-call contract explicit at the MIR level so the backend
//! can emit `return_call` / `return_call_indirect` without re-detecting the
//! pattern during code generation.
//!
//! The pass is identity-safe at opt_level 0 (returns `false` / zero count).

use crate::mir::{MirFunction, Operand, Terminator};

/// Run the tail-call detection pass on a single function.
///
/// Returns the number of terminators rewritten.  At `opt_level 0` the pass
/// is a no-op (returns 0) so debug builds retain the full call-frame chain.
pub fn detect_tail_calls(func: &mut MirFunction, opt_level: u8) -> usize {
    if opt_level == 0 {
        return 0;
    }

    let mut rewrites = 0;
    for block in &mut func.blocks {
        let new_term = match &block.terminator {
            // Direct call in tail position: return call(args…)
            Terminator::Return(Some(Operand::Call(name, args))) => {
                Some(Terminator::TailCall {
                    func: name.clone(),
                    args: args.clone(),
                })
            }
            // Indirect call in tail position: return callee(args…)
            Terminator::Return(Some(Operand::CallIndirect { callee, args })) => {
                Some(Terminator::TailCallIndirect {
                    callee: callee.clone(),
                    args: args.clone(),
                })
            }
            _ => None,
        };

        if let Some(term) = new_term {
            block.terminator = term;
            rewrites += 1;
        }
    }
    rewrites
}
