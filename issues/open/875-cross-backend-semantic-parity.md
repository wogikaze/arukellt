---
Status: open
Created: 2026-09-19
Updated: 2026-09-20
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

- [ ] cyclic/shared graph を両 backend で一致させる
- [ ] closure/ADT/collection の結果を一致させる
- [ ] numeric/evaluation/Unicode fixture を一致させる
- [ ] target error を真の capability 差に限定する
- [ ] artifact hash/runtime/tool versions を receipt に保存する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する

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
