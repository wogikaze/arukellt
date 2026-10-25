# ADR-086: std::collections の線形コレクション ADT — Deque / Queue / Stack / LinkedList

ステータス: **ACCEPTED** — 線形コレクションをモジュール名前空間の公開型・メソッド・イテレータとして定義し、free function から移行する

提案日: 2026-10-25

決定日: 2026-10-25

関連: [ADR-000](ADR-000-process.md)、[ADR-014](ADR-014-stability-labels.md)、[ADR-018](ADR-018-language-docs-classification.md)、[ADR-044](ADR-044-trait-method-syntax-adopted.md)、[ADR-046](ADR-046-free-function-eradication.md)、[ADR-059](ADR-059-language-v2-ownership-copy-mutability.md)、[ADR-060](ADR-060-language-v2-owned-adts-box-slice.md)、[ADR-066](ADR-066-language-v2-generic-public-api.md)、[ADR-076](ADR-076-language-v2-iterator-consumer-boundary.md)、[ADR-078](ADR-078-language-v2-parametric-trait-impl.md)、[ADR-080](ADR-080-language-v2-generic-element-boxing-policy.md)、[#710](../../issues/open/710-stdlib-linear-collections-type-surface.md)、[#711](../../issues/open/711-stdlib-docs-rich-reference-quality.md)

## 文脈

`std/collections/linear.ark` は Deque と PriorityQueue を `Vec<i32>` の内部レイアウト
（`[head, tail, count, cap, ...data]`）に対する free function 群として公開している。
`std/collections/linked_list.ark` も同じく `Vec<i32>` のノードプールを free function で
操作する。どちらも型ではなく関数が公開面であり、要素型は `i32` に固定され、生成 docs の
**Structs** には現れない。

[#710](../../issues/open/710-stdlib-linear-collections-type-surface.md) は、deque / queue /
stack / list を「linear」という広いbucketの関数群ではなく、名前付きの再利用可能な型として
公開することを求めている。[#697](../../issues/open/697-vec-operation-extension.md) は
`Vec<T>`、[#687](../../issues/done/687-hashmap-hashset-rust-parity-gap.md) は hash
collections を所有し、線形コレクションの ADT 面はどちらも所有していない。

言語側は [ADR-044](ADR-044-trait-method-syntax-adopted.md) と
[ADR-046](ADR-046-free-function-eradication.md) により、公開 API の正規形を
trait / method / associated function とし、ユーザー可達 free function を残さない。
[#691](../../issues/done/691-iterator-trait-lazy-adapters.md) と
[ADR-076](ADR-076-language-v2-iterator-consumer-boundary.md) により `Iterator` /
`IntoIterator` / `FromIterator` の producer / consumer 境界は既にある。
[ADR-078](ADR-078-language-v2-parametric-trait-impl.md) は nominal constructor を self 型の
head とする parametric impl を合法化し、[ADR-080](ADR-080-language-v2-generic-element-boxing-policy.md)
は `Vec<T>` の要素 storage を固定している。したがって、コレクション型の公開面を
free function のまま維持する理由はない。

一方、Arukellt v2 の公開型サーフェスには Rust のような一般の `&T` / `&mut T` 参照型がなく、
[ADR-060](ADR-060-language-v2-owned-adts-box-slice.md) は `Slice<T>` を読み取り専用 view と
したうえで、完全な borrow / lifetime 検査を後続に残している。イテレータの
「borrowed / mutable / owning」を Rust と同じ参照の三種として設計することはできない。

## 提案する決定

### 1. 公開名前空間は定義モジュールの型とする

公開型の名前空間を次に固定する。

| 型 | 公開パス | 定義モジュール |
|----|----------|----------------|
| `Deque<T>` | `std::collections::deque::Deque<T>` | `std/collections/deque.ark` |
| `Queue<T>` | `std::collections::deque::Queue<T>` | `std/collections/deque.ark` |
| `Stack<T>` | `std::collections::deque::Stack<T>` | `std/collections/deque.ark` |
| `LinkedList<T>` | `std::collections::linked_list::LinkedList<T>` | `std/collections/linked_list.ark` |

`use std::collections::deque::Deque` のように型を import して `Deque<T>` と書ける。
`std::collections::Deque<T>` という flat 名は採用しない。Arukellt の `pub use` は
モジュールパスを再公開するが型名を短縮しないため、flat 名を作るには
`std/collections/mod.ark` に全型を宣言するか、型の facade を新設する必要がある。
それは無関係な ADT を 1 ファイルへ結合し、`std::collections` の名前空間を衝突に晒す。
通常の型は定義モジュールに置く既存慣行（`std::core::iter::VecIter`、`std::io::IoError`）
を維持する。

Queue と Stack は Deque と同じ ring-buffer storage を使う制限付き newtype なので、同じ
`deque` モジュールに置く。LinkedList は storage と計算量契約が異なるため別モジュールとする。
`std::collections::linear` は Deque の公開 home としては退役させる。PriorityQueue は
heap であり本 ADR の線形 ADT ではないため対象外とし、`linear` モジュールに残る `pq_*` は
別 owner の設計に委ねる。

### 2. 型サーフェス

すべて struct とし、enum は導入しない。Deque の empty / non-empty や small / large を
公開 enum で表すと内部表現が公開契約へ漏れる。Queue / Stack は Deque の制限付き
newtype、LinkedList は node-pool struct とする。フィールドはすべて非公開とし、
利用者は associated function と method だけを使う。

```ark
let d: Deque<i32> = Deque::new<i32>()
d.push_back(1)
d.push_front(0)

let q: Queue<i32> = Queue::with_capacity<i32>(8)
q.push(1)
q.push(2)

let s: Stack<i32> = Stack::new<i32>()
s.push(1)

let list: LinkedList<i32> = LinkedList::new<i32>()
list.push_back(1)
```

`Deque<T>` は growable ring buffer を backed storage とし、両端の push / pop を
amortized O(1) で提供する。`Queue<T>` と `Stack<T>` は `Deque<T>` を 1 field で包む。
`LinkedList<T>` は `Vec` backed の node pool と free list を持ち、両端操作を O(1)、
front-to-back traversal を O(n) とする。

### 3. 構築・空状態・capacity

- 構築は associated function に限る。`new` と `with_capacity` を置き、
  `Deque::new<i32>()` のように型引数を明示する形を正規とする。
- `with_capacity(cap)` は `cap < 0` を 0 として扱い、panic しない。
- `pop_front` / `pop_back` / `pop` / `front` / `back` / `peek` は空のとき
  `Option<T>::None` を返す。旧 free function の「0 を返す」「panic する」という
  sentinel 契約は新しい型へ持ち込まない。
- `Deque<T>` / `Queue<T>` / `Stack<T>` は `capacity()` と `reserve(additional)` を持つ。
  `reserve` は `len + additional` 以上を保証し、`additional <= 0` は no-op とする。
- `clear()` は要素数を 0 にし、capacity を保持する。`Deque` は O(1)、
  `LinkedList` は free list の再構築のため O(n) とする。
- `LinkedList<T>` は `new` のみを持ち、`capacity` / `reserve` は公開しない。
  node pool の容量は実装詳細とする。

### 4. メソッド

Deque は両端を明示する。Queue と Stack は型名で向きが決まるため `push` / `pop` を
正規名とし、Deque の `push_*` / `pop_*` の alias は置かない。

| 型 | メソッド |
|----|----------|
| `Deque<T>` | `push_front`, `push_back`, `pop_front`, `pop_back`, `front`, `back`, `len`, `is_empty`, `clear`, `capacity`, `reserve`, `iter`, `iter_mut` |
| `Queue<T>` | `push`（back）, `pop`（front）, `front`, `back`, `len`, `is_empty`, `clear`, `capacity`, `reserve`, `iter` |
| `Stack<T>` | `push`（back）, `pop`（back）, `peek`（back）, `len`, `is_empty`, `clear`, `capacity`, `reserve`, `iter` |
| `LinkedList<T>` | `push_front`, `push_back`, `pop_front`, `pop_back`, `front`, `back`, `len`, `is_empty`, `clear`, `iter`, `iter_mut` |

`Deque<T>` に `get(index)` / `set(index, value)` は置かない。現行 free API に index access は
なく、要求もない。必要になった場合は追加の API 判断とする。

Queue の `iter()` は front-to-back、Stack の `iter()` は bottom-to-top（push 順）で
走査する。Stack の `pop()` は top から取り出すため走査順の逆になる。この違いは
API ドキュメントに明記し、暗黙の LIFO 走査を仮定させない。

### 5. イテレータ

Rust の `&T` / `&mut T` を前提にせず、コレクション handle を共有する読み取り cursor と、
直前に yield した要素を置換できる書き込み cursor を分ける。

| struct | 生成元 | 走査 | 追加操作 |
|--------|--------|------|----------|
| `DequeIter<T>` | `Deque::iter`, `Queue::iter`, `Stack::iter` | front-to-back（Stack は bottom-to-top） | なし |
| `DequeIterMut<T>` | `Deque::iter_mut` | front-to-back | `set_current(value: T) -> Option<T>` |
| `DequeIntoIter<T>` | `IntoIterator` for `Deque` / `Queue` / `Stack` | front-to-back（Stack は bottom-to-top） | なし |
| `LinkedListIter<T>` | `LinkedList::iter` | front-to-back | なし |
| `LinkedListIterMut<T>` | `LinkedList::iter_mut` | front-to-back | `set_current(value: T) -> Option<T>` |
| `LinkedListIntoIter<T>` | `IntoIterator` for `LinkedList` | front-to-back | なし |

- 6 個の iterator struct は `Iterator` を実装し、`next` / `collect` / `count` を持つ。
  `collect` は [ADR-076](ADR-076-language-v2-iterator-consumer-boundary.md) に従い
  `Vec<Self::Item>` を返す。
- 各 iterator struct は `IntoIterator<IntoIter = Self>` も実装する。
- `set_current` は直前に `next` が返した要素を置換し、置換前の値を返す。
  まだ要素を yield していない、または既に終端へ達している場合は `None` を返し、
  コレクションを変更しない。
- `DequeIntoIter<T>` と `LinkedListIntoIter<T>` はコレクションを所有する。
  `into_iter` の receiver は [ADR-059](ADR-059-language-v2-ownership-copy-mutability.md) の
  move 境界に従い、呼び出し後の元 binding を再利用しない。
- iterator の生存中にコレクションを構造変更してはならない。現行の型検査はこの排他を
  静的に強制しないため、API 契約として明記し、borrow / lifetime 検査の採択後に
  強制へ移す。Queue と Stack は要素の任意置換を公開せず、`iter_mut` を持たない。
  FIFO / LIFO の契約に不要な書き込み面を増やさないためである。
- `QueueIter` / `StackIter` のような薄い別名 struct は作らない。Queue と Stack は
  Deque の制限付き型であり、同じ走査型を共有する。

### 6. trait impl

対象とする impl を次に固定する。すべて static dispatch であり、`dyn` は使わない。

| 型 | 実装する trait | 条件 |
|----|----------------|------|
| `Deque<T>` | `IntoIterator`, `FromIterator<DequeIter<T>>`, `FromIterator<DequeIntoIter<T>>`, `FromIterator<VecIter<T>>`, `Clone`, `Debug`, `Eq` | `Clone`: `T: Clone`、`Debug`: `T: Debug`、`Eq`: `T: Eq` |
| `Queue<T>` | 上記と同じ source の `FromIterator`、`IntoIterator`, `Clone`, `Debug`, `Eq` | 同上 |
| `Stack<T>` | 上記と同じ source の `FromIterator`、`IntoIterator`, `Clone`, `Debug`, `Eq` | 同上 |
| `LinkedList<T>` | `FromIterator<LinkedListIter<T>>`, `FromIterator<LinkedListIntoIter<T>>`, `FromIterator<VecIter<T>>`, `IntoIterator`, `Clone`, `Debug`, `Eq` | 同上 |

- `FromIterator::from_iter` は source iterator を順に push する。Deque / Queue /
  LinkedList は back、Stack は top へ積む。
- `Clone` は logical order を保持し、`Deque` / `Queue` / `Stack` では観測可能な
  capacity も保持する。`Eq` は capacity を無視し、長さと logical order の要素で比較する。
- `Debug` は `Deque [1, 2, 3]` のように型名を前置し、要素の `fmt_debug` を並べる。
  空なら `Deque []` とする。
- `Default`, `Hash`, `Ord`, `Display`, `Copy` は初期サーフェスに含めない。
  `Default` は `new` が明示 constructor であり現時点の要求がない。`Hash` は
  container 用の hash policy と利用例がなく、`Ord` は Queue / Stack の順序契約を
  余計に固定する。`Display` は collection の人間向け表現を一意に決められず、
  `Copy` は storage の所有と矛盾する。
- 任意の `I: Iterator` を受ける `FromIterator<I>` は実装しない。
  [ADR-078](ADR-078-language-v2-parametric-trait-impl.md) §2 は self 型に現れない
  型パラメータを持つ impl を拒否するため、`I` を self 型 `Deque<T>` に出現させられない。
  初期サーフェスは concrete source（自分の iterator と `VecIter<T>`）に限定する。

### 7. 生成 docs のデータモデルと表示

`std/manifest.toml` の `[[types]]` を公開型・trait の registry として拡張する。

- `kind` を追加し、`"struct"` / `"enum"` / `"trait"` のいずれかを必須にする。
  既存 entry は移行時に補う。名前の hardcode（現在の `Option` / `Result` / `Ordering`）で
  enum を判定しない。
- `doc`（一行 summary）、`doc_category`、`methods`（公開 inherent method と associated
  function の名前）、`trait_impls`（公開 trait 名）を追加する。
- `methods` は signature や本文の正本ではない。signature と doc comment は source を正とし、
  registry は公開面の membership と deprecation の解決に使う。#711 の extractor が
  source から signature / example を補う。
- `[[functions]]` の `deprecated_by` は `Owner::member` 形式を解決できるようにする。
  これにより stable な free function を method へ deprecate するとき、
  method を manifest の関数 entry として偽装せずに replacement を指せる。
- 生成 Markdown の `reference.md` は「Prelude Types」ではなく **Public Types** とし、
  `## Structs` / `## Enums` / `## Traits` に分ける。`arukellt doc --html` も同じ
  `kind` を使い、現在の Structs / Enums の分類を名前 hardcode から切り離す。
- `docs/stdlib/modules/collections.md` に `std::collections::deque` と
  `std::collections::linked_list` の module section を追加し、型・method・trait impl を
  一覧する。型 anchor は `type-<Name>`、method anchor は `method-<Owner>::<name>` とし、
  #711 の stable anchor 方針に接続する。
- 生成物は generator 入力（manifest / source doc comment / curated module page）から
  再生成する。生成済み Markdown を手編集しない。

### 8. 現行 free function からの移行

| 旧 API | 新 API |
|--------|--------|
| `deque_new` | `Deque::new<T>()` / `Deque::with_capacity<T>(n)` |
| `deque_push_front(d, v)` | `d.push_front(v)` |
| `deque_push_back(d, v)` | `d.push_back(v)` |
| `deque_pop_front(d)` | `d.pop_front()`（`Option<T>`） |
| `deque_pop_back(d)` | `d.pop_back()`（`Option<T>`） |
| `deque_front(d)` / `deque_back(d)` | `d.front()` / `d.back()`（`Option<T>`） |
| `deque_len(d)` / `deque_is_empty(d)` | `d.len()` / `d.is_empty()` |
| `deque_clear(d)` | `d.clear()` |
| `deque_to_vec(d)` | `d.iter().collect()` |
| `linked_list_new()` | `LinkedList::new<T>()` |
| `linked_list_push_front/back(l, v)` | `l.push_front(v)` / `l.push_back(v)` |
| `linked_list_pop_front/back(l, fallback)` | `l.pop_front()` / `l.pop_back()`（`Option<T>`） |
| `linked_list_front/back(l, fallback)` | `l.front()` / `l.back()`（`Option<T>`） |
| `linked_list_len/is_empty/clear(l)` | `l.len()` / `l.is_empty()` / `l.clear()` |
| `linked_list_to_vec(l)` | `l.iter().collect()` |

- `deque_*` は manifest で `stable` なので、[ADR-014](ADR-014-stability-labels.md) と
  [ADR-046](ADR-046-free-function-eradication.md) D4 に従い、少なくとも 1 回の完全 release を
  含む deprecation window の後に削除する。`deprecated_by` は `Deque::<method>` を指す。
- 旧 Deque は `Vec<i32>` の内部レイアウトを公開していたため、`Deque<T>` への移行は
  source 型の変更を伴う。deprecation window の間は旧 free function と旧レイアウトを
  `std/collections/linear.ark` に残し、新型は別モジュールで並行提供する。
- `linked_list_*` は manifest 未登録で stable 保証がない。in-tree caller と fixture を
  新型へ移した後、migration note とともに削除する。ADR-014 の deprecation window は
  要求しない。
- `pq_*` は本 ADR の対象外であり、移行表にも含めない。PriorityQueue の型化は
  別 issue / ADR が所有する。
- 旧 API の panic / 0 sentinel と新 API の `Option<T>` は観測挙動が異なる。
  migration guide で empty case の扱いを明記する。旧 `deque_new` の固定 capacity 16 は
  `Deque::new` では保証せず、capacity が必要な場合は `Deque::with_capacity` を使う。

### 9. stability

- `Deque<T>` / `Queue<T>` / `Stack<T>` / `LinkedList<T>` と 6 個の iterator struct は
  **provisional** で導入する。型名、method 名、iterator の分割は固まっているが、
  both target の parity と trait impl の完了までは細部が変わりうる。
- `experimental` にはしない。既存の stable な `deque_*` の置換先を、事前告知なしの
  破壊的変更に晒す experimental に置くのは移行契約として不整合である。
- **stable** への昇格条件は、`wasm32-gc` と `wasm32` の compile / validate / runtime
  parity、docs の Structs 登録、対象 trait impl、少なくとも 1 release の利用実績が
  揃った時点とする。
- 旧 `deque_*` の deprecation と削除は ADR-014 / ADR-046 D4 に従う。新型が provisional の
  間も migration guide で replacement を明示し、stable と誤認させない。

### 10. 実装フェーズ（依存順）

実装の進捗や完了条件は
[#710](../../issues/open/710-stdlib-linear-collections-type-surface.md) が所有する。
本 ADR は依存順だけを固定する。

1. **Phase 0 — 前提**: declared generic struct の associated constructor
   (`Deque::new<T>()`) と bounded parametric impl
   (`impl<T: Clone> Clone for Deque<T>`) の typecheck / lowering を fixture で確認する。
   `[[types]]` の `kind` / `methods` / `trait_impls` と `deprecated_by` 解決を
   generator / checker に追加する。両 target の parity fixture を用意する。
2. **Phase 1 — Deque<T>**: `std/collections/deque.ark` に ring buffer、constructor、
   method、`DequeIter` / `DequeIterMut` / `DequeIntoIter`、`IntoIterator` を実装する。
3. **Phase 2 — Queue<T> / Stack<T>**: Deque の newtype と制限付き method を実装し、
   iterator を共有する。
4. **Phase 3 — LinkedList<T>**: node pool、method、3 個の iterator、`IntoIterator` を
   実装する。
5. **Phase 4 — trait impl**: `FromIterator`（concrete source）、`Clone`、`Debug`、`Eq` を
   実装する。`FromIterator<I>` の一般化は行わない。
6. **Phase 5 — docs / migration**: manifest の type entry、module page、Structs / Enums
   表示を追加し、`deque_*` を deprecate、`linked_list_*` を削除、fixture と example を
   新 API へ移す。
7. **Phase 6 — removal**: deprecation window の満了後に旧 `deque_*` と旧
   `linear` の Deque 実装を削除する。PriorityQueue の処遇は別 owner の完了を待つ。

## 帰結

- 線形コレクションは **Structs** に載る nominal 型になり、利用者は free function 名ではなく
  receiver method と associated constructor でコードを書ける。ADR-046 の公開面と一致する。
- `Vec<i32>` の内部レイアウトは公開契約から外れる。旧 free function の利用者は
  deprecation window 中に型を `Deque<T>` / `LinkedList<T>` へ変更する必要がある。
- Queue と Stack は Deque の薄い newtype なので storage の二重実装を避けられるが、
  Deque の method をすべては公開しない。型ごとの契約が docs に現れる。
- `iter_mut` は Rust の `&mut T` ではなく `set_current` を持つ cursor になる。
  borrow / lifetime 検査が入るまでは、イテレータ生存中の構造変更を型で禁止できない。
- `FromIterator` は concrete source に限定される。任意の iterator から collection を
  構築する一般形は、ADR-078 の impl パラメータ制約を変える別の判断を要する。
- `std::collections::linear` は Deque の home ではなくなり、PriorityQueue の legacy
  surface だけが残る。PriorityQueue の型化は別の設計判断になる。
- docs は型・method・trait impl を registry から生成できるようになり、#711 の
  rich reference と #710 の Structs 要件を同じデータモデルで満たせる。

## 却下した代替案

### `std::collections::Deque<T>` の flat 名前空間

`std::collections` 直下に型を置けば import は短くなるが、Arukellt の `pub use` は
型名を短縮しないため、`std::collections/mod.ark` に全型を宣言するか型 facade を作る
必要がある。無関係な ADT を 1 ファイルへ結合し、名前衝突と変更結合を増やすため採用しない。

### `std::collections::linear::Deque<T>` を維持する

既存ファイルを動かさない最小差分だが、「linear」bucket の関数群という現状の認知を
そのまま残し、PriorityQueue という heap と同じ module に Deque を置き続ける。
#710 の目的と衝突するため採用しない。

### Queue / Stack を `type Queue<T> = Deque<T>` の alias にする

Arukellt に公開 type alias の surface がなく、alias では Deque の両端 API を制限できない。
型ごとの FIFO / LIFO 契約と docs の Structs entry を作れないため採用しない。

### Rust と同じ `&T` / `&mut T` を返す iterator にする

現行 v2 に一般の参照型と borrow checker がなく、`Slice<T>` も読み取り view に留まる。
参照を模した ABI を先に固定すると、ADR-060 の後続 borrow 設計と衝突するため採用しない。
読み取り cursor と `set_current` cursor で同じ利用を表現する。

### PriorityQueue<T> も同時に型化する

PriorityQueue は heap ordering を持ち、線形コレクションの両端契約とは異なる。
比較関数 / `Ord` bound / heap layout の判断が必要であり、#710 の scope を越える。
別 issue / ADR に分離する。

### 任意の `I: Iterator` を受ける `FromIterator<I>` を実装する

`I` が self 型 `Deque<T>` に現れないため ADR-078 §2 の unconstrained parameter に当たる。
compiler-owned blanket rule を新設するか、`impl` の制約を緩める別 ADR が必要であり、
本 ADR では concrete source に限定する。

### `List<T>` を `Nil | Cons(T, List<T>)` の enum にする

永続的な単方向リストは O(1) の両端操作と node 再利用という既存実装の契約を満たさず、
recursive enum の layout 判断も必要になる。既存の node-pool 実装に対応する
`LinkedList<T>` struct を採用する。

## 再検討条件

- borrow / lifetime 検査が採択され、`&T` / `&mut T` を公開 iterator の item として
  安全に表現できるようになった場合。`set_current` cursor を参照ベースの API へ
  置き換えるか、併存させるかを再検討する。
- ADR-078 の impl パラメータ制約が変わり、`FromIterator<I>` や `IntoIterator` の
  一般形を coherence と termination を保ったまま表現できるようになった場合。
- `Default` / `Hash` / `Ord` の利用例が現れ、container の意味論を固定できる場合。
- index access (`get` / `set`) や range removal が線形コレクションの必須要件になった場合。
- PriorityQueue の型化、または `std::collections::linear` の module 名をどう退役させるか
  を決める場合。
- docs の method / trait impl extractor が #711 で整い、`methods` の name list では
  不足する signature / example を manifest で持つ必要が出た場合。

## 関連文書

- [ADR-014: 言語仕様と Stdlib API の安定性ラベル](ADR-014-stability-labels.md)
- [ADR-018: 言語ドキュメント分類](ADR-018-language-docs-classification.md)
- [ADR-044: trait とメソッド構文](ADR-044-trait-method-syntax-adopted.md)
- [ADR-046: 公開 free function の根絶](ADR-046-free-function-eradication.md)
- [ADR-059: 所有権・Copy・可変性](ADR-059-language-v2-ownership-copy-mutability.md)
- [ADR-060: owned ADT・Box・Slice](ADR-060-language-v2-owned-adts-box-slice.md)
- [ADR-066: generic 公開 API と HashMap](ADR-066-language-v2-generic-public-api.md)
- [ADR-076: Iterator の producer / consumer 境界](ADR-076-language-v2-iterator-consumer-boundary.md)
- [ADR-078: parametric trait impl](ADR-078-language-v2-parametric-trait-impl.md)
- [ADR-080: ジェネリクス要素 boxing ポリシー](ADR-080-language-v2-generic-element-boxing-policy.md)
- [issue #710](../../issues/open/710-stdlib-linear-collections-type-surface.md)
- [issue #711](../../issues/open/711-stdlib-docs-rich-reference-quality.md)
- [issue #697](../../issues/open/697-vec-operation-extension.md)
- [issue #687](../../issues/done/687-hashmap-hashset-rust-parity-gap.md)
- [issue #691](../../issues/done/691-iterator-trait-lazy-adapters.md)
- [issue #701](../../issues/done/701-associated-function-syntax.md)
- [issue #703](../../issues/open/703-monomorphic-api-bold-cutover.md)
- [issue #707](../../issues/done/707-trait-self-return-type-support.md)
- [issue #709](../../issues/done/709-stdlib-trait-first-api-policy.md)
- [issue #917](../../issues/open/917-stdlib-eradication-execution.md)
- `std/collections/linear.ark`, `std/collections/linked_list.ark`, `std/collections/vec.ark`
- `std/manifest.toml`, `docs/stdlib/generation-schema.md`
- `scripts/gen/generate-docs.py`, `scripts/gen/generate-stdlib-docs.sh`
