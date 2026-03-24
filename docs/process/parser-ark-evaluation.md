# parser.ark 翻訳評価レポート

parser.rs (Gloss markup parser, 1028 行 Rust) → parser.ark (arukellt v0) の翻訳結果を分析し、言語の書きやすさと v1 改善候補を特定する。

---

## 1. 数値比較

| メトリクス | parser.rs (Rust) | parser.ark (arukellt v0) | 比率 |
|-----------|-----------------|-------------------------|------|
| 行数 | 1,028 | 1,412 | 1.37x |
| バイト数 | 43,613 | 52,419 | 1.20x |

行数増加 (37%) のうち、意味的な複雑さの増加はほぼゼロ。増分の大半は**表面構文の貧しさを手展開で埋めた儀式的コード**。

---

## 2. v0 の強み（機能した設計）

### 2.1 代数的データ型

`enum` / `struct` / `match` は Rust とほぼ同じ感覚で書ける。parser のような分岐の多いコードを移植できたことで、**言語コアは破綻していない**ことが実証された。

```
enum Tag {
    Heading(i32),
    Paragraph,
    CodeBlock(String, String),
    ...
}

match tag {
    Tag::Heading(level) => ...,
    Tag::Paragraph => ...,
}
```

### 2.2 Option / Result

`match` ベースの `Option<T>` / `Result<T, E>` は自然に書ける。パターンの網羅性チェックがある前提なら、Rust と同等の安全性を提供。

### 2.3 関数呼び出し統一

`.method()` が無くても、`starts_with(text, prefix)` のような呼び出し形式は**慣れれば読みやすい**。引数が 1 つ増える程度の不便さは本質的ではない。

### 2.4 型注釈

型推論と明示的型注釈のバランスは適切。`let mut v: Vec<String> = Vec_new_String()` は冗長だが明確。

---

## 3. v0 の弱み（行数増加の原因分析）

### 3.1 Iterator 不在（最大の要因、推定 +100〜120 行）

`.iter()`, `.enumerate()`, `.any()`, `.find()`, `.take_while()`, `.map()` がすべて `while i < len(v)` + 手動インデックスに展開された。

**問題の本質**: 記法の問題ではなく、**「標準的な走査の意図」がコードから消える**こと。LLM は手展開された while ループで off-by-one / 境界バグを起こしやすい。

Rust:
```rust
if para.iter().all(|l| l.trim().is_empty()) { ... }
```

Arukellt v0:
```
let mut all_empty = true
let mut k = 0
while k < len(para) {
    if len(trim(get(para, k))) != 0 {
        all_empty = false
    }
    k = k + 1
}
if all_empty { ... }
```

### 3.2 Closure → 名前付き関数（推定 +60〜80 行）

Rust のインライン closure（`pop_section`, `close_sections_until`, `parse_cells`, `is_table_line` 等）をトップレベル関数に分解。v0 は closure をサポートしているが、ローカルな文脈を大量にキャプチャする必要がある場合、引数の爆発が起きるため名前付き関数が選ばれた。

### 3.3 `format!` 不在（推定 +40〜60 行）

文字列構築が `concat(a, concat(b, concat(c, d)))` のネスト地獄になる。可読性が著しく低下。

Rust:
```rust
format!("Unknown embed type '{}': only 'card' is supported.", type_name)
```

Arukellt v0:
```
concat("Unknown embed type '", concat(type_name, "': only 'card' is supported."))
```

3 要素ならまだ読めるが、5 要素以上のネストは崩壊する。

### 3.4 `for` ループ不在（推定 +30〜40 行）

`for` が無いこと自体の行数影響は小さい。`while` との差は `let mut i = 0` と `i = i + 1` の 2 行分。ただし iterator と組み合わさると影響が大きくなる。

### 3.5 翻訳時のバグ：`break` の不使用

**`break` / `continue` は v0 仕様に含まれている**にもかかわらず、初回翻訳では sentinel hack（`j = len(lines) + 1`）で代替していた。このパターンは：

1. **実際のバグを含んでいた** — sentinel は `j = len(lines)` にリセットするため、`break` 後の正しい `j` 値を保持しない
2. **LLM が v0 仕様を正しく認識しなかった**証拠 — 仕様に `break` があっても、制限の多い言語という印象で「break もない」と推定した

修正後は `break` を直接使用。-4 行。

---

## 4. 翻訳パターン一覧

| Rust パターン | Arukellt v0 翻訳 | 行数影響 |
|-------------|-----------------|---------|
| `impl` ブロック | モジュールレベル関数 (`TypeName_fn`) | +20〜30 |
| `Iterator` trait | `Vec` に全イベントを collect | +10 |
| `.iter().any()` 等 | `while` + フラグ変数 | +4〜6 per use |
| `matches!(c, 'a'..='z')` | `char_to_i32(c)` + 明示的範囲比較 | +3〜4 per use |
| `format!("...")` | ネスト `concat()` | +1〜3 per use |
| `&str` | `String`（GC 参照） | 0（意味論変更のみ） |
| `break` / `continue` | **`break` / `continue`**（v0 対応済み） | 0 |
| `if let Some(x) = ...` | `match ... { Some(x) => ..., None => ... }` | +2 per use |
| `v.push(x)` | `push(v, x)` | 0（記法差のみ） |
| inline closure | named helper function | +5〜10 per closure |

---

## 5. 使用した stdlib 関数

翻訳で前提としたが未実装の stdlib 関数：

### 必須（なければ parser が書けない）

| 関数 | シグネチャ |
|------|----------|
| `str_find` | `fn(String, String) -> Option<i32>` |
| `substring` | `fn(String, i32, i32) -> String` |
| `starts_with` | `fn(String, String) -> bool` |
| `ends_with` | `fn(String, String) -> bool` |
| `trim` / `trim_start` | `fn(String) -> String` |
| `split` | `fn(String, String) -> Vec<String>` |
| `concat` | `fn(String, String) -> String` |
| `char_at` | `fn(String, i32) -> char` |
| `char_to_i32` | `fn(char) -> i32` |

### あると便利

| 関数 | 用途 |
|------|------|
| `wrapping_mul_i64` | FNV-1a ハッシュ |
| `char_at_char_index` | Unicode 文字インデックスアクセス |
| `char_byte_len` | UTF-8 バイト長 |
| `byte_at` | 生バイトアクセス |

---

## 6. LLM フレンドリ性の評価

### 成功

- 構文の正規形が少ない → LLM が生成しやすい
- 型注釈が明示的 → 推論ミスが減る
- メソッドなし → 名前空間衝突が起きない
- `match` が式 → 条件分岐の表現力は十分

### 失敗

- **LLM 自身が v0 の `break` 存在を見落とした** — 制限言語という先入観で仕様を過小推定
- `while` 手展開は off-by-one バグの温床
- `concat` ネストは括弧の対応ミスを誘発
- 型特化コンストラクタ（`Vec_new_String`）は LLM が型名を間違えやすい

### 結論

v0 は「書ける」が、**手展開パターンの多さが LLM のエラー率を上げる**。これは文法の問題ではなく、**安全で正規な反復パターンの欠如**が原因。

---

## 7. v1 改善候補（優先順位付き）

翻訳結果に基づく、実効性の高い順：

### P1: `for` ループ（trait 不要の限定版）

**効果**: 推定 -80〜100 行（parser.ark 基準）

trait ベースの Iterator を待たずに、**限定版 for** を導入：

```
// 範囲ベース
for i in 0..len(v) {
    let item = get(v, i)
}

// Vec 走査（組み込み）
for item in values(v) {
    ...
}
```

設計コストが低く、while 手展開の大半を解消。LLM のバグ率に最も効く改善。

### P2: 文字列補間

**効果**: 推定 -40〜60 行、可読性大幅向上

```
f"Unknown embed type '{type_name}': only 'card' is supported."
```

concat ネストの完全解消。diagnostics 実装の品質にも直結。

### P3: Iterator / 高階走査

**効果**: 推定 -60〜80 行

P1 の `for` が入った後、`any`, `find`, `all`, `map`, `filter` を Vec に対して提供。trait ベースでも builtin でも良い。

### P4: メソッド構文

**効果**: 読みやすさ向上、行数影響小

`push(v, x)` → `v.push(x)` は見た目の問題。本質改善ではないが、Rust/Go/Python 経験者の学習コストを下げる。

### 保留: tuple pattern in match

parser ではほとんど使わなかったが、複数値を同時に match する場合に必要になる。

---

## 8. 設計への影響

### 確認された設計上の強み

1. **コア型システムは想像より健全** — enum/struct/match/Option/Result は parser 規模のコードを支えられる
2. **関数呼び出し統一はメソッドなしでも成立** — 引数 +1 のコストは許容範囲
3. **GC + 参照セマンティクスは parser には透明** — 所有権の心配なく書けた

### 確認された設計上の弱み

1. **本当に弱いのは trait より前に、反復と文字列構築** — 高度な抽象化ではなく、最低限の表面構文が不足
2. **LLM は制限言語の仕様を過小推定する傾向** — `break` 見落とし事件が証拠。ドキュメントの「何ができるか」を強調する必要

### 一言でまとめると

> v0 は parser を書ける。  
> ただし冗長な while 方言の山になる。  
> v1 の最優先は「高度な抽象化」ではなく「while 手展開を減らす最低限の制御構文追加」。
