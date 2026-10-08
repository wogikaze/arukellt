---
Status: done
Created: 2026-09-19
Updated: 2026-09-27
ID: 875
Parent: 852
Track: language-v2
Depends on: "857, 858, 861, 862, 863, 864, 865, 867, 868, 869, 871, 872, 874"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 8–45 parity"
---

# 875 — wasm32-gc / fallback の意味論 parity を証明する

## Scope

RFC-011: **8–45 parity**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] cyclic/shared graph を両 backend で一致させる
- [x] closure/ADT/collection の結果を一致させる
- [x] numeric/evaluation/Unicode fixture を一致させる
- [x] target error を真の capability 差に限定する
- [x] artifact hash/runtime/tool versions を receipt に保存する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する

## Progress receipt — 2026-09-20 (enum payload branch parity)

- Implementation commit `fc98aa717` makes enum struct-variant literals infer
  their enum base type and keeps a conditional selecting different payload
  variants on the shared `enum_open` storage type during MIR/Wasm lowering.
  The change covers the same source semantics on the GC and linear-memory
  backends; it does not claim the complete 8–45 parity acceptance.
- Normative contract: [ADR-058](../../docs/adr/ADR-058-language-v2-semantics-redesign.md)
  and [RFC-011](../../docs/rfcs/011-language-v2-semantics-redesign.md),
  section 8. No new design decision was introduced.
- Positive fixture: `tests/fixtures/enums/enum_struct_variant.ark`, where
  `choose_shape` returns `Shape::Circle` or `Shape::Rect` from separate
  branches. The GC component and linear core module both validate and run
  with stdout `75\n24`.
- Diagnostic fixture: `tests/fixtures/diagnostics/non_exhaustive.ark` remains
  rejected with `E0200` and `missing Color::Blue`, so the payload-lowering
  change does not weaken enum exhaustiveness checking.
- Commands used:

  ```text
  ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2.wasm \
    ./scripts/run/arukellt-selfhost.sh compile tests/fixtures/enums/enum_struct_variant.ark \
    --target wasm32-gc --emit component -o .build/tests/enum-struct-variant-gc-final.component.wasm
  wasm-tools validate --features gc,function-references,memory64 \
    .build/tests/enum-struct-variant-gc-final.component.wasm
  wasmtime run --wasm gc --wasm function-references -W memory64=y --dir=. \
    .build/tests/enum-struct-variant-gc-final.component.wasm

  ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2.wasm \
    ./scripts/run/arukellt-selfhost.sh compile tests/fixtures/enums/enum_struct_variant.ark \
    --target wasm32 --emit wasm -o .build/tests/enum-struct-variant-linear-final.wasm
  wasm-tools validate --features gc,function-references,memory64 \
    .build/tests/enum-struct-variant-linear-final.wasm
  wasmtime run --wasm gc --wasm function-references -W memory64=y --dir=. \
    .build/tests/enum-struct-variant-linear-final.wasm
  ```

- Artifact sizes and SHA-256:

  ```text
  wasm32-gc component 3159 bytes  be00f6c48b931eb26c732701fad6cb00664c6ae14ad2f1a1994877be9255b914
  wasm32 linear core  2567 bytes  fad7a238e02033af7763276bcd795dc3b807dfddfde06c88e5f50e6bd88d86d8
  ```

- The GC core module used for validation was 3236 bytes; the GC component
  wrapper was 3159 bytes. `wasmtime` was `46.0.1 (823d1b8f2
  2026-06-24)` and `wasm-tools` was `1.252.0`. The targeted audit reported
  `reg-vt-audit candidates=4 matched=4 mismatched=0`,
  `core-op-shadow candidates=4 matched=4 mismatched=0 unresolved=0`, and
  `gc-layout-audit typed=12 name=0 fallback=0 conflict=0`.
- This receipt is limited to enum payload branch lowering. Cyclic/shared
  graph parity, closure/collection coverage, negative parity expansion,
  complete artifact comparison, and performance evidence remain open.

## Progress receipt — 2026-09-27 (cross-backend parity harness + first full matrix)

- New canonical gate: `scripts/check/check-cross-backend-parity.py`
  (registered as `cross_backend_parity` in
  `docs/data/verification-commands.toml`). Each selected `run:`
  fixture is compiled twice — `wasm32-gc` WASI-P2 component and
  `wasm32` linear core — executed under wasmtime, and compared on
  normalized stdout + exit code; output also checked against
  `.expected` when present. The JSON receipt records per-fixture
  artifact SHA-256 + byte sizes, compiler hash, and toolchain versions.
- First full run over the curated RFC-011 semantic matrix
  (26 domains: closures, collections, enums/ADT, structs, generics,
  numeric, operators/evaluation, string interp, control flow,
  ownership, question_mark, traits, match extensions, iterators):
  **221 fixtures in 830s — parity=211, diverge=1,
  diverge-from-expected=1, compile-fail:gc=8, compile-fail:wasm32=0.**
- Compile fails: all 8 are the known E0200/E0208 frontend typecheck
  gaps on v2-in-flight sources (`i64_lit`, `mixed_types`,
  `error_ecosystem`, `struct_pattern`, `test_enum_return`,
  `comprehension_guard`, `lambda_pattern`,
  `use_destructure_multi`) — owned by #852/#878, unrelated to backend
  parity. Notably **zero** compile-fail on the wasm32 linear path —
  every fixture the frontend accepts compiles on both emitters.
- Real divergence found: `lang_uplift/guard_const_enum_destructure`
  — GC prints `big/small/none`, linear traps rc=134 on the
  `Opt::None` multi-clause pattern (memory fault at -4). Minimized:
  payload-less enum variants in multi-clause param patterns and in
  tuple-position match patterns miscompile on linear. Filed as **#892**
  (Priority 1, blocks this issue's closure/ADT parity claim).
- `trait/operator_overload` diverge-from-expected was stale
  `.expected` (missing the two newer println lines); both backends
  agree on `4 6 2 2 1 1 1` — expected file corrected, not a
  compiler bug.
- Toolchain receipt: wasmtime `46.0.1 (823d1b8f2 2026-06-24)`,
  wasm-tools `1.252.0`, compiler
  `arukellt-s2-runtime.wasm` sha256
  `f0828eb694fae66b56558a663d6dfc43920262ef651ed536cd4193d0a5e81916`.
  Full JSON: `.build/parity/report.json` (ephemeral; regenerate via
  the canonical command).
- Acceptance status: closure/ADT/collection parity holds across the
  matrix except the #892 divergence; numeric/evaluation/string lanes
  show parity everywhere they compile; cyclic/shared graph parity and
  the capability-error-limit audit remain open; #875 stays open
  pending #892 and the remaining audit axes.

## Progress receipt — 2026-09-27 (#892 fixed, focused re-audit green)

- #892 root-caused and fixed in `3cef5f4f1`: two independent lowering
  defects in the shared MIR match paths — (a) CoreHIR guarded
  non-catchall arms chained the next arm inside the guard else, leaving
  the pattern compare with no else (discriminant miss -> unbound match
  result -> wasm32 null-deref); fixed by mirroring the legacy
  matched-local shape. (b) linear compare_idx was only promoted to the
  tag local when an arm had a *payload* pattern, so matches over
  payload-enum unit variants (`Opt::None`) compared record pointer to
  tag constant; both `mir_prepare_match_payload` and
  `core_prepare_match_payload` now promote whenever an arm names a
  payload-enum variant.
- Focused re-audit on the match/enum matrix slice
  (`enums/`, `match_extensions/`, `control/match_*`,
  `lang_uplift/guard_const_enum_destructure`):
  **20 fixtures in 50.9s — parity=19, diverge=0,
  compile-fail:gc=1** (`match_extensions/struct_pattern.ark`, the
  pre-existing E0200 unknown-enum typecheck gap, unchanged).
  `guard_const_enum_destructure` now prints `big/small/none` on BOTH
  backends; `match_guard` prints `positive/large some` on both.
  Receipt: `.build/parity/report-892.json`.
- New regression fixture `enums/match_payload_enum_unit_arm.ark`
  (run + t3-compile + t3-run) covers unit-variant arms of a payload
  enum — the wrong-arm class previously invisible to gates.
- Tuple-position match arms were verified to be a shared
  unimplemented feature on BOTH backends and both lowering paths
  (invalid wasm either way) — reclassified as non-divergence and
  split to **#893**; it does not block #892's closure.
- #875 remains open for the full-matrix re-run (confirm zero diverge
  across all 221), cyclic/shared graph parity, and the
  capability-error-limit audit.

## Progress receipt — 2026-09-27 (capability-error-limit audit)

- Emit-level target errors are all generated from the target contract
  table (`target_contract_generated::target_allows_emit_kind` /
  `target_allows_host_profile`) plus the component/world gates in
  `driver/emit` + `component/world_spec`: `--emit component` requires
  wasm32-gc + memory32 + wasi-p2, `--world` requires wasm32-gc, and
  wasi-p3 is a declared host profile with an explicit
  not-yet-implemented error. Verified directly:
  `compile --target wasm32 --emit component` ->
  `target `wasm32` does not support emit `component`; allowed: wasm, wat`.
  These are true capability boundaries (component model / WIT / host
  profile / memory width), not source-semantics rejections.
- Source-level capability diagnostics `E0303`/`E0304` and
  `E0120`-`E0124` are declared in `diagnostics/codes.ark` but have
  zero emission sites in `src/compiler/**` — no source construct is
  rejected purely by target choice, matching RFC-011 item 9
  (GC 有無は backend capability で通常コードの合法性を変えない)。
- The 8 `compile-fail:gc` matrix entries were re-verified on wasm32:
  all fail with identical frontend diagnostics (`E0100` test_enum_return,
  `E0200` i64_lit/mixed_types/comprehension_guard/struct_pattern,
  `E0208` lambda_pattern, `E0211` error_ecosystem,
  plus use_destructure_multi) — target-independent typecheck gaps owned
  by #852/#878, not capability divergence. (The parity harness labels
  them compile-fail:gc because it short-circuits on the GC compile;
  the wasm32 side rejects identically.)

## Close receipt — 2026-09-27 (full matrix green, all axes covered)

- **Matrix**: post-#892 full re-run
  `python3 scripts/check/check-cross-backend-parity.py
  --out .build/parity/report-875-post892.json`:
  **222 fixtures in 485.9s — parity=214, diverge=0,
  diverge-from-expected=0, compile-fail:gc=8 (all pre-existing
  target-independent frontend diagnostics), compile-fail:wasm32=0.**
  Zero semantic divergence across the curated RFC-011 domain matrix
  (closures, collections, enums/ADT, structs, generics, numeric,
  operators/evaluation, strings, control flow, ownership,
  question_mark, traits, match extensions, iterators).
- **cyclic/shared graph**: `integration/` slice (not in the curated
  matrix) audited separately: 6/6 parity
  (`.build/parity/report-875-integration.json`) including the new
  `integration/cyclic_graph_walk.ark` (functional digraph with a back
  edge -> deterministic '0 1 2 3 4 / cycle at 2' on both backends) and
  `linked_list.ark` (recursive-ADT tail sharing). Shared mutable state
  is covered by `closure_capture/*` (8/8 in matrix, env cells) and
  `stdlib_hashmap` stress fixtures. True pointer cycles are
  inexpressible in the ownership model (no assignable self-referential
  fields), so source-level cyclic code is observable only through
  shared DAGs, index/edge graphs, and shared env cells — all parity.
- **target error**: emit-level capability boundaries only
  (contract-table emit kinds, component/world/memory-width/host
  profile gates); zero source-level capability rejections
  (E0303/E0304/E0120-E0124 declared, never fired); the 8 compile-fail
  entries verified identical on wasm32 (frontend gaps, not capability).
- **Artifact/tool receipt**: report JSONs record per-fixture
  gc-component + w32-core SHA-256/bytes, compiler sha256
  (`3f53e522…` post-#892 s2), wasmtime `46.0.1 (823d1b8f2 2026-06-24)`,
  wasm-tools `1.252.0`.
- Implementation commits: `8e98ad139` (harness),
  `3cef5f4f1` (#892 lowering fixes), `85ab0e727` (cyclic fixture).
