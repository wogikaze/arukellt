# Arukellt v0 設計進捗レポート

**最終更新**: 2026-03-25  
**ステータス**: ⚠️ 設計進行中 — 完了していない

> **注意**: このドキュメントは旧来「設計完了レポート」として作成されたが、その後新たな要件（wasm32/AtCoder ターゲット等）が追加されており、v0 設計は完了していない。未設計の要件については比較分析と段取りが必要。

---

## 完了したフェーズ

### Phase 1: 言語コア設計（完了）

すべての主要 ADR 決定完了：

| ADR | 決定内容 | ドキュメント |
|-----|---------|-------------|
| ADR-0001 | Harness bootstrap | `docs/adr/ADR-0001-harness-bootstrap.md` |
| ADR-002 | Wasm GC 採用 | `docs/adr/ADR-002-memory-model.md` |
| ADR-003 | 制限付き monomorphization | `docs/adr/ADR-003-generics-strategy.md` |
| ADR-004 | v0 trait なし | `docs/adr/ADR-004-trait-strategy.md` |
| ADR-005 | LLVM は Wasm 従属 | `docs/adr/ADR-005-llvm-scope.md` |
| ADR-006 | 3層 ABI | `docs/adr/ADR-006-abi-policy.md` |

**成果物**:
- 型システム設計: `docs/language/type-system.md`
- 構文設計: `docs/language/syntax.md`
- メモリモデル: `docs/language/memory-model.md`
- Wasm 機能分類: `docs/platform/wasm-features.md`
- コンパイラパイプライン: `docs/compiler/pipeline.md`

### Phase 2: LLM-readiness（完了）

LLMが壊しても直せる設計の完成：

| 項目 | 成果物 | 内容 |
|------|--------|------|
| 診断システム | `docs/compiler/diagnostics.md` | エラーコード分類、fix-it提案、LLM向けパターン |
| API 正規化 | `docs/stdlib/cookbook.md` | 正解パターン固定、禁止パターン明記 |
| Quickstart | `docs/quickstart.md` | 10分で書き始められるガイド |
| 統合仕様 | `docs/spec/v0-unified-spec.md` | v0 完全仕様（frozen） |
| stdlib 更新 | `docs/stdlib/core.md`, `io.md` | canonical style 統一 |

**設計原則**:
- 1エラー1原因（連鎖エラー抑制）
- expected/actual 必須
- fix-it 提供
- 関数呼び出しのみ（メソッド構文なし）
- 型特化関数（trait なし）

---

## 完成した設計ドキュメント群

### コアドキュメント（必読）

| ドキュメント | 内容 | 対象読者 |
|-------------|------|---------|
| `README.md` | プロジェクト入口 | 全員 |
| `docs/quickstart.md` | 10分ガイド | 言語利用者 |
| `docs/spec/v0-unified-spec.md` | v0 完全仕様 | 実装者 |
| `docs/stdlib/cookbook.md` | API パターン集 | 言語利用者・LLM |
| `docs/compiler/diagnostics.md` | エラー診断 | 実装者 |

### 設計詳細ドキュメント

**ADR（意思決定記録）**:
- 6 ADR 完了
- すべての主要決定が記録済み

**言語仕様** (`docs/language/`):
- memory-model.md - Wasm GC メモリモデル
- type-system.md - 型システム詳細
- syntax.md - 構文仕様
- error-handling.md - エラー処理

**プラットフォーム** (`docs/platform/`):
- wasm-features.md - Wasm 機能3層分類
- abi.md - ABI 方針
- wasi-resource-model.md - WASI 資源モデル

**標準ライブラリ** (`docs/stdlib/`):
- README.md - 追加順序
- cookbook.md - 使用パターン集
- core.md - core API 完全仕様
- io.md - I/O API 完全仕様

**設計トレードオフ** (`docs/design/`):
- gc-mono-tradeoff.md - GC+mono の緊張解決
- value-semantics.md - 値セマンティクス定義
- gc-c-abi-bridge.md - GC⇔C ABI 境界
- trait-less-abstraction.md - trait なし環境での抽象化
- reference-control.md - 参照過多への制御

**プロセス** (`docs/process/`):
- agent-harness.md - エージェントガイド
- decision-guide.md - 意思決定ガイド
- llm-readiness-plan.md - LLM 対応計画
- v0-scope.md - v0 スコープ定義

---

## プロジェクト統計

- **ドキュメント数**: 34 ファイル
- **コミット数**: 20+ commits（設計フェーズ）
- **ADR**: 6 決定完了
- **TODO タスク**: 29/29 完了（100%）

---

## 設計の特徴

### 強み

1. **一貫性**: すべてのドキュメントが統一された canonical style
2. **完全性**: 実装に必要な仕様がすべて揃っている
3. **LLM 対応**: エラー診断、fix-it、パターン集が完備
4. **検証可能**: `verify-harness.sh` で設計の完全性をチェック可能

### トレードオフの明確化

すべての重要なトレードオフを文書化：
- GC vs 手動管理 → GC（LLM フレンドリ優先）
- mono vs uniform → 制限付き mono（サイズと性能のバランス）
- trait vs 型特化 → 型特化（v0 の単純さ優先）

### 意図的な制約

v0 で意図的に除外した機能：
- trait / interface
- メソッド構文
- for ループ
- 演算子オーバーロード
- ネストしたジェネリクス

これらは v1 以降で再検討（設計の単純さを優先）。

---

## 次のフェーズ: Phase 3（実装）

### 優先順位

1. **コンパイラ実装**
   - Lexer: トークン定義完了（`docs/compiler/pipeline.md`）
   - Parser: AST 定義完了
   - Type Checker: 型推論アルゴリズム定義済み
   - Wasm Emitter: GC 型マッピング定義済み

2. **診断システム実装**
   - エラーコード: E00xx-E03xx 定義済み
   - fix-it 戦略: 定義済み
   - テストケース: パターン集完備

3. **標準ライブラリ実装**
   - Phase 1: mem, option, result
   - Phase 2: string, vec
   - Phase 3: fs, clock, random

### 実装の開始条件

- [x] すべての ADR 決定完了（ただし ADR-002 に補足決定追加済み）
- [x] 型システム設計完了（wasm32 lowering 追記）
- [x] 構文設計完了
- [x] エラー診断設計完了
- [x] API 仕様完了
- [x] 統合仕様書作成完了
- [x] 文書間一貫性確認完了（2026-03-24 横断レビュー）
- [ ] wasm32 ターゲット設計（arena/RC hybrid の詳細仕様）
- [ ] その他未洗い出し要件の比較設計

**判定**: 設計進行中 ⚠️ 実装開始前に残要件の設計が必要

### 文書間一貫性の確認（2026-03-24）

横断レビューで発見された8件の不整合を修正：

**高優先度（実装阻害レベル）:**
- [x] Generics 記法を `<T>` に統一（ADR-003, syntax, type-system, spec）
- [x] clone セマンティクスを deep copy に統一（value-semantics, memory-model）
- [x] I/O 境界名を Capabilities/IOError/print に統一（syntax.md）
- [x] Vec API 命名を型特化形式に統一（spec, syntax）

**中優先度（仕様明確化）:**
- [x] v0 禁止構文を AST から削除（pipeline.md を v0 仕様に合わせた）
- [x] wasi-resource-model.md のステータスを DECIDED に修正
- [x] quickstart.md の `RelPath_from` に `?` 演算子を追加

**低優先度（メタ文書）:**
- [x] completion-report.md を最新状態に更新

---

## 検証

設計の完全性検証：

```bash
./scripts/verify-harness.sh
```

**結果**: All checks passed (5/5) ✅

---

## 結論

**Arukellt v0 の設計は進行中。新たな要件が追加されており、未設計の比較・段取りが残っている。**

すべての主要決定が確定し、実装に必要な仕様が文書化されている。
LLM が壊しても直せる診断システム、正規化された API、完全な統合仕様により、
「書ける言語」から「壊れても直せる言語」への移行は進んでいるが、未設計の要件が残っている。

次のステップはコンパイラ実装の開始。

---

## 関連

- [README.md](../../README.md) - プロジェクト入口
- [v0-unified-spec.md](../spec/v0-unified-spec.md) - v0 完全仕様
- [llm-readiness-plan.md](llm-readiness-plan.md) - LLM 対応計画
- [decision-guide.md](decision-guide.md) - 意思決定ガイド
