---
Status: open
Created: 2026-07-14
Updated: 2026-07-14
ID: 796
Track: code-quality
Depends on: "795"
Orchestration class: ready
Orchestration upstream: 795
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop plan CQ-16
---

# 796 — CQ-16: duplicated knowledge and SSOT consolidation

## Summary

Classify duplicated compiler knowledge, select one existing owner for genuine
same-knowledge duplication, and add deterministic drift checks where views are derived.

## Scope

- Audit phase, target/profile, builtin/intrinsic, type spelling, opcode/tag,
  diagnostic, layout slot, WIT ABI, CLI, parser precedence, API export, and docs lists.
- Distinguish same knowledge, coincidental similarity, derived view, and
  compatibility spelling.
- Consolidate only genuine synchronization obligations.

### Scope correction (2026-07-14)

The original issue incorrectly treated two different obligations as one:

- ADR-007 is **ACCEPTED**. Target aliases, canonical targets, and host profiles
  therefore remain in CQ-16 and must be implemented from one product-contract SSOT.
- ADR-042 is **PROPOSED**. Its D1-D8 semantic-operation migration is not an
  implementation obligation and is tracked separately by #798, whose start
  condition is ADR-042 acceptance.

`docs/data/core-ops.toml` remains a `status = "scaffold"` future semantic
registry, not an authoritative compiler input. Today `std/manifest.toml` owns
public paths, documentation, stability, and deprecation, while resolver,
typechecker, and emitter registrations remain migration debt. CQ-16 completes
when every audited category has a disposition and same-knowledge duplication
within **accepted** contracts has one owner; it does not silently adopt a
proposed architecture. This corrects the contract boundary rather than
weakening Acceptance. The larger architecture migration has its own owner,
dependency ADR, acceptance, and rollback boundary in #798.

## Non-goals

- No mega-registry, speculative global schema, or unification of coincidentally
  similar local logic.
- No new registry where an accepted SSOT already exists.

## Acceptance

- [ ] Every listed knowledge category has a recorded disposition under the corrected accepted-contract scope
- [ ] Same-knowledge duplication in accepted contracts has one owner
- [ ] Compatibility aliases are separated from canonical representation
- [ ] Derived views have deterministic generation and drift checks
- [ ] Existing registries are reused and local knowledge remains local
- [ ] Code, tests, docs, and generated views are synchronized
- [ ] Before/after owner inventory is recorded below

## Validation commands

- `python3 scripts/manager.py quality structure`
- `python3 scripts/manager.py docs regenerate`
- `python3 scripts/manager.py docs check`
- `python3 scripts/manager.py verify quick`
- Targeted mapping and generator tests named in completion evidence

## Completion evidence

Audited owners and dispositions:

| Knowledge | Owner / disposition |
|---|---|
| compiler phases/numbers | `compiler/phases.ark`; six unused `driver.ark` copies removed |
| phase display tags | `compiler/phase_tags.ark`; derived locally from phase IDs |
| target/profile/capability | existing `project-state.toml`, `capabilities.toml`, `cli-surface.toml`; accepted alias migration gaps remain |
| builtin/intrinsic/stdlib symbols | `std/manifest.toml` plus `core-ops.toml`; compatibility spellings remain an active migration |
| primitive/vec/GC type spelling | compatibility aliases stay at input predicates; exact duplicate Vec GC resolver removed from `ctx_gc_type_locals.ark`, owner is `ctx_gc_type.ark` |
| MIR/CoreHIR/Wasm opcode and tags | subsystem-local `kinds`/`opcodes`; coincidental numeric similarity not centralized |
| diagnostics/warnings | existing diagnostics/warnings registries; no new registry |
| scratch/local/GC offsets | subsystem-local layout constants; not moved to a mega-registry |
| WIT canonical ABI | component/WIT subsystem and existing manifests |
| CLI options/subcommands | `docs/data/cli-surface.toml` and generated views |
| parser token/precedence | parser-local tables; not globalized |
| public exports/docs lists | `std/manifest.toml`; publication checks remain canonical |

Commit `fc3ca5dd` removes 77 lines of exact manual duplication. No generator was
introduced for local predicates. Existing docs regeneration and structure drift
checks remain the derived-view gate.

This issue is not complete: `docs/current-state.md` still records accepted
target alias and stdlib/core-op migration gaps. Claiming that no manual
target/builtin duplication remains would be false. Those gaps must be resolved
or explicitly narrowed before the unchecked Acceptance items can be completed.

## Primary artifacts

- `docs/data/*.toml`
- `src/compiler/compiler/`
- `src/compiler/diagnostics/`
- `src/compiler/wasm/`
- `scripts/gen/`

## Remaining risks

- Compatibility spellings can look like accidental duplication.
- Moving subsystem-local facts to global data can increase coupling.
- Target aliases and manifest/core-op ownership remain split in accepted current-state
  gaps; this blocks CQ-16 closure and therefore CQ-17 issue closure.

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
