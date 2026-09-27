---
Status: open
Created: 2026-09-28
Updated: 2026-09-28
ID: 897
Parent: 852
Track: language-v2
Depends on: ""
Related: "ADR-058, RFC-011, #896, #852"
Orchestration class: implementation-ready
Orchestration upstream: 896
Blocks v{N}: language-v2
Priority: 1
Source: "#896 stage-3 self-compile receipt — parse_full bound to wrong FnSig"
---

# 897 — 修飾名呼び出しの signature 衝突と修飾型注釈の型同一性欠陥

## Scope

stage-3（s2 が `src/compiler/main.ark` を wasm32 へ再コンパイル）で、
`parse_full` が `ast::parse` の戻り値を `?` 付き Result として扱い
frontend が早期に誤診断を吐いて停止する。二つの欠陥が原因:

1. **修飾呼び出しの signature 衝突**:
   `typechecker/call_fn.ark` の呼び出し解決は `mod::f` で
   `fn_sig_index` を引き、miss すれば bare `f` へ退避して
   first-registered の `FnSig` を採用する。`mod::f` の
   signature は module-qualified key で登録されていないため、
   `types::parse` / `json::parse` / `ast::parse` のような
   bare-name 衝突では常に最初に登録された別 module の
   `FnSig` が選ばれ、戻り値型・引数型が静かに入れ替わる。
   resolver 側は qualified 名を受理済みなので診断も出ない。

2. **修飾型注釈の型同一性**: `-> mod::T` と書かれた戻り値注釈は
   `resolve_type_name` で leaf へ落とさず、`T` とは別 identity の
   named type を生成するため、宣言が `-> T` の関数と同じ型を
   指していても不一致として扱われる。

あわせて、compiler source 側に残っていた v2 strictness 違反
（record field 書き込みの非 `mut` binding、i64 literal suffix
不足、`emit_result_if_needed` arity、prelude auto-import
対象外名の未 import）が stage-3 を追加で阻害していた。これらは
同一の再現経路（stage-3 self-compile）を共有するため本 issue で
併せて扱う。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Reproducer

`python3 scripts/manager.py selfhost build-compiler` 後に flat
overlay 上で stage-3 を実行（`arukellt-s2-runtime` が
`src/compiler/main.ark` を `--target wasm32` へ compile）すると、
修正前は `parse_full` 内の `ast::parse` が `json::parse` の
`FnSig`（Result 戻り値）に束縛され、`?` marker が残ったまま
phase 6 まで到達しない。qualified annotation 側は
`-> types::Span` 等の注釈が `Span` と別 identity になり
return-type mismatch を引く。

## Acceptance

- [ ] module-qualified な `mod::f` / `mod::CONST` 呼び出しが
      所属 module の `FnSig` に束縛される（bare-name 衝突時も）。
- [ ] `-> mod::T` 型注釈が `T` と同じ型 identity に解決される。
- [ ] stage-3 self-compile が `compilation succeeded (phase 6)`
      で完走し、bootstrap wasm が生成される。

## Required evidence

- implementation commit SHA
- stage-3 self-compile 成功ログ（flat overlay）
- verify lane 結果

## False-done guard

- [ ] Acceptance の一部だけで done にしない
