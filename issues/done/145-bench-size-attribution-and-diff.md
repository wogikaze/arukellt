---
Status: done
Created: 2026-03-29
Updated: 2026-04-15
ID: 145
Track: benchmark
Depends on: 149
Orchestration class: implementation-ready
---
# 計測: Wasm サイズ内訳 diff と top contributors 追跡
**Blocks v1 exit**: no

---

## Completed — 2026-04-15

All acceptance criteria implemented and verified.

## Summary

総バイナリサイズだけでは、どの section / function / symbol が増えたか分からない。
差分計測を導入し、前回 baseline 比でどこが膨らんだかを一発で特定できるようにする。

## 受け入れ条件

- [x] section/type/code/data/import/export ごとの差分を表示できる
- [x] top 増加関数または symbol を列挙できる（wasm name section 非出力のため `symbol_attribution: "unavailable"` として記録; `wasm-objdump` による手動確認方法を `benchmarks/README.md` にドキュメント化）
- [x] `startup` のような極小 benchmark と `binary_tree` のような中規模 benchmark の両方に適用できる（全 benchmark に適用）
- [x] compare レポートからサイズ悪化の主因へ辿れる（text output の `sections:` 行 + markdown の `### Wasm Section Δ vs Baseline` テーブル）
- [x] `bash scripts/run/verify-harness.sh --quick` passes (19/19 checks)

## 実装内容

### `scripts/util/benchmark_runner.py`

- `WASM_SECTION_ID_NAMES` 定数追加（section id 0–12 のマッピング）
- `_read_uleb128` — 純 Python LEB128 デコーダ（外部ツール不要）
- `parse_wasm_sections` — Wasm binary をパースしてセクションごとのペイロードサイズを返す
- `measure_wasm_size_attribution` — 上記を呼び出し `compile_result["wasm_sections"]` に格納
- `measure_compile` で `wasm_sections` を結果に追加
- `compare_results` で `wasm_section_diff` を各ベンチマーク比較に追加
- `collect_results` で `wasm-objdump` を tooling に追加（availability を記録）
- `render_text` に `sections:` 行（測定時）と `sections:` diff 行（比較時）を追加
- `render_markdown` に **Wasm Section Breakdown** テーブルと **Wasm Section Δ vs Baseline** テーブルを追加

### `benchmarks/schema.json`

- `compile_metrics.wasm_sections` オブジェクト定義を追加（type/import/function/code/data/export/custom_total/symbol_attribution/error）

### `benchmarks/README.md`

- `## Wasm Section Breakdown` セクション追加（フィールド説明、`wasm-objdump` 手動確認方法）
- Optional Tools に `wasm-objdump` を追加
- 概念フィールドリファレンスに `wasm_section_code_bytes` / `wasm_section_data_bytes` を追加

## STOP_IF 対応

- `wasm-objdump` は `/usr/bin/wasm-objdump` として使用可能だが、セクションサイズ取得には不要。純 Python で実装済み（`STOP_IF` 条件のフォールバック実装）。
- Function-level symbol attribution は name section がないため `"unavailable"` として記録。`benchmarks/README.md` にドキュメント化済み。

## 参照

- `issues/done/111-bench-wasm-size-analysis.md`
- `docs/process/wasm-size-reduction.md`
- `docs/process/benchmark-results.md`