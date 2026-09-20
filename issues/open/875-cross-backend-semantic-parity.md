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
