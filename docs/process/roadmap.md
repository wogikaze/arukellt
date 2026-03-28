# Arukellt v1–v5 ロードマップ概要

> **Source of truth**: この文書は `docs/process/roadmap-v{N}.md` と `docs/process/roadmap-cross-cutting.md` の索引であり、全体設計原則 (第 0 章) を定義する。
> 各版の実装状況は `docs/current-state.md` を参照すること。

---

## 版の一覧と中核目標

| 版 | 状態 | 中核目標 | 詳細 |
|----|------|---------|------|
| v1 | **完了** (2026-03-27) | Wasm GC ネイティブ対応 | [roadmap-v1.md](roadmap-v1.md) |
| v2 | 未着手 | Component Model 完全対応 | [roadmap-v2.md](roadmap-v2.md) |
| v3 | 未着手 | 標準ライブラリ整備 | [roadmap-v3.md](roadmap-v3.md) |
| v4 | 未着手 | 最適化 (4 軸定量目標) | [roadmap-v4.md](roadmap-v4.md) |
| v5 | 未着手 | セルフホスト | [roadmap-v5.md](roadmap-v5.md) |

横断事項 (依存関係マップ・リスク管理・意思決定基準) → [roadmap-cross-cutting.md](roadmap-cross-cutting.md)

---

## 第 0 章: 全体設計原則

以下の 10 原則は v1 から v5 まで一貫して適用される。各原則は「Arukellt でどう実装するか」まで具体化している。

### 原則 1: LLM-friendly 設計の一貫性 (ADR-003, ADR-004)

Arukellt は「LLM がコード生成・理解・変換する際にエラーを起こしにくい」ことを言語仕様の最優先目標とする。この方針は v5 まで変更しない。

**適用規則**:
- **ADR-003 (モノモーフィゼーション)**: ジェネリクスは型パラメータ最大 2 個、ネストジェネリクス (`Vec<Vec<T>>`) は v3 で必要性を再評価する。解禁する場合は ADR-009 として記録。v1–v2 ではネストジェネリクスを禁止。
- **ADR-004 (トレイト段階的導入)**: v3 で P3 (traits) 、v4 で P4 (methods) を評価する。各フェーズの開始条件: 前フェーズの全テストが通り、ADR-004 補遺に設計が記録されていること。v2 完了前にメソッド構文を追加しない。
- **予測可能なコード生成**: T3 の GC-native emitter は同一 MIR から同一 Wasm バイナリを生成すること。非決定的な emitter は許容しない (再現ビルド検証の前提)。

### 原則 2: IR 3 層の維持

```
フロントエンド: Lex → Parse → Bind → Analyze → Resolve → Check + BuildCoreHIR
ミドルエンド:   LowerToMIR → MIRValidate → MIROptimize
バックエンド:   BackendPlan → WasmEmit / LLVMEmit → BackendValidate
```

- **CoreHIR (`crates/ark-hir`)**: `INTERFACE-COREHIR.md` の凍結仕様を維持。フロントエンド変更は CoreHIR インタフェースを通じてのみバックエンドに影響する。
- **MIR (`crates/ark-mir`, `MirModule`, `TypeTable`)**: v4 で最適化パスを `MIROptimize` フェーズに追加する。最適化パスは `crates/ark-mir/src/passes/` に独立ファイルで実装し、`--opt-level` フラグで個別 on/off 可能にする。
- **Provenance 統一**: `MirModule` の `Provenance::LegacyAst` を v3 完了時点で廃止し、`CoreHir` に統一する。廃止時期は ADR-010 として記録。
- **バックエンド分岐**: `BackendPlan` に `EmitKind::Component` を v2 で追加、`EmitKind::Wasm` と `EmitKind::Llvm` を維持。

### 原則 3: ABI の 3 層保護 (ADR-006)

ADR-006 の 3 層 ABI (internal / Wasm public / native C ABI) は v5 まで不変。v2 で Layer 2B (Component Model canonical ABI) を有効化するが、Layer 4 の追加は引き続き禁止。

**v2 での変更範囲**:
- `crates/ark-wasm/src/component/` の `wit.rs`, `canonical_abi.rs` を拡張。
- `--emit component` フラグの有効化。これは Layer 2B の解禁であり、Layer 4 追加ではない。
- Component Model の有効化に際して ADR-008 を新規作成し、`wasm-tools component new` の内製化 vs 外部依存の判断を記録する。

### 原則 4: ランタイム責務の明確な境界

| 責務 | 担当 |
|------|------|
| Heap 管理 (alloc/GC) | Wasm GC ランタイム (HostGC) |
| I/O (stdout/stderr/fs) | WASI P2 (T3 の場合) |
| コンパイル時型チェック | `ark-typecheck` |
| ABI 変換 (canonical ABI) | `ark-wasm/component/canonical_abi.rs` |
| 線形メモリ | WASI I/O バッファのみ (1 ページ固定, T3) |
| 関数ポインタ間接呼び出し | `call_ref` (Table なし) |

T3 の線形メモリ 1 ページ固定は v5 まで変更しない。WASI P3 (async) は T5 スコープであり v5 では扱わない。

### 原則 5: 標準ライブラリ設計方針

- **`std/manifest.toml`** は stdlib の単一ソース。新しいビルトインの追加は manifest.toml の更新を伴う。
- **intrinsic vs Ark-native の基準**: Wasm 命令に直接マップできる操作は intrinsic (`__intrinsic_*`)。組み合わせで表現できる操作は Ark-native (`std/prelude.ark`)。
- **命名規約移行**: 現行の `Vec_new_i32`, `map_i32_i32` はモノモーフ名。v3 でジェネリック API に移行する。移行ガイドを `docs/migration/v2-to-v3.md` に書く。
- **API 安定性**: Stable / Unstable / Deprecated の 3 段階。v3 リリース後に Stable とした API は v4・v5 で破壊的変更をしない (deprecation 1 マイナー版経過後のみ)。

### 原則 6: テスト戦略

- **fixture harness (346+ 件)**: `tests/fixtures/manifest.txt` 駆動。新しい言語機能は fixture を追加してから実装する (TDD)。
- **カテゴリ管理**: `t3-compile:` (コンパイル検証) / `t3-run:` (実行検証) / `component:` (v2 追加) / `bench:` (v4 追加)。
- **unit tests**: 各クレートの `#[test]` 。現行 95 件。新クレート (`ark-mir/passes/`) 追加時は unit test を同梱する。
- **e2e テスト**: Component 相互運用テストは v2 から `tests/e2e/` に追加する。
- **セルフホスト検証**: v5 で Stage1/Stage2 の fixpoint 検証を `scripts/verify-bootstrap.sh` として実装。

### 原則 7: ベンチマーク戦略

- **既存 5 ベンチマーク** (fib, vec-ops, string-ops, struct-create, parity-check): `benchmarks/` に固定。
- **拡張タイミング**: v4 で binary_tree(depth=15), json_parse を追加。
- **比較対象**: v4 から C (gcc -O2), Rust (--release), Go を対象に追加。
- **T1/T3 パリティ計測**: v2 で T1 vs T3 のバイナリサイズ・実行時間比較表を `docs/process/benchmark-results.md` に追加。
- **CI 組み込み**: v4 で `scripts/verify-harness.sh` の perf gate を拡張し、閾値超過を failure にする。

### 原則 8: 互換性と破壊的変更

- **CLI 互換性**: `--emit wasm` はデフォルトで T3 出力。`--emit component` は v2 で有効化。`--target t1` で T1 を明示指定。既存の引数体系を変えない。
- **stdlib API 移行**: v3 でモノモーフ名からジェネリック名へ変更する場合、旧名を 1 マイナー版 Deprecated として残す。`docs/migration/v2-to-v3.md` に移行手順を書く。
- **IR 変更**: `MirModule`, `TypeTable`, `CoreHIR` のフィールド変更は ADR として記録する。バックエンドとフロントエンドに影響する変更は個別に対応計画を書く。

### 原則 9: ドキュメント方針

- **`docs/current-state.md`**: 各版のリリース時に必ず更新する。実装状況のソース。
- **ADR**: 設計判断ごとに記録。判断の理由・却下した代替案・影響範囲を含む。現行 ADR-001–ADR-007; v2 で ADR-008 追加予定。
- **`docs/migration/`**: 版間の破壊的変更に対して移行ガイドを書く。`v0-to-v1.md` は既存。`v1-to-v2.md`, `v2-to-v3.md`, `v3-to-v4.md`, `v4-to-v5.md` を各版リリース時に作成。
- **言語仕様凍結**: v5 着手前に `docs/language/spec.md` の凍結版を作成し、以降の仕様変更は ADR 必須とする。

### 原則 10: リリース判定方針

リリース可能の定義: 「`scripts/verify-harness.sh` の全ゲート通過 + 版固有の追加ゲート通過 + 必須ドキュメント完備」。

**全版共通ゲート (現行 16 点)**:
1. docs 構造チェック
2. ADR 判定
3. 言語仕様
4. clippy (--deny warnings)
5. build (`cargo build --workspace --exclude ark-llvm`)
6. unit tests
7. fixture harness (全件 pass)
8. stdlib manifest 整合
9. baseline 収集
10. perf gate (コンパイル時間・バイナリサイズ)
11–16: その他静的検証

**版固有の追加ゲート**:
- v2: `--emit component` の e2e smoke test (wasmtime + jco)
- v3: stdlib API 安定性マトリクスの確認
- v4: 4 軸定量目標の達成確認 (fib, vec-ops ベンチマーク)
- v5: Stage1/Stage2 fixpoint 検証 (`scripts/verify-bootstrap.sh`)

---

## セルフレビュー (ロードマップ概要)

1. **情報量の偏り**: 各版の詳細は `roadmap-v{N}.md` に等分配分。この概要は構造のみ。
2. **verify-harness.sh との整合**: 各版の完了条件を `scripts/verify-harness.sh` の拡張として実装可能な形で記述した。
3. **過去の問題の反映**: bridge mode 型不整合 (T3)、manifest.txt 不整合が原則 6・8 の背景。
4. **次版受け渡し条件**: 各 `roadmap-v{N}.md` 末尾に判定可能な形で記載。
5. **ドキュメントパス**: 全ドキュメントを `docs/` パス付きで指定。
6. **スコープの明確化**: 原則 1–10 で「やること」「やらないこと」を明文化。
7. **布石の確認**: v3 HashMap → v5 セルフホスト; v1 GcTypeRegistry → v2 canonical ABI 変換。
8. **過剰設計チェック**: 原則は Arukellt 固有の文脈に限定。
9. **LLM の逃げ道**: 全原則を「何を、どの条件で」まで具体化。
10. **ADR 反映**: ADR-001–007 を全て第 0 章に取り込んだ。
11. **クレート構成変更**: `crates/ark-mir/src/passes/` (v4) の追加を原則 2 に記載。
12. **issues 整合**: issues #019–#027 は v1 のスケルトンとして roadmap-v1.md に記載。
