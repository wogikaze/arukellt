---
Status: done
Created: 2026-10-10
Updated: 2026-10-11
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

## Progress (2026-10-11)

Indexed lookup landed: MonoIndex now keeps FNV-1a open-addressed
accelerators (same convention as the lowering ctx mono-inst map in
ctx_api_module.ark): a name to names-position map plus per-base mangled
entry chains, so mono_index_count / mono_index_mangled_at /
mono_index_has_mangled are probe + short chain walks instead of linear
scans over all entries. A/B evidence: pre/post-change compiler outputs
are byte-identical on the 9 generics fixtures, a 5-instantiation smoke
program, and component/resource_v3/resource_basic.

Demand-closure model landed (same day): record_mono_instance is now the
sole mint and every mint is demanded; demand edges are recorded at real
call sites, template-site rebinds, and var-method sites (method calls
on still-open type-var receivers record `method`+[?T] and rebind
resolves `Receiver::method` through the impl table). Deleted:
MONO_PROPAGATE_INSTANCE_BUDGET, mono_args_within_depth,
mono_type_ctor_depth, record_propagated_mono_instance, and all
propagate_* sibling/arg speculation - the over-approximation itself was
the termination hazard (adapter x method ascent never converged).
Module-merge rebind is gated on demanded so non-demanded marker records
can no longer mint sentinel-arg junk. Two latent bugs the stopgaps had
masked got real fixes: sig_decl dropped the last parameter of bodiless
trait methods (`fn next(self: Self)` registered param_count 0, making
infer_trait_method_call unreachable for self-only methods), and
infer_trait_method_call's call edge was never recorded as demand.

Evidence: for_values now compiles in ~1s (was >300s under demanded
owner-prop); hashmap_entries_values ~2s (was >90s). Compiler outputs
are byte-identical vs the stopgap build on all 19 probed fixtures
(9 generics, multi_inst, resource_basic, and 8 iterator/collection
fixtures), with 0 untyped-scrutinee diagnostics where the no-cap build
previously emitted 52.

Remaining: none of the listed stopgap scope. Close-candidate pending
wider lane verification and the $issue-close-review checklist.
