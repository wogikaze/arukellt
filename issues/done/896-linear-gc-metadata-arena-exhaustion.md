---
Status: done
Created: 2026-09-27
Updated: 2026-09-28
ID: 896
Parent: 874
Track: language-v2
Depends on: ""
Related: "ADR-058, RFC-011, #873, #874, #879, #852"
Orchestration class: implementation-ready
Orchestration upstream: 874
Blocks v{N}: language-v2
Priority: 1
Source: "#879 benchmark receipt — bench_stdlib_json_parse wasm32 runtime trap"
---

# 896 — linear GC metadata arena が collect 発火前に枯渇し unreachable で trap する

## Scope

wasm32 linear fallback の tracing GC runtime（#873/#874）で、
allocation 1 件ごとに 16 byte の metadata record を固定 64 KiB の
metadata arena（`LINEAR_GC_METADATA_BYTES`）に commit する。
collect は payload watermark `LINEAR_GC_COLLECT_THRESHOLD_BYTES`
（64 MiB）を関数 entry で超えたときだけ発火するため、
小さな record を多数確保する workload では arena が約 4096 record で
`GLOBAL_LINEAR_GC_ROOT_CURSOR` に衝突し、
`emit_linear_gc_runtime_commit_top_body` の guard が
`unreachable` を実行する（`src/compiler/wasm/linear_gc_runtime.ark`）。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Reproducer

`benchmarks/bench_stdlib_json_parse.ark`（parser::parse + json_get 系 +
stringify_pretty を 50 回反復）を wasm32 linear で実行すると、
13 回目前後の `JsonStrResult_new`（record commit）で trap する:

```
0: <wasm function 51>
1: JsonStrResult_new
2: json_parse_string_at
3: parse_value_at
4: parse_value_at
5: parse
6: main
wasm trap: wasm `unreachable` instruction executed
```

さらに #879 の 20-workload suite では allocation-heavy な wasm32 linear 側が
8 件（enum_dispatch / struct_graph / error_chain / http_parser /
log_processor / config_loader / json_parse / toml_ops）すべて同型の
`unreachable` trap で落ちる。enum_dispatch は 100k 回の enum literal
確保で arena 上限を大幅に超えるため、この枯渇が支配的な linear
runtime 障害として観測される。`root_push` 側の guard も同じ arena
境界を共有するため、root stack 側の混雑でも同症状が出る点に注意。

同じ source を wasm32-gc component として実行すると正しく `2550` を
返す。ループなしの単発実行、および各呼び出しの個別使用は linear でも
正常なので、parser ロジックではなく累積 allocation による枯渇が原因。

## Acceptance

- [x] `bench_stdlib_json_parse` が wasm32 linear で 50 回反復を
      完走し `2550` を返す（wasm32-gc と parity）。
- [x] metadata arena 枯渇前に collect または metadata compaction が
      発火する（payload watermark だけに依存しない）。
- [x] arena / root 領域が本当に尽きた場合の失敗は `unreachable` の
      まま残さず、識別可能な error 経路または documented OOM 規約にする。

## Required evidence

- implementation commit SHA
- positive fixture（多数小 record の反復 alloc が collect 後も存活）+
  必要なら diagnostic fixture
- wasm32 / wasm32-gc 両経路の compile/validate/runtime gate 出力

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #874 の担当項目を evidence 付きで更新する

## Open design questions

- 発火条件を `METADATA_CURSOR` が `ROOT_CURSOR` に近づいたら
  collect する形にするか、sweep が free list に戻す record を即座に
  reuse して commit をそもそも増やさない形にするか。
- metadata arena を拡張/動的化するか（root 領域と隣接するため
  配置契約の変更が必要）。

## Done (2026-09-28)

Implementation commit: `96b099b26`（design note: docs/plans/
wasm32-linear-memory-lowering.md 「#896 metadata arena exhaustion」）。

- `LINEAR_GC_METADATA_BYTES` を 64 KiB → 64 MiB へ拡大
  （live set ~4M records 相当、root stack と共有領域は維持）。
- entry safepoint に metadata-pressure 発火条件を追加:
  `root_cursor - metadata_cursor < LINEAR_GC_PRESSURE_FREE_BYTES`
  かつ free list 空なら collect。payload watermark 以外の発火経路で
  同じ safepoint を使うため健全性の根拠は不変（Open design
  questions の前者を採用）。
- `commit_top` が swept free-list record を再利用するようにし、
  cursor の単調増加を停止（後者も併用。reused record の旧 payload
  は untracked 化する bounded leak として記録）。
- 真の枯渇は bare `unreachable` から `proc_exit` status
  120（metadata arena）/ 121（root stack）の documented OOM
  exit code へ変更。

Evidence:

- `bench_stdlib_json_parse` wasm32 linear で完走し `2550`
  （wasm32-gc parity）を確認。
- 新 fixture `tests/fixtures/target_capability/
  linear_gc_metadata_pressure.ark`（小 record churn + live
  data 存活）を `run:` / `t3-run:` に登録、wasm32 p1 で
  compile+run PASS、wasm32-gc p2 component でも PASS。
- stage-3 self-compile（s2 → wasm32 compiler emit）が phase 6
  完走、s3 が実プログラムを compile 可能。
- `test_linear_gc_runtime_contract` 5 件、
  `test_linear_gc_runtime_e2e` 5 件、`verify lane` PASS。
- wasm32-gc emit 経路は無変更（parity は既存 #879 receipt）。
