---
Status: done
Created: 2026-09-28
Updated: 2026-09-29
ID: 907
Parent: 852
Track: language-v2
Depends on: none
Related: "#880, #896, #895, #852"
Orchestration class: design-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 2
Source: "#880 audit residual — bench_stdlib_toml runs on wasm32 linear post-#896 but produces 1000300000 instead of 8000 (wasm32-gc correct); identical on pre-session post891 s2"
---

# 907 — bench_stdlib_toml が wasm32 linear で誤値（GC は 8000、linear は 1000300000）

## Scope

`benchmarks/bench_stdlib_toml.ark`（`std::toml` の parse / get /
as_* / stringify を 100 iterations 反復する workload）は、
wasm32-gc component では期待値 `8000` を正しく観測するが、
wasm32 linear-memory core module では `1000300000` を出力する
（= 全 iteration で error / fallback branch 側に落ちた checksum）。
コンパイルは成功し、trap もしないため、linear backend 上の
silent semantic divergence であり、#896（metadata arena 拡大）で
実行自体が可能になった後に初めて観測可能になった潜在的な
fallback 正確性バグである。

- wasm32-gc component: `8000`（correctness=pass、benchmark runner
  の .expected と一致）。
- wasm32 linear: `1000300000`（correctness=fail）。実行は trap
  せず完走する。
- 同一出力が post891 s2（本セッションの typechecker/emit 変更
  以前の artifact）でも再現することを確認済み — 既存バグであり
  #903–#906 系の regression ではない。

## Evidence

- `benchmarks/bench_stdlib_toml.ark`（`.expected` = `8000`）。
- 再現コマンド: `scripts/run/arukellt-selfhost.sh compile
  --target wasm32 -o toml.wasm benchmarks/bench_stdlib_toml.ark &&
  wasmtime run --dir=. toml.wasm` → `1000300000`。`--target
  wasm32-gc --emit component` 経路は `8000`。
- current-wasm32.json (2026-09-28 quick run): `toml_ops`
  `status=ok`, `stdout="1000300000"`, `correctness=fail`。
  current-wasm32-gc.json 同 run は `correctness=pass`。
- 1000300000 = 100 iter × 1000000（match の最深 fallback branch が
  全 iteration で選択された形）。parse 自体の早期失敗または
  `toml_get` の lookup 系が linear で常に None / Err 側へ落ちる
  可能性が高い。

## Design notes

- #879 の 2026-09-27 paired benchmark run では wasm32 linear は
  allocation-heavy workload 群が metadata arena 枯渇で trap して
  いた（8/20 trap）。#896 で arena が 64 KiB→64 MiB に拡大し、
  本 workload は trap せず完走するようになった — 実行経路が
  通ったことで、それまで観測不能だった fallback の意味論差分が
  表面化した形。
- 再現 probe として stdlib_toml fixture 群との差分絞り込みから
  始める（`toml_parse` 単発は stdlib_toml fixtures で正常に
  動くなら、ループ内 accumulation / 反復再利用に起因する linear
  GC 生存性・store 経路の問題の可能性）。
- 修復候補の方向: (a) linear backend の match arm / Option
  payload 読み出し経路、 (b) linear GC での文字列 / struct field
  store または load の alignment・lifetime、 (c) iterate 中の
  slot reuse。いずれも emit / runtime 層の調査が必要。

## Acceptance

- [x] `bench_stdlib_toml` が wasm32 linear で `8000`（GC と同一
      の観測結果）を出力する。
- [x] 再現 root cause が特定され、線形 fallback 側の emit /
      runtime で修正される。
- [x] linear-only の回帰 fixture（または bench 系の観測）が追加
      される。

## Required evidence

- implementation commit SHA
- wasm32 linear 実行での正しい出力（`8000`）の receipt

## Done (2026-09-29)

### Root cause

`core_match.ark` の scrutinee 直値解決が expr kind を確認せず
`mir_core_lookup_local(ctx, scrut_expr)` を呼んでいた（cf18eba95 で
`is_ident` ガードが除去された回帰）。CoreHIR の `FIELD_ACCESS` expr は
`text` にレシーバではなくフィールド名だけを持つ（`parse_dot_suffix`
が `Token_text(field_tok)` を格納）ため、`match ks.parts` が
`ctx_lookup_local("parts")` で同名の無関係 local — 兄弟 arm の
`Result::Ok(parts)` 束縛 — に誤解決された。wasm32 emit では scrutinee
が生きている `ks.parts` 投影ではなく、その未初期化 / stale local の
`i32.load` を tag として読み、payload 抽出も同じ wrong base から読んで
いた。`toml_parse` では header-path arm の `parts` 束縛が衝突し、
kv arm の `toml_insert_key` に `len == 0` の腐った vec が渡され
`"empty key"` Err → 全 iteration fallback branch → `1000300000`。
GC 経路は同名 local の誤解決が typed ref の `struct.get` 型不一致として
表面化せず正しく projection を emit していた。

### Fix

`src/compiler/mir/lower/core_match.ark`: scrutinee の直値 lookup を
`mir_core_expr_is_ident || mir_core_expr_is_path` に限定（`body_match.ark`
の `NK_IDENT` ガードと同じ規約。path は `a::b` 形式で local 名と衝突
しない）。`CALL` / `METHOD_CALL` / `FIELD_ACCESS` / `STRUCT_LIT` 等、
member/callee 名を text に持つ kind は `lower_core_expr` 経由で正しく
評価される。

### Evidence

- `bench_stdlib_toml` wasm32 linear: `8000`（correctness=pass）。
- 最小再現（`.build/ark907` probe8/9 + fixture化）: pre-fix s2 は
  `a/err/err/err`（stale local の tag != 0 → Err arm）または
  `0xfffffffc` trap（未初期化 slot の payload deref）、post-fix は
  `a/b/h/k`。
- 回帰 fixture: `match_extensions/match_field_access_scrutinee.ark`
  を `run:` + `t3-run:` に登録（pre-fix `a/err/err/err`、post-fix
  両 backend で `a/b/h/k`、`.expected` 一致）。
- `selfhost fixture-parity` 全体: PASS=1525 FAIL=259 — fail 数は
  pre-fix と同一（spot-check は post891 でも同一再現の pre-existing
  backlog のみ）。新規 regression 0 件。
- `verify lane`、issue index 再生成を実施。

### Residual（本 issue の scope 外）

- probe 中に `Result<i64>` payload + while 内 `i64` 累算の組み合わせで
  linear emit が `expected i64, found i32` の invalid wasm を出す兆候を
  観測したが、最小再現には至らず本 issue とは別要因の可能性。再現したら
  別 issue で追跡する。
