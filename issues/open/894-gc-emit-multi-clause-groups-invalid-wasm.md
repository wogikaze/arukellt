---
Status: open
Created: 2026-09-27
Updated: 2026-09-27
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

- [ ] 上記 2 fixture が wasm32-gc emit で valid Wasm を生成する
- [ ] wasmtime run で expected output を両 backend で一致させる
- [ ] manifest で 2 fixture を `run:` + （parity 対象化なら）t3-run へ昇格する
- [ ] 回帰 fixture を残す（既存 2 fixture の昇格で足りるか判定）

## Required evidence

- implementation commit SHA
- wasm-tools validate / wasmtime run output on both backends
- 原因となった emitter 箇所と修正内容の要約

## False-done guard

- [ ] wasm32 linear のみで done にしない（GC emit が本 bug の核心）
- [ ] check-only 登録のまま run 昇格せず done にしない
- [ ] 親 #852 の担当項目を evidence 付きで更新する

