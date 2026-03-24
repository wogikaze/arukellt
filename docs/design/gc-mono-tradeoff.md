# GC + Monomorphization トレードオフ

ADR-002（GC採用）と ADR-003（制限付きmono）の緊張関係を明確化する。

---

## 問題

GC と monomorphization は**逆方向**の設計選択:

| 選択 | サイズへの影響 | 性能への影響 |
|------|---------------|-------------|
| Wasm GC | 小さくなる（管理ルーチン不要） | 遅くなりうる（GCオーバーヘッド） |
| Monomorphization | 大きくなる（型ごとにコード生成） | 速くなる（specialized） |

これらは打ち消し合う。「両方の良いところを取る」は幻想。

---

## v0 の優先順位

**「単純さ」を最優先。サイズ・性能は次点。**

理由:
- LLM フレンドリ = 予測可能な動作
- v0 は「動く」ことが目標
- 最適化は v1 以降

---

## 具体的な制限

### 1. Specialization の範囲

| 型 | Specialization | 理由 |
|----|----------------|------|
| `i32`, `i64`, `f32`, `f64` | ✅ 常にspecialize | 値型は specialized が必須 |
| `bool`, `char` | ✅ specialize | 値型 |
| `String`, `Vec[T]` | ⚠️ 参照として統一 | サイズ優先 |
| `Option[T]` (T=値型) | ✅ specialize | tagged union |
| `Option[T]` (T=参照型) | ⚠️ nullable ref | サイズ優先 |
| `Result[T, E]` | ✅ specialize | 常に tagged union |
| ユーザー struct | ⚠️ 参照として統一 | サイズ優先 |

### 2. 統一表現（Uniform Representation）

参照型は以下の統一表現を使用:

```wasm
;; 全ての参照型は (ref $object) として扱える
(type $object (struct))
```

ただしフィールドアクセス時に downcast が必要:

```wasm
;; Vec[String] と Vec[i32] は同じ $vec 型
;; ただし要素取得時に型チェック
(ref.cast $string (array.get $vec_data ...))
```

### 3. コード生成ルール

```
generic function の生成:
1. 全ての型パラメータを「値型」「参照型」に分類
2. 値型の組み合わせごとに specialized 版を生成
3. 参照型は統一表現で共有

例: fn foo<T>(x: T) -> T
- foo<i32>: specialized
- foo<i64>: specialized
- foo<String>: 共通の foo<ref> を使用
- foo<Vec<i32>>: 共通の foo<ref> を使用
```

---

## サイズ上限

v0 での目安:

| ケース | 目標サイズ |
|--------|-----------|
| Hello World | 2KB 以下 |
| 数値計算（フィボナッチ等） | 5KB 以下 |
| ファイル読み書き | 10KB 以下 |
| 中規模アプリ（500行相当） | 50KB 以下 |

これを超えた場合は mono の範囲を見直す。

---

## 性能への影響

GC + 統一表現 による性能低下:
- 参照型のフィールドアクセス: +1 downcast
- 配列アクセス: +1 bounds check + downcast

### downcast コスト許容基準

`ref.cast` は Wasm ランタイムが型情報を即座にチェックする命令。
理論上は分岐予測が効く範囲なら無視できるが、定量基準を設ける：

| 指標 | 許容ライン | 超過時のアクション |
|------|-----------|-----------------|
| hot loop 内の downcast 比率 | 全命令の 10% 以下 | 値型特化を検討 |
| 連続 downcast（配列走査等） | 3 連続以下 | インライン化を検討 |
| downcast 失敗率（実行時） | 0%（型安全なら起きない） | コンパイラバグ |

**v0 での対応方針**:
- 値型（i32/i64/f32/f64）は downcast 不要（specialized）
- 参照型は downcast 必須だが、hot path は限定的
- **計測優先**: 実装後にベンチマークで確認し、超過時に mono 範囲を拡大

**ワーストケース見積もり**: `Vec<String>` の全走査
```
// 要素アクセスごとに +1 ref.cast（String への downcast）
// 1000 要素 → 1000 downcast
// Wasm ランタイムの ref.cast は ≈ 1ns（V8/wasmtime 実測値）
// → 1μs のオーバーヘッド（許容範囲）
```

許容範囲:
- C 比 2.0x 以内（v0 目標）
- C 比 1.5x 以内（v1 目標）

---

## 検証方法

ベンチマーク実行時に以下を測定:
1. バイナリサイズ
2. 実行時間
3. mono で生成された関数の数

サイズ上限を超えた場合のアクション:
1. まず統一表現の範囲を広げる
2. それでもダメなら mono の制限を強化

---

## 関連

- ADR-002: GC 採用
- ADR-003: 制限付き monomorphization
- `docs/language/memory-model.md`: 型の表現
