# ADR-093: Hash collection の snapshot iterator / owned entry と retain 契約

ステータス: **ACCEPTED** — borrow の代わりに snapshot cursor と owned entry を公開し、
`HashMap` / `HashSet` の `iter` / `entry` / `retain` / `FromIterator` を確定する

改訂: 2026-10-31 — `IntoIterator for HashMap` は owned `(K, V)` pair を yield する。
borrowed `Entry` / `iter` は参照・lifetime 機構の不在により引き続き不採用である。

決定日: 2026-10-30

関連: [ADR-044](ADR-044-trait-method-syntax-adopted.md)、[ADR-046](ADR-046-free-function-eradication.md)、[ADR-059](ADR-059-language-v2-ownership-copy-mutability.md)、[ADR-060](ADR-060-language-v2-owned-adts-box-slice.md)、[ADR-066](ADR-066-language-v2-generic-public-api.md)、[ADR-076](ADR-076-language-v2-iterator-consumer-boundary.md)、[ADR-078](ADR-078-language-v2-parametric-trait-impl.md)、[ADR-080](ADR-080-language-v2-generic-element-boxing-policy.md)、[ADR-086](ADR-086-linear-collection-adts.md)、[ADR-088](ADR-088-language-v2-call-boundary-borrow-and-mut-parameters.md)、[#917](../../issues/open/917-stdlib-eradication-execution.md)、[#974](../../issues/open/974-hashmap-iteration-traps.md)

## 文脈

`HashMap` / `HashSet` の Rust 相当 surface のうち、`entry` / `iter` / `retain` と
`HashMap` の `FromIterator` は「collection iterator 設計」待ちとして複数 lane で
繰り返し延期されてきた（#917 の hashmap-generic / hashset-capacity / collection
residuals）。決定時点の surface は `keys()` / `values()` / `entries()` の eager snapshot と、
`IntoIterator for HashMap` の keys 走査だった。

Arukellt v2 には一般的な `&T` / `&mut T` がなく、[ADR-060](ADR-060-language-v2-owned-adts-box-slice.md)
は `Slice<T>` を読み取り view に留めている。Rust の `iter()` / `Entry<'a, K, V>` を
borrow として表現すると、borrow checker なしで map の構造変更と cursor の生存期間を
静的に排他できず、壊れた ABI を先に固定することになる。一方で snapshot を cursor として
公開することは、コレクションの観測順序が implementation-defined である現行契約と整合する。

`VecIter<(K, V)>` は tuple 要素の generic cursor として実装できるように見えるが、
決定時点では両 target で `next()` が payload を落とす（linear では 0 を返し、GC では
型が unbound になる）ことを実測していた。この不具合は後続の compiler 修正で解消済み
である（下記「帰結」「再検討条件」）。

### 2026-10-31 再検討 — borrowed API の可否

borrow を前提とする `Entry<'a, K, V>` / `iter() -> impl Iterator<Item = (&K, &V)>`
が実装可能かを、compiler の参照機構から確認した。結論は **実装不能** である。

- lexer / parser に参照型の構文はない。`&` は bitwise AND の `TK_AMP` としてのみ存在し、
  型注釈 `&T` / `&mut T` を解析する経路がない。
- `std/core` に `ref` 系 module はなく、typechecker に参照型 tag もない。ADR-088 の
  「暗黙借用」は呼び出し中の引数 mode と可変 place の規則であり、第一級の参照値や
  lifetime を導入しない。ADR-060 の `Slice<T>` は
  `Slice::View(Vec<T>, start, end)` の読み取り view で、source の生存期間を型で拘束しない。
- borrow checker がないため、cursor の生存中に map を構造変更する操作を静的に禁止できない。
  参照風の綴りで owned snapshot を返す API は、呼び出し側に borrowed semantics を誤認させる
  fake になる。borrowed `Entry` / `iter` は公開しない。

## 決定

### 1. `HashMap::iter` は snapshot cursor を返す

- `HashMap::iter(self) -> HashMapIter<K, V>` を公開する。
- `HashMapIter<K, V>` は `keys: Vec<K>` / `values: Vec<V>` / `index` / `length` を持ち、
  `Iterator<Item = (K, V)>` と `IntoIterator<IntoIter = Self>` を実装する。
- `HashMap::iter` は `keys()` と `values()` の snapshot を取る。両者は同じ bucket 順で
  走査されるため index で対応する。cursor の生存中に map を変更しても cursor は影響を
  受けない。順序は実装定義であり、`Eq` の代用にしない。
- `IntoIterator for HashMap<K, V>` は同じ `HashMapIter<K, V>` を返し、`Item = (K, V)` と
  する。`for (k, v) in map` と `for pair in map.iter()` は同じ owned pair を yield する。
  これは 2026-10-30 時点の keys 走査から明示的に変更する。HashMap は provisional であり、
  旧 `for k in map` は型エラーになるため、利用側は pairs へ移行する。
  `iter()` と別 cursor 型を作らないことで、snapshot の意味と item 型を 1 箇所に固定する。

### 2. `HashMap::entry` は owned handle を返す

- `HashMap::entry(self, key: K) -> Entry<K, V>` を公開する。
- `Entry<K, V>` は map handle と key を所有し、borrow ではない。`key()` /
  `or_insert(default)` / `or_insert_with(make)` / `or_default()` を持ち、
  `or_insert*` は呼ばれた時点で map を lookup し、無ければ insert して値を返す。
  `or_default()` は `i32` 値なら `0`、`String` 値なら空文字列を挿入する。
  fn 値を保持する pair は default を持たないため `or_default` を公開しない。
  `entry` から `or_insert` の間に別の書き込みがあれば観測される。
- `Entry` は 6 つの supported pair で利用できる。`Entry.map` の GC field は
  annotation の `HashMap<K, V>` を pair の storage alias へ正規化してから
  fixed GC slot に写像する。この正規化が無いと String 値を含む pair の field が
  String→i32 slot に落ち、struct validation で壊れる。
- `Entry` は map の生存期間を越えて保持してはならない。borrow checker が入るまでの
  契約として docs に明記する。
- `Entry` の method は pair ごとの concrete impl に置く。generic な
  `Entry<K, V>` field は GC layout で `HashMap` を bucket struct ref に写像し、
  flat pair の Vec store と一致しないため、concrete impl で field 型を確定させる。

### 3. `retain` は owned pair predicate を取る

- `HashMap::retain(self, keep: fn((K, V)) -> bool)` を公開する。predicate は
  borrowed `&K` / `&V` ではなく owned pair を 1 個受け取る。v2 に参照 pair がなく、
  owned pair 1 個が所有権契約にそのまま対応するためである。決定時点では
  multi-parameter callback の std method ABI が String を含む signature を往復
  できなかったが、この ABI 制約は解消済みである（two-arg predicate の採用可否は
  surface 変更として別途判断する）。
- 実装は keys / values の snapshot に対して predicate を評価し、拒否された key を
  live store から削除する。predicate の実行中に bucket layout は変更しない。
- `HashSet::retain(self, keep: fn(T) -> bool)` も同じ 2 段階で実装する。predicate は
  snapshot に対して評価し、残す要素を再挿入する。
- 計算量は snapshot O(n) + 拒否 key ごとの remove である。flat i32 store の remove は
  O(n) なので最悪 O(n²) であり、この点は provisional の制約として docs に残す。

### 4. `FromIterator` は snapshot cursor を source にする

- 6 つの supported pair それぞれに
  `impl FromIterator<HashMapIter<...>> for HashMap<...>` を置く。
- source は `m.iter()` と `m.into_iter()` の両方を受ける。どちらも
  `HashMapIter<K, V>` を返すため、`HashMap::from_iter(m.iter())` と
  `HashMap::from_iter(m.into_iter())` は同じ concrete impl に解決される。
  `HashMap` は builtin type なので、typechecker が source cursor の mangled pair から
  対応する concrete impl を選び、MIR がその impl へ解決する。
- `VecIter<(K, V)>` は決定時点の tuple cursor 不具合のため `FromIterator` の
  source にしなかった。不具合は解消済みだが、本 ADR の source 契約は
  `HashMapIter` snapshot のまま維持する。generic な
  `impl<K, V> FromIterator<HashMapIter<K, V>> for HashMap<K, V>` は body で
  `HashMap::new<K, V>()` が pair store に lower されず再帰するため採用しない。

### 5. `HashSet::iter` / `retain`

- `HashSet<i32>` と `HashSet<String>` の `iter()` は `VecIter<T>` を返す。要素は
  `to_vec()` snapshot と同順（bucket order、実装定義）である。
- `retain` は §3 と同じ 2 段階で、snapshot の predicate 評価後に kept 要素を再挿入する。
- `HashSet` の既存 `IntoIterator` / `FromIterator<VecIter<T>>` は変更しない。

## 帰結

- `entry` / `iter` / `retain` / `FromIterator` は borrowed API ではなく、snapshot と
  owned handle として一貫した契約を持つ。borrow が無いことを型で隠さない。
- `iter()` は O(n) の snapshot を作る。`for (k, v) in map` も同じ snapshot を作るため、
  大きな map を繰り返し走査する用途では `keys()` / `values()` / `entries()` と同じ
  コストであり、borrow 実装が入るまでの制約である。
- `Entry` は key の lookup を `or_insert*` の時点で行うため、「entry を開いた時点の
  値」を保持しない。`or_default()` は `i32` / `String` 値の定型挿入を短くするが、
  fn 値の pair には無い。borrow 実装が入ったら `Entry<'a, K, V>` へ置き換えるか、
  snapshot 版を併存させるかを再検討する。
- `retain` の predicate は owned pair なので、Rust の `retain(|&k, &v| ...)` とは
  綴りが異なる。migration 時は `retain(|(k, v)| ...)` 相当の 1 引数 predicate へ
  書き換える。
- `VecIter<(K, V)>` の tuple payload 不具合と、std method の multi-parameter String
  callback ABI は解消済みである。両 target の fixture で
  `VecIter<(i32, i32)>` / `VecIter<(String, i32)>` が pair を yield することを、
  probe で std method が `fn(String, V) -> bool` の callback を呼べることを確認した。
  本 ADR の公開 surface は snapshot cursor / owned pair predicate のままとするが、
  これらの修正は後続の pair cursor / two-arg predicate を妨げない。
- `HashMap` の `IntoIterator` は pairs を yield する。`for (k, v) in map` は
  `for pair in map.iter()` と同じ item 型であり、Rust の綴りに一致する。旧 keys 走査は
  2026-10-31 の改訂で廃止したため、`for k in map` は移行が必要である。

## 却下した代替案

### borrowed `&K` / `&V` を返す iterator / `Entry<'a, K, V>`

現行 v2 に一般参照と borrow checker がなく、cursor 生存中の構造変更を静的に禁止
できない。参照を模した ABI を先に固定すると [ADR-060](ADR-060-language-v2-owned-adts-box-slice.md)
の後続 borrow 設計と衝突する。

### `VecIter<(K, V)>` を `iter` / `FromIterator` の cursor にする

決定時点では tuple 要素の generic `VecIter` の `next()` が payload を落とし、
borrowed 風の見た目だけを提供する fake になるため採用しなかった。不具合は解消済みで
pair cursor として成立するが、本 ADR の `iter` / `FromIterator` の公開契約は
`HashMapIter` snapshot を維持する（既存 surface の変更は別の設計判断とする）。

### `Entry<K, V>` に `HashMap<K, V>` field を持たせ、generic method で実装する

`Entry.map` の GC field は pair ごとに異なる fixed slot へ写像される。method を
concrete pair impl に置くと field 型と戻り値型が impl ごとに確定し、typechecker の
method 解決も pair 名で一意になる。generic impl は field 型の正規化と store 選択を
method 解決時まで遅らせるため採用しない。

### multi-parameter `fn(K, V) -> bool` predicate を std method に置く

決定時点では std module の method signature で `fn(String, V) -> bool` の callback
ABI が call_indirect の引数/戻り値を失い、runtime で type mismatch になった。この
ABI 制約は解消済みで two-arg predicate は lower できるが、`retain` の predicate
surface は owned pair 1 個のまま固定する（Rust 風の two-arg への移行は API 変更と
して別途判断する）。

### `HashMap` の `IntoIterator` を keys のまま維持する

2026-10-30 時点では、既存の `for k in map` を無言で意味変更しないことを優先して
keys 走査を維持した。2026-10-31 の再検討では、HashMap が provisional であること、
`iter()` と同じ `HashMapIter` を共用できること、Rust の `for (k, v) in map` と
一致することを優先し、pairs への明示的な変更を採用した。旧 keys 走査は migration
対象であり、同じ綴りのままでは型エラーになる。

## 再検討条件

- borrow / lifetime 検査が採択され、`&T` / `&mut T` を cursor と `Entry` に安全に
  表現できるようになった場合。その時点で borrowed `iter` / `Entry<'a, K, V>` を
  再設計し、snapshot 版との併存または置換を判断する。
- `HashMapIter<K, V>` の代わりに `VecIter<(K, V)>` を `iter` / `FromIterator` の
  cursor にする場合。tuple 要素の generic cursor は両 target で lower できることを
  確認済みだが、`iter()` の戻り値型を変える surface 変更であり、本 ADR では採用しない。
- std method の multi-parameter callback ABI が修正され、`fn(K, V) -> bool` を
  String pair でも使えるようになった場合。→ 解消済み。`fn(String, i32) -> bool` /
  `fn(String, String) -> bool` の std method 呼び出しを両 target で確認した。
  `retain` を two-arg へ移行するかは未決である。
- `retain` の計算量が問題になり、bucket 再構築を 1 pass で行う store target が必要に
  なった場合。
- `HashMap` を provisional から stable へ昇格する場合。keys→pairs の migration が
  完了し、`or_default` を含む Entry surface の pair 網羅が docs と fixture で固定されて
  いることを昇格条件に含める。

## 関連文書

- [ADR-044: trait とメソッド構文](ADR-044-trait-method-syntax-adopted.md)
- [ADR-046: 公開 free function の根絶](ADR-046-free-function-eradication.md)
- [ADR-059: 所有権・Copy・可変性](ADR-059-language-v2-ownership-copy-mutability.md)
- [ADR-060: owned ADT・Box・Slice](ADR-060-language-v2-owned-adts-box-slice.md)
- [ADR-066: generic 公開 API と HashMap](ADR-066-language-v2-generic-public-api.md)
- [ADR-076: Iterator の producer / consumer 境界](ADR-076-language-v2-iterator-consumer-boundary.md)
- [ADR-078: parametric trait impl](ADR-078-language-v2-parametric-trait-impl.md)
- [ADR-080: ジェネリクス要素 boxing ポリシー](ADR-080-language-v2-generic-element-boxing-policy.md)
- [ADR-086: 線形コレクション ADT](ADR-086-linear-collection-adts.md)
- [issue #974](../../issues/open/974-hashmap-iteration-traps.md)
- [issue #917](../../issues/open/917-stdlib-eradication-execution.md)
- `std/collections/hash_map.ark`, `std/collections/hash_set.ark`,
  `std/collections/hash.ark`, `std/collections/hash_string.ark`,
  `std/collections/hash_fn.ark`, `std/collections/hash_str_fn.ark`
