---
Status: open
Created: 2026-07-14
Updated: 2026-07-14
ID: 796
Track: code-quality
Depends on: "795"
Orchestration class: ready
Orchestration upstream: None
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

- [x] Every listed knowledge category has a recorded disposition under the corrected accepted-contract scope
- [x] Same-knowledge duplication in accepted contracts has one owner
- [ ] Compatibility aliases are separated from canonical representation
- [ ] Derived views have deterministic generation and drift checks
- [ ] Existing registries are reused and local knowledge remains local
- [ ] Code, tests, docs, and generated views are synchronized
- [ ] Before/after owner inventory is recorded below

## Reopened blocking findings (2026-07-14 CQ-18 audit)

1. **core-ops.toml ownership contradiction**: Three documents disagree:
   - `docs/data/core-ops.toml`: `status = "scaffold"`
   - `docs/current-state.md`: current owner is manifest + compiler-local registration
   - `docs/directory-ownership.md`: `core-ops.toml = product (SSOT input)`
   While ADR-042 is PROPOSED, `core-ops.toml` must NOT be described as
   `product (SSOT input)` or current compiler SSOT. `directory-ownership.md`
   must be corrected to: future semantic registry scaffold, hand-maintained
   proposal input, not consumed authoritatively by compiler, migration
   owner: #798.
2. **CQ-17 target documentation incomplete**: Active/user-facing surfaces
   still contain old target names as current spec (see #797 blocking
   findings). CQ-16 Acceptance "code, tests, docs, and generated views are
   synchronized" is not met while active docs show deprecated names as
   canonical.
3. **generated-file registration claim is inaccurate**: Completion evidence
   states all 4 target views are registered in `.generated-files`, but only
   2 are registered. Partial-generation files (`target-contract-summary.md`,
   `current-state.md` target section) are not in the manifest. Either extend
   the schema or correct the claim.

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
| target/profile/capability | `project-state.toml` uniquely owns target profiles, host profiles, alias input/policy/replacement; `capabilities.toml` owns host capabilities |
| builtin/intrinsic/stdlib symbols | current public-path/docs/stability/deprecation owner is `std/manifest.toml`; compiler-local semantic registration remains migration debt; `core-ops.toml` is only the proposed ADR-042 scaffold tracked by #798 |
| primitive/vec/GC type spelling | compatibility aliases stay at input predicates; exact duplicate Vec GC resolver removed from `ctx_gc_type_locals.ark`, owner is `ctx_gc_type.ark` |
| MIR/CoreHIR/Wasm opcode and tags | subsystem-local `kinds`/`opcodes`; coincidental numeric similarity not centralized |
| diagnostics/warnings | existing diagnostics/warnings registries; no new registry |
| scratch/local/GC offsets | subsystem-local layout constants; not moved to a mega-registry |
| WIT canonical ABI | component/WIT subsystem and existing manifests |
| CLI options/subcommands | `docs/data/cli-surface.toml` and generated views |
| parser token/precedence | parser-local tables; not globalized |
| public exports/docs lists | `std/manifest.toml`; publication checks remain canonical |

Commit `fc3ca5dd` removed 77 lines of phase/Vec exact manual duplication. The
target migration then changed ownership as follows:

| Before | After |
|---|---|
| aliases split between ADR prose, `lint/deprecated_table.ark`, driver checks, help, and extension | `docs/data/project-state.toml` `[[target_aliases]]` |
| old target spelling propagated through CLI, resolver, MIR, and emitter | CLI boundary emits W0002/error then stores canonical target plus canonical host profile |
| compiler default `wasm32-wasi-p2` | target `wasm32-gc`, host `wasi-p2` |
| extension legacy-only enum/default and literal component args | canonical enum plus generated `target-contract.generated.js` |
| `core-ops.toml` described as present compiler SSOT | explicit `status = "scaffold"`; current owners documented, migration isolated in #798 |

Generated views are `main/target_contract_generated.ark`, the extension target
contract, and target summary/docs. They are registered in `.generated-files`;
`generate-docs.py --check`, structure checks, and `test_target_contract.py`
reject source/view drift and duplicate/invalid aliases. Compiler operational
source contains no old target spelling outside the generated alias contract.
Allowed occurrences are the alias SSOT/views, compatibility tests, migration or
historical documentation, changelog, and archived fixture/baseline evidence.

Targeted runtime evidence on the rebuilt selfhost compiler:

- all six deprecated aliases exit 0 on `check`, emit `W0002`, and report the
  SSOT replacement; `wasm32-wasi-p3` resolves to `wasm32-gc` + `wasi-p3`;
- `native` and `wasm32-freestanding` exit 2 with explicit errors;
- alias `wasm32-wasi-p2` and canonical `wasm32-gc` + `wasi-p2` compile to the
  same 849-byte Wasm (`sha256 dc469928...57b`);
- resolver gating rejects `std::host::udp` on `wasm32` + `wasi-p1` with E0500
  and accepts it on `wasm32-gc` + `wasi-p2`.

ADR-042 remains PROPOSED. Implementing D1-D8 here would have introduced an
unadopted owner. Issue #798 therefore owns scaffold exit, manifest semantic/type
references, ordered resolver/typechecker/MIR/docs migration, validation,
differential tests, no-dual-truth cutover, and rollback boundaries.

Validation at commit `6347a4fe` (2026-07-14):

- `python3 scripts/manager.py quality structure` — PASS
- `python3 scripts/manager.py docs regenerate` — PASS, deterministic
- `python3 scripts/manager.py docs check` — PASS (4/4)
- `python3 scripts/manager.py verify quick` — PASS
- `python3 -m unittest scripts.tests.test_target_contract` — PASS (5/5)
- `python3 scripts/check/check-issue-health.py` — PASS
- `git diff --check` — PASS

## Primary artifacts

- `docs/data/*.toml`
- `src/compiler/compiler/`
- `src/compiler/diagnostics/`
- `src/compiler/wasm/`
- `scripts/gen/`

## Remaining risks

- Compatibility spellings can look like accidental duplication.
- Moving subsystem-local facts to global data can increase coupling.
- Compatibility fixtures and archived evidence intentionally retain old names;
  new operational source is protected by `test_target_contract.py`.
- ADR-042 migration remains blocked until the proposal is accepted; #798 owns it
  and is not represented as completed current architecture.

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
