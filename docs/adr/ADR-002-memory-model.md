# ADR-002: GC vs non-GC

ステータス: ★ OPEN — **最優先で決めること**

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

## 決定

**選択肢 A: Wasm GC 前提を採用する**

決定日: 2026-03-24

### 決定理由

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

### 許容するリスク

- wasmtime の Wasm GC 最適化は 2025 年時点で未成熟（改善待ち）
- ホスト GC の実装品質に性能が左右される
- GC オーバーヘッドがワークロード依存で発生

### フォールバック計画

v0 完成後のベンチマークで性能問題が判明した場合:
- 特定の hot path のみ linear memory に最適化
- v1 で hybrid approach を検討

## 結果

以下のドキュメントを更新する:

- [x] `docs/language/memory-model.md` — メモリモデル詳細仕様（新規作成）
- [x] `docs/language/type-system.md` — ADR-002 待ちセクション更新
- [x] `docs/platform/wasm-features.md` — reference types / Wasm GC を Layer 1 に
- [x] `docs/core.md` — String / Vec の実装方針を GC 前提で記載
