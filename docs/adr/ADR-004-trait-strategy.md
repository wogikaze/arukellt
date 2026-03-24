# ADR-004: trait を v0 に入れるか

ステータス: **DECIDED**

決定日: 2026-03-24

## 文脈

trait（または interface）は以下のすべてに影響する:

- iter の設計（trait がないと for-each が汚くなる）
- HashMap の equality/hash 抽象化
- 演算子オーバーロード
- 動的ディスパッチ（vtable）の使い時
- LLM フレンドリ性（最も LLM が壊しやすい解決規則を持つ機能）

**LLM フレンドリを本気で目標にするなら、trait はかなり遅らせるべき。**

LLM が壊しやすいのは型構文ではなく「どの impl が選ばれるか」「どの解決規則が走るか」。trait を入れるなら、coherence/orphan rule を最初から制約しないと崩壊する。

## 選択肢

### 選択肢 A: v0 では trait なし

generics はあるが、trait/interface はない。静的ディスパッチのみ。

利点:
- 解決規則が単純
- LLM フレンドリ
- コンパイラ実装が軽い
- エラーメッセージが追いやすい

欠点:
- iter が書けない（for 構文のサポートが難しい）
- HashMap の Key 制約を表現できない
- 演算子オーバーロードなし

→ 結果として v0 の std は `mem / option / result / string / vec` まで。`iter / HashMap` は Phase 2 以降。

### 選択肢 B: 最小限 trait を v0 に入れる

`Display`, `Eq`, `Hash`, `Into/From` 相当のごく少数の組み込み trait のみ。ユーザー定義 trait は禁止。

利点:
- 演算子や比較が書ける
- HashMap の Key 制約を付けられる

欠点:
- 「組み込みと非組み込みの差」が LLM には不透明
- 解決規則を部分的に実装する必要がある（coherence の半端な実装は後で壊れる）

### 選択肢 C: 静的ディスパッチ trait のみ（Phase 2 相当を前倒し）

dyn を禁止し、静的ディスパッチのみの trait システムを入れる。

利点:
- iter が自然に書ける
- 将来の拡張パスが綺麗

欠点:
- trait 解決規則を最初から全部実装する必要がある
- v0 のスコープとして重い

## 現時点の方向性

v0 スコープでは **選択肢 A**（trait なし）が最も安全。

- iter / HashMap を後回しにする覚悟があるなら A
- for 構文がどうしても必要なら B（組み込みイテレータプロトコルのみ）

**trait を入れるタイミングで決めること:**
- coherence/orphan rule の制約を先に仕様化する
- 解決規則を単純化した上で入れる
- 演算子オーバーロードは trait と同時に設計する（分離は混乱する）

## 決定

**選択肢 A: v0 では trait なしを採用する**

### 決定内容

v0 では trait / interface を導入しない。以下を受け入れる:

- `iter` と `HashMap` は v0 に入れない
- `for` 構文は v0 に入れない
- 演算子オーバーロードは v0 に入れない

### 代替として提供するもの

**組み込み比較関数**:

```
// 明示的な比較関数（trait なしで動作）
fn i32.eq(a: i32, b: i32) -> bool
fn i32.lt(a: i32, b: i32) -> bool
fn String.eq(a: String, b: String) -> bool

// パターンマッチでの等値比較
match x {
    0 => "zero",
    1 => "one",
    _ => "other",
}
```

**while ループで代替**:

```
// for がないため while で書く
let mut i = 0
while i < len(v) {
    let item = get(v, i)
    match item {
        Some(x) => {
            // process x
        },
        None => {},
    }
    i = i + 1
}
```

**v1 段階的導入**: trait 導入前に、限定版 `for`（P1: `for i in 0..n`, `for x in values(v)`）と文字列補間（P2）を先行導入可能。組み込み反復プロトコルで橋渡しし、trait 導入（P3）後にメソッド構文（P4）と演算子オーバーロード（P5）を追加する。

### 根拠

1. **LLM フレンドリ性**
   - trait 解決規則は LLM が最も壊しやすい
   - coherence/orphan rule を正確に守るのは難しい
   - 「どの impl が選ばれるか」の判断ミスが多い

2. **コンパイラ実装の単純化**
   - trait 解決器は実装が複雑
   - v0 のスコープを小さく保つ

3. **段階的な導入が可能**
   - v0 で基本機能を安定させてから trait を追加
   - 後から trait を入れても既存コードは壊れない

## 結果

以下のスコープ変更を反映:

- [x] `docs/stdlib/README.md`: iter/HashMap を Phase 2 以降に
- [x] `docs/language/syntax.md`: for 構文なし、impl/メソッド構文なし
- [x] `docs/process/v0-scope.md`: trait なしの明記
