# Selfhost compile latency: Memory64後の作業計画

ステータス: 計画（実装は開始条件を満たしてから）  
親 issue: [#829](../../issues/open/829-selfhost-latency-phase-reprofile-hotspot.md)  
調査メモ: [`../research/selfhost-compile-latency-root-cause.md`](../research/selfhost-compile-latency-root-cause.md)

## 目的

Memory64 / fixpoint問題の解消後、stage-3自己コンパイル約23.5分の内訳を実測し、最大のボトルネックを一つ選んで改善する。

現時点では`decl_emit`、`sync`、`propagate`、`emit`のどれが支配的か確定していない。phase時間を取得する前に#824の実装を開始してはならない。

## 開始条件

次の条件が満たされてから開始する。

* `selfhost fixpoint --build`がs2とs3を生成できる
* 生成されたs2とs3が`wasm-tools validate`を通る
* stage-3実行時に必要なMemory64設定が正式なrunnerへ反映されている
* flat overlayの同時更新競合がない
* 他のselfhost compileが動いていない状態で計測できる

開始条件を満たしていない場合は、性能改善へ進まず、未達条件を報告する。

## 作業1: 実時間phase計測を復旧する

`ARUKELLT_OVERLAY_KEEP_CLOCK=1`で生成する`arukellt-s2-clock.wasm`のvalidate失敗を修正する。

調査対象:

* `scripts/selfhost/checks.py::build_clock_capable_s2`
* `src/compiler/mir/lower/entry_timing.ark`
* `src/compiler/driver/debug.ark`
* `src/compiler/wasm/intrinsic_clock.ark`
* `src/compiler/wasm/intrinsics/helpers_clock_p2.ark`
* Memory64でのclock値、timestamp field、変換命令のi32/i64整合性

整数pack、黙殺truncate、clock stubへの復帰など、計測値を偽る回避策は禁止する。

完了条件:

* clock対応artifactが`wasm-tools validate`を通る
* `hello.ark`のcompileが成功する
* `--time`の`total`が外部壁時間と概ね一致する
* 通常のclock-stubbed s2生成経路を壊していない
* clock対応artifactをvalidateする自動テストまたはgateがある

既知症状（2026-07-20）: 既存`.build/selfhost/arukellt-s2-clock.wasm`は
`func 4697` で `expected i64, found i32`（wasm-tools / wasmtime compile とも失敗）。

## 作業2: machine-readableなlatency receipt

推奨出力: `.build/selfhost/selfhost-latency-receipt.json`

必須: 実行条件、artifact SHA、環境、Memory64/overlay、wall/RSS、全phase ms、
MIR規模 before→after、validate結果、warning量、phase合計と外部wallの整合、
可能なら1秒間隔の`rss_samples`（`/proc`）。

全0ms（clock stub）やphase欠落を成功扱いにしない。

## 作業3: 基準値を再計測する

| Workload | 目的 | 回数 |
|---|---|---:|
| `hello.ark compile` | 固定費 | 3 |
| `src/compiler/main.ark check` | frontend | 1 |
| pinned→s2 `build-compiler` | stage-2比較 | 1 |
| s2→s3 stage-3 compile | phase内訳 | 1 |

## 作業4: 次の実装を一つ選ぶ

| 判定 | 条件 | 次手 |
|---|---|---|
| A | `decl_emit` ≥ 35% total かつ ≥ 1.5× 第2位 | #824 |
| B | `propagate` / `sync` 支配的 | 専用 child issue |
| C | `emit` 支配的 | emitter 専用 issue |
| D | 時間分散・RSS増が主 | #826 |
| 判定不能 | phase不整合・競合・失敗 | 計測修復のみ |

## 作業5–6

選択した最大ボトルネックを一つだけ改善（before/after receipt）。  
`verify quick`、clock gate、reachability BFS、`fixpoint --build`、docs同期。  
#823は実時間 receipt 取得後に close review。#824は A 判定時のみ実装。
