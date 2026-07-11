# ADR-002: GC vs non-GC

ステータス: **ACCEPTED** — ベンチマーク実測（2026-03-25）により **選択肢 A: Wasm GC 前提を採用**
（2026-07-05 追補: wasmtime / Node.js / ブラウザの 3 ランタイムクロス計測で再確認。結論は不変）

## 文脈

GC と非GCは「性能差」ではなく「設計方針の分岐点」。この決定が固まるまで、以下は設計できない:

- 値表現とレイアウト（移動するか否か）
- クロージャのキャプチャ方式
- FFI 境界での所有権モデル
- std の String / Vec / HashMap 実装
- エラー処理でのアロケーション前提

「両対応」は許されない。GC-native profile と linear-memory fallback の二層は「比較」ではなく「二重実装」になる。

## 選択肢

### 選択肢 A: Wasm GC 前提

Wasm GC 命令セット（`ref`, `struct`, `array` 等）を使い、ホストの GC に管理を委ねる。

利点:

- ライフタイム管理が言語設計から消える → LLM フレンドリ方向に有利
- Wasm バイナリにメモリ管理ルーチンを同梱しなくて済む → バイナリサイズ有利
- Wado の事例: Hello World 1090 bytes、pi_approx 11786 bytes、C 比 1.08–1.23x を達成
- クロージャ・コンテナの実装が単純になる

欠点:

- GC オーバーヘッドがある（ワークロード依存）
- ホスト GC の実装品質に性能が左右される
- wasmtime の Wasm GC 最適化はまだ弱い（2025年時点）
- native バックエンドで GC を持ち込む必要がある

影響する設計:

- 参照はすべて GC 管理
- 値セマンティクスは copy で表現
- FFI 境界での ownership transfer は明示的ハンドル
- Option/Result は GC heap 上の tagged union

### 選択肢 B: linear memory 前提（非GC）

Wasm の linear memory に自前でアロケータを持ち、GC を使わない。

利点:

- メモリレイアウトが完全に予測可能
- GC ポーズなし
- 実行速度の上限が高い
- native バックエンドとの対称性が高い

欠点:

- 何らかの所有権モデルが必要（Rust 風 / RC / 領域ベース / 明示解放）
- LLM フレンドリと相反しやすい（borrow checker は LLM が壊しやすい）
- コンテナ・クロージャの実装が重くなる
- バイナリにアロケータが含まれる

影響する設計:

- 所有モデルを何か選ぶ必要がある（下記参照）
- `&T` / `&mut T` 相当の参照型
- Drop/finalizer のタイミング

#### 選択肢 B の中での所有モデル候補

| モデル | 特徴 |
|--------|------|
| Rust 風 move + borrow | 最も安全。最も LLM に厳しい |
| 簡易 borrow（局所解析のみ） | ライフタイム注釈なし。可変参照は1つまで |
| 参照カウント (RC) | 実装が単純。循環参照問題あり |
| 領域ベース (region/arena) | GC なしで一括解放。API が特殊 |
| 明示解放 (C 風) | 最も単純。安全性なし |

## 判断に必要な比較実験

`process/benchmark-plan.md` に詳細あり。最低限以下を同条件で測定してから決定する:

| ケース | 何を測るか |
|--------|-----------|
| Hello World | Wasm バイナリサイズ |
| 文字列連結 100 回 | アロケーション負荷 |
| Vec push/pop 10k | コンテナ性能 |
| 二分木 depth 20 | 再帰 + 参照 |
| Result 多用コード | エラー経路のオーバーヘッド |
| WASI ファイル読込 | I/O 経路 |

## 決定（取り消し → 実測再確定）

### 前回決定の取り消し（2026-03-25）

前回（2026-03-24）の決定は、ベンチマーク未実施のまま確定していたためプロセス違反として取り消した。

### 実測結果（2026-03-25）に基づく再決定

**選択肢 A: Wasm GC 前提を採用する**

決定日: 2026-03-25（実測確定）

`benchmark-plan.md` の判定基準「**それ以外 → LLMフレンドリを優先して GC を採用**」に該当。

| ケース | 比率 (GC/Linear 時間) | 1.5x 超? |
|--------|----------------------|----------|
| hello | 1.41x | No |
| string_concat | 0.80x (GC 優位) | No |
| vec_pushpop | 1.47x | No |
| binary_tree | 1.83x | **Yes** |
| result_heavy | 1.12x | No |
| file_read | 1.06x | No |

linear が 1.5x 以上高速なケース = 1（binary_tree のみ）。採用基準 2 ケース以上を満たさないため GC を採用。

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

### 前回決定の理由（参考・再評価対象）

1. **LLM フレンドリ性を最優先**
   - ライフタイム管理・所有権モデルが言語設計から消える
   - borrow checker のような複雑な規則が不要
   - LLM が書き間違える箇所が大幅に減少

2. **外部実績の存在**
   - Wado (F# → Wasm GC コンパイラ) の実績:
     - Hello World: 1090 bytes
     - pi_approx: 11786 bytes
     - 実行性能: C 比 1.08–1.23x
   - 「GC でも目標性能を達成できる」ことが実証済み

3. **バイナリサイズの有利性**
   - メモリ管理ルーチンをバイナリに含めなくて済む
   - GC ランタイムはホスト側が提供

4. **実装の単純さ**
   - クロージャ・コンテナの実装が単純
   - 所有権モデルの選択が不要
   - FFI 境界の設計が明確

### 前回想定していたリスク（参考・再評価対象）

- wasmtime の Wasm GC 最適化は 2025 年時点で未成熟（改善待ち）
  - **2026-07-05 計測で確認**: wasmtime 43.0.1 でも `vec_pushpop` (20.86x) / `result_heavy` (16.99x) で GC が大幅低速。Cranelift の GC codegen 最適化は V8 に比べ未成熟。
  - 一方 V8 (Node.js v23 / Chrome 147) では同 fixture で GC が同等〜高速。ホストエンジン依存性が顕著。
- ホスト GC の実装品質に性能が左右される
  - **2026-07-05 計測で確認**: V8 は実用域、wasmtime は改善途上。ランタイム間で 10–20x 程度の GC 性能差あり。
- GC オーバーヘッドがワークロード依存で発生

### 前回のフォールバック計画（参考・再評価対象）

将来のベンチマーク再評価時に性能問題が判明した場合:

- 特定の hot path のみ linear memory に最適化
- 将来 hybrid approach を検討

## 補足決定（2026-03-25）: AtCoder 向け `wasm32` ターゲット

### 背景

AtCoder 等の非 GC ランタイム向けに、同一言語意味論を linear memory へ lowering する
バックエンド（`wasm32`）を許可する。主決定（言語セマンティクス = Wasm GC）は変えない。

- 禁止: 言語設計に ownership / borrow checker を組み込む「二重設計」
- 許可: GC セマンティクスを linear memory に lowering するコンパイラバックエンド

### ターゲット

| ターゲット | 役割 | メモリ |
|-----------|------|--------|
| `wasm32-gc` | primary（ADR-013）。ホスト GC | Wasm GC |
| `wasm32` | supported。AtCoder / 非 GC | linear lowering（実装計画は plans） |

### 言語機能とターゲット能力（2026-07-11 改訂）

言語仕様は **Wasm GC を正**とする。次は言語から削除しない。

| 機能 | `wasm32-gc` | `wasm32` |
|------|-------------|----------|
| 循環参照グラフ | 許可（ホスト GC） | **target capability error**（未対応） |
| `Weak<T>` / 弱参照 | 許可（実装は別 issue） | **unsupported** |
| finalizer 意味論 | 許可（実装は別 issue） | **unsupported** |

AtCoder 向けサブターゲットの都合で、primary 言語仕様からこれらの機能を削らない。
`wasm32` で使えない機能はコンパイル時にターゲット能力エラーとする。

`wasm32` の arena + RC hybrid 等の **実装手順**は
[`docs/plans/wasm32-linear-memory-lowering.md`](../plans/wasm32-linear-memory-lowering.md) に置く。

## 関連

- [ADR-007](ADR-007-targets.md)
- [`docs/plans/wasm32-linear-memory-lowering.md`](../plans/wasm32-linear-memory-lowering.md)
- `docs/process/benchmark-results.md` — 採択時の実測根拠
