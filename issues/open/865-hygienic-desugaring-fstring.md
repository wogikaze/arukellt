---
Status: open
Created: 2026-09-19
Updated: 2026-09-20
ID: 865
Parent: 852
Track: language-v2
Depends on: "729"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 28, 29"
---

# 865 — Hygienic desugaring と f-string lowering を実装する

## Scope

RFC-011: **28, 29**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] lang item/pre-resolved symbol/intrinsic の canonical mechanism を選ぶ
- [x] comprehension/iteration/f-string を hygienic に lower する
- [x] user shadowing で sugar の意味を変えない
- [x] 埋め込み式の評価順を保持する
- [x] core form との equivalence fixture を追加する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する

## Implementation receipt (close blocked by #729, 2026-09-20)

- Implementation: `9783627b5` (`language-v2: make desugaring hygienic`) changes
  f-string interpolation and concatenation from bare calls to typed receiver
  methods, and emits comprehension builders as compiler-owned `Vec::push`.
  `b83d87ccf` (`test: assert fstring core equivalence`) adds the explicit core
  form equivalence assertion.
- Normative contract: [ADR-077](../../docs/adr/ADR-077-language-v2-hygienic-desugaring.md),
  RFC-011 items 28–29, and the f-string/comprehension sections of the language
  specification. The canonical mechanism is receiver/type-based semantic
  lowering; source bindings cannot capture generated calls.
- Fixtures: `string_interp/fstring_hygiene.ark` shadows `concat`, `to_string`,
  `fmt_debug`, `into_iter`, `next`, `collect`, and `push`; it also records
  left-to-right side effects and compares f-string output with
  `to_string().concat(...)`. `diagnostics/fstring_embedded_unresolved.ark`
  preserves ordinary E0100 resolution diagnostics inside an interpolation.
- Verification: current stage-2 `selfhost build-compiler` PASS (19.7s),
  `verify lane` PASS (quality changed, lint ratchet, and format checks), and
  `docs regenerate` PASS. The positive fixture compiles and validates as both
  `wasm32-gc` core Wasm and `wasm32` linear Wasm; the GC artifact also packages,
  validates, and runs as a WASI P2 component. Wasmtime output is identical on
  both paths: `left`, `right`, `iteration`, `7`, `8`, `fstring_hygiene ok`.
  The existing `fstring_debug_specifier` fixture also compiles/validates on both
  targets and runs successfully on the linear artifact. The negative fixture
  returns `error[E0100|resolve]: undefined name: unknown_value`.
- WAT inspection of the GC positive artifact shows `i32::fmt_debug`,
  `String::concat`, `Vec_String::into_iter`, `StringIter::next`,
  `Vec::into_iter`, and `VecIter::next` call targets; the shadow closures are
  not called. The full historical fixture backlog and bootstrap parity issues
  remain outside this focused acceptance receipt.
- Close review: `REQUEST_CHANGES`. The declared dependency `#729` is still
  `Status: open`; this receipt does not silently remove or reinterpret that
  dependency. Move this issue to `issues/done/` after #729 is resolved or the
  dependency is explicitly re-triaged with repository evidence.
