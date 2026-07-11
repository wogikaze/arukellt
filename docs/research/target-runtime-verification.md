# ターゲットランタイム検証 — 実装・動作確認レポート

ステータス: **調査メモ（決定記録ではない）**

調査日: 2026-07-11

---

## 目的

ADR-007 が定義する 3 ターゲット（`wasm32` / `wasm32-gc` / `native`）のうち、
外部ツールチェイン依存がある以下 5 経路について、実装状況と実際の動作を確認する。

1. wabt 1.0.34 + iwasm 2.4.1 (WAMR) — `wasm32` ターゲット
2. jco transpile — `wasm32-gc` → JS 変換
3. Chrome (V8) — `wasm32-gc` ブラウザ実行
4. Node.js — `wasm32-gc` サーバーサイド実行
5. wac — `wasm32-gc` component composition

---

## ローカル環境

| ツール | バージョン | パス |
|--------|-----------|------|
| wasmtime | 46.0.1 | `~/.wasmtime/bin/wasmtime` |
| wabt | 1.0.27 | `/usr/bin/wasm2wat` 等 |
| iwasm | 2.4.3 (GC=OFF) | `~/.local/bin/iwasm` |
| wasm-tools | 1.252.0 | `~/.cargo/bin/wasm-tools` |
| jco | 1.25.2 | npm global |
| Chrome | google-chrome | `/usr/bin/google-chrome` |
| Node.js | v23.6.0 | `~/.nvm/versions/node/v23.6.0/bin/node` |
| wac | wac-cli 0.10.0 | `~/.cargo/bin/wac` |

> **注意**: wabt は ADR-007 が指定する 1.0.34 ではなく 1.0.27 がローカルに
> インストールされている。iwasm は 2.4.3（ADR-007 指定は 2.4.1）。
> iwasm の GC サポートは OFF（`WASM_ENABLE_GC: 0`）。

---

## 1. wabt + iwasm — `wasm32` ターゲット

### 結論: **動作確認済み** ✅

### テスト

```
.ark → compile --target wasm32-wasi-p1 → .wasm
```

| ステップ | ツール | 結果 |
|---------|--------|------|
| コンパイル | wasmtime (selfhost) | ✅ 1030 bytes の core wasm を生成 |
| wasmtime 実行 | wasmtime | ✅ `Hello from Arukellt!` |
| iwasm 実行 | iwasm 2.4.3 | ✅ `Hello from Arukellt!` |
| wasm2wat (WAT逆変換) | wabt 1.0.27 | ✅ 有効な WAT を生成 |
| wasm-interp (インタプリタ実行) | wabt 1.0.27 | ❌ `invalid import "wasi_snapshot_preview1.fd_write"` |

### 詳細

- **iwasm で実行可能**: `iwasm hello.wasm` で WASI P1 プログラムが正常実行される
- **wasm-interp は非対応**: wabt のインタプリタは WASI import を解決しないため実行不可。WAT 変換（`wasm2wat`）は正常動作
- **コンパイラのターゲット名**: 実装は `wasm32-wasi-p1` を使用。ADR-007 の canonical 名 `wasm32` とその alias 変換は**未実装**
- **AtCoder 固有の機能**: フィクスチャ・提出フォーマット・制約チェック等は**存在しない**

### 実装のギャップ

| 項目 | ADR-007 | 実装 |
|------|---------|------|
| canonical ターゲット名 | `wasm32` | `wasm32-wasi-p1`（alias 未実装） |
| wabt バージョン | 1.0.34 指定 | ローカルは 1.0.27 |
| iwasm バージョン | 2.4.1 指定 | ローカルは 2.4.3 |
| AtCoder フィクスチャ | 言及あり | なし |
| iwasm 統合スクリプト | 暗黙 | なし（手動 `iwasm` 実行のみ） |

---

## 2. jco transpile — `wasm32-gc` → JS 変換

### 結論: **transpile 成功・Node.js実行成功（パッチ必要）** ⚠️

### テスト

```
.ark → compile --target wasm32-wasi-p2 → core.wasm
     → p2_component_wrap.py → component.wasm
     → jco transpile → ESM + JS glue
     → node (run.run())
```

| ステップ | ツール | 結果 |
|---------|--------|------|
| コンパイル (core wasm) | wasmtime (selfhost) | ✅ 1672 bytes |
| Component 化 | `p2_component_wrap.py` (in-tree) | ✅ 6291 bytes |
| jco transpile | jco 1.25.2 | ✅ ESM + JS glue 生成 |
| Node.js 実行 (パッチ前) | node v23.6.0 | ❌ `SyntaxError: Unexpected eval or arguments in strict mode` |
| Node.js 実行 (パッチ後) | node v23.6.0 | ✅ `Hello from Arukellt!` |

### 重要な発見: issue #037 のブロッカーは解消済み

`issues/blocked/037-jco-gc-support.md` は jco が Wasm GC 型を含むコンポーネントの
transpile に失敗すると記録していたが、**jco 1.25.2 では transpile が成功する**。

- エラー `"array indexed types not supported without the gc feature"` は発生しない
- jco 1.25.2 は GC 型を含むコンポーネントを正常に transpile する

### 既知のバグ: jco の `arguments` 予約語問題

jco が生成する JS コードに `function arguments(arg0, arg1)` という関数宣言が含まれる。
`arguments` は strict mode (ESM) で予約語のため、Node.js / Chrome で `SyntaxError` が発生する。

**原因**: `wasi:cli/environment.arguments` import の lowered 関数名が `arguments` になる。
jco のコード生成バグ（Arukellt 側の問題ではない）。

**回避策**: 生成された JS をパッチする:
```bash
sed -i 's/function arguments(/function _cliArguments(/' output.js
sed -i 's/ arguments, / _cliArguments as arguments, /' output.js
```

### 追加依存: `@bytecodealliance/preview2-shim`

jco transpile 出力は `@bytecodealliance/preview2-shim` パッケージに依存する。
実行ディレクトリで `npm install @bytecodealliance/preview2-shim` が必要。

### ライブラリコンポーネントの制限

in-tree component wrapper (`p2_component_wrap.py`) は `wasi:cli/command` 型の
コンポーネントを生成するため、`run` export のみ公開される。
`add` / `mul` 等のライブラリ関数を component export するには、
カスタム WIT world + `wasm-tools component embed/new` が必要。

現状、`wasm-tools component embed` は core wasm の `wasi:cli/stdout` import を
WIT world が宣言していないとエラーになるため、ライブラリコンポーネントの
独立生成は追加の WIT 設計が必要。

---

## 3. Chrome (V8) — `wasm32-gc` ブラウザ実行

### 結論: **Chrome jco component E2E は未検証** ❌

分類:

| 経路 | 状態 |
|------|------|
| Node.js E2E（jco transpile → patch → run） | **verified**（手動パッチ後） |
| Chrome core Wasm（`WebAssembly.instantiate` + WASI shim） | **verified**（bench インフラ） |
| Chrome jco component E2E（HTTP → ESM import → WASI shim → run export） | **not yet verified** |

### テスト

| ステップ | ツール | 結果 |
|---------|--------|------|
| jco transpile 出力の Node.js 実行 | node v23.6.0 (V8) | ✅ verified |
| Chrome headless での ESM 読み込み | google-chrome | ❌ `file://` では ESM import 不可。HTTP E2E 未実施 |
| 直接 WebAssembly.instantiate | `scripts/perf/run-browser-bench.mjs` | ✅ core Wasm のみ verified |

### 詳細

- **Chrome は Wasm GC をネイティブサポート**: V8 は WasmGC に対応済み
- **Node 成功 ≠ Chrome jco E2E**: 同じ V8 でも、jco 生成 ESM の HTTP 配信・
  preview2-shim・`run` export までの完全経路は未実証。推測で「動くはず」と書かない
- **ベンチマークインフラあり**: `scripts/perf/run-browser-bench.mjs` は
  puppeteer-core 経由で headless Chrome を起動し、直接 `WebAssembly.instantiate()` で
  **core Wasm** を実行する（component / jco 経路ではない）
- **Playground v1 は実行非対応**: parse/format/check のみ。run 機能は v2 計画

### ブラウザ jco E2E に必要な条件（未達）

1. jco transpile で ESM + JS glue を生成
2. `arguments` 予約語バグをパッチ
3. `@bytecodealliance/preview2-shim` を bundler で組み込むか CDN 経由で配信
4. HTTP サーバー経由で配信し、Chrome で ESM import → WASI shim → run まで実行
5. 上記を再現可能なスクリプト / CI に固定する

---

## 4. Node.js — `wasm32-gc` サーバーサイド実行

### 結論: **動作確認済み** ✅（パッチ + npm install 必要）

### テスト

```
node --input-type=module -e "
import * as m from './hello_wrapped.component.js'
m.run.run()  // → "Hello from Arukellt!"
"
```

| ステップ | 結果 |
|---------|------|
| jco transpile | ✅ |
| `arguments` パッチ | ✅ (手動) |
| `npm install @bytecodealliance/preview2-shim` | ✅ |
| `m.run.run()` 実行 | ✅ `Hello from Arukellt!` 出力 |

### 詳細

- **ADR-007 の経路が動作**: compile → component wrap → jco transpile → node 実行
- **`run` export の呼び出し方**: `m.run.run()`（namespace object 経由）
- **stdin 制限**: ADR-007 が言及する stdin 非対応は jco の JS glue 側の制限。
  stdout は正常動作。
- **Node.js 22+ が必要**: WasmGC サポートのため。ローカルは v23.6.0 で問題なし。

### 自動化されていないステップ

1. `p2_component_wrap.py` による component 化（手動実行）
2. jco transpile（手動実行）
3. `arguments` 予約語パッチ（手動 sed）
4. `npm install @bytecodealliance/preview2-shim`（手動実行）

これらを統合した `arukellt run --target node` のようなラッパーは存在しない。

---

## 5. wac — `wasm32-gc` component composition

### 結論: **動作確認済み** ✅

### テスト

```
bash tests/component-interop/compose/run.sh → PASS compose smoke
```

| ステップ | ツール | 結果 |
|---------|--------|------|
| math_lib.ark → core wasm | Arukellt compiler | ✅ |
| component embed | wasm-tools | ✅ |
| component new (with adapter) | wasm-tools | ✅ |
| add(40,2) 実行 | wasmtime | ✅ `42` |
| runner (Rust) build | cargo component | ✅ |
| wac plug | wac 0.10.0 | ✅ composed-component.wasm 生成 |
| composed run() | wasmtime | ✅ `42` |

### 詳細

- **ADR-034 Phase 1-3 完全実装**: CLI scaffold + WIT 検証 + wac plug 委譲
- **`arukellt compose` サブコマンド**: `compose_cmd.ark` でパス検証・WIT 検証・
  依存グラフ出力を実装。実際のバイナリ合成は `arukellt-selfhost.sh` が
  `wac plug` に委譲する。
- **テストフィクスチャ**: `tests/component-interop/compose/` に
  Ark provider + Rust socket の E2E テストが存在。
- **ゲートスクリプト**: `gate-443` (Phase 3 検証) と `gate-665` (E2E) が存在。
  wac が未導入の場合は SKIP 扱い。

### 修正が必要だった点

`tests/component-interop/compose/runner-cargo/Cargo.toml` に `[workspace]` テーブルが
なく、ルート `Cargo.toml` の workspace と衝突していた。
空の `[workspace]` を追加して解決。

---

## 総合まとめ

### 動作確認マトリックス

| 経路 | ツール | 動作 | 備考 |
|------|--------|------|------|
| wasm32 → iwasm | iwasm 2.4.3 | ✅ | そのまま実行可能 |
| wasm32 → wabt (WAT) | wabt 1.0.27 | ✅ | wasm2wat のみ。wasm-interp は WASI 非対応 |
| wasm32-gc → wasmtime | wasmtime 46.0.1 | ✅ | component wrap 後 |
| wasm32-gc → jco → Node.js | jco 1.25.2 + node 23 | ✅ verified | `arguments` パッチ + npm install 必要 |
| wasm32-gc → jco → Chrome | jco 1.25.2 + chrome | ❌ not yet verified | HTTP + ESM + shim + run の E2E 未実施。core Wasm のみ verified |
| wasm32-gc → Chrome core Wasm | puppeteer + V8 | ✅ verified | jco 経路ではない |
| wasm32-gc → wac compose | wac 0.10.0 | ✅ | compose smoke test PASS |

### ブロッカー状況

| Issue | 状態 | 詳細 |
|-------|------|------|
| #037 (jco GC support) | **解消済み** | jco 1.25.2 で GC 型 transpile が成功 |
| jco `arguments` 予約語バグ | **未解決** | jco のコード生成バグ。パッチで回避可能 |
| ADR-007 alias policy | **未実装** | `wasm32-wasi-p1` → `wasm32` 変換が未実装 |
| AtCoder 統合 | **未実装** | フィクスチャ・提出フォーマット等なし |
| Playground v2 (run) | **未実装** | v1 は parse/check/format のみ |

### 推奨アクション

1. **issue #037 を更新**: jco 1.25.2 で transpile が成功することを記録し、
   blocked → open に移動
2. **jco `arguments` バグをレポート**: jco upstream に報告
3. **Node.js 実行パイプラインを自動化**: compile → wrap → jco → patch → run の
   統合スクリプトを作成
4. **ADR-007 alias policy を実装**: `wasm32-wasi-p1` → `wasm32` 等の変換を
   コンパイラに実装
5. **ライブラリコンポーネントの WIT 設計**: `add`/`mul` 等の関数を
   component export するための WIT world + wasm-tools 経路を確立
