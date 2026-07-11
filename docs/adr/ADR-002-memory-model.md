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

「両対応」は許されない、の意味は次のとおりである。

- **禁止**: GC 用と linear 用で**異なる言語意味論・stdlib・ownership model**を持つこと
- **許可**: **一つの言語意味論**（Wasm GC ベース）を複数の target representation へ
  lowering すること（例: `wasm32-gc` はホスト GC、`wasm32` は同一意味の linear lowering）

GC-native profile と「別言語としての」linear fallback の二重設計は比較ではなく二重実装になる。

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

### クロスランタイム再計測（2026-07-05）— 判定要旨

2026-03-25 の決定後、wasmtime / Node.js / headless Chrome で小規模 fixture を再計測した。
詳細表・環境・スクリプトは
[`docs/research/linear-vs-gc-cross-runtime-2026-07-05.md`](../research/linear-vs-gc-cross-runtime-2026-07-05.md)。

**この計測から言えること（限定）:**

- V8（Node / browser）では、今回の小規模 fixture において GC 版が致命的に遅い
  という証拠は得られなかった
- wasmtime では一部ケース（`vec_pushpop` / `result_heavy`）で GC が大幅低速
- アロケーション量・GC 回数・実アプリ規模の fixture は未計測。browser の極小結果は
  分解能不足（`0.000 ms`）を含む

**GC 採択の主根拠は性能勝利ではない:**

1. 所有権規則を言語から排除できる（LLM-friendly）
2. stdlib / closure / container 実装が単純になる
3. primary runtime で許容範囲に入る可能性がある（上記の限定的証拠）

wasmtime の遅延は一時的制約として監視する（リスク）。結論（選択肢 A）は不変。

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

- wasmtime の Wasm GC 最適化は、小規模 fixture でも一部ケースで大幅低速になりうる
  （詳細倍率・版は research 文書）。改善監視対象
- ホスト GC の実装品質に性能が左右される
- **今回の小規模 fixture では V8 上で重大な退行を確認しなかった。**
  実アプリ規模、GC 頻度、メモリ量を含む実用性評価は未完了
- GC オーバーヘッドがワークロード依存で発生しうる

詳細: [`docs/research/linear-vs-gc-cross-runtime-2026-07-05.md`](../research/linear-vs-gc-cross-runtime-2026-07-05.md)

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
| 循環参照グラフ | **許可**（通常のホスト GC 到達可能性） | **target capability error**（未対応） |
| `Weak<T>` / 弱参照 | **未採択**（ADR-043: Phase 0 拡張に依存しない。将来の独立 ADR） | **unsupported** |
| finalizer 意味論 | **未採択**（同上。実行保証・順序・resurrection 等は未定義） | **unsupported** |

循環参照は MVP GC で扱えるため言語仕様として許可する。
`Weak<T>` と finalizer は Wasm Weak References / Finalization 等の未標準化機能、
または独自 runtime エミュレーションの意味論定義が必要なため、**言語仕様に採択しない**。
将来採択する場合は独立 ADR で観測可能意味論を固定する（ADR-043）。

AtCoder 向けサブターゲットの都合で、primary 言語仕様から循環参照を削らない。
`wasm32` で使えない機能はコンパイル時にターゲット能力エラーとする。

`wasm32` の arena + RC hybrid 等の **実装手順**は
[`docs/plans/wasm32-linear-memory-lowering.md`](../plans/wasm32-linear-memory-lowering.md) に置く。

## 関連

- [ADR-007](ADR-007-targets.md)
- [`docs/plans/wasm32-linear-memory-lowering.md`](../plans/wasm32-linear-memory-lowering.md)
- `docs/process/benchmark-results.md` — 採択時の実測根拠
