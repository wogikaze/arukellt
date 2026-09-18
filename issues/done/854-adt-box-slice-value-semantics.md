---
Status: done
Created: 2026-09-19
Updated: 2026-09-19
ID: 854
Parent: 852
Track: language-v2
Depends on: "853"
Related: "ADR-058, ADR-060, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 5–7, 25"
---

# 854 — ADT / Box / slice の値意味論を統一する

## Scope

RFC-011: **5–7, 25**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] Option/Result/enum を owned ADT として定義する
- [x] `Box<T>` を heap 上の `T` の unique owner として実装する
- [x] slice を borrow/view として定義する
- [x] GC object/tagged/unboxed は backend detail に限定する
- [x] move/Copy/borrow fixture を追加する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する

## Completion evidence (2026-09-19)

- Implementation: `ca6b85ad8`.
- Normative contract: [ADR-060](../../docs/adr/ADR-060-language-v2-owned-adts-box-slice.md); RFC-011 items 5–7 and 25 now reference the adopted contract. Normative [language spec](../../docs/language/spec.md) and [memory model](../../docs/language/memory-model.md) were synchronized; generated ADR/stdlib references were regenerated.
- Owned ADT: `Option` / `Result` use structural `Copy` only when payloads are `Copy`; user enums remain explicit-`Copy` opt-in and backend tags/layout stay private.
- Box: `Box_new` consumes a non-`Copy` value and `unbox` consumes the unique owner; identity fallback paths were removed from the lowerer and intrinsic fallback.
- Slice: `Slice<T>` is a non-`Copy` bounded read view; `as_slice` retains the source binding, while `slice_len` / `slice_get` consume the view under the current stdlib API. Full lifetime / mutation-exclusion checking is explicitly deferred to later borrow/closure work; this is not a hidden skip.
- Positive fixture: `tests/fixtures/ownership/adt_probe.ark` and `.expected`.
- Negative fixture: `tests/fixtures/diagnostics/adt_move_after_use.ark` and `.diag` (E0212 after reusing a moved `Box<T>` binding).
- GC primary receipt: `wasm32-gc` core output and WASI P2 component output both validated; component ran under Wasmtime GC/function-references/Memory64 and printed `42`, `1`, `1`, `8`.
- Linear fallback receipt: `wasm32` output validated and ran under Wasmtime with the same `42`, `1`, `1`, `8` output.
- Direct compiler receipts: GC layout audit `typed=76 name=0 fallback=0 conflict=0`; registration audit `candidates=6 matched=6 mismatched=0`; CoreOp shadow `candidates=12 matched=12 unresolved=0`.
- Required gates: `python3 scripts/manager.py fmt --check` (`checked=1954 failed=0 baseline-skipped=10`), `python3 scripts/manager.py verify lane` (PASS), `python3 scripts/gen/generate-docs.py --check` (PASS), and `python3 scripts/check/check-adrs.py` (PASS).
- No new TODO/skip/hack or semantic-debt allow-list entry was added. Existing linear tracing-GC runtime work remains tracked by #873/#874 and is not claimed by this issue.
