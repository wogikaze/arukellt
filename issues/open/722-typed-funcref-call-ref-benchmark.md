---
Status: open
Created: 2026-07-15
Updated: 2026-07-15
ID: 722
Track: wasm-feature
Depends on: none
Orchestration class: design-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 3
Source: ADR-008 改訂（2026-07）+ ADR-033 Phase A — Typed Function References は Wasm 3.0 で shipped 済み
---

# Typed Function References (`call_ref`) ベンチマーク計測

## Summary

Typed Function References は Wasm 3.0 で Phase 5 shipped 済み（ADR-008 改訂）。
wasmtime 46、V8 14.6 でデフォルト有効。

現在 Arukellt は `funcref`（untyped）+ `call_indirect` + function table で
クロージャ/HOF を実装している（ADR-033 Baseline）。typed function references
`(ref $func_type)` と `call_ref` 命令は未使用。

本 issue は ADR-033 の段階的移行計画（Phase A/B/C）をトラッキングする。
ADR-033 は「段階移行するという決定」のみを記録し、Phase の詳細は本 issue に委譲する。

## Current state

### クロージャ/HOF の実装

- `src/compiler/wasm/sections_table.ark:12-65` — funcref table + element section
- `src/compiler/wasm/call_indirect.ark:14-22` — `call_indirect` 命令の emit
- `src/compiler/mir/lower/call_indirect_emit.ark:17-34` — 間接呼び出しの MIR lowering

### 移行フェーズ（ADR-033 から委譲）

- **Baseline (now)**: `call_indirect` for all closure/HOF dispatch
- **Phase A (emitter audit)**: HOF call site のうち callee が既知の `ref.func`
  （direct function values, monomorphic callbacks）を特定し、型インデックスが
  静的に分かって table slot が不要な場合に `call_ref` を emit
- **Phase B (nullable refs)**: `Option<fn ...>` / nullable function-reference の
  null チェックを手動比較から `br_on_null` / `br_on_non_null` に切替
  （GC type system が許す場合）
- **Phase C (benchmark gate)**: 代表的な fixture で `call_indirect` vs `call_ref`
  の性能比較を実施し、≥5% improvement で `call_ref` を audited patterns の
  default に採用（issue #069 acceptance benchmark）

現在は Baseline で止まっている。

## What to measure

### 1. call site 分類

HOF / クロージャの call site を以下の3つに分類:

| 分類 | 説定 | 現在の emit | `call_ref` 適用可否 |
|------|------|------------|-------------------|
| **A: 静的直接** | 呼び出し先の関数が静的に分かる（monomorphic callback） | `call_indirect` | ✅ `call_ref` に切替可能 |
| **B: 静的型付き** | 呼び出し先の型は分かるが関数は動的（trait object 相当） | `call_indirect` | ✅ `call_ref` に切替可能（table 不要） |
| **C: 完全動的** | 呼び出し先も型も動的 | `call_indirect` | ❌ `call_indirect` のまま |

### 2. ベンチマーク項目

以下のベンチマークで `call_indirect` (現状) vs `call_ref` (Phase A) を比較:

- `benchmarks/` 配下の HOF を使うベンチマーク（`map`/`filter`/`fold` 等）
- クロージャキャプチャを含むベンチマーク
- 関数ポインタ渡しのベンチマーク

計測環境:
- wasmtime 46（`--invoke` method または CLI）
- V8 14.6 (Node.js 26) — ブラウザ向けの参考値

### 3. 計測指標

- 実行時間（中央値、p99）
- バイナリサイズ（table section 削減効果）
- 型セクションサイズ（typed funcref 型定義の追加）

## Acceptance criteria

### Phase A (emitter audit)

- [ ] HOF / クロージャ call site の分類（A/B/C）が完了する
- [ ] 分類 A（静的直接）の call site 数が把握できる
- [ ] `call_ref` を emit するプロトタイプ（実験ブランチ）が作成される

### Phase B (nullable refs)

- [ ] `Option<fn ...>` / nullable function-reference の null チェック箇所を特定
- [ ] `br_on_null` / `br_on_non_null` への切替可否を評価

### Phase C (benchmark gate)

- [ ] ベンチマークで `call_indirect` vs `call_ref` の性能比較が完了する
- [ ] バイナリサイズの変化が計測される
- [ ] ≥5% improvement の判断基準に対する評価が記録される
- [ ] 計測結果に基づき、Phase A 移行を進めるか/見送るかの推奨が記載される

## Note

- 本 issue は ADR-033 から委譲された Phase A/B/C の**計測・評価**が目的。
  本格的な emitter 変更は計測結果次第で別 issue にする。
- `call_ref` に移行しても `call_indirect` は完全には削除できない（分類 C のため）
- table section は分類 C が残る限り必要だが、サイズは削減される可能性がある

## Related

- ADR-008: WasmGC Post-MVP 拡張機能（#5 Typed Function References）
- ADR-033: Typed Function References (`call_ref`) HOF Migration
- ADR-002: Memory Model (Wasm GC 採用)
- ADR-007: コンパイルターゲット整理（wasm32-gc）
