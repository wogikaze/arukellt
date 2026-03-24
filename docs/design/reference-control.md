# 参照過多への制御戦略

GC 環境で「すべてがヒープに逃げる」リスクを制御する。

---

## 問題

GC を採用すると:
- ヒープ割り当てが「安い」と感じる
- すべてを参照型にする誘惑
- 結果: 予測不能なコストモデル

**「書きやすいが予測不能」な言語になるリスク。**

---

## 制御戦略

### 1. 値型を明確に維持

プリミティブ型は**絶対に**ヒープに逃げない:

```
let x: i32 = 42      // スタック上
let y = x + 1        // スタック上で計算
```

コンパイラは以下を保証:
- i32/i64/f32/f64/bool/char は常にスタック
- 小さい tuple もスタック

### 2. 構造体のインライン化

小さい struct は「論理的に参照型」だが、フィールドに埋め込む:

```
struct Point { x: f64, y: f64 }
struct Line { start: Point, end: Point }

// Line のメモリレイアウト:
// ┌─────────┬─────────┬─────────┬─────────┐
// │ start.x │ start.y │ end.x   │ end.y   │
// └─────────┴─────────┴─────────┴─────────┘
// Point への参照は持たない（インライン）
```

**ルール**: フラットな struct はインライン化を試みる。

### 3. スタック脱出分析（将来）

v1 以降で検討:
- 関数内でのみ使われるオブジェクトはスタックに割り当て
- 脱出する場合のみヒープへ

```
fn example() {
    let p = Point { x: 1.0, y: 2.0 }  // スタックでよい
    p.x + p.y                          // 脱出しない
}

fn example2() -> Point {
    let p = Point { x: 1.0, y: 2.0 }  // ヒープ必須
    p                                  // 戻り値として脱出
}
```

v0 では全ての struct をヒープに割り当てる（単純さ優先）。

---

## 明示的なコスト表示

### コンパイラ警告（将来）

```
warning: large struct copied by value
  --> src/main.ark:10:5
   |
10 |     let copy = big_struct
   |     ^^^^^^^^^^^^^^^^^^^^
   |
   = note: BigStruct is 256 bytes
   = help: consider using a reference
```

v0 では提供しない。v1 以降で検討。

### ドキュメントでのコスト明示

std の各関数にコストを記載:

```
/// Appends an element to the vector.
/// 
/// # Cost
/// - Time: O(1) amortized
/// - Space: may trigger reallocation
fn Vec<T>.push(self, val: T)
```

---

## GC 境界の可視化

### 値型と参照型の区別を構文で明示

案 1: 型名で区別（採用）
- 値型: 小文字始まり or プリミティブ
- 参照型: 大文字始まり

```
let x: i32 = 42      // 値型（小文字）
let s: String = ...  // 参照型（大文字）
```

案 2: 明示的マーカー（不採用）
```
let x: value i32 = 42   // 冗長
let s: ref String = ... // 冗長
```

### 参照の共有を明示

同じオブジェクトを共有していることを意識させる:

```
let s1 = String::from("hello")
let s2 = s1  // s1 と s2 は同じオブジェクト

// 明示的なコメントを推奨
let s2 = s1  // sharing: s1 and s2 point to the same object
```

---

## 設計指針

### 「参照のコストはゼロではない」

以下のコストを意識:
1. ヒープ割り当て: ~10ns
2. GC トレース: オブジェクト数に比例
3. 間接参照: キャッシュミスの可能性

### 「必要なものだけ参照型に」

| ケース | 推奨 |
|--------|------|
| 小さい座標 (x, y) | tuple `(f64, f64)` |
| 設定オブジェクト | struct (参照型 OK) |
| 大きなデータ | Vec/String (参照型必須) |
| 一時的な中間値 | 値型で返す |

### 「clone は意識して使う」

```
// Bad: 意図せず大量コピー
fn process(items: Vec[String]) -> Vec[String] {
    let result = items  // 参照コピー（意図通り？）
    ...
}

// Good: 意図を明示
fn process(items: Vec[String]) -> Vec[String] {
    let result = items.clone()  // 明示的なコピー
    ...
}
```

---

## v0 での現実的なアプローチ

| 項目 | v0 での対応 |
|------|------------|
| 値型の維持 | ✅ プリミティブは常に値型 |
| struct のインライン化 | ⚠️ 限定的（フラット struct のみ） |
| スタック脱出分析 | ❌ v1 以降 |
| コスト警告 | ❌ v1 以降 |
| コスト文書化 | ✅ std に記載 |

---

## 測定とフィードバック

ベンチマーク時に以下を測定:
1. GC ヒープのオブジェクト数
2. GC 発生回数
3. GC ポーズ時間

異常値が出た場合:
1. 参照型の使用箇所を特定
2. 値型への置き換えを検討
3. 必要に応じて設計を見直す

---

## 関連

- `docs/design/value-semantics.md`: 値セマンティクス
- `docs/design/gc-mono-tradeoff.md`: サイズ/性能トレードオフ
- ADR-002: GC 採用
