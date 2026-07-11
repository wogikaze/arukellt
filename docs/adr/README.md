# ADR

> このファイルは `python3 scripts/gen/generate-docs.py` により生成されます。
設計判断の記録。現行 reality の正本ではない。

## 現状スナップショット

- これらの文書は設計判断の記録であり、現行挙動の正本ではない。
- 現行の正本: [../current-state.md](../current-state.md)。

ステータスライフサイクル: [ADR-000-process.md](ADR-000-process.md)
（`PROPOSED` → `ACCEPTED` → `SUPERSEDED`。補助: `REJECTED` / `DEFERRED`）。

台帳検査: `python3 scripts/check/check-adrs.py`。

## 採択

| ファイル | タイトル | 要約 |
|----------|----------|------|
| [ADR-000-process.md](ADR-000-process.md) | ADR-000: ADR プロセスとステータスライフサイクル | ステータス: ACCEPTED — ADR の識別子・状態遷移・後継関係の規則を固定する |
| [ADR-001-harness-bootstrap.md](ADR-001-harness-bootstrap.md) | ADR-001: Harness Bootstrap Strategy | ステータス: ACCEPTED — NEPLg2・vibe-lang・wadoの合成ベースラインharnessを採用 |
| [ADR-002-memory-model.md](ADR-002-memory-model.md) | ADR-002: GC vs non-GC | ステータス: ACCEPTED — ベンチマーク実測（2026-03-25）により 選択肢 A: Wasm GC 前提を採用 |
| [ADR-003-generics-strategy.md](ADR-003-generics-strategy.md) | ADR-003: generics 戦略 | ステータス: ACCEPTED — Monomorphization（型ごとのコード生成）を採用する |
| [ADR-005-llvm-scope.md](ADR-005-llvm-scope.md) | ADR-005: LLVM IR バックエンドの役割制限 | ステータス: ACCEPTED — LLVM IRバックエンドはWasm意味論に従属 |
| [ADR-006-abi-policy.md](ADR-006-abi-policy.md) | ADR-006: 公開 ABI を 3 層に固定 | ステータス: ACCEPTED — 公開ABIは3層まで（内部・WASM・native） |
| [ADR-007-targets.md](ADR-007-targets.md) | ADR-007: コンパイルターゲット整理 | ステータス: ACCEPTED — ターゲットを3系統に確定（wasm32 / wasm32-gc / native） |
| [ADR-008-component-wrapping.md](ADR-008-component-wrapping.md) | ADR-008: Component Model ラッピング戦略 | ステータス: ACCEPTED — in-tree 実装により wasm-tools への依存を除去（前倒し完了） |
| [ADR-009-import-syntax.md](ADR-009-import-syntax.md) | ADR-009: Import 構文の決定 — ソースモジュール参照と Component Model 境界の分離 | ステータス: ACCEPTED — use std::host::stdioの::-separated形式をソースモジュール参照として確定 |
| [ADR-010-extended-const.md](ADR-010-extended-const.md) | ADR-010: Extended Const Expressions (Wasm) | ステータス: ACCEPTED — 実装見送り。heap pointer 初期化は単純定数で十分 |
| [ADR-011-wasi-host-layering.md](ADR-011-wasi-host-layering.md) | ADR-011: host-bound stdlib API は std::host:: に隔離する | ステータス: ACCEPTED — host-boundなstdlib APIはstd::host::に隔離 |
| [ADR-013-primary-target.md](ADR-013-primary-target.md) | ADR-013: wasm32-wasi-p2 をプライマリターゲットとして選定する | ステータス: ACCEPTED — wasm32-wasi-p2（旧称 T3）をプライマリターゲットとして選定 |
| [ADR-014-stability-labels.md](ADR-014-stability-labels.md) | ADR-014: 言語仕様と Stdlib API の安定性ラベル | ステータス: ACCEPTED — 4段階の安定性ラベル（stable/provisional/experimental/unimplemented）を採用 |
| [ADR-015-no-panic-in-user-paths.md](ADR-015-no-panic-in-user-paths.md) | ADR-015: ユーザー到達パスの No-Panic 品質基準 | ステータス: ACCEPTED — ユーザー到達パスでのpanic禁止 |
| [ADR-017-playground-execution-model.md](ADR-017-playground-execution-model.md) | ADR-017: Playground Execution Model and v1/v2 Product Contract | ステータス: ACCEPTED — client-side hybrid実行モデル（v1はサーバーサイドexecutorなし、v2はブラウザでcompile+run） |
| [ADR-018-language-docs-classification.md](ADR-018-language-docs-classification.md) | ADR-018: 言語ドキュメント分類 — Normative / Explanatory / Transitional | ステータス: ACCEPTED — 3つのドキュメントクラス（normative/explanatory/transitional）を採用 |
| [ADR-019-anchor-permalink-policy.md](ADR-019-anchor-permalink-policy.md) | ADR-019: リンクチェックカバレッジポリシー | ステータス: ACCEPTED — リンクチェックカバレッジポリシーを採用 |
| [ADR-021-playground-share-url-format.md](ADR-021-playground-share-url-format.md) | ADR-021: Playground Share URL Format — Encoding, Versioning, and Round-Trip Contract | ステータス: ACCEPTED — fragmentベースのshare URL形式（versioned path structure） |
| [ADR-022-playground-deployment-and-caching.md](ADR-022-playground-deployment-and-caching.md) | ADR-022: Playground のデプロイとアセットキャッシュ戦略 | ステータス: ACCEPTED — GitHub Pagesで静的ホスティング（Fastly CDN経由） |
| [ADR-023-package-registry-resolution.md](ADR-023-package-registry-resolution.md) | ADR-023: パッケージレジストリ解決の設計 | ステータス: ACCEPTED — Registry lookupモデル（local > workspace > registry）を採用 |
| [ADR-024-selfhost-mir-explicit-cfg-before-ssa.md](ADR-024-selfhost-mir-explicit-cfg-before-ssa.md) | ADR-024: Selfhost MIR は SSA 形成前に明示的 CFG を採用する | ステータス: ACCEPTED — Selfhost MIRはSSA形成前に明示的なCFGを採用 |
| [ADR-029-selfhost-native-verification-contract.md](ADR-029-selfhost-native-verification-contract.md) | ADR-029 — セルフホストネイティブ検証契約 | ステータス: ACCEPTED |
| [ADR-031-import-syntax-wit-unification.md](ADR-031-import-syntax-wit-unification.md) | ADR-031: import 構文と WIT パッケージ識別子の統合 | ステータス: ACCEPTED — 二層分離を確定。use は Layer S、import は Layer C に予約 |
| [ADR-033-call-ref-hof-migration.md](ADR-033-call-ref-hof-migration.md) | ADR-033: クロージャ呼び出しを call_ref に移行 | ステータス: ACCEPTED — 段階移行。table-free パターンが揃うまで call_indirect をベースラインとする |
| [ADR-034-component-composition-linking.md](ADR-034-component-composition-linking.md) | ADR-034: Component 合成を wac plug に委譲 | ステータス: ACCEPTED — Phase 3 wac 委譲 landed (#443, 2026-06-15) |
| [ADR-040-typed-mir-signature-registry.md](ADR-040-typed-mir-signature-registry.md) | ADR-040: Semantic Type Spine — 意味情報を保存する背骨 | ステータス: ACCEPTED — Semantic Type Spine（SignatureRegistry / MonoInstanceTable）を MIR の正本とする。実装進捗は issue #724 で追跡。 |
| [ADR-041-in-file-test-syntax.md](ADR-041-in-file-test-syntax.md) | ADR-041: In-file Test Syntax — test Declarations | ステータス: ACCEPTED — Phase 1 (構文・型チェック・ディスカバリ・カバレッジ採用 #715) 完了、Phase 2 (実行モデル) 未実装 |
| [ADR-043-wasm-gc-post-mvp.md](ADR-043-wasm-gc-post-mvp.md) | ADR-043: WasmGC Post-MVP 拡張機能 — 設計調査と Arukellt v5 評価 | ステータス: ACCEPTED — v4 では Post-MVP GC 拡張を実装しない; 本文は v5 設計の参考調査として保持 |

## 提案

| ファイル | タイトル | 要約 |
|----------|----------|------|
| [ADR-035-wasm-gc-implementation.md](ADR-035-wasm-gc-implementation.md) | ADR-035: Wasm GC Implementation Plan | ステータス: PROPOSED — 段階実装中（Phase 0 完了、Phase 1-3 部分完了、Phase 4 進行中） |
| [ADR-036-trait-stdlib-redesign.md](ADR-036-trait-stdlib-redesign.md) | ADR-036: Trait-based Stdlib Redesign Strategy | ステータス: PROPOSED — 688-697 完了後に実行される stdlib 再設計の戦略 ADR |
| [ADR-037-std-simd.md](ADR-037-std-simd.md) | ADR-037: std::simd — Explicit SIMD Library API | ステータス: PROPOSED — 明示的 SIMD ライブラリ API と v128 第一級型の導入を提案 |
| [ADR-038-operator-overload-traits.md](ADR-038-operator-overload-traits.md) | ADR-038: Operator Overload Trait Surface | ステータス: PROPOSED — #688 完了後に実装される演算子オーバーロードの設計 ADR |
| [ADR-039-question-mark-operator.md](ADR-039-question-mark-operator.md) | ADR-039: Question Mark Operator (?) and Error Conversion | ステータス: PROPOSED — #688/#692 完了後に実装される ? 演算子の設計 ADR |
| [ADR-042-intrinsic-layer-separation.md](ADR-042-intrinsic-layer-separation.md) | ADR-042: Intrinsic Layer Separation — 意味と実装の分離 | ステータス: PROPOSED |

## 保留

| ファイル | タイトル | 要約 |
|----------|----------|------|
| [ADR-004-P4-method-syntax-evaluation.md](ADR-004-P4-method-syntax-evaluation.md) | ADR-004 P4: メソッド構文の評価 | ステータス: DEFERRED — 評価保留（trigger待ち） |

## 後継済み

| ファイル | タイトル | 要約 |
|----------|----------|------|
| [ADR-008-wasm-gc-post-mvp.md](ADR-008-wasm-gc-post-mvp.md) | ADR-008: WasmGC Post-MVP 拡張機能 — 設計調査と Arukellt v5 評価 | ステータス: SUPERSEDED — 番号重複解消のため ADR-043 へ移番 |
| [ADR-025-use-paths-vs-wit-package-identifiers.md](ADR-025-use-paths-vs-wit-package-identifiers.md) | ADR-025: ソースモジュールパスと WIT パッケージ識別子 — 衝突ポリシーと構文探索 | ステータス: SUPERSEDED — ADR-031 に統合（探索メモ） |
| [ADR-026-import-vs-wit-package-syntax.md](ADR-026-import-vs-wit-package-syntax.md) | ADR-026: ソース import と WIT パッケージ構文 — 決定記録 | ステータス: SUPERSEDED — ADR-031 に統合 |
| [ADR-032-playground-compiler-wasm-runner.md](ADR-032-playground-compiler-wasm-runner.md) | ADR-032: プレイグラウンド v2 コンパイラ Wasm とブラウザ実行モデル | ステータス: SUPERSEDED — ADR-017 の v2 節へ統合 |
