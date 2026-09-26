---
Status: done
Created: 2026-09-19
Updated: 2026-09-26
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

## Progress receipt — 2026-09-26 (issue closing)

Commits: implementation \`9783627b5\` + \`b83d87ccf\`
(2026-09-20); closure-emit repair \`83a7923c0\` (this receipt).
Branch \`docs/adr-058-language-v2-semantics\`.

Dependency re-triage (recorded explicitly per the close rule above):
the \`Depends on: 729\` edge is re-triaged with repository
evidence. The declared dependency was meaningful while acceptance
item 1 — the canonical lang-item mechanism — was undecided. ADR-077
settled it with compiler-owned receiver/method-reference lowering,
which runs on the existing binding machinery; nothing in
\`9783627b5\` / \`b83d87ccf\` touches
\`data/core-ops.toml\`, the SignatureRegistry, or the alias
machinery #818 owns. The remaining #729 child #818 is further gated
on #698 for the portable-SIMD \`legacy_emitter\` carve-out, so the
edge would hold a verified v2 leaf hostage to an unrelated registry
epic. #865 is closed; if the #729 close-gate later finds desugaring
still rides on removed legacy machinery, that is a mechanical
follow-up, not a semantics change — the contract lives in ADR-077,
not in the binding layer.

Evidence (re-verified on current stage-2, 2026-09-26):

- \`string_interp/fstring_hygiene.ark\`: wasm32 linear compiles
  and runs \`left\` / \`right\` / \`iteration\` /
  \`7\` / \`8\` / \`fstring_hygiene ok\`; wasm32-gc
  component compiles, validates, and runs identically.
- \`string_interp/fstring_basic.ark\`, \`fstring_expr.ark\`,
  \`fstring_debug_specifier.ark\`: wasm32-gc components compile
  and run with expected output.
- \`diagnostics/fstring_embedded_unresolved.ark\`: still errors
  \`E0100: undefined name: unknown_value\` inside the
  interpolation.
- Gate: \`python3 scripts/manager.py verify lane\` PASS (comment
  policy, semantic-debt, lint smoke, ratchet, editorconfig, quality
  contract, fmt, lint ratchet).

Regression found and fixed during this review: every
\`__closure_*_env\` trampoline on wasm32-gc pushed the unpacked
env field as one extra call argument whenever the closure captured
nothing, because \`closure_split_packed("")\` returned a single
empty capture name. The phantom \`i32\` env field made
capture-less closure modules invalid (wasm type mismatch: ref vs
i64). \`83a7923c0\` skips empty segments in
\`closure_split_packed\`; \`closure_basic\`,
\`closure_capture\`, \`capture_struct\`,
\`higher_order\`, and the full fstring suite now validate and run
on both targets. Filed under #872 scope.
