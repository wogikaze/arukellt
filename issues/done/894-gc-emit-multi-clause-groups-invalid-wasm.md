---
Status: done
Created: 2026-09-27
Updated: 2026-09-28
ID: 894
Parent: 852
Track: language-v2
Depends on: ""
Related: "ADR-058, RFC-011, #877, #892, #893"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "#877 normative-example fixture extraction"
---

# 894 — wasm32-gc emit: 複数 multi-clause 関数群で invalid core Wasm

## Scope

wasm32-gc の emit で、1 ファイルに複数の multi-clause 関数グループ
（`fn f(lit) -> T = e` 形式の節群）があると invalid core Wasm を生成する。
各グループ単体では正しく emit される。wasm32 linear emit は正常。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Reproducer

`tests/fixtures/spec_examples/multi_clause_functions.ark`
（7 節群: int-literal / bool / Option::Some / string / binding / typed）
と `tests/fixtures/spec_examples/clause_guards.ark`
（guard 節群 2 個: `| n > 0` + `| pt.x > 0`）を
`--target wasm32-gc` でコンパイルすると:

```
$ wasm-tools validate out.wasm
error: func 18 failed to validate
Caused by:
    0: type mismatch: expected (ref null $type), found i64 (at offset 0x863)
```

clause_guards 側は `func 17: expected anyref, found i64`。

## Bisect notes（2026-09-27, #877 作業中に採取）

- 単一節群は全種 emit 成功: int-lit（m1）/ string-lit（m2）/ enum-variant
  （m3）/ bool（m4）/ guard（m8, 3 節 `| n > 0` など）/ mixed literal+binding
  （m9）/ struct-field guard（m7）。
- 2 節群同士の組合せで失敗: classify+greet（m5: `expected i64, found
  (ref null $type)`）、classify+describe（m10: `expected anyref, found i64`）。
- wasm32（linear）emit は全組合せで valid。失敗は GC emit 固有。
- 症状は 2 方向: `found i64 where ref expected` と `found ref where i64
  expected` の両方が出る。emit 側で節群をまたぐ共有状態（clause dispatch
  関数スロット、scrutinee/result の型テーブル index など）が疑わしい。
- `arukellt check`（typecheck）は全ケースで通る。破壊は emit 段のみ。
- #892 で修正した shared MIR match lowering と同系統の経路を疑う
  （multi-clause は match へ脱糖される）。

## Acceptance

- [x] 上記 2 fixture が wasm32-gc emit で valid Wasm を生成する
- [x] wasmtime run で expected output を両 backend で一致させる
- [x] manifest で 2 fixture を `run:` + （parity 対象化なら）t3-run へ昇格する
- [x] 回帰 fixture を残す（既存 2 fixture の昇格で足りるか判定）

## Required evidence

- implementation commit SHA
- wasm-tools validate / wasmtime run output on both backends
- 原因となった emitter 箇所と修正内容の要約

## False-done guard

- [x] wasm32 linear のみで done にしない（GC emit が本 bug の核心）
- [x] check-only 登録のまま run 昇格せず done にしない
- [x] 親 #852 の担当項目を evidence 付きで更新する

## Done (2026-09-28)

Implementation commit: `ebd99196c`.

Root cause: unannotated fn parameters were bound as scalar i64 in the
emitted func signature while match lowering treated them as GC refs.
The signature record was never re-stamped once the body proved a
managed type, so every unannotated-param function that was not
scalar-typed produced invalid Wasm in both directions — not only
multi-clause groups, but any shape the issue's bisect combined.

- `mir/lower/param_sig_infer.ark` (new) back-patches the param
  signature + local-type side table (`ctx_set_func_param_type` alone
  loses to the stale side-table entry).
- `match_payload_prepare.ark` infers the scrutinee type per arm
  shape before payload prep: variant arms → Option element recovered
  from the return type (`option:i32`) else `enum:open`; string
  literal arms → `String`; `ident.field` guards → the field's
  owning struct; `s => s` bind-return arms → the annotated ref
  return type.
- `control.ark` / `entry_body_result.ark` stamp a param returned
  directly when the declared return type is a ref
  (`fn id(s) -> String = s`).
- String literal patterns now carry a parser op mark and lower to
  `__intrinsic_string_eq` compares in both the AstNode and CoreHIR
  match paths (linear backend uses the CoreHIR path); the or-pattern
  compare accumulates results explicitly instead of relying on
  unreachable-br stack discards.
- Regression: both spec fixtures promoted to `run:` + `t3-run`,
  and `multi_clause_functions` gained an `echo` group
  (`"ping" → "pong"`, `s => s`) covering the bind-return shape.
  The stale `negate(true)` expected line was corrected to `false`
  per the clause semantics.

Evidence:

- `wasm-tools validate` PASS on wasm32-gc core modules for both
  fixtures; `$echo`-shape probe valid with
  `(param $__arg0 (ref null 14))`.
- wasmtime run on wasm32-gc component: `multi_clause_functions` →
  `zero other false 7 0 goodbye pong zzz life unknown`,
  `clause_guards` → `pos neg zero right left center`; both match
  `.expected` byte-for-byte, and the wasm32 linear runs match too.
- `verify lane` PASS (thin-wrapper advisories are module-boundary
  facades consistent with the existing match.ark / ctx_struct_lookup
  convention; the one ≥200-char line introduced mid-work was split).

Known gaps (not part of this issue's acceptance):

- `fn f(x) = g(x)` arg pass-through with an unannotated managed
  param still relies on callee-sig stamping at arg sites.
- CoreHIR-path scrutinee stamping on GC is not implemented; the
  AstNode path covers the promoted fixtures.
