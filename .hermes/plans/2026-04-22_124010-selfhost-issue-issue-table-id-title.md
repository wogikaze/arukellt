# Non-Selfhost Implementation-Ready Issues

Generated: 2026-04-22

## Filters applied

- selfhost / phase5 / phase6 / phase7 フォーカス issues を除外
- `blocked-by-upstream` を除外
- `Status: done` を除外
- dependency が open issue を指している場合も除外

---

## Table

| #   | Title                                                     | 計画                                                                                      |
|-----|-----------------------------------------------------------|-------------------------------------------------------------------------------------------|
| 044 | std::collections::hash: HashMap/HashSet 汎用化           | `std/collections/hash.ark` に generic HashMap<K,V>/HashSet<T> 追加 + fixture              |
| 045 | std::collections: Deque, PriorityQueue                   | `std/collections/deque.ark`, `priority_queue.ark` 新規 + fixture                          |
| 047 | std::collections: Arena, SlotMap, Interner / Rope        | `std/collections/arena.ark` 他 + fixture (v3 non-blocking)                                |
| 051 | std::time + std::random: 時刻・期間・乱数                | `std/time/mod.ark`, `std/random/mod.ark` 追加 + WASI clock/random binding                 |
| 112 | bench: C/Rust/Go/Grain 自動比較スクリプト                | `scripts/util/bench_compare_langs.py` 追加 + README                                       |
| 285 | Legacy lowering path 隔離・撤去                           | `crates/ark-*/` legacy flags に deprecated guard + CoreHIR stub 条件分岐削除              |
| 513 | Stdlib: prelude 直叩き前提を減らし wrapper surface を優先 | `std/` 各モジュールの公開 API を整理し prelude.ark の unsafe re-export を削減             |
| 514 | Stdlib: 実装品質監査 (hash/parsing/collection)           | `std/` 全体を走査し algorithm の O(N²) / weak hash 箇所を特定 → 修正 or issue 分割       |
| 516 | Stdlib: raw helper と推奨 facade の境界再設計            | `std/` の `_raw` 関数群に `#[internal]` アノテーション + 推奨 facade 関数を追加          |
| 517 | Stdlib: canonical naming / module layering 第2監査       | env/text モジュール inventory + snake_case 統一 + module path 整理                        |
| 520 | Stdlib: allocation/complexity/perf footgun 監査          | `std/` 全体で alloc/free パターンを走査 → hot path の allocation を削減                  |
| 523 | Stdlib TOML: parse failure / supported subset contract   | `std/toml/mod.ark` に parse error 型定義 + unsupported 仕様を docs に明記                |
| 524 | Stdlib FS: `exists` の意味を path existence に揃える     | `std/fs/mod.ark` の `exists` を path-existence のみに縮退 or `try_open` probe に変更     |
| 531 | Scripts consolidation to Python manager (Epic)           | Phase 2 以降: 残シェルスクリプトを `scripts/manager.py` サブコマンドに移植               |
| 541 | bench: struct graph (nested structs / recursive types)   | `benchmarks/bench_struct_graph.ark` + expected + baseline                                 |
| 542 | bench: error chain (Result/error propagation)            | `benchmarks/bench_error_chain.ark` + expected + baseline (dep #515 merged ✓)             |
| 544 | bench: suite reorganization and docs integration         | `benchmarks/` ディレクトリ整理 + `docs/benchmarks/` 更新                                  |
| 545 | bench: real-world workloads                              | JSON parse / file-process / sort workload fixtures を追加                                  |
| 546 | release: binary smoke tests                              | `scripts/manager.py verify` に smoke test ステップ追加 + CI job                           |
| 547 | release: determinism check                               | 2-build wasm sha256 比較スクリプト + CI gate                                               |
| 550 | release: formatter CLI-LSP parity                        | formatter 出力を CLI / LSP で diff 比較するテスト fixture 追加                            |

---

## Wave 候補 (PRIMARY_PATH 非重複)

Wave 2 推奨:
- #541 bench-struct-graph (benchmarks/)
- #542 bench-error-chain (benchmarks/)
- #544 bench-suite-reorg (benchmarks/ — #540 close 済みなので解放)
- #523 stdlib-toml-failure-contract (std/toml/)
- #524 stdlib-fs-exists-semantics (std/fs/)

Wave 3 推奨:
- #514 stdlib-quality-audit (std/ 横断 read-only 監査 → report)
- #517 stdlib-naming-v2 (std/ env+text)
- #546 release-smoke-tests (scripts/ + .github/)
- #547 release-determinism (scripts/ + .github/)
