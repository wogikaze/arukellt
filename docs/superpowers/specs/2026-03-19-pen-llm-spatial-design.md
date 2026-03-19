# `.pen` を LLM 向け半構造化データとして使う空間設計

## 概要

この設計の目的は、`research.pen` を「人間が眺める KJ 法キャンバス」から、「LLM が安定して読める半構造化データ」へ変換することにある。

対象用途は次の 2 つ。

- `pen -> 設計議論`
- `pen -> 言語仕様生成 / コード生成`

前提として、LLM は単なる座標の近接関係だけでは安定した推論が難しい。したがってこの設計では、空間配置そのものに意味を与えつつ、ノード名と関係名にも意味を冗長化する。

## 問題設定

現状の `research.pen` は、KJ 法ベースの自由配置としては十分に整理されている。実際に `Island::...` や `Nameplate::...` や `Relation::...` という命名はすでに入っている。

ただし、LLM の入力として見ると次の問題がある。

- 近い配置が「なぜ近いのか」を機械的に確定できない
- 色やサイズが主に視覚補助であり、機械的契約になっていない
- relation の意味が人間には読めても、抽出規則としてはまだ弱い
- 設計議論と仕様生成の間に安定した中間表現がない

このため、微妙なレイアウト変更がそのまま解釈の揺れにつながる。

## 採用方針

採用するのは `型付き議論レーン` モデルである。

基本方針は次の 4 点。

- 列は設計論点を表す
- 行は議論段階を表す
- ノードは型付き主張として扱う
- 関係線は型付き関係として扱う

ここでいう「型付き」は型システムの意味だけではなく、ノードや関係が持つ役割が明示されることを指す。

## 空間意味論

### 1. Island

`Island` は大きなトピック境界を表す。

例:

- `Island::Null Safety`
- `Island::Type Inference`
- `Island::Async Model`

1 つの Island は、1 つの設計テーマについての議論と根拠を収める最小の文脈単位とする。

### 2. Column

列は設計論点を表す。`.pen` 上の列コンテナ名は `Lane::` を使う。初期セットは次の 4 列を採用する。

- `Syntax`
- `Type`
- `Effect / Error`
- `Runtime`

必要なら将来 `Module` や `Interop` や `Tooling` を追加できるが、最初から列を増やしすぎない。

### 3. Row

行は議論段階を表す。読み順もこの順序に固定する。

- `Goal`
- `Decision`
- `Rule`
- `Example`
- `Risk`

意味は次の通り。

- `Goal`: なぜこの設計が必要か
- `Decision`: 採用した高水準の設計判断
- `Rule`: 仕様に落とせる具体規則
- `Example`: 規則を支える具体例や反例
- `Risk`: 欠点、トレードオフ、未解決点

### 4. Space Contract

LLM はまず Island を見てトピック境界を取り、その後に列で論点、行で議論段階を取る。

つまり座標は「近いから関連」ではなく、次のように読む。

- 同じ列: 同じ論点領域
- 同じ行: 同じ議論段階
- 上から下: 根拠から規則、規則から例、例からリスクへ進む

## ノード契約

空間配置だけでは曖昧さが残るため、各ノードは名前に役割を埋め込む。

ここでの用語は次の通り。

- `Column`: 視覚上の縦列という概念
- `Lane::...`: その Column を表現する `.pen` 上のコンテナ名
- `Row::...`: 行を表現する `.pen` 上のコンテナ名

### 必須 prefix

- `Island::`
- `Lane::`
- `Row::`
- `Claim::Goal::`
- `Claim::Decision::`
- `Claim::Rule::`
- `Claim::Example::`
- `Claim::Risk::`
- `SourceTag::`
- `TradeoffTag::`

### claim ノードの標準形

推奨形:

`Claim::<Stage>::<short-id>`

例:

- `Claim::Decision::non-null-default`
- `Claim::Rule::nullable-only-explicit`
- `Claim::Risk::verbosity-cost`

本文テキストには人間向けの自然文を置き、`name` には安定した短い識別子を置く。

### claim が持つべき意味

1 つの claim は最小限次の情報に対応する。

- `island`
- `domain`
- `stage`
- `claim_id`
- `text`
- `sources`
- `tradeoffs`
- `status`

`.pen` の生データ上ではこれを完全な JSON プロパティで持つ必要はない。少なくとも `name` と子テキストと補助タグで再構成できればよい。

## relation 契約

relation も見た目の矢印ではなく意味を持つエッジとして扱う。

### 必須 prefix

- `Rel::supports`
- `Rel::conflicts`
- `Rel::derived-from`
- `Rel::refines`
- `Rel::example-for`

必要最小限は `supports`, `conflicts`, `derived-from` の 3 種で十分。

### relation の役割

- `supports`: 根拠や補助
- `conflicts`: トレードオフや競合
- `derived-from`: ある判断が別の判断や出典から導出されたこと
- `refines`: 抽象判断が具体規則に展開されたこと
- `example-for`: 例がどの規則や判断に対応するか

relation の意味は path 名で決まり、見た目の線種や色は補助とする。

## LLM 読解アルゴリズム

LLM が `.pen` を読むときの基本順序は次の通り。

1. `Island` でトピック境界を取る
2. ノードの座標から `column -> domain`, `row -> stage` を決める
3. `Claim::*` の `name` から claim 種別と短い ID を取る
4. 子テキストから人間向け本文を取る
5. `SourceTag::*` と `TradeoffTag::*` を付随情報として結合する
6. `Rel::*` を辿って supports/conflicts/derived-from を再構成する
7. これらを中間表現へ正規化する

重要なのは、座標は主契約ではなく補助契約だという点である。主契約は `name` と typed relation に置く。

## 中間表現

`.pen` から直接仕様文やコードを生成しない。まず `semantic graph` へ正規化する。

概念上の例:

```json
{
  "island": "null-safety",
  "domain": "type",
  "stage": "decision",
  "claim_id": "non-null-default",
  "text": "参照型は non-null default にする",
  "sources": ["dr1:35", "dr1:36"],
  "tradeoffs": ["verbosity"],
  "status": "adopted"
}
```

relation も別途 graph edge として保持する。

この中間表現を唯一の安定した生成入力とする。

## 生成パイプライン

パイプラインは次の順にする。

1. `.pen` から Island, Row, Column, Claim, Rel を抽出する
2. `semantic graph` に正規化する
3. graph から議論用の要約を作る
4. 同じ graph から仕様断片を作る
5. 仕様断片からコード生成プロンプトやサンプルを作る

### 議論用出力

議論用には次を出す。

- 未決定の decision
- conflict が解消されていない論点
- source が不足している claim
- rule に落ちていない goal

### 仕様用出力

仕様用には次を出す。

- 採用済み decision の一覧
- rule の文章化
- grammar 断片
- typing rule 断片
- evaluation/runtime rule 断片
- examples

### コード生成用出力

コード生成には `.pen` をそのまま渡すのではなく、spec 断片と semantic graph の relevant subgraph を渡す。

## 現在の `research.pen` からの移行

現状の `research.pen` は完全に捨てる必要はない。むしろ次の順で段階移行できる。

1. 既存の `Island::...` は残す
2. `Nameplate::...` をトピック見出しとして残しつつ、実データノードとは分離する
3. sticky note 群を `Claim::*` に改名する
4. relation を `Rel::*` へ置き換える
5. source を本文埋め込みから `SourceTag::*` に分離する
6. レーン構造を導入して row/column 契約を固定する

最初から全ノードに完全契約を課すより、トピック単位で Island を 1 つずつ移行する方が安全である。

## 運用規則

### ルール

- 色は主契約にしない
- サイズは主契約にしない
- 近接だけで意味を表現しない
- 1 ノード 1 claim を守る
- claim 本文に source や tradeoff を混ぜすぎない

### 禁止

- 「なんとなく近いから関連」という配置
- relation の意味を色だけで区別すること
- decision と rule を 1 ノードに同居させること
- 人間にはわかるが名前から意味が取れないノード名

## エラーハンドリング

抽出時に曖昧さが出た場合の扱いを先に決める。

- Island が不明: claim を不正ノードとして隔離
- Row/Column が不明: claim を `unclassified` として報告
- relation 型が不明: `unknown-relation` として報告
- source 不足: 議論出力では警告、仕様出力では保留扱い

この方針により、LLM が勝手に補完して仕様へ昇格させる事故を抑える。

## テスト方針

この設計が成立しているかは、見た目ではなく抽出結果で検証する。

### 検証項目

- 同じ Island を複数回抽出しても同じ semantic graph になる
- 位置微調整で graph が変わらない
- claim 名変更が抽出に反映される
- conflict が議論出力に確実に出る
- adopted rule だけが仕様出力に入る

### 段階的検証

- まず 1 Island だけを手作業で contract 化する
- そこから抽出スクリプトの期待出力を固定する
- 仕様断片が人間レビューで納得できるかを見る

## 採用決定

この設計では次を採用する。

- `型付き議論レーン`
- `row semantic + column semantic`
- `prefix ベースの naming contract`
- `typed relation`
- `semantic graph` を挟む生成パイプライン

## 保留事項

現時点で未確定の点は次の通り。

- 列集合を 4 列で固定するか、テーマごとに拡張可能にするか
- claim の `status` を `.pen` 上でどう持つか
- source/tag を別ノードにするか子要素にするか
- semantic graph を JSON にするか別の軽量 DSL にするか

## 推奨する次ステップ

次にやるべきことは実装ではなく、`research.pen` の 1 Island を選んでこの契約へ写し替える計画を作ることである。

候補としては次が扱いやすい。

- `Null Safety`
- `Type Inference`
- `Async Model`

この 1 テーマでテンプレートと抽出規則を固めてから、他の Island へ広げる。
