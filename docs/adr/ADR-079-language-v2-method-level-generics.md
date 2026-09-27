# ADR-079: Language v2 メソッド型パラメータ（method-level generics）

ステータス: **ACCEPTED** — impl 内の fn name<U>(...) が impl 型パラメータとは別にメソッド固有の型パラメータを宣言でき、呼び出しごとに推論・単相化する

決定日: 2026-09-27

関連: [ADR-003](ADR-003-generics-strategy.md)、[ADR-062](ADR-062-language-v2-trait-core-coherence.md)、[ADR-066](ADR-066-language-v2-generic-public-api.md)、[ADR-076](ADR-076-language-v2-iterator-consumer-boundary.md)、[ADR-078](ADR-078-language-v2-parametric-trait-impl.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#860](../../issues/done/860-iterator-unification-integration.md)、[#887](../../issues/open/887-method-level-generics.md)

## 文脈

これまで impl VecIter<T> のメソッドシグネチャに現れる型変数はすべて impl ヘッダで束縛されていた。`fn map(self: VecIter<T>, f: fn(T) -> U)` のようなメソッド固有の型変数 U は書けず、コールバックが item 型を変える lazy adapter は MapIter<T, U> の literal 構築を利用者へ強いるしかなかった。VecIter<T>::map や FilterIter<T>::map が T -> T に固定されていたのはこの型システム上の欠落であり、RFC-011 item 37 の残差として #887 に引き継がれていた。

一方で ADR-003 と #860 は generic free function に対して「呼び出しごとに型変数を引数から推論し、単相化する」経路をすでに確立している。メソッド呼び出しが同じ表現力を持たない理由はなく、receiver の impl 変数とメソッド固有変数を一段の推論対象として扱えば同じ機構で実現できる。

## 決定

### 1. 構文

impl ブロック内のメソッドおよび associated function は、関数名の直後に独自の型パラメータリストを宣言できる。

    impl VecIter<T> {
        fn map<U>(self: VecIter<T>, f: fn(T) -> U) -> MapIter<T, U> { ... }
    }

有効な型パラメータ列は impl 型パラメータにメソッド型パラメータをこの順で連結したものとする。impl が非 generic の場合はメソッド側の宣言だけが有効パラメータになる。宣言した型変数は引数型・戻り値型・本体で参照でき、impl 型パラメータと同じスコープ規則で扱う。

### 2. 呼び出し側推論

呼び出しでは impl 型パラメータが receiver から、メソッド型パラメータが引数との unification から束縛される。推論は free generic function と同じ fresh variable の仕組みで行い、各変数は呼び出しごとに独立して決まる。

具象 receiver への呼び出しで引数によって決まらないメソッド型パラメータが残った場合は、単相化できないため型エラーとする（cannot infer type parameter U in method call name）。receiver がまだ型変数を含む場合（enclosing generic function 内や型の未確定な field access）は推論を正当に延期する既存の経路を維持し、エラーにしない。

### 3. bound variable の unification

推論変数がすでに束縛済みであるとき、その変数を別の型へ暗黙に再束縛することは言語意味論として認めない。束縛済み変数はその束縛型と unify し、不一致なら通常の型ミスマッチ診断になる。これにより、fn(T) -> T を期待する位置へ fn(i32) -> String を渡すような誤用は、実行時 trap ではなくコンパイル時診断になる。

### 4. 単相化と backend parity

メソッド型パラメータを含む呼び出しは receiver 型と全ての型引数を key に instantiation ごとの本体を生成する。wasm32-gc（typed reference / GC 表現）と linear-memory wasm32（linear 表現）で格納形式は異なってよいが、推論結果・診断・実行可能な pipeline の source semantics は一致させる。

### 5. stdlib への適用

正規例として VecIter<T>::map<U> が MapIter<T, U> を、FilterIter<T>::map<U> が FilterMapIter<T, U> を返す。T -> T の adapter は U = T の特例としてそのまま使える。

## 帰結

- v.iter().map(itos) のような item-type-changing pipeline が stdlib の公開 API から直接書ける。MapIter<T, U> / FilterMapIter<T, U> の literal 構築は引き続き合法だが、利用者が型を自前で名指しする必要はなくなった。
- receiver が未確定な呼び出しは引き続き延期推論されるため、generic な wrapper 関数内のメソッド呼び出しは既存どおり動作する。
- メソッド型パラメータの明示指定（it.map::<String>(f) のような turbofish 構文）は本 ADR では導入しない。引数から決まらない変数をユーザーが明示的に束縛する手段が必要になった時点で別 ADR とする。

## 却下した代替案

### adapter を T -> T に固定し続ける

MapIter<T, U> の literal 構築で型変更を表現できるため一見足りるが、利用者に内部型名と arity を常に要求し、lazy pipeline の chain を記述できない。RFC-011 item 37 の「共通の Trait 面」という目標に反する。

### メソッド型パラメータを impl パラメータへ畳み込む

MapIter 側に型変数を持たせてメソッドを非 generic のままにする案は、VecIter 自体の arity を増やすか adapter 型を別名化する必要があり、呼び出し側の型指定を改善しない。

### 束縛済み推論変数の再束縛を許す

最初の束縛を捨てて後勝ちにする挙動は、整合しない引数列を暗黙に受理してコンパイル後の実行時 trap に押し込む。unification の意味論として破綻しているため却下する。

## 再検討条件

- turbofish や戻り値型注釈による明示束縛が必要な API が現れた場合。
- enclosing generic function の宣言型変数と具象型が衝突したときの診断を厳密化する場合（現行は宣言変数を軟束縛として扱う経路が残る）。
- trait メソッド宣言自体に型パラメータを持たせる場合、impl 側との整合規則を別途定義する。

## 関連文書

- [RFC-011 item 37](../rfcs/011-language-v2-semantics-redesign.md)
- [ADR-003](ADR-003-generics-strategy.md) — generic の monomorphization 方針
- [ADR-062](ADR-062-language-v2-trait-core-coherence.md) — static dispatch / coherence
- [ADR-076](ADR-076-language-v2-iterator-consumer-boundary.md) — Iterator の producer / consumer 境界
- [ADR-078](ADR-078-language-v2-parametric-trait-impl.md) — parametric impl の合法化
