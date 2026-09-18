---
Status: done
Created: 2026-09-19
Updated: 2026-09-19
ID: 855
Parent: 852
Track: language-v2
Depends on: "853"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 10"
---

# 855 — Closure capture / escape / lifetime を実装する

## Scope

RFC-011: **10**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] shared/mutable borrow と move capture の規則を固定する
- [x] 短命 borrow を保持する escaping closure を reject する
- [x] return/store/callback の fixture を追加する
- [x] capture/lifetime diagnostic に正確な span を付ける
- [x] backend 表現で source semantics を変えない

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

- Implementation: `093e2e914`; parser, AST cache, typechecker borrow state,
  capture summary, escape checks, diagnostics, and fixture registration are
  included in that commit.
- Normative contract: [ADR-061](../../docs/adr/ADR-061-language-v2-closure-capture-lifetime.md),
  [RFC-011 §4.1.1](../../docs/rfcs/011-language-v2-semantics-redesign.md),
  [language spec](../../docs/language/spec.md), and
  [memory model](../../docs/language/memory-model.md).
- Positive fixtures: `closure_capture/borrow_local.ark` and
  `closure_capture/move_local.ark`; both compiled for `wasm32-gc` and
  `wasm32`, validated, and ran as `borrow:3` and `move:4` on both paths.
  GC component artifacts were validated with `wasm-tools` and ran under
  Wasmtime GC/function-references/Memory64.
- Diagnostic fixtures: return, aggregate store, and callback escape produce
  E0213; mutable-borrow conflict produces E0214; move-after-capture produces
  E0212. Reported spans are `6:12`, `6:19`, `10:12`, `6:5`, and `6:18`.
- Quality receipt: with the locally generated current selfhost artifact,
  `ARUKELLT_SELFHOST_WASM=.build/bootstrap-shim-855-raw-v2/selfhost/arukellt-s2-runtime.wasm
  python3 scripts/manager.py verify lane --gate quality-quick` passed;
  comment policy, ADR, semantic-debt, docs generation, and `git diff --check`
  also passed.
- Bootstrap boundary: the canonical pinned `python3 scripts/manager.py
  selfhost build-compiler` remains blocked by the pre-existing #854 generic
  enum payload return mismatch (`unbox` / `slice_get`). An isolated baseline
  checkout reproduces the same failure before #855; this receipt therefore
  uses the current generated artifact and does not claim a pinned bootstrap
  refresh. Managed closure-environment lowering remains #872/#874 scope.
- No new TODO, SKIP, semantic-debt workaround, or untracked acceptance gap
  was introduced.
