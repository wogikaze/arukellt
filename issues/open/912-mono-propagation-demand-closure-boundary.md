---
Status: open
Created: 2026-10-10
Updated: 2026-10-10
ID: 912
Track: compiler-internal
Depends on: none
Orchestration class: design-needed
Orchestration upstream: none
Blocks v{N}: language-v2
Priority: 2
Source: 908 lane stopgaps - propagation budget + depth caps mask missing demand model
---

# 912 - Monomorphization needs a principled demand-closure boundary

## Summary

908 surfaced that record_mono_instance is called both for genuinely
demanded instances and for speculative propagation guesses, and emit
previously could not tell them apart. The lane added a demanded flag
demanded=true only on real call-site/rebind paths plus two stopgaps:
MONO_PROPAGATE_INSTANCE_BUDGET=300 and mono_args_within_depth (depth<=2,
var-reject) gating owner-side speculative propagation. Those caps are
heuristics, not semantics: a program shaped unluckily can still blow the
budget or lose a required instance.

## What a fix looks like

Define instance demand as a closure over the MIR actually reachable from
entry points: an instance is demanded iff some emitted body can reach a
call/variant whose static resolution is that instance. Speculative
registration may continue for typechecker queries but must be free of
global caps; emission consumes the demanded set. The same boundary
should replace the linear mono_index_mangled_at / mono_index_count scans
used during lowering with an indexed lookup.

## Related

- #851 selfhost compiler core rewrite - the demand model is a natural
  input to the rewrite's module/visibility boundary design.
- #910 emit-side write-site reanalysis - adjacent but independent
  quadratic in the emit phase.
