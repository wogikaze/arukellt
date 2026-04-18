# WASI P2 ネイティブ: P1 アダプタ不要のコンポーネント直接生成

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 074
**Depends on**: —
**Track**: wasi-feature
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v4 exit**: no

**Status note**: WASI feature — deferred to v5+. Requires WASI P2 runtime maturity.

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/074-wasi-p2-native-component.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

現在の Component Model 出力は「Core Wasm → WASI P1 adapter → Component」という
2段変換パイプラインを使っている (`wasm-tools component new` + P1 アダプタ)。
WASI P2 ネイティブ対応では、Core Wasm が直接 WIT インターフェースをインポート/エクスポートする
コンポーネントを生成し、P1 アダプタオーバーヘッドをなくす。

## 背景

`wasm-tools component new` + `wasi_snapshot_preview1.reactor.wasm` は
アダプタモジュールのサイズ (~100KB) と変換オーバーヘッドを伴う。
P2 ネイティブでは Core Wasm が直接 `wasi:io/streams` 等をインポートするため、
バイナリサイズと起動時間が改善する。

## 受け入れ条件

1. `--wasi-version p2` フラグで P2 ネイティブコンポーネントをコンパイル
2. Core Wasm に `wasi:cli/environment@0.2.x` 等を直接 import するセクション生成
3. P1 アダプタなしで wasmtime 17+ で実行可能
4. バイナリサイズが P1 アダプタ版より 80KB 以上削減されることを確認

## 実装タスク

1. `ark-wasm/src/emit/t3_wasm_gc.rs`: WASI P2 モード分岐 (import 名を P2 形式に変更)
2. `ark-wasm/src/component/wrap.rs`: P2 ネイティブの場合 `component new` を迂回
3. WIT world 出力を `wasi:cli/command` ベースに変更

## 実装ノート

- Task 2 (wrap.rs p2_native): 実装済み。`p2_native` パラメータで P1 アダプタをスキップ可能。
- Task 1 (T3 emitter import names): 部分実装済み (2026-04-19)。
  - T3 emitter の import section に P2 モード分岐を実装済み (`wasi:cli/stdout@0.2.0` 等)。
  - P2 import indices を正しく追跡するために別フィールドを追加済み。
  - **P2 stdio (print helpers) は未実装**: print helper architecture と P2 import signature の不一致により validation error が発生。
    - P1 import は iovec + retptr を使用するが、P2 import は直接パラメータを使用する。
    - この問題を解決するには print helper architecture を大幅に変更する必要がある。
  - stdio を使用しないプログラムの場合、P2 ネイティブコンポーネントの生成は正常に動作。
- Task 3 (WIT world): 部分実装済み (2026-04-19)。
  - `--wasi-version p2` 時に自動的に `wasi:cli/command` world spec を使用するように実装。
  - `main` 関数を `run` に rename して `wasi:cli/command` の required export に対応。
  - 外部 WASI パッケージ (`wasi:cli@0.2.0` 等) の依存を回避するために `use_imports` をスキップ。
  - **wasi:cli/command の完全な統合には追加のアーキテクチャ変更が必要**: Component Model はインターフェースの export を期待するが、現在の実装は関数を export している。
    - `wasi:cli/run/run` インターフェースを正しく export するには、T3 emitter の export architecture を変更する必要がある。
- `--wasi-version p2` フラグは現在部分的に機能する (stdio を使用しないプログラムの場合、コンパイルは成功するが wasmtime での実行は未検証)。

## 実証テスト結果 (2026-04-18)

wasmtime 43.0.1 (issue で言及されている 17+ より遥かに新しいバージョン) でテスト実施:

1. **T3 emitter 生成 Core Wasm (P1 imports) + wasmtime**: ✅ 成功
   - `wasmtime --wasm gc test.wasm` で正常実行
   - WASI P1 ランタイムは安定しており、GC 機能も正常に動作

2. **Component wrapping (P1 adapter)**: ✅ 成功
   - `wasm-tools component new test.wasm --adapt wasi_snapshot_preview1.reactor.wasm` で 20KB のコンポーネント生成
   - P1 アダプタ自体は 97KB

3. **Component execution**: ❌ 失敗
   - `no exported instance named 'wasi:cli/run@0.2.6'` エラー
   - T3 emitter は `_start` export を生成しているが、Component Model は `wasi:cli/run/run` export を期待
   - これは export 構造の不一致であり、ランタイムの成熟度とは無関係

## 実装テスト結果 (2026-04-19)

P2 ネイティブコンポーネントの部分的実装をテスト:

1. **基本 P2 コンポーネントコンパイル (stdio 未使用)**: ✅ 成功
   - `--wasi-version p2 --emit component` でコンパイル成功
   - 単純なプログラム (stdio 未使用) で 724 bytes (P1 は 690 bytes)
   - 単純なプログラムではサイズ差は小さいが、WASI 関数を使用するプログラムでは P1 アダプタ (~97KB) を省略できるため 80KB 以上の削減が期待される

2. **P2 stdio (print helpers)**: ❌ 未実装
   - print helper architecture と P2 import signature の不一致により validation error
   - P1 import は iovec + retptr を使用、P2 import は直接パラメータを使用
   - この問題を解決するには print helper architecture を大幅に変更する必要がある

3. **wasi:cli/command 統合**: ❌ 部分的に未実装
   - WIT world 生成は実装済み (main -> run rename, use_imports スキップ)
   - Component Model はインターフェース export を期待するが、現在の実装は関数を export
   - `wasi:cli/run/run` インターフェースを正しく export するには export architecture の変更が必要

4. **wasmtime 実行**: ❌ 未検証
   - `no exported instance named 'wasi:cli/run@0.2.6'` エラー
   - インターフェース export の問題を解決する必要がある

## 結論

**「WASI P2 ランタイムの成熟度が不足している」という理由付けは誤り。**

実際の問題:
1. T3 emitter が P1 インポート (`wasi_snapshot_preview1`) を生成している (部分実装済み)
2. Component Model は `wasi:cli/run/run` export を期待しているが、T3 emitter は `_start` を生成 (部分的に解決済み)
3. P2 ネイティブコンポーネントには import/export 構造の変更が必要 (部分的に実装済み)

wasmtime 43.0.1 は GC 機能と Component Model の両方をサポートしており、ランタイム自体は成熟している。実装が必要なのは T3 emitter の変更であり、ランタイムの成熟度を待つ理由はない。

**2026-04-19 現在の実装状況:**

部分的な P2 ネイティブコンポーネントサポートが実装済み:
- ✅ T3 emitter に P2 import 分岐を実装済み (`wasi:cli/stdout@0.2.0` 等)
- ✅ WIT world に wasi:cli/command 自動検出を実装済み
- ✅ stdio を使用しないプログラムの P2 コンポーネントコンパイルが成功
- ❌ P2 stdio (print helpers) は未実装 (print helper architecture の大幅な変更が必要)
- ❌ wasi:cli/command の完全な統合は未実装 (export architecture の変更が必要)

**残課題:**

P2 ネイティブコンポーネントの完全な実装には以下のアーキテクチャ変更が必要:
1. **P2 stdio**: print helper architecture を P2 import signature に対応するよう再設計
   - P1 import は iovec + retptr を使用、P2 import は直接パラメータを使用
   - 現在の shim architecture では対応できない

2. **wasi:cli/command 統合**: export architecture を変更してインターフェースを export する
   - Component Model はインターフェース export を期待する
   - 現在の実装は関数を export している

これらの変更は T3 emitter のアーキテクチャに影響するため、別の issue で扱うことが推奨される。

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md`
- `docs/spec/spec-WASI-0.2.10/specifications/wasi-0.2.10/`
