# ADR-002: GC vs non-GC

ステータス: **DECIDED** — ベンチマーク実測（2026-03-25）により **選択肢 A: Wasm GC 前提を採用**

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
- ホスト GC の実装品質に性能が左右される
- GC オーバーヘッドがワークロード依存で発生

### 前回のフォールバック計画（参考・再評価対象）

v0 完成後のベンチマークで性能問題が判明した場合:

- 特定の hot path のみ linear memory に最適化
- v1 で hybrid approach を検討

## 補足決定（2026-03-25）: AtCoder コンパイルターゲット追加

### 背景

AtCoder は wabt 1.0.34 + iwasm 2.4.1 を実行環境として使用する。
これらのランタイムは Wasm GC 命令セット（`ref struct`, `array` 等）に非対応。
競技プログラミングの提出先として AtCoder を目標環境に含めるため、対応を要件とする。

### 主決定との関係

主決定（言語セマンティクス = Wasm GC ベース）は変更しない。
「両対応は許されない」は **言語仕様レベル** の方針であり、コンパイラの出力ターゲット追加はこれに抵触しない。

- 禁止: 言語設計に ownership / borrow checker を組み込む「二重設計」
- 許可: GC セマンティクスを linear memory に lowering して出力する「コンパイラバックエンドの追加」

### コンパイルターゲット一覧

| ターゲット | フラグ | 対応ランタイム | メモリ管理 |
|-----------|--------|--------------|----------|
| `wasm-gc` | デフォルト | wasmtime / V8 / SpiderMonkey | ホスト GC |
| `wasm32` | `--target wasm32` | wabt 1.0.34 / iwasm 2.4.1 / 非GC 環境 | arena + RC hybrid |

### wasm32 プロファイル: メモリ管理モデル

「arena ベース + escape/共有のみ RC」の hybrid モデルを採用する。
pure arena と pure RC の二者択一より、これが最も実用的。

#### arena（デフォルト）

- 実行単位ごとに region を確保（典型: main 全体で 1 arena）
- オプション: 「関数内一時領域 + 昇格領域」の二層構成
- 対象: 短命な値・AST 的な木・文字列・配列・小オブジェクト
- プログラム終了時に一括解放（解放コード不要）
- AtCoder 本番提出の標準

#### RC: escape / 共有値のみ

| 対象 | 判定 |
|------|------|
| 関数外へ escape する値 | RC に昇格 |
| 複数箇所から共有される値 | RC に昇格 |
| クロージャ環境（キャプチャ変数） | RC に昇格 |
| 上記以外 | arena に留まる |

循環参照: コンパイル時 reject（当面禁止。将来 `Weak<T>` で解決）。

#### 実装順序

1. **arena のみ**（先行）: linear memory 版を最小構成で成立させる
2. **arena + RC hybrid**（次段）: escape 解析でオブジェクトを arena / RC に分類

### wasm32 プロファイルが課す言語設計制約

以下は GC 依存かつ lowering 不可能なため、言語仕様から除外する:

| 除外機能 | 理由 |
|---------|------|
| finalizer の実行タイミング保証 | arena では解放タイミングが不定 |
| 弱参照（`Weak<T>`） | GC の到達可能性に依存（当面禁止、将来追加予定） |
| 循環参照グラフのユーザー作成 | RC ではリークが発生 |

---

## 結果

以下のドキュメントを更新する:

- [x] `docs/language/memory-model.md` — メモリモデル詳細仕様
- [x] `docs/language/type-system.md` — ADR-002 待ちセクション更新
- [x] `docs/platform/wasm-features.md` — reference types / Wasm GC を Layer 1 に
- [x] `docs/stdlib/core.md` — String / Vec の実装方針を GC 前提で記載
- [x] `docs/process/benchmark-results.md` — 実測値と判定根拠を記録
- [x] `docs/platform/wasm-features.md` — wasm32 ターゲット層を追記
- [x] `docs/language/memory-model.md` — wasm32 lowering モデルを追記

## Implementation Status (2026-03-27)

GC-native codegen is **complete** in the T3 emitter (`t3_wasm_gc.rs`).
All 346 fixture tests pass. `verify-harness.sh` exits 0 (16/16 checks).

### Key design decisions implemented

| Feature | GC-native approach |
|---------|-------------------|
| Strings | `(array (mut i8))` — bare GC byte array, `array.len` for length |
| User structs | `(struct (field ...))` — direct GC struct with typed fields |
| Enums | Subtype hierarchy + `br_on_cast` for pattern matching |
| Option/Result | Enum subtypes: base (empty) + Some/Ok/Err variants |
| Vec\<T\> | `(struct (ref $arr_T) i32)` — GC struct + GC array backing |
| HashMap\<K,V\> | `(struct (ref $arr_K) (ref $arr_V) i32)` — array-backed linear scan |
| Generics | `anyref` polymorphism with `ref.i31` boxing/unboxing |
| Tuples | `__tupleN_any` structs with anyref fields for generic contexts |
| Linear memory | 1 page (64KB), used **only** for WASI I/O marshaling |
| Closures | Parameter-passing (captures as extra args), `call_ref` for HOF |
| Global section | No `heap_ptr` — all allocation via `struct.new`/`array.new` |
