---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 430
Track: playground
Depends on: 429
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 3
---

# Playground: diagnostics worker と incremental parse loop の性能予算を作る
- [x] 性能予算（例: 典型ファイルでの更新時間）が定義される。
# Playground: diagnostics worker と incremental parse loop の性能予算を作る

## Summary

UI スレッドを詰まらせずに diagnostics を出すため、worker 化、debounce、incremental-ish update の方針を決めて実装する。これは単なる設計 issue ではなく、実際に大きめの source でも入力体験を壊さない予算を設定する。

## Current state

- parser/formatter を Wasm で動かしても、毎キー入力で全文解析すると体感が悪化しうる。
- worker / debounce / caching の設計が未定。
- playground v1 の usable さはここに強く依存する。

## Acceptance

- [x] diagnostics が worker 経由で実行される。
- [x] debounce / update policy が実装される。
- [x] 性能予算（例: 典型ファイルでの更新時間）が定義される。
- [x] 簡単な perf 測定または benchmark がある。

## References

- ``crates/ark-parser/``
- ``crates/ark-diagnostics/``
- ``docs/index.html``