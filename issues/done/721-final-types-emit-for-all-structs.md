---
Status: done
Created: 2026-07-15
Updated: 2026-10-12
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

Arukellt にはユーザー向けの継承機能がないため、
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

- [x] struct の final 判定ロジックが追加される
      （実装場所: typechecker ではなく emit 層。Arukellt にはユーザー宣言の
      サブタイピングが存在しないため「user struct = final」は emit 時点で
      一意に決まり、型チェッカー側の判定パスは冗長（YAGNI / ADR-048）。
      判定は `gc_struct_sig_as_final`
      （src/compiler/wasm/sections_types_gc.ark）が `GS_f*` → `SubF_GS_f*`
      へ変換する単一の funnel で行い、enum ベース型だけが `Sub0_` を保持する）
- [x] 継承されない struct が `(sub final ...)` で出力される
      （commit ba33c5405。closure_capture/capture_struct で
      `(sub final (struct ...))` を確認）
- [x] enum バリアントは従来通り `(sub final ...)` で出力される（既存動作維持）
      （`SubF+14_*` → `(sub final 28 (struct ...))`、
      regression/type_table/enum_match_variants.ark で確認）
- [x] enum ベース型は `(sub ...)` (open) のまま維持される
      （offset 14 の `Sub0_GS_f0_i32` → type 28 `(sub (struct ...))`、
      モジュール内で唯一の open subtype）
- [x] wasmparser validation が全 fixture で通る
      （canonical gate `python3 scripts/check/check-t3-wasm-validate.py -j 8`:
      544 pass / 0 validate-fail / 0 compile-fail / 2 skip — skip は
      stdlib_hashmap 2 件の linear-host compile timeout で、gc host では
      ~0.7s でコンパイル・検証・実行できることを別途確認済み）
- [x] wasmtime 46 で全 `t3-run` fixture が実行できる
      （全 489 件を compile → validate → P2 component → wasmtime run で検証:
      485 pass。残り 4 件は本変更と無関係 — runtime trap 2 件は
      `sub final` → `sub` 書き換え A/B で同一 trap を再現（既存バグ、
      issue #920 で追跡）、compile timeout 2 件は linear-host の
      mark_candidate 病理（#851）で gc host では全 pass。
      receipt: `docs/research/receipts/721-t3-run-execution-20261012.json`）
- [x] バイナリサイズの変化を計測・記録する
      （commit ba33c5405 記録: capture_struct 4336 → 4372 bytes (+36)。
      2026-10-12 再計測: 同 fixture 現在 4633 bytes。型セクションは
      final 化で縮小ではなく微増 — `sub final` ラッパの型宣言オーバーヘッド
      1 byte/型 + rec-group 構造。JIT devirtualization / ref.cast 高速化が
      本来の目的でありサイズ縮小は期待されていなかった）

## Implementation cost estimate

ADR-008 に基づく推定:

| 工程 | コスト |
|------|--------|
| 型チェック: `final` 判定ロジック | 小 (1–2 日) |
| T3 emitter: `sub final` 出力 | 小 (0.5 日) |
| stdlib の `final` 候補の特定・適用 | 中 (3 日) |
| **合計** | **4–6 日** |

## Resolution notes (2026-10-12)

実装は commit ba33c5405 で着地済み。final 判定は typechecker ではなく
emit 層に置いた: 言語にユーザー宣言のサブタイピング構文が存在しないため
「user struct = final」は emit 時点で一意に決まり、判定を表す state を
型チェッカーに持たせる必要がない（ADR-048 の KISS/YAGNI）。実装上の判定は
`sections_types_gc.ark` の `gc_struct_sig_as_final`（`GS_f*` →
`SubF_GS_f*`）と固定型テーブル（全 struct エントリが `SubF_GS_*`、
enum ベース型のみ `Sub0_GS_f0_i32` を保持）に集約されており、canon
type への入口は `append_gc_struct_sigs` の単一 funnel のみ。

検証（正規コマンド / 正規パイプライン）:

- `python3 scripts/check/check-t3-wasm-validate.py -j 8`（verify quick の
  canonical gate）: 544 pass / 0 validate-fail / 0 compile-fail / 2 skip。
- 全 489 `t3-run:` fixture を fixture-parity と同一パイプライン
  （compile → validate → P2 component → wasmtime 46 run → .expected 比較）
  で実行: 485 pass、2 件の runtime trap は wasm-level で `sub final` →
  `sub` 書き換え A/B を行い同一 trap を再現 → 本変更と無関係の既存バグ
  として issue #920 に切り出し。2 件の compile timeout は linear host の
  既知の遅延クラス（#916 notes 同様; #851 mark_candidate 病理）で、
  gc selfhost host では ~0.7s で compile / validate / 実行 / golden 一致。
  証拠: `docs/research/receipts/721-t3-run-execution-20261012.json`。
- バイナリサイズ: commit 時点で capture_struct 4336 → 4372 bytes (+36B)。
  final 化で型セクションは縮小ではなく微増する（sub-final wrapper の
  宣言オーバーヘッド）。目的は JIT devirtualization / ref.cast 削減であり
  サイズ縮小は期待値ではなかった。

## Related

- ADR-008: WasmGC Post-MVP 拡張機能（#4 Final Types）
- ADR-002: Memory Model (Wasm GC 採用)
- ADR-007: コンパイルターゲット整理（wasm32-gc）
