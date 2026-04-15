# ADR-008: WasmGC Post-MVP 拡張機能 — 設計調査と Arukellt v5 評価

ステータス: **SURVEY** (v4 では実装しない; v5 設計判断の参考資料)

決定日: 2026-04-15

---

## 文脈

WebAssembly GC MVP (Phase 4) は 2023 年末に主要ランタイムへ統合され、
Arukellt v4 の主ターゲット T2/T3 はこれを前提としている (ADR-002, ADR-007)。

WebAssembly CG は現在、GC MVP を超える複数の "Post-MVP" 拡張提案を進めている。
これらは v4 のロードマップには含まれないが、v5 以降の言語設計判断
(型システム拡張・ランタイム統合・メモリ効率改善) に直接影響する。

本 ADR は各提案の現状・Arukellt としての活用可能性・実装コスト推定を記録し、
v5 設計フェーズへの入力とする。**実装决定ではない。**

参照仕様: `docs/spec/spec-3.0.0/proposals/gc/Post-MVP.md` (upstream draft)

---

## 調査対象の Post-MVP 提案一覧

| # | 提案 | W3C/CG ステータス | 優先度 (v5 視点) |
|---|------|-------------------|-----------------|
| 1 | Static Fields | Phase 1 | 高 |
| 2 | Weak References & Finalization | Phase 1 | 高 |
| 3 | Type Parameters (Generics) | Phase 1 | 中 |
| 4 | Final Types | Phase 1 | 中 |
| 5 | Typed Function References 改善 | Phase 3+ (v1.0) | 低 (既存機能の整備) |
| 6 | WasmGC × Exception Handling 統合 | Phase 2 | 中 |

---

## 1. Static Fields (静的フィールド)

### 提案概要

`structtype` に `static` フィールドを追加する。静的フィールドはインスタンスではなく
型に帰属し、Wasm モジュール内の全インスタンスで共有される。
アクセスは `struct.get_s <typeidx> <fieldidx>` / `struct.set_s` 命令で行う。

```wat
;; 提案シンタックス (draft)
(type $Counter (struct
  (static field $count (mut i64))
  (field $id i32)
))
(struct.set_s $Counter $count (i64.const 0))
```

### Arukellt での活用可能性

**高い価値がある。** 現時点で Arukellt は以下の手段で静的状態を模倣している:

- Wasm グローバル変数 (`global.mut`) — 型安全でなく、GC オブジェクトを保持できない
- ヒープ上のシングルトン struct — 参照を別のグローバルで保持する必要がある

Static fields が実装されれば:

1. `static` キーワード付きフィールドを `struct.get_s` / `struct.set_s` に直接コンパイル
2. `std::core::Counter` 等のモジュールレベルカウンターをクリーンに実装できる
3. メモリモデル (ADR-002) の「GC オブジェクトをグローバルに保持できない」制約が解消

#### 詳細設計案 (Arukellt v5 向け)

```
// Arukellt 構文案
struct Counter {
    static mut count: i64 = 0;
    id: i32,
}

impl Counter {
    fn increment() {
        Counter::count += 1;
    }
    fn get() -> i64 {
        Counter::count
    }
}
```

コンパイルフロー:

```
HIR: StaticField { owner: TypeId, field: FieldIdx, ty: Ty }
     ↓
MIR: GetStaticField / SetStaticField
     ↓
WasmGC emit: struct.get_s <typeidx> <fieldidx>
             struct.set_s <typeidx> <fieldidx>
```

- `static mut` フィールドは MIR の `Mutability::Mutable` でタグ付けし、
  shared reference を通じたアクセスは型チェックで禁止
- スレッド安全性 (shared-everything-threads 提案) は v5 スコープ外

### 実装コスト推定

| 工程 | コスト |
|------|--------|
| 構文追加 (`static` modifier) | 小 (1–2 日) |
| HIR/MIR 追加 | 小 (1–2 日) |
| T2/T3 emitter 追加 | 小 (1 日) |
| 型チェック (`static` に対する `&mut` 制約) | 中 (2–3 日) |
| stdlib 更新 | 中 (既存 workaround の置換) |
| **合計** | **1–2 週間** |

**ブロッカー**: ランタイム (wasmtime) が `struct.get_s` / `struct.set_s` を実装するまで
T3 では使用できない。現時点 (2026-04) では Phase 1 のため数年先の見込み。

---

## 2. Weak References & Finalization (弱参照とファイナライザ)

### 提案概要

GC マネージドオブジェクトへの `weakref<t>` 型参照を導入する。
弱参照は GC 到達可能性に影響せず、参照先が回収されると `null` になる。
`WeakMap` / ファイナライザ API も concurrent に設計される。

```wat
;; 提案シンタックス (draft)
(local $wr (weakref $MyStruct))
(ref.weak $wr)          ;; WeakRef を作成
(ref.deref_weak $wr)    ;; null | ref $MyStruct を返す
```

### Arukellt での活用可能性

**中〜高。** 主なユースケース:

1. **キャッシュ/メモ化**: キャッシュエントリが外部から参照されなくなったら自動削除
2. **Observer パターン**: Observer が死んでいれば自動的に通知リストから除去
3. **デバッグ/リーク検出**: `std::debug::WeakRef` として診断ツールに提供

ファイナライザは Arukellt の決定論的セマンティクスと相性が悪い面があるため、
`finalize` コールバックは **非同期 / best-effort 実行** として言語仕様の外に置く方針を推奨する。

#### 詳細設計案 (Arukellt v5 向け)

```
// Arukellt 構文案
use std::gc;

let obj = Box::new(42);
let weak: gc::Weak<i32> = gc::Weak::new(&obj);

match weak.upgrade() {
    Some(val) => println!("{}", val),
    None      => println!("collected"),
}
```

内部表現:

```
HIR: WeakRef<T>  →  WasmGC: weakref<$T_struct>
     Weak::new   →  ref.weak
     .upgrade()  →  ref.deref_weak + null check → Option<T>
```

ファイナライザ登録:

```rust
// std::gc::Finalizer trait (v5 案)
trait Finalizable {
    fn finalize(&mut self);
}
```

- `finalize` は GC 回収直前に呼ばれる保証はなく、ランタイム依存
- `unsafe` ブロック内のみで登録可能とし、決定論的コードから分離

### 実装コスト推定

| 工程 | コスト |
|------|--------|
| `gc::Weak<T>` 型システム統合 | 中 (3–5 日) |
| HIR/MIR `WeakRef` ノード | 中 (2–3 日) |
| T2/T3 emitter (`ref.weak`, `ref.deref_weak`) | 小 (1 日) |
| `Option<T>` 変換の型チェック | 中 (2 日) |
| `Finalizable` trait + unsafe 制約 | 大 (1 週間) |
| stdlib `std::gc` モジュール実装 | 中 (3 日) |
| **合計** | **3–4 週間** |

**ブロッカー**: `weakref<t>` 型は WasmGC MVP に含まれない。
Phase 1 提案であり、標準化は 2027 年以降の見込み。
T1 (linear memory) には適用不可。

---

## 3. Type Parameters (型パラメータ / Wasm レベルジェネリクス)

### 提案概要

`structtype` / `arraytype` に型パラメータを付与し、Wasm バイナリ内での
汎用コンテナ定義を可能にする。

```wat
;; 提案シンタックス (draft)
(type $Pair (param $A $B)
  (struct (field $fst $A) (field $snd $B)))
(type $IntPair (instance $Pair i32 i64))
```

### Arukellt での活用可能性

**中程度。** 現在 Arukellt は型消去 (type erasure) + `anyref` キャスト、または
モノモーフィズム展開によってジェネリクスを実装している。
Wasm レベルの型パラメータが実現すれば:

- **モノモーフィズム展開が削減**: バイナリサイズ圧縮 (特に大規模ジェネリクス)
- **型安全な汎用コンテナ**: `Vec<T>` を `arraytype` の型パラメータで表現
- **相互運用性向上**: 他言語が生成した汎用 Wasm 型との安全な型合わせ

ただし Arukellt のコンパイル戦略 (現在はモノモーフ優先) との整合を再検討する必要がある。
ADR-003 (Generics Strategy) を更新対象として記録しておく。

### 実装コスト推定

| 工程 | コスト |
|------|--------|
| 型システム拡張 (型パラメータの HIR 表現) | 大 (2 週間) |
| MIR ジェネリクス戦略変更 | 大 (2–4 週間) |
| T2/T3 emitter 対応 | 大 (1–2 週間) |
| stdlib `Vec<T>` 等の移行 | 大 (1 週間) |
| **合計** | **6–9 週間** |

**ブロッカー**: Phase 1 の中でも最も設計議論が多い提案。
実装は 2028 年以降が現実的。v5 では「調査・設計のみ」とし、
ADR-003 の将来検討項目として管理する。

---

## 4. Final Types (final 修飾子)

### 提案概要

`structtype` / `arraytype` に `final` 修飾子を付与し、
subtyping (サブ型付け) を禁止する。
`ref.cast` のランタイムコストを削減し、JIT 最適化を促進する。

```wat
(type $Leaf (sub final (struct (field i32))))
;; $Leaf を親とする struct は定義できない
```

### Arukellt での活用可能性

**中程度。** Arukellt の現在の WasmGC 出力では、すべての struct に
デフォルトで `(sub ...)` を付けており、継承が不要な型にも冗長なサブタイプ宣言が残る。

`final` を適用できる場面:

- クローズドな値型 (`struct` 意味論のもの)
- パフォーマンスクリティカルなコンテナ (`Vec` の内部ノード等)
- `enum` バリアント (discriminant 付き struct)

実装方針:

```
型チェックで `open_subtype = false` の struct を特定
  → emit 時に (sub final ...) を付与
```

### 実装コスト推定

| 工程 | コスト |
|------|--------|
| 型チェック: `final` 判定ロジック | 小 (1–2 日) |
| T2/T3 emitter: `sub final` 出力 | 小 (0.5 日) |
| stdlib の `final` 候補の特定・適用 | 中 (3 日) |
| **合計** | **4–6 日** |

**特記**: Phase 3+ で標準化が進んでいる。wasmtime は
`(sub final ...)` を既にサポートしている箇所があり、
早期に部分適用できる可能性がある (v4 後半の軽微改善候補)。

---

## 5. Typed Function References 改善

### 提案概要

`func.ref` 型精度向上、`ref.func` の型推論強化、
クロージャ構造体との統合改善などを含む一連の整備。
MVP 時点での `funcref` / `ref null func` の型制限を緩和する。

### Arukellt での活用可能性

**低〜中。** 現在の Arukellt クロージャ実装は
`struct { fn_ptr: funcref, captures: ref $ClosureEnv }` パターンを用いており、
typed funcref の精度が上がるとキャストコストが削減される。
ただし実用上すでに動作しており、優先度は低い。

### 実装コスト推定

既存の funcref 使用箇所のみ整理。**小 (2–3 日)**。

---

## 6. WasmGC × Exception Handling 統合

### 提案概要

Wasm Exception Handling (exnref 提案) と WasmGC の統合:
例外オブジェクトが GC 管理下の struct として定義可能になる。
`exnref` が `anyref` のサブタイプとなり、`ref.cast` でダウンキャストできる。

### Arukellt での活用可能性

**中。** Arukellt は現在:

- T1 (linear memory): 例外なし / panic = 即 abort
- T2/T3: Wasm 例外命令を使わず、`Result<T, E>` で値ベースエラー伝播

GC integration により:

1. `throw` / `catch` をネイティブ Wasm 例外で実装し、
   `E` 型を GC struct として直接保持できる
2. 外部 JS コードから投げられた例外を `anyref` / `exnref` でキャッチ
3. C FFI 境界での例外漏れを型システムで防ぐ

ただし `Result<T, E>` の値渡しモデルは Arukellt の設計に合致しており、
「例外命令への移行」はパフォーマンス以外の動機に乏しい。
**相互運用シナリオに限り採用を検討**する。

### 実装コスト推定

| 工程 | コスト |
|------|--------|
| `exnref` 型の HIR/MIR 表現 | 中 (3–5 日) |
| T3 emitter: `throw` / `catch` GC 統合 | 中 (1 週間) |
| `std::ffi::Exception` wrapperの設計 | 大 (2 週間) |
| **合計** | **3–4 週間** |

---

## まとめ表

| 提案 | 活用可能性 | 実装コスト | v5 優先度 | 早期適用可能か |
|------|-----------|-----------|-----------|----------------|
| Static Fields | 高 | 低〜中 | 高 | Phase 1完了後 |
| Weak References | 中〜高 | 中〜大 | 高 | 2027 年以降 |
| Type Parameters | 中 | 大 | 中 | 2028 年以降 |
| Final Types | 中 | 低 | 中 | 部分的に今すぐ |
| Typed FuncRef 改善 | 低〜中 | 低 | 低 | 順次適用 |
| Exceptions 統合 | 中 | 中〜大 | 低〜中 | 相互運用時のみ |

---

## v5 設計への推奨事項

1. **Static fields** を v5 の最優先 WasmGC 拡張として位置付ける。
   現在のグローバル変数 workaround を置換できる唯一の直接代替手段。

2. **Final types** は wasmtime 対応次第で v4 後半 (マイナーリリース) に
   emitter 側のみ先行対応できる。型チェックロジックは最小変更で済む。

3. **Weak references** は `std::gc` モジュールとして v5 初期リリースに含める。
   ただし `Finalizable` trait は `unsafe` 境界に隔離し、決定論的コードへの
   漏れを防ぐ設計とする。

4. **Type parameters** は ADR-003 (Generics Strategy) を revisit する際に
   再評価する。現在のモノモーフ戦略との整合コストが高いため、v5 後半以降。

5. **Exception handling 統合** は JS 相互運用 (T2) のユースケースが
   具体化した時点で設計を開始する。それまでは `Result<T, E>` を維持。

---

## 参照

- `docs/spec/spec-3.0.0/proposals/gc/Post-MVP.md`
- `docs/spec/spec-3.0.0/OVERVIEW.md`
- ADR-002: Memory Model (Wasm GC 採用決定)
- ADR-003: Generics Strategy
- ADR-006: ABI Policy (Layer 2B)
- ADR-007: Compile Targets
- [WebAssembly GC Proposal Post-MVP](https://github.com/WebAssembly/gc/blob/main/proposals/gc/Post-MVP.md)
- [WebAssembly Weak References Proposal](https://github.com/WebAssembly/weak-references)
- [Wasm Exception Handling v2](https://github.com/WebAssembly/exception-handling)
