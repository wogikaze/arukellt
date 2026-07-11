# Linear vs GC クロスランタイム計測（2026-07-05）

ステータス: 調査メモ（決定記録ではない）  
関連 ADR: [ADR-002](../adr/ADR-002-memory-model.md)

本ファイルは計測表・環境・スクリプト一覧の正本。ADR-002 には判定要旨のみ残す。

---

### クロスランタイム再計測（2026-07-05）

2026-03-25 の決定時は wasmtime 単一ランタイムで計測していた。2026-07-05 に
`scripts/perf/compare-linear-vs-gc.{sh,py}` により **wasmtime / Node.js / ブラウザ
（headless Chrome）** の 3 ランタイムで同条件再計測を行い、決定を再確認した。

計測環境:

- **wasmtime** 43.0.1 (Cranelift) — wasmtime-py 経由、インスタンス化 1 回 + `_start` 反復呼び出し
- **Node.js** v23.6.0 (V8 12.9) — ネイティブ `WebAssembly` API + JS 製 WASI shim
- **browser** — Google Chrome 147 (V8) headless、puppeteer-core 経由

全ランタイムで同一の JS 製 WASI P1/P2 shim を使用し、stdout capture・ウォームアップ・
反復計測の条件を公平に揃えた（`scripts/perf/wasi-shim.mjs`）。

#### 計測結果（中央値 ms、5 iterations / 2 warmups）

| Fixture | Target | wasmtime | node | browser |
|---------|--------|----------|------|---------|
| hello | linear | 0.124 | 0.050 | 0.000 |
| hello | gc | 0.069 | 0.030 | 0.000 |
| string_concat | linear | 0.129 | 0.064 | 0.000 |
| string_concat | gc | 0.146 | 0.042 | 0.000 |
| vec_pushpop | linear | 2.472 | 2.909 | 2.100 |
| vec_pushpop | gc | 51.579 | 2.778 | 0.900 |
| binary_tree | linear | 5.865 | 5.667 | 5.800 |
| binary_tree | gc | 6.517 | 5.171 | 5.700 |
| result_heavy | linear | 0.889 | 0.518 | 0.800 |
| result_heavy | gc | 15.095 | 0.465 | 0.700 |
| file_read | linear | 4.251 | 0.837 | 0.800 |
| file_read | gc | — | — | — |

> `file_read` の GC 版は ADR-035 Phase 1 部分実装の制限で fs モジュールの
> GC codegen が未対応のためコンパイル/実行エラー。他 5 fixture は両ターゲット
> で全ランタイム正常終了。

#### ランタイム別の GC/Linear 比

| Fixture | wasmtime | node | browser |
|---------|----------|------|---------|
| hello | 0.56x | 0.60x | — (極小) |
| string_concat | 1.13x | 0.65x | — (極小) |
| vec_pushpop | **20.86x** | 0.95x | **0.43x** |
| binary_tree | 1.11x | 0.91x | 0.98x |
| result_heavy | **16.99x** | 0.90x | 0.88x |

#### 判定

- **wasmtime** では `vec_pushpop` (20.86x) と `result_heavy` (16.99x) で GC が大幅低速。
  これは wasmtime の Wasm GC 最適化が未成熟であること（下記「リスク」の通り）が
  主因。Cranelift の GC codegen は V8 に比べ最適化余地が大きい。
- **Node.js / browser (V8)** では逆に GC が同等か高速なケースが大半。
  `vec_pushpop` は browser で GC が 0.43x（2.3x 高速）、`result_heavy` は
  node/browser とも GC が 0.88–0.90x（GC 優位）。
- V8 の GC 最適化は実用域に達しており、ブラウザ・Node.js ターゲットでは
  GC 採用の性能上の懸念は実質的に解消された。
- **結論は不変**: wasmtime の最適化未成熟は一時的制約（改善トレンド）であり、
  LLM フレンドリ性・実装単純さ・バイナリサイズの利点が性能面の懸念を上回る。
  将来の wasmtime 最適化進展で vec_pushpop / result_heavy の遅延は解消期待。

#### 計測インフラ

| ファイル | 役割 |
|---------|------|
| `scripts/perf/compare-linear-vs-gc.py` | 比較オーケストレーション（コンパイル → 3 ランタイム計測 → JSON/MD 出力） |
| `scripts/perf/wasi-shim.mjs` | WASI P1/P2 import shim（Node/browser 共用） |
| `scripts/perf/run-node-bench.mjs` | Node.js 計測アダプタ |
| `scripts/perf/run-browser-bench.mjs` | ブラウザ計測アダプタ（puppeteer-core + Chrome headless） |
| `docs/process/linear-vs-gc-results.json` | 計測結果 JSON |
| `docs/process/linear-vs-gc-report.md` | 計測結果 Markdown レポート |

実行方法: `bash scripts/perf/compare-linear-vs-gc.sh --iterations 10 --warmups 2`

