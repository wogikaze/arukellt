---
Status: open
Created: 2026-07-15
Updated: 2026-07-15
ID: 721
Track: wasm-feature
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: ADR-008 改訂（2026-07）— Final Types は Wasm 3.0 で Phase 5 shipped 済み
---

# Final Types (`sub final`) を全 struct に適用

## Summary

Wasm GC の `sub final` 構文は Wasm 3.0 で Phase 5 shipped 済み（ADR-008 改訂）。
wasmtime 46、V8 14.6 (Chrome 146 / Node.js 26) でデフォルト有効。

現在 Arukellt は enum バリアントに対してのみ `(sub final ...)` を出力している
（`ctx_gc_enum_sig.ark:77,81` の `SubF` プレフィックス）。通常の struct は
`Sub0_`（open subtype）または `GS_f`（plain struct）で、final になっていない。

Arukellt にはユーザー向けの継承機能がない（ADR-004: v0 は trait なし）ため、
大半の struct はサブタイプされることがない。これらに `(sub final ...)` を適用することで:

1. JIT コンパイラのデバタイラライズ（devirtualization）が促進される
2. `ref.cast` のランタイムコストが削減される
3. 型セーフティの保証が強化される

## Current state

### 既に final を出力している箇所

- `src/compiler/mir/lower/ctx_gc_enum_sig.ark:77,81` — enum バリアント
  - `gc_enum_unit_variant_sig` → `SubF+14_S_f0_i32`
  - `gc_enum_payload_variant_sig` → `SubF+14_S_f0_i32` + fields

### final になっていない箇所

- `src/compiler/wasm/sections_types_emit.ark:23-24` — `Sub0_` プレフィックス
  → `(sub ...)` with 0 supertypes (open subtype)
- `src/compiler/wasm/sections_types_emit.ark:29-30` — `GS_f` プレフィックス
  → plain `(struct ...)` (sub なし)

### 型シグネチャ生成箇所

- `src/compiler/wasm/sections_types_gc.ark:138` — `Sub0_GS_f0_i32` (open subtype)
- その他 struct 型シグネチャを生成する箇所

## Plan

### Phase 1: 型チェック側 — final 判定ロジック

型チェッカーで「この struct がサブタイプされる可能性があるか」を判定する:

- **デフォルト: final** — 継承機能がないため、通常の struct は全て final
- **例外: open** — enum のベース型（バリアントの共通親）など、
  サブタイプが必要な型のみ open のまま

判定条件:
- ユーザー定義の struct → final
- enum のベース型 → open（バリアントがサブタイプとしてぶら下がるため）
- trait object 用の struct → open（将来の拡張のため、ただし v0 では該当なし）

### Phase 2: emitter 側 — `SubF` プレフィックスの出力

型シグネチャ生成箇所で、final と判定された struct に `SubF` プレフィックスを付ける:

- `sections_types_gc.ark` の `Sub0_GS_f0_i32` → `SubF_S_f0_i32` または同等
- `emit_canon_type_entries` は既に `SubF` を処理できる（`sections_types_emit.ark:20-21`）

### Phase 3: 検証

- wasmparser で validation が通ることを確認
- wasmtime 46 で実行できることを確認
- 既存の fixture が全て通ることを確認

## Acceptance criteria

- [ ] 型チェッカーに struct の final 判定ロジックが追加される
- [ ] 継承されない struct が `(sub final ...)` で出力される
- [ ] enum バリアントは従来通り `(sub final ...)` で出力される（既存動作維持）
- [ ] enum ベース型は `(sub ...)` (open) のまま維持される
- [ ] wasmparser validation が全 fixture で通る
- [ ] wasmtime 46 で全 `t3-run` fixture が実行できる
- [ ] バイナリサイズの変化を計測・記録する（final 化により型セクションが縮小する可能性）

## Implementation cost estimate

ADR-008 に基づく推定:

| 工程 | コスト |
|------|--------|
| 型チェック: `final` 判定ロジック | 小 (1–2 日) |
| T3 emitter: `sub final` 出力 | 小 (0.5 日) |
| stdlib の `final` 候補の特定・適用 | 中 (3 日) |
| **合計** | **4–6 日** |

## Related

- ADR-008: WasmGC Post-MVP 拡張機能（#4 Final Types）
- ADR-002: Memory Model (Wasm GC 採用)
- ADR-004: trait を v0 に入れるか（v0 は trait なし → 継承なし → final 可能）
- ADR-007: コンパイルターゲット整理（wasm32-gc）
