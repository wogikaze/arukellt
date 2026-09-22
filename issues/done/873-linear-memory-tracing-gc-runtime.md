---
Status: done
Created: 2026-09-19
Updated: 2026-09-23
ID: 873
Parent: 852
Track: language-v2
Depends on: "870"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 8 (fallback runtime)"
---

# 873 — linear-memory tracing GC fallback runtime を実装する

## Scope

RFC-011: **8 (fallback runtime)**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] linear-memory allocator を実装する
- [ ] root registration/update を実装する
- [ ] tracing collector を実装する
- [ ] cycle collection を可能にする
- [ ] fallback artifact が Wasm GC instructions を要求しない
- [ ] runtime contract を文書化する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する

## Progress receipt (not done; 2026-09-20)

- Implementation: `6cb9bb01f` (`language-v2: add linear-memory tracing runtime foundation`). The `wasm32` emitter now reserves linear metadata, emits allocation records, provides explicit root-frame entry/push/update/leave functions, and emits fixed-point mark/sweep and free-list allocation functions without Wasm GC instructions in the runtime source. `47995d5bf` adds the Memory64-to-memory32 P2 `cabi_realloc` boundary adapter, and `8ecb6b2a6` adds a guarded source-only Box/Slice overlay for the stale pinned bootstrap. The fixture import migration is `8ac3714b2`.
- Normative updates: [ADR-075](../../docs/adr/ADR-075-language-v2-linear-memory-tracing-runtime.md), the [linear-memory lowering plan](../../docs/plans/wasm32-linear-memory-lowering.md), and RFC-011 section 2.2. The plan records that automatic collection remains disabled until root lowering is complete, so live values cannot be reclaimed during this migration step.
- Positive/negative contract: `scripts/tests/test_linear_gc_runtime_contract.py` passes (`Ran 3 tests ... OK`); the target-capability diagnostic fixture remains the applicable negative path. The ADT ownership positive/negative fixtures now use the explicit `std::host::stdio` import required by the v2 source contract.
- Verification: `python3 scripts/manager.py selfhost build-compiler` passes (stage-2 output; the command reports and applies the guarded structural overlay when the pinned stage-0 compiler encounters the generic enum payload mismatch). Current-source `adt_probe.ark` compiles and validates for both `wasm32-gc` and `wasm32`; the GC core also packages and validates as a P2 component. `wasm32` WAT contains no `struct.*`, `array.*`, or `ref.*` instructions. `scripts.tests.test_selfhost_overlay` and `scripts.tests.test_memory64_p2_realloc` pass (17 tests total), and `verify lane` passes its format/lint/quality gates.
- Runtime acceptance receipt: `055de4955` adds `scripts.tests.test_linear_gc_runtime_e2e`. The test compiles the positive target-capability source for `wasm32`, resolves the emitted private runtime function group from the artifact, appends a verification-only adapter, and then runs `wasm-tools validate` plus Wasmtime. The adapter proves that a rooted two-node cycle remains allocated, that removing the root permits collection, and that both cycle records are reused; the linear artifact contains no `struct.*`, `array.*`, or `ref.*` instructions. `2b4180c8b` registers the ADT positive/diagnostic fixtures in `tests/fixtures/manifest.txt`; the manifest set now matches disk (`2079 == 2079`). The focused runtime contract and e2e tests pass (`Ran 4 tests ... OK`).
- Close review: `python3 scripts/manager.py verify quick` was executed but returned exit code 1 because of repository-wide pre-existing failures in the #652 close gate, #798 HashMap core-op registry, quality ratchet, MIR/T3 validation backlog, and related baseline checks. Those failures are outside this runtime slice and are not hidden by a baseline or SKIP. The pinned bootstrap has not yet been refreshed to a generic-enum-capable fixpoint, and automatic collection remains disabled until #874 connects managed-value lowering and root updates, so this issue remains open.

## Progress receipt — 2026-09-20 (compiler root-frame integration)

- Implementation commit `aed3af031` connects the compiler-generated linear
  function bodies to the private root-frame ABI. The code section records the
  four root runtime indices, linear functions reserve a frame cursor local,
  conservative i32/ref locals are pushed at entry, local writes update their
  slots, and return/fallthrough paths leave the frame. The Wasm GC target keeps
  its scratch layout and body emission unchanged.
- The integration deliberately does not emit `collect`. It covers the current
  i32 linear representation, but descriptor-aware managed writes across every
  allocation path, closure environments, HashMap values, and the final
  automatic-collection gate remain open under #874.
- Verification: `python3 scripts/manager.py selfhost build-compiler` PASS;
  `python3 scripts/manager.py verify lane` PASS; the focused contract test
  passes (`Ran 5 tests ... OK`); and
  `python3 scripts/tests/test_linear_gc_runtime_e2e.py` passes (`Ran 1 test
  ... OK`). The e2e artifact validates the rooted-cycle retention and
  unreachable-cycle reuse probe without `struct.*`, `array.*`, or `ref.*`.
- This receipt advances the compiler integration boundary but does not satisfy
  the full runtime acceptance list or close #873. Automatic collection remains
  disabled until #874 proves complete managed-root coverage.

## Completion receipt — 2026-09-23 (automatic collection enabled; runtime complete)

- The remaining blocker from the previous receipt — "automatic collection
  remains disabled until #874 connects managed-value lowering and root
  updates" — is resolved. `f3c56bbef` mirrors every managed local write
  into root slots (generic store path, struct/array constructors,
  `array_set` staging, ref test/cast fallbacks, future copies), and
  `2aaadcbf4` emits a watermark-gated `LINEAR_GC_COLLECT` call at every
  linear function entry: the operand stack is empty there and all live
  frames' managed locals are already mirrored, so the mark phase cannot
  produce a false negative. The 64 MiB watermark amortizes the full-heap
  sweep for selfhost-scale programs; smaller programs simply never
  collect and grow the heap instead.
- Acceptance mapping with repo evidence:

  | Acceptance | Evidence |
  |---|---|
  | linear-memory allocator | `wasm/linear_gc_runtime.ark` `emit_linear_gc_runtime_alloc_body` (free-list + bump + watermark); routed via `helpers_core_heap::emit_heap_alloc_from_stack` across all managed paths (`6cb9bb01f`, `9e81a367e`, `2aaadcbf4`) |
  | root registration/update | `root_frame_enter`/`root_push`/`root_update`/`root_frame_leave` + `linear_root_lowering.ark` entry emission and `emit_update_after_local_set` (`aed3af031`, `f3c56bbef`) |
  | tracing collector | `emit_linear_gc_runtime_collect_body`: clear marks → root scan → fixed-point transitive mark → free-list sweep (`6cb9bb01f`, `2aaadcbf4`) |
  | cycle collection | `test_rooted_cycle_survives_and_unreachable_cycle_is_reused` probe: rooted 2-node cycle survives, unrooted cycle reclaimed and reused (`055de4955`) |
  | no Wasm GC instructions | every e2e artifact asserts no `struct.*`/`array.*`/`ref.*` in printed WAT; `wasm-tools validate` PASS |
  | runtime contract documented | ADR-075 (ACCEPTED) invariants + `docs/plans/wasm32-linear-memory-lowering.md` updated through `449bb5651` |

- End-to-end proof on real programs:
  `test_function_entry_collects_when_watermark_exceeds_threshold` forces
  the watermark past the threshold and observes the reset plus a
  non-empty free list after entering compiled `$main`;
  `test_source_level_live_data_survives_entry_collection` runs
  `linear_gc_entry_collect.ark` and `linear_gc_hashmap_survives.ark`
  (~160 MiB churn, ≥2 collections) with in-source assertions holding on
  both `wasm32` and `wasm32-gc` (`b2144f1c1`, `449bb5651`).
- Verification receipts (this session):

  ```text
  python3 scripts/manager.py selfhost build-compiler        PASS
  python3 scripts/manager.py verify lane                   PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py  PASS (5 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py       PASS (3 tests)
  python3 scripts/manager.py docs check                    PASS (4/4)
  ```

- Known bounds (documented, trap-not-corrupt per ADR-075): the 64 KiB
  metadata region caps live allocation records at 4096 and root slots
  share the same region; both bounds trap on exhaustion. Source-level
  true pointer cycles remain inexpressible (structs store `i32`
  indices) — that is a language-expressiveness item owned by
  #872/#874, not a runtime gap. Exact pointer-field descriptors stay
  conservative per ADR-075 and are tracked under #874.
