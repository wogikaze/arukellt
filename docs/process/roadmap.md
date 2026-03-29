# Arukellt v1–v5 ロードマップ概要

> **Current state source of truth**: 実装の現在地と open work はまず `docs/current-state.md` と `issues/open/index.md` を参照してください。
> この文書は v1–v5 の大きな設計整理と履歴のための roadmap overview です。

---

## 版の一覧と中核目標

| 版 | 状態 | 中核目標 | 詳細 |
|----|------|---------|------|
| v1 | **完了** (2026-03-27) | Wasm GC ネイティブ対応 | [roadmap-v1.md](roadmap-v1.md) |
| v2 | **完了** (2026-03-28) | Component Model 完全対応 | [roadmap-v2.md](roadmap-v2.md) |
| v3 | **完了** (2026-03-28, tracked issues 039–059 are in `issues/done/`) | 標準ライブラリ整備 | [roadmap-v3.md](roadmap-v3.md) |
| v4 | 未着手 | 最適化 (4 軸定量目標) | [roadmap-v4.md](roadmap-v4.md) |
| v5 | 未着手 | セルフホスト | [roadmap-v5.md](roadmap-v5.md) |

横断事項 → [roadmap-cross-cutting.md](roadmap-cross-cutting.md)

## Current queue note

現在の active queue は `issues/open/` にある 7 件で、主に以下を扱っています。

- WASI Preview 2 native component output
- `std::host::*` namespace migration
- shared host capability rollout (`stdio` / `fs` / `env` / `process` / `clock` / `random`)

そのため、この roadmap は「現在の open issue 一覧」ではなく、中長期の設計整理として読んでください。

## v2 完了時の注記

- `--emit component` / `--emit wit` / `--emit all` は `wasm32-wasi-p2` で利用可能
- ADR-008 で component wrapping 方針を記録済み
- jco browser-side bindings は引き続き upstream blocked (`issues/blocked/037`)
- v2 の詳細な完了条件と制約は [roadmap-v2.md](roadmap-v2.md) と `docs/current-state.md` を参照

## v3 完了時の注記

- stdlib track の issues 039–059 は `issues/done/` に移動済み
- manifest-backed stdlib reference / module docs / stability docs は生成済み
- prelude migration と v3 fixture integration は完了済み
- 現在の open queue は v3 stdlib ではなく WASI / host capability track に移っています

---

## 第 0 章: 全体設計原則

以下の原則は roadmap の全体像を示すために残しています。実際の現状態の判定には `docs/current-state.md` を優先してください。

### 原則 1: LLM-friendly 設計の一貫性

Arukellt は「LLM がコード生成・理解・変換しやすい」ことを最優先の設計目標に置きます。

### 原則 2: IR 層の責務分離

フロントエンド / ミドルエンド / バックエンドの境界を保ち、`ark-driver::Session` を通じて orchestration します。

### 原則 3: ABI 境界の保護

internal ABI / Wasm public ABI / canonical ABI の境界を分け、必要な変更は ADR で記録します。

### 原則 4: ランタイム責務の明確化

型検査・IR 変換・ABI 変換・host capability 付与の責務を混ぜないことを原則にします。

### 原則 5: stdlib の単一ソース

`std/manifest.toml` と `std/*.ark` / doc comments を stdlib surface の一次情報として扱います。

### 原則 6: manifest-driven verification

fixture harness は `tests/fixtures/manifest.txt` 駆動で、verification contract は `scripts/verify-harness.sh` が担います。デフォルトは fast local gate とし、fixture sweep などの重い検証は明示フラグまたは CI 側で実行します。

### 原則 7: perf gate の分離

重い perf 比較は correctness gate から分離しつつ、基準ケースは `tests/baselines/` に固定します。

### 原則 8: 破壊的変更は移行ガイド付き

CLI / stdlib / module surface の変更には migration docs を伴わせます。

### 原則 9: current-first documentation

履歴文書・設計文書・完了報告よりも `docs/current-state.md` を優先します。

### 原則 10: release = verification + docs

リリース可能性は verification gate 通過と必須 docs 更新で判定します。

---

## 読み方

- **今なにが open か** → `issues/open/index.md`
- **今なにが動くか** → `docs/current-state.md`
- **なぜその方針になったか** → `docs/adr/`
- **v1〜v5 の大枠設計** → この文書と `roadmap-v{N}.md`
