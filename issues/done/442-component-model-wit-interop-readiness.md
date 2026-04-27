---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 442
Track: component-model
Depends on: 299, 300
Orchestration class: implementation-ready
Blocks v1 exit: False
Priority: 4
# Component Model: WIT / Component 生成を「出力できる」から「相互運用可能」に引き上げる
---
# Component Model: WIT / Component 生成を「出力できる」から「相互運用可能」に引き上げる

## Summary

Component Model 対応を、単に `--emit component` が動く状態から、実際に他言語・他runtimeと接続できる相互運用機能へ昇格させる。WIT生成・型マッピング・ABI整合を含む。

## Current state

- component emit は存在。
- canonical ABI coverage が不完全。
- async は未対応。
- downstream（jco など）未接続。

## Acceptance

- [x] WIT が正確に生成される。
- [x] import/export が他実装と接続可能。
- [x] 型（list/record/variant/result/resource）が正しくマッピングされる。
- [x] interop テスト（他言語）が追加される。
- [x] unsupported 機能は compile-time error になる。

## References

- `crates/ark-wasm/`
- `crates/ark-mir/`
- `docs/target-contract.md`
- `docs/current-state.md`