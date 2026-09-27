# ADR-080: 言語 v2 のジェネリクス要素 boxing ポリシー

ステータス: **ACCEPTED** — コンテナ/ジェネリクス要素は要素クラスで storage を選び、具象要素を一様 boxing しない

決定日: 2026-09-27

---

## 文脈

`Vec<T>`・`HashMap<K, V>`・`Option<T>` などのコンテナ/ジェネリクス
要素を wasm32-gc 上でどの GC 表現へ下ろすかは、実装が個別に収束して
きたが、判断として固定されていなかった。要素 storage の選択を
emitter が呼び出し側ヒューリスティクスで再決定すると、同一の
`Vec<T>` に対して new / push / get が別の array type を選ぶドリフト
の余地が残る。

関連する既定決定: ADR-035 が semantic value と storage lowering の
分離(`lower_value_type` / `lower_storage_type`)と `Vec<T>` =
`{ buffer: ref Array<StorageT>, len: i32 }` を採択済みである。
本 ADR は `StorageT` の具体的な選択規則を固定する。言語意味論は
表現に依存せず、linear-memory fallback(ADR-075)も同じ意味論を
managed record で実現する。

## 決定

1. コンテナ/ジェネリクス要素の storage は、emit 時点で解決済みの
   要素名から **要素クラス** を選び、クラスごとに固定の GC 型を使う。
   monomorphization 単位の exact-`T` array type は作らない
   (defined type 数が instantiation 数に比例して増えるのを防ぐ)。

   | 要素クラス | GC storage |
   |---|---|
   | i32 family(i32/u32/i16/i8/u16/u8/char/bool) | `(array (mut i32))` |
   | i64/u64 | `(array (mut i64))` |
   | f64/f32 | `(array (mut f64))` |
   | v128 | `(array (mut v128))` |
   | `String` | `(array (mut (ref null String-payload)))` |
   | `fn` 値(全 signature) | `(array (mut (ref null __fnpair)))` |
   | enum family | `(array (mut (ref null enum base)))` |
   | 名前付き struct・入れ子コンテナ | `(array (mut structref))` |

   Vec のヘッダは常に `{ buffer: ref array, len: i32 }` の 2 field
   struct であり、capacity は `array.len` から取る(ADR-035 を維持)。
   HashMap<K, V> は key/value クラス別の specialized struct
   (keys array + values array)を使い、要素クラス選択は Vec と共通。

2. ref 要素配列は nullable 要素とし、`get` は semantic non-null を
   `ref.as_non_null` で復元する(ADR-035 §2 の storage/value 分離を
   維持)。default 値を持たない non-null ref 要素へ
   `array.new_default` を発行しない。

3. 具象要素は一様 boxing しない。i31ref や structref wrapper で
   scalar も ref も包む方式は、scalar lane の割り当て・cast コストを
   要素アクセスへ持ち込むため採用しない。i31ref の使用は既存の
   scalar helper に限定し、要素 storage へ新規に持ち込まない。

4. 未解決の要素型パラメータ(`vec:T` など、substitute が emit 時点で
   残る経路)は i32 family の scalar dead-shell convention に従う。
   この経路で ref 要素を保持することは契約外であり、黙った誤変換を
   許す永続仕様にはしない。診断化は実装ギャップとして issue で追跡する。

5. `Option`/`Result`/enum payload は variant subtype の具象 field で
   保持し、tuple は struct field、closure env は capture ごとの
   field、`fn` 値は一律 `__fnpair { funcref, structref }` とする。
   いずれも要素クラス選択と同じ原則に従い、boxing wrapper は使わない。

6. storage 選択の正本は `array_type_from_type_name`(Vec 要素)と
   HashMap の key/value クラス選択とする。emitter は同じ
   `vec:T` から常に同じ array/vec 型を導出しなければならない。

## 帰結

- defined type 数が instantiation 数に比例せず、type section が
  固定コストの class type 群で済む。
- scalar lane は unboxed のままなので、boxing の割り当てと
  cast が数値 Vec のアクセスに入らない。
- structref supertype 配列は get 時の downcast を必要とする。
  exact-`T` 配列への精緻化は型安全/性能の実測が出た場合のみ
  再検討する。
- 未解決要素パラメータ経路は dead-shell に落ちるため、ref 要素が
  到達しうるかの診断整備が残課題となる。

## 代替案と却下理由

- **全要素を i31ref / structref wrapper で boxing する**: 実装が
  単純になるが、scalar lane の割り当て・cast コスト増と数値 Vec の
  劣化が大きい。採用しない。
- **monomorphization 単位で `(array (mut (ref T)))` を生成する**:
  defined type が instantiation 数に比例して増え、型セクションと
  レイアウト表の固定コストが増える。測定可能な利益が出た場合のみ
  再検討する。
- **dead-shell を永続契約にする**: 未解決 T 経路の ref 要素は現行で
  診断なしに scalar convention へ落ちる。黙った誤変換を許す契約には
  せず、診断化または経路の到達不能化を後続 issue で追跡する。

## 再検討条件

- 未解決要素パラメータ経路に diagnostic が入り、dead-shell が
  到達不能になった場合。
- exact-`T` array が型安全または性能上の測定可能な利益を示した場合。
- HashMap/Vec の要素クラス追加で class type の固定コストが
  有意に増えた場合。

## 関連文書

- [ADR-003-generics-strategy.md](ADR-003-generics-strategy.md)
- [ADR-035-wasm-gc-implementation.md](ADR-035-wasm-gc-implementation.md)
- [ADR-058-language-v2-semantics-redesign.md](ADR-058-language-v2-semantics-redesign.md)
- [ADR-075-language-v2-linear-memory-tracing-runtime.md](ADR-075-language-v2-linear-memory-tracing-runtime.md)
- [RFC-011](../rfcs/011-language-v2-semantics-redesign.md) item 8
- Issue #872(closure/collection lowering)、#891(borrow-capture env cells)
