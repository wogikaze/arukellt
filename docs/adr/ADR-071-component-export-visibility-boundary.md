# ADR-071: `pub` と Component export、WIT identity の境界

ステータス: **ACCEPTED** — source visibility、Component export、WIT identity を別契約として固定する
日付: 2026-09-19
トラック: language-v2
Issue: [#867](../../issues/open/867-component-wit-visibility-boundary.md)
関連: [ADR-006](ADR-006-abi-policy.md)、[ADR-009](ADR-009-import-syntax.md)、[ADR-031](ADR-031-import-syntax-wit-unification.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)

---

## 文脈

Arukellt の `pub` は source module 間の可視性を表す Layer S の指定である。
一方、WIT と Component Model の export は、外部 Component 境界へ公開する Layer C の契約である。
これまでの実装は `pub fn` を Component export root として再利用していたため、source module に公開しただけの関数が Component と raw Wasm の外部面へ流れ込んでいた。

また、`import "namespace:package/interface@version"` の WIT identity は、`use path::to::module` の source namespace と同じ名前解決規則ではない。
WIT の version を読み捨てる比較と、core Wasm import の ABI 用 version 短縮も、language identity と backend 表現を混同しやすい境界になっている。

## 決定

### 1. source visibility と Component export を独立させる

- `pub` は Layer S の source visibility だけを変更する。
- `pub` だけでは Component export、raw Wasm export、WIT export root を作らない。
- `export fn` は Layer C の Component export declaration であり、source visibility は変更しない。
- `pub export fn` は source module からも可視で、Component 境界へも export する。
- この issue の言語面では `export` を関数 declaration にだけ許可する。`export struct`、`export enum`、`export trait`、`export use` などは parser diagnostic とする。
- modifier の順序は `[pub] [async] export fn` とする。`export fn`、`pub export fn`、`async export fn`、`pub async export fn` を受け付ける。modifier を別順序で組み合わせる契約は追加しない。

Component export surface、WIT text、reachability root、contract validation は同じ `is_component_export` declaration fact を参照する。
`fn main` と `_start` の command-world 特例は Component export declaration とは別の契約として維持する。

### 2. WIT import の canonical identity を固定する

Layer C の source declaration は次の形式を使う。

```ark
import "namespace:package/interface@0.1.0" as host
```

`namespace:package/interface` の version 省略形は、読み込んだ WIT document の version へ解決する source shorthand とする。
version を明示した場合は、読み込んだ document の full semantic version と完全一致しなければならない。
不一致や malformed identifier は parser/resolver diagnostic とし、別の document へ黙って fallback しない。

language-level の canonical WIT identity は `namespace:package/interface@full-version` とする。
document に version がない場合だけ `namespace:package/interface` を canonical identity とする。
core Wasm import module が Component ABI の都合で `@major.minor` へ正規化する場合、その文字列は target-specific ABI rendering であり、language identity ではない。

`use ...::...` は引き続き Layer S の source namespace であり、WIT package identifier と相互変換しない。

### 3. name mangling を language contract から除外する

内部 symbol、Wasm export symbol、native backend symbol の spelling は compiler-private または ABI implementation detail とする。
language specification は具体的な separator、prefix、suffix、mangled name の例を stable contract として規定しない。
WIT の kebab-case 名や canonical ABI の import/export identity は、外部 WIT contract に必要な名前として別途規定してよい。

## 代替案と却下理由

### `pub fn` を暗黙 export として維持する

source module の可視性と外部 ABI の公開を一つの modifier に結びつけるため却下する。
private-to-source だが Component には公開したい関数、および source には公開するが Component には出したくない関数を表現できない。

### manifest または CLI の export list だけで指定する

宣言と export surface の距離が大きく、関数の型検証・reachability・WIT text が別々の設定へ分散するため却下する。
source code 上の declaration fact を正本にする。

### attribute 形式（例: `#[export]`）を導入する

既存の attribute 契約を拡張し、parser・formatter・診断の追加面を作る割に、今回必要な関数 modifier より利点がないため採用しない。

## 帰結

既存の Component fixture と library example は `pub fn` から `export fn` または `pub export fn` へ移行する。
source module fixture は `pub fn` を維持し、両者の責務を regression test で分ける。

AST cache は declaration fact を保持する。旧 version 1 cache に bit が存在しない場合は `is_component_export = false` として読み込めるため、cache invalidation なしでも旧 cache が暗黙 export を再生しない。

WIT parser、manifest validator、resolver matcher は同じ identifier grammar と version comparison を使う。
GC-native lowering と linear-memory lowering はこの source/Component contract を共有し、export surface の差を target によって作らない。

## 再検討条件

- `export fn` を `wasm32-gc` と `wasm32` の両方で同じ declaration fact として lowering できない場合。
- WIT document の full version を比較する canonical path が、既存の Component composition contract と両立しないことが実証された場合。
- Component Model が関数以外の export declaration を source modifier として要求し、別の安定契約が必要になった場合。

## 関連

- [ADR-006: 公開 ABI 境界の分類](ADR-006-abi-policy.md)
- [ADR-009: Import 構文の決定](ADR-009-import-syntax.md)
- [ADR-031: import 構文と WIT パッケージ識別子の統合](ADR-031-import-syntax-wit-unification.md)
- [RFC-011: 言語 v2 の意味論再設計案](../rfcs/011-language-v2-semantics-redesign.md)
- [#867](../../issues/open/867-component-wit-visibility-boundary.md)
