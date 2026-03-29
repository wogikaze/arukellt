# Arukellt 標準ライブラリ総合設計書

**多言語標準ライブラリ調査を踏まえた LLM-first / Wasm-first 設計**

- 作成日: 2026-03-28
- 対象: Arukellt v3 以降の標準ライブラリ設計
- 設計前提: 実装コストより surface area と一貫性を優先

## 目次

- **1. 結論サマリ** — 何を採用し、何を捨てるか
- **2. 設計条件と現状認識** — Arukellt の現状・制約・不整合の整理
- **3. 多言語標準ライブラリ調査** — Python / Rust / Go / Java / Kotlin / JavaScript / C++ / OCaml / Zig / Haskell / Wasm
- **4. 設計原則** — LLM-first / Wasm-first / explicitness / small prelude
- **5. 提案モジュール構成** — core / text / bytes / collections / seq / io / wasm / component / test
- **6. 型・エラー・プリルード方針** — String/Bytes 分離、Error enum、tiny prelude
- **7. コレクションとデータ構造** — Vec 以外を正面から標準化
- **8. 文字列・バイト列・エンコーディング** — UTF-8 / binary / builder / rope / leb128
- **9. シーケンス・アルゴリズム** — eager/lazy 両輪
- **10. I/O・時間・環境・プロセス** — WASI/CLI に必要な基礎面
- **11. Wasm / WIT / Component Model** — ターゲット言語らしい std::wasm / std::wit / std::component
- **12. 安定性・ドキュメント・テスト** — Stable / Experimental / Internal
- **13. Arukellt での実装戦略** — compiler intrinsic と source 実装の分担
- **14. 移行計画** — 現在の prelude / monomorph 名からの移行
- **付録** — 比較表・API 草案・参考資料

## 1. 結論サマリ

本設計書の結論は、「Arukellt の標準ライブラリは、Rust の層分け、Go の単純さ、Python/Kotlin の実用性、JavaScript の binary ergonomics、OCaml/Haskell の代数的型、C++ の data structure breadth、そして Wasm Component Model の型境界」を折衷した構成にするべき、という一点に尽きる。

ただし折衷といっても全部を平均化するのではない。利用者が毎日触る API は小さく、明示的で、名前から挙動が読めることを最優先にする。大きさは prelude ではなくモジュール全体で確保する。つまり「tiny prelude, large explicit stdlib」を採用する。

また Arukellt は Wasm を主戦場にしているため、一般言語の stdlib では脇役になりがちな Bytes / LEB128 / endian / typed buffer / WIT / Component / resource handle を first-class に据える。ここが Python や Kotlin の単なる模倣ではなく、Arukellt らしさになる。

### 採用する主要判断

- 標準ライブラリは `std::prelude`, `std::core`, `std::text`, `std::bytes`, `std::collections`, `std::seq`, `std::path`, `std::time`, `std::random`, `std::wasm`, `std::wit`, `std::component`, `std::test`, `std::host::*` を中核に再構成する。
- host-bound API は `std::host::http` のように明示し、`std::wasi::p1` / `std::wasi::p2` のような version namespace は導入しない。
- `String` と `Bytes` を厳密に分離し、UTF-8 と raw bytes を混同しない。
- `Vec`, `HashMap`, `HashSet` だけでなく、`Deque`, `BTreeMap`, `IndexMap`, `BitSet`, `PriorityQueue`, `Arena`, `SlotMap`, `Interner`, `Rope` まで標準化する。
- ユーザーに見せる API は namespace function を基本にし、trait 解決や extension magic に依存しすぎない。
- Wasm 向け要素として `leb128`, `endian`, `ByteCursor`, `ModuleBuilder`, `WitType`, `World`, `ResourceHandle`, `lift/lower` を標準装備する。
- エラーは `Result<T, Error>` を基準にし、`String` エラーは移行互換に限定する。

## 2. 設計条件と現状認識

今回の依頼条件は明確である。第一に「実装の重さは考えない」。第二に「Arukellt で実装する」。第三に「LLM が書きやすいこと」を最上位の usability 指標として扱う。第四に「データ構造と Wasm でよく使う要素を必ず含める」。したがって、最小限の stdlib ではなく、長期の自己記述性と自己ホスト性を見据えた大きめの stdlib を前提に設計する。

同梱された v3-review 文書群から見る現在の Arukellt は、GC-native な Wasm を主経路に持ち、Option/Result/Vec/String を中心とする prelude wrapper と compiler intrinsic の併用で動いている。現行 public API には `Vec_new_i32` や `map_i32_i32` のような monomorph 名が残っており、v3 の目的はこれを module-based な stdlib に昇格させることにある。

一方で文書群には履歴層も混在している。trait を禁止していた旧設計、現在は traits/methods/nested generics が存在するという注記、v3 ロードマップでは traits を後送している記述が並存している。したがって本設計書は、コンパイラ内部の trait 能力に依存しない user-facing API を基準にする。具体的には「名前空間でまとまった free function 群」を第一級の surface にする。メソッド sugar は後から生やせるが、逆は難しい。

### 現状との差分

| 項目 | 現状 | 本設計での扱い |
|---|---|---|
| メモリモデル | GC-native Wasm が主経路 | GC 前提の値表現を採用しつつ、`std::wasm` では linear-memory/binary utilities も提供 |
| 公開 API | prelude 中心、旧 monomorph 名が残る | tiny prelude + explicit modules に分離 |
| 文字列/バイト | String 中心、binary utilities は薄い | `String` と `Bytes` を完全分離 |
| データ構造 | Vec と一部 HashMap が中心 | Map/Set/Deque/BTree/Index/Arena/SlotMap/Interner まで標準化 |
| I/O | WASI bridge はあるが surface が薄い | fs/path/io/process/time/random を揃える |
| Wasm 特化 API | ビルダ/codec 群が薄い | `std::wasm`, `std::wit`, `std::component` を新設 |

## 3. 多言語標準ライブラリ調査

ここでは「どの言語が優れているか」ではなく、「Arukellt が何を学ぶべきか」を抽出する。評価軸は次の五つである。① discoverability、② naming consistency、③ binary/text/data-structure coverage、④ error model、⑤ Wasm/portability との相性。

### 一覧

| 対象 | 特に学ぶ点 | Arukellt への翻訳 |
|---|---|---|
| Python | batteries-included、`collections`/`collections.abc` の分割 | 大きいが発見しやすい。用途別 module 名が強い |
| Rust | `core` / `alloc` / `std` の層分け、`Option`/`Result`/`Vec` の基礎力 | Arukellt も層分けを採るべき |
| Go | 小さい package と error value、`bytes`/`strings`/`os` の単純さ | free function 中心 API の教科書 |
| Java/Kotlin | 大規模 collection utilities、lazy sequence、time/text の厚み | batteries を厚くする際の整理法が参考になる |
| JavaScript | `Map`/`Set`/`Array` に加え TypedArray / ArrayBuffer / DataView が強い | Wasm/binary で不可欠な視点 |
| C++ | containers / iterators / algorithms / ranges の広い分類 | データ構造カタログの広さが参考 |
| OCaml/Haskell | Stdlib/Prelude 的基礎面、代数的データ型の自然さ | Option/Result/Seq の見せ方に活きる |
| Zig | systems 寄り utility と stdlib の実務感 | バイナリ処理と platform utility 設計の参考 |
| Wasm Component Model | WIT, canonical ABI, resource, list, option, result | Arukellt 専用の `std::wasm` / `std::component` 設計根拠 |

### 3.1 Python

Python の標準ライブラリは、言語コアだけではなく多種の practical module を同梱する「batteries-included」型である。`collections` は built-in container の代替や補助を整理し、`collections.abc` は container interface を明確化している。Arukellt に置き換えると、`std::collections` を単なる Vec/Map の倉庫ではなく、利用パターンごとに発見しやすい地図として設計する必要がある。

Python から学ぶべきは、抽象の深さではなく、利用者が「どこを見ればよいか」を推測しやすい module 名とドキュメント構成である。Arukellt でも `std::text`, `std::bytes`, `std::collections`, `std::json` のように用途ベースの切り方を優先する。

### 3.2 Rust

Rust は `core` を dependency-free foundation、`alloc` を heap-allocated values and collections、`std` を portability layer として分離している。この三層は Arukellt に非常に相性がよい。なぜなら Arukellt でも「言語の根」「GC/heap 上のコレクション」「WASI/target 依存 API」を混ぜない方が説明しやすいからである。

また `Option`, `Result`, `Vec`, `String`, `HashMap` のような基本型が ecosystem 全体の lingua franca になっている点も重要である。Arukellt でも ecosystem 全体の共通語になる基本型を最初に決め、外部 package が独自コンテナを乱立しないようにするべきである。

### 3.3 Go

Go は trait-heavy ではなく package-heavy であり、`os`, `strings`, `bytes`, `errors` などの単純で直線的な API が強い。これは LLM フレンドリ設計に直結する。Arukellt でも `bytes::read_u32_le`, `path::join`, `errors::is` のような、「関数名だけで用法が読める」設計を基本にするべきである。

Go の error value 文化も有益である。例外や暗黙 throw ではなく、`Result<T, Error>` をベースに据えると、LLM は制御フローを壊しにくい。

### 3.4 Java / Kotlin

Java/Kotlin はコレクション操作、文字列処理、時間、I/O、並行性など広い面を厚く持つ。特に Kotlin は higher-order functions と eager collection / lazy sequence の二層を整理しており、Arukellt の `std::seq` 設計に参考になる。

一方で Java/Kotlin 的な overload と extension の氾濫をそのまま持ち込むべきではない。Arukellt では surface を増やすより、`vec::map`, `seq::map`, `string::split`, `bytes::split` のように型と namespace で整理した方が壊れにくい。

### 3.5 JavaScript

JavaScript の最大の示唆は、Web/Wasm の世界では binary data が first-class である、という点にある。`TypedArray`, `ArrayBuffer`, `DataView` はテキスト中心言語の補助ではなく、Web API と Wasm 間をつなぐ主要部品である。Arukellt でも `Bytes`, `ByteBuf`, `ByteView`, `Cursor`, `Endian` utilities を本体に入れるべきである。

また `Map`, `Set`, `Error`, `JSON` の built-in 性は、日常タスクに必要な面を package 外へ追い出しすぎない利点を示している。Arukellt でも JSON や Base64 は「外部 crate 推奨」にせず、標準で持つ価値が高い。

### 3.6 C++

C++ 標準ライブラリは containers, iterators, algorithms, ranges, strings, numerics, time, I/O, concurrency と広いカテゴリーを持つ。Arukellt が同じ深さまで行く必要はないが、「データ構造カタログが狭すぎると利用者はすぐ独自実装に逃げる」ことは学ぶべきである。

特に `optional`, `variant`, `string_view`, `span`, `priority_queue`, `bitset` に相当する面は、Arukellt でも compiler/self-host/Wasm toolchain の実務上かなり有用である。

### 3.7 OCaml / Haskell

OCaml の `Stdlib` 自動オープンや Haskell の Prelude は、最初から見える基礎面の設計が言語体験を左右することを示す。一方で prelude が大きすぎると namespace 汚染が起きる。Arukellt は tiny prelude を採りつつ、代数的データ型とパターンマッチに自然につながる `Option`, `Result`, `Ordering`, `Range` などを核に置くべきである。

また persistent data structure や composable function style は、`Seq` や immutable collection view を設計する際の指針になる。

### 3.8 Zig

Zig stdlib は allocator-aware だが、それ以上に「systems 作業に必要な utility が標準で見つかる」点が強い。Arukellt は GC 言語なので allocator surface をそのまま持ち込む必要はないが、binary, fs, path, formatting, process, hash, random などの実務 utility は標準で持つ価値が高い。

### 3.9 Wasm / WIT / Component Model

WIT は primitive types だけでなく `list`, `option`, `result`, `tuple`, `record`, `variant`, `enum`, `resource`, `flags` を明示し、canonical ABI は strings/lists/resource handles を言語間で橋渡しする。Arukellt が Wasm を主戦場にする以上、これらは「FFI 付録」ではなく標準ライブラリの中核ユースケースである。

したがって `std::wasm` は単なる byte writer ではなく、`Module`, `Section`, `Instr`, `ValType`、`std::wit` は `Type`, `Interface`, `World`, `PackageId`、`std::component` は `Own<T>`, `Borrow<T>`, `HandleTable`, `lift/lower` を提供するべきである。

## 4. 設計原則

### 原則 1: tiny prelude, large explicit stdlib

自動 import されるものは最小限に抑え、明示 import で広いライブラリへ到達させる。prelude が大きいと LLM も人も名前衝突と discoverability で負ける。

### 原則 2: text と binary を分離

UTF-8 text は `String`、raw binary は `Bytes`。変換は明示。`String` を byte bag として使わせない。

### 原則 3: concrete names over hidden magic

trait bound の推論や extension method より、`bytes::read_u32_le` のような具体名を優先する。

### 原則 4: safe default, explicit fast path

`get` は `Option` を返し、`get_unchecked` や `unsafe_*` は別 namespace に隔離する。

### 原則 5: eager と lazy を分ける

コレクション上の即時計算と `Seq` による遅延計算を分け、`Vec` そのものを何でも屋にしない。

### 原則 6: Wasm-targeted batteries を標準化

LEB128, endian, byte cursor, WIT, component resource handle まで標準に含める。

### 原則 7: error を構造化する

`Result<T, String>` は移行経路として残しても、最終的な正準形は `Result<T, Error>` にする。

### 原則 8: doc-first / example-first

全 public API は short example を持ち、example は harness で実行検証する。

## 5. 提案モジュール構成

推奨構成は「small root, deep modules」である。`std::*` 配下に機能別 module を置き、re-export は意図的に限定する。`prelude` は convenience の場、`core` は意味論の場、`collections` と `bytes/text` は日常実務の場、`wasm/wit/component` は Arukellt 固有の強みの場と位置付ける。

また host-specific API は `std::host::*` に隔離する。`std::*` 直下には pure または host-agnostic な API だけを置き、WIT / Component 境界はさらに別層で `import "wasi:..."` に委ねる。

### モジュール一覧

| モジュール | 役割 | 主要内容 |
|---|---|---|
| std::prelude | 最小限の自動 import | Option, Result, String, Bytes, Vec, Some/None, Ok/Err, panic, assert |
| std::core | 言語の基礎型と基礎関数 | option, result, ordering, range, cmp, math, convert, hash, panic |
| std::text | UTF-8 text | string, char, builder, rope, pattern, fmt |
| std::bytes | raw binary / endian / cursor | bytes, byte_buf, byte_view, cursor, endian, hex, base64, leb128 |
| std::collections | 汎用データ構造 | vec, deque, hash_map, hash_set, btree_map, index_map, bit_set, arena, slot_map, interner |
| std::seq | 遅延列・アルゴリズム | seq, iter, collect, sort, search, group |
| std::path | path string helpers | join, parent, file_name, extension |
| std::time / std::random | 純粋な duration / seeded helper | duration, seeded rng, shuffle |
| std::host::* | 明示的 host capability | stdio, fs, env, process, clock, random, http, sockets |
| std::json / std::toml / std::csv | 実務データ形式 | parse, stringify, stream decode |
| std::wasm | Wasm binary utilities | opcode, valtype, instr, section, module builder, binary reader/writer |
| std::wit | WIT 型と world/interface | type, record, variant, resource, package, parser/printer |
| std::component | canonical ABI / resource handles | own/borrow, lift/lower, handle table, adapter helpers |
| std::test | assertion / snapshot / test data | assert_eq, assert_ne, expect_err, snapshot, bench-lite |

### 5.1 正準 import 例

```ark
use std::prelude
use std::text::string
use std::bytes
use std::collections::{vec, hash_map, interner}
use std::seq
use std::wasm::{binary, leb128}
use std::component

let name = string::from_utf8(bytes::from_hex("6869")?)?
let ids = vec::from_array<i32>([1, 2, 3])
let table = hash_map::new<String, i32>()
let syms = interner::new<String>()
let mod = binary::module_builder()
```

### 5.2 Host API layering

WASI 依存 API は version (`p1`, `p2`) ではなく capability で見せる。version 差分は backend と target support matrix の責務にする。

| 層 | 例 | 目的 |
|---|---|---|
| pure / host-agnostic stdlib | `std::path`, `std::time`, `std::random` | host access を伴わない surface |
| explicit host API | `std::host::stdio`, `std::host::fs`, `std::host::env`, `std::host::process`, `std::host::clock`, `std::host::random`, `std::host::http`, `std::host::sockets` | host capability を explicit に扱う |
| component boundary | `import "wasi:cli/stdin@0.2.10"` | WIT / world / interface 宣言 |

この設計により、`std::host::http` は T3/T5 系専用 module として自然に定義できる。T1 で使えない capability は compile-time error にすればよく、`std::wasi::p1::http` のような不自然な namespace を増やさずに済む。

## 6. 型・エラー・プリルード方針

Arukellt stdlib の成功は、関数数より型設計で決まる。特に `String` / `Bytes` / `Vec<T>` / `Seq<T>` / `Option<T>` / `Result<T, E>` / `Error` の境界が曖昧だと、巨大 stdlib はすぐに泥化する。

ここで重要なのは、LLM に「どの型を今持っているのか」を見失わせないことである。したがって text, binary, lazy sequence, keyed collection, ordered collection を別々の型として保つ。

### 6.1 Prelude

| 区分 | 内容 |
|---|---|
| 含める | Option, Result, String, Bytes, Vec, Some, None, Ok, Err, panic, assert |
| 含めない | HashMap, HashSet, path/fs, json, bytes utilities, sort/search, parse helpers, wasm APIs |
| 理由 | 最初から見える語彙を小さくし、名前衝突と説明量を抑える。 |

### 6.2 Error の正準形

```ark
pub enum Error {
    InvalidArgument(String),
    IndexOutOfBounds { index: i32, len: i32 },
    ParseError { kind: String, input: String },
    Utf8Error,
    IoError(IoError),
    NotFound(String),
    AlreadyExists(String),
    PermissionDenied(String),
    Timeout,
    WasmError(WasmError),
    ComponentError(ComponentError),
}

pub type StdResult<T> = Result<T, Error>
```

Wasm/WIT/binary utilities を真面目に扱うなら、`u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `f32` を言語/stdlib 基礎型として揃える必要がある。現状 API が主に `i32`, `i64`, `f64` を中心としていても、Bytes・LEB128・endianness・resource handle には unsigned 幅付き整数が不可欠である。

したがって本設計では stdlib 設計の前提条件として scalar set の拡張を含める。これは optional ではない。

## 7. コレクションとデータ構造

「Vec と HashMap があれば十分」はコンパイラ・Wasm ツールチェーン・自己ホストを考えると成立しない。Arukellt は最初から、一般アプリ向け collection と compiler/tooling 向け data structure の両方を持つべきである。

### 7.1 採用データ構造一覧

| 型 | 役割 | 採用理由 |
|---|---|---|
| Vec<T> | 連続領域・最頻出 | 必須 Stable |
| Deque<T> | 両端 push/pop | キュー処理・BFS・scheduler |
| HashMap<K,V> | 平均 O(1) 参照 | 一般 map、symbol table |
| HashSet<T> | 集合 | membership / visited |
| BTreeMap<K,V> | 順序付き map | range query / deterministic order |
| BTreeSet<T> | 順序付き set | sorted unique |
| IndexMap<K,V> | 挿入順保存 map | JSON/object, stable emit |
| IndexSet<T> | 挿入順保存 set | stable dedup |
| BitSet | 密なフラグ集合 | graph/compiler/dfa |
| PriorityQueue<T> | 優先度付きヒープ | scheduler / shortest path |
| Arena<T> | 安定 ID と一括所有 | AST/HIR/MIR |
| SlotMap<K,V> | 削除に強い stable handle map | resource tables / handle stores |
| Interner<T> | 値↔ID 双方向化 | symbol / string dedup |
| Rope | 大きい text の編集 | formatter / IDE / parser |

### 7.2 Vec / Deque

```ark
pub fn new<T>() -> Vec<T>
pub fn with_capacity<T>(cap: i32) -> Vec<T>
pub fn len<T>(v: Vec<T>) -> i32
pub fn is_empty<T>(v: Vec<T>) -> bool
pub fn get<T>(v: Vec<T>, index: i32) -> Option<T>
pub fn set<T>(v: Vec<T>, index: i32, value: T) -> Result<(), Error>
pub fn push<T>(v: Vec<T>, value: T)
pub fn pop<T>(v: Vec<T>) -> Option<T>
pub fn insert<T>(v: Vec<T>, index: i32, value: T) -> Result<(), Error>
pub fn remove<T>(v: Vec<T>, index: i32) -> Option<T>
pub fn clear<T>(v: Vec<T>)

pub fn deque_new<T>() -> Deque<T>
pub fn deque_push_front<T>(d: Deque<T>, value: T)
pub fn deque_push_back<T>(d: Deque<T>, value: T)
pub fn deque_pop_front<T>(d: Deque<T>) -> Option<T>
pub fn deque_pop_back<T>(d: Deque<T>) -> Option<T>
```

### 7.3 HashMap / HashSet / IndexMap

```ark
pub fn new<K, V>() -> HashMap<K, V>
pub fn insert<K, V>(m: HashMap<K, V>, key: K, value: V) -> Option<V>
pub fn get<K, V>(m: HashMap<K, V>, key: K) -> Option<V>
pub fn contains_key<K, V>(m: HashMap<K, V>, key: K) -> bool
pub fn remove<K, V>(m: HashMap<K, V>, key: K) -> Option<V>
pub fn keys<K, V>(m: HashMap<K, V>) -> Seq<K>
pub fn values<K, V>(m: HashMap<K, V>) -> Seq<V>

pub fn index_map_new<K, V>() -> IndexMap<K, V>
pub fn index_map_iter<K, V>(m: IndexMap<K, V>) -> Seq<(K, V)>
```

### 7.4 Arena / SlotMap / Interner

```ark
pub fn arena_new<T>() -> Arena<T>
pub fn arena_alloc<T>(a: Arena<T>, value: T) -> ArenaId<T>
pub fn arena_get<T>(a: Arena<T>, id: ArenaId<T>) -> Option<T>

pub fn slot_map_new<K, V>() -> SlotMap<K, V>
pub fn slot_insert<K, V>(m: SlotMap<K, V>, value: V) -> K
pub fn slot_get<K, V>(m: SlotMap<K, V>, key: K) -> Option<V>
pub fn slot_remove<K, V>(m: SlotMap<K, V>, key: K) -> Option<V>

pub fn interner_new<T>() -> Interner<T>
pub fn intern<T>(i: Interner<T>, value: T) -> Symbol
pub fn resolve<T>(i: Interner<T>, sym: Symbol) -> Option<T>
```

Arena/SlotMap/Interner を stdlib に含めることに違和感があるかもしれない。しかし Arukellt が自己ホスト・Wasm ツールチェーン・IDE 支援・parser/formatter を視野に入れるなら、これらは周辺 package に追い出すより共通実装として固定した方が ecosystem の断片化を防げる。

## 8. 文字列・バイト列・エンコーディング

`String` は「Unicode text」、`Bytes` は「raw binary」という二分法を徹底する。`String` の中身をバイト配列として直接舐める API は提供しない。必要なら `string::to_utf8_bytes` と `string::from_utf8` を通る。

この分離は LLM にとっても重要である。text/binary の混同は LLM が最も起こしやすいバグの一つで、型で防ぐ価値が大きい。

### 8.1 主要型

| 型/群 | 用途 | 主要 API |
|---|---|---|
| String | UTF-8 text | split, trim, replace, lines, chars, normalize(optional) |
| StringBuilder | 大量連結用 | append, append_char, append_line, build |
| Rope | 大きな可変 text | slice, insert, delete, line index |
| Bytes | immutable-ish byte sequence | len, slice, concat, compare |
| ByteBuf | 可変 byte buffer | push_u8, extend, reserve, freeze |
| ByteView | 読み取り view | subview, as_slice, cursor |
| ByteCursor | 逐次 decode/encode | read_u32_le, read_var_u32, write_f64_be |
| encoding | hex/base64/utf8/leb128 | parse/format utilities |

### 8.2 Bytes 系 API 草案

```ark
pub fn bytes_new() -> Bytes
pub fn bytes_from_array(xs: Vec<u8>) -> Bytes
pub fn bytes_len(b: Bytes) -> i32
pub fn bytes_slice(b: Bytes, start: i32, end: i32) -> Result<Bytes, Error>
pub fn bytes_concat(a: Bytes, b: Bytes) -> Bytes
pub fn bytes_eq(a: Bytes, b: Bytes) -> bool

pub fn buf_new() -> ByteBuf
pub fn buf_with_capacity(cap: i32) -> ByteBuf
pub fn buf_push_u8(buf: ByteBuf, x: u8)
pub fn buf_extend(buf: ByteBuf, bytes: Bytes)
pub fn buf_freeze(buf: ByteBuf) -> Bytes

pub fn read_u32_le(c: ByteCursor) -> Result<u32, Error>
pub fn write_u32_le(c: ByteCursor, x: u32) -> Result<(), Error>
pub fn read_var_u32(c: ByteCursor) -> Result<u32, Error>
pub fn write_var_u32(c: ByteCursor, x: u32) -> Result<(), Error>
```

### 8.3 String 系 API 草案

```ark
pub fn new() -> String
pub fn from_utf8(bytes: Bytes) -> Result<String, Error>
pub fn to_utf8_bytes(s: String) -> Bytes
pub fn len_bytes(s: String) -> i32
pub fn len_chars(s: String) -> i32
pub fn slice_bytes(s: String, start: i32, end: i32) -> Result<String, Error>
pub fn split(s: String, sep: String) -> Vec<String>
pub fn lines(s: String) -> Seq<String>
pub fn trim(s: String) -> String
pub fn starts_with(s: String, prefix: String) -> bool
pub fn ends_with(s: String, suffix: String) -> bool
pub fn contains(s: String, needle: String) -> bool
pub fn replace(s: String, from: String, to: String) -> String
```

## 9. シーケンス・アルゴリズム

Vec に何でも詰め込むと API が膨れ上がる。そこで Arukellt では「保持する型」と「計算する型」を分離し、遅延パイプラインは `Seq<T>` で表現する。これは Kotlin の Sequence と Rust iterator の中間的な立ち位置で、trait 解決を露出しすぎない形にする。

`Seq<T>` は generator 的 lazy stream だが、surface は module function で統一する。`seq::map`, `seq::filter`, `seq::take`, `seq::collect_vec` を覚えればよく、`Vec` 側は storage responsibility に集中できる。

### 9.1 `Seq<T>` API 草案

```ark
pub fn from_vec<T>(v: Vec<T>) -> Seq<T>
pub fn range_i32(start: i32, end: i32) -> Seq<i32>
pub fn map<T, U>(s: Seq<T>, f: fn(T) -> U) -> Seq<U>
pub fn filter<T>(s: Seq<T>, f: fn(T) -> bool) -> Seq<T>
pub fn flat_map<T, U>(s: Seq<T>, f: fn(T) -> Seq<U>) -> Seq<U>
pub fn take<T>(s: Seq<T>, n: i32) -> Seq<T>
pub fn skip<T>(s: Seq<T>, n: i32) -> Seq<T>
pub fn enumerate<T>(s: Seq<T>) -> Seq<(i32, T)>
pub fn collect_vec<T>(s: Seq<T>) -> Vec<T>
pub fn collect_hash_set<T>(s: Seq<T>) -> HashSet<T>
pub fn fold<T, U>(s: Seq<T>, init: U, f: fn(U, T) -> U) -> U
pub fn any<T>(s: Seq<T>, f: fn(T) -> bool) -> bool
pub fn all<T>(s: Seq<T>, f: fn(T) -> bool) -> bool
```

### 9.2 主要アルゴリズム

| アルゴリズム | 対象 | 備考 |
|---|---|---|
| sort / stable_sort | Vec<T> / mutable sequence | 比較器つき/なしの両方 |
| binary_search | BTree/Vec sorted data | 検索結果は `Result<i32, i32>` 形式でもよい |
| partition / group_by | データ加工 | ETL / compiler passes で多用 |
| dedup / unique | set への橋渡し | stable 版と hash 版を分ける |
| zip / chunk / window | sequence combinator | parser・signal 処理・binary 解析で有用 |

## 10. I/O・時間・環境・プロセス

Arukellt は Wasm 主体であっても、CLI・ファイル入出力・パス操作・時刻・乱数・環境変数・終了コードが標準で揃っていなければ自己ホストやスクリプト用途が成立しない。

ここでは capability model を将来に残しつつ、まずは user-facing surface を整える。`path::join`, `time::duration_ms`, `random::shuffle_i32` のような pure surface と、`host::fs::read_to_string`, `host::env::args` のような host surface を名前で分ける設計がよい。

ただし将来の `wasi:http` や `wasi:sockets` は、この pure 層に混ぜない。これらは `std::host::http`, `std::host::sockets` のような target-gated module に置き、T1 では使えないことを明示する方がよい。

### 10.1 モジュール

| モジュール | 最低限必要な surface |
|---|---|
| std::path | join, parent, file_name, extension, normalize, is_absolute |
| std::time | duration_ms, duration_us, duration_ns |
| std::random | seeded_random, seeded_range, shuffle_i32 |
| std::host::stdio | print, println, eprintln |
| std::host::fs | read_to_string, write_string |
| std::host::env | args, arg_count, arg_at, has_flag, var |
| std::host::process | exit, abort |
| std::host::clock | monotonic_now |
| std::host::random | random_i32, random_i32_range, random_bool |

### 10.1.1 `std::*` と `std::host::*` の分担

- `std::path`, `std::time`, `std::random` は pure subset を提供する
- `std::host::stdio`, `std::host::fs`, `std::host::env`, `std::host::process`, `std::host::clock`, `std::host::random` は explicit host facade とする
- `std::host::http`, `std::host::sockets` は T3/T5 系専用とし、T1 には載せない
- 同じ概念に p1/p2 両実装がある場合は module 名を増やさず、target ごとの backend 実装で吸収する

### 10.2 基本 API 草案

```ark
pub fn duration_ms(start: i64, end: i64) -> i64
pub fn seeded_random(seed: i32) -> i32

pub fn read_to_string(path: String) -> Result<String, Error>
pub fn write_string(path: String, contents: String) -> Result<(), Error>
pub fn args() -> Vec<String>
pub fn exit(code: i32) -> Never
pub fn var(name: String) -> Option<String>
pub fn monotonic_now() -> i64
```

## 11. Wasm / WIT / Component Model

Arukellt の標準ライブラリで最も差別化すべき箇所がここである。多くの言語では Wasm 支援は外部 crate や SDK に逃がされるが、Arukellt はコンパイラ自身が Wasm を中心に設計されている。ならば `std::wasm` と `std::component` を「普通の stdlib module」として扱うべきである。

特に binary writer / reader, LEB128, val types, sections, module builder, WIT types, resource handles, canonical ABI lift/lower は、Wasm 向けツールやコンパイラ、自動生成コード、component glue を書く上で何度も出てくる。

ここで重要なのは、`std::host::*` と `import "wasi:..."` を混同しないことである。前者は Arukellt ソースから使う host API facade、後者は WIT interface / world を直接宣言する component boundary である。

### 11.1 モジュール内訳

| モジュール | 主要内容 |
|---|---|
| std::wasm::types | ValType, NumType, RefType, FuncType, Limits |
| std::wasm::opcode | 命令 enum / mnemonic / stack effect metadata |
| std::wasm::binary | module builder, section builder, parser, printer |
| std::wasm::leb128 | varint codec, size calculators |
| std::host::http / sockets | target-gated host capability facade |
| std::wit | Type, Record, Variant, Enum, Flags, Resource, Interface, World |
| std::component | Own<T>, Borrow<T>, HandleTable, CanonicalString, CanonicalList, lower/lift |

### 11.2 `std::wasm` API 草案

```ark
pub fn module_builder() -> WasmModuleBuilder
pub fn add_type(m: WasmModuleBuilder, ty: FuncType) -> TypeId
pub fn add_func(m: WasmModuleBuilder, sig: TypeId, body: Vec<Instr>) -> FuncId
pub fn add_export_func(m: WasmModuleBuilder, name: String, func: FuncId)
pub fn encode_module(m: WasmModuleBuilder) -> Bytes
pub fn decode_module(bytes: Bytes) -> Result<WasmModule, Error>

pub fn read_var_u32(c: ByteCursor) -> Result<u32, Error>
pub fn write_var_i64(c: ByteCursor, x: i64) -> Result<(), Error>
```

### 11.3 `std::wit` / `std::component` API 草案

```ark
pub enum WitType {
    Bool,
    S32,
    U32,
    String,
    List(Box<WitType>),
    Option(Box<WitType>),
    Result(Box<WitType>, Box<WitType>),
    Tuple(Vec<WitType>),
    Record(Vec<(String, WitType)>),
    Variant(Vec<(String, Option<WitType>)>),
    Enum(Vec<String>),
    Flags(Vec<String>),
    Resource(String),
}

pub fn world_new(name: String) -> World
pub fn world_import(world: World, iface: Interface)
pub fn world_export(world: World, iface: Interface)
pub fn print_wit(world: World) -> String
```

## 12. 安定性・ドキュメント・テスト

標準ライブラリの品質は、型システムだけでなく運用ルールでも決まる。Arukellt は v3 以降、少なくとも `Stable`, `Experimental`, `Internal` の三段階ラベルを manifest に持つべきである。

また全 API に 1 つ以上の executable example を付与し、reference 文書・cookbook・migration guide を分離する。reference は完全性、cookbook は用途別導線、migration は破壊的変更の受け皿として役割が違う。

### 12.1 Stability

| ラベル | 意味 | 例 |
|---|---|---|
| Stable | 後方互換を守る正規 API | Vec/String/Bytes/HashMap/fs/path/time/test の基礎面 |
| Experimental | 設計継続中だが公開する | Rope/SlotMap/WIT parser/component adapter など |
| Internal | compiler/runtime 用で公開保証なし | intrinsic wrapper、backend-private helpers |

### 12.2 ドキュメント運用ルール

- reference: すべての公開関数・型・エラーを網羅する。
- cookbook: JSON 処理、CLI、Wasm binary emit、component glue など task-centric に書く。
- migration: 旧 monomorph 名から新 API への写像を固定する。
- examples: 全部 harness で compile/run し、ドキュメント腐敗を防ぐ。

## 13. Arukellt での実装戦略

「Arukellt で実装する」という条件を満たすため、本設計では surface の大半を source-backed module とし、真に compiler intrinsic が必要な部分だけを backend hook に落とす。言い換えると、`Vec`/`String`/`Bytes`/`HashMap` の primitive operations と WASI bridge を除けば、多くのアルゴリズムや高水準 API は Arukellt 自身で書く。

こうすると stdlib 自体が言語の実例集になり、LLM も stdlib 実装パターンを学習しやすい。自己ホストへの橋にもなる。

### 13.1 レイヤ分担

| レイヤ | 内容 |
|---|---|
| compiler intrinsic 必須 | Vec raw storage, String/Bytes primitive ops, Hashing primitive, WASI bridge, GC runtime hooks |
| source-backed で実装 | algorithms, seq adapters, JSON parser, path logic, rope, arena, slot map, interner, WIT printer |
| hybrid | HashMap/BTreeMap/IndexMap 本体は intrinsic + source wrapper の折衷でもよい |

1. Scalar completeness (`u8/u16/u32/u64/i8/i16/f32`) を先に入れる。
1. `Bytes`, `ByteBuf`, `ByteCursor`, `endian`, `hex`, `base64`, `leb128` を実装する。
1. `Vec`, `HashMap`, `HashSet`, `Deque`, `BTreeMap`, `IndexMap`, `BitSet` を標準化する。
1. `Seq` と algorithms 層を実装し、既存 monomorph HOF を包み直す。
1. `path`, `fs`, `io`, `process`, `env`, `time`, `random` を揃える。
1. `wasm`, `wit`, `component` を追加し、component emit/tooling に使う。
1. 最後に prelude を縮小し、migration guide と deprecation warning を入れる。

### 13.3 命名規約

| 分類 | 命名 |
|---|---|
| 生成 | new / with_capacity / from_* |
| 問い合わせ | len / is_empty / contains / starts_with |
| 変更 | push / insert / remove / clear / retain |
| 失敗可能 | try_* または Result を返す明示名 |
| 危険 API | unsafe_*または unchecked_* を別 namespace |
| 変換 | to_*/ as_* / into_* の意味を厳密化 |

## 14. 移行計画

現在の Arukellt には `Vec_new_i32`, `map_i32_i32`, `filter_i32`, `fs_read_file` のような prelude/intrinsic 直結名が混在している。これを一気に削除すると既存コードが壊れるため、移行は「新 API 追加 → 旧 API deprecated 化 → 旧 API 除去」の三段階で行う。

また v3 では prelude を維持しつつ module import を可能にし、v4 以降で prelude 縮小を完成させるのが現実的である。

### 14.1 旧 API → 新 API

| 旧 API | 新 API |
|---|---|
| `Vec_new_i32()` | `vec::new<i32>()` |
| `map_i32_i32(v, f)` | `seq::map(vec::into_seq(v), f)` または `vec::map(v, f)` |
| `filter_i32(v, f)` | `seq::filter(vec::into_seq(v), f)` |
| `sort_i32(v)` | `vec::sort(v)` |
| `concat(a, b)` | `string::concat(a, b)` |
| `split(s, sep)` | `string::split(s, sep)` |
| `fs_read_file(path)` | `use std::host::fs` + `fs::read_to_string(path)` |
| `clock_now()` | `use std::host::clock` + `clock::monotonic_now()` |
| `random_i32()` | `use std::host::random as host_random` + `host_random::random_i32()` |

### 14.2 移行ルール

- 旧 API は最低 1 マイナー版は警告のみで維持する。
- migration guide に自動変換表を載せる。
- harness に deprecated usage gate を追加し、stdlib 自身は旧 API を使わない。
- prelude 縮小は二段階で行い、既存 fixture を先に module import へ移す。

## 付録 A. 設計判断の比較表

| 判断軸 | 採用 | 理由 |
|---|---|---|
| Prelude サイズ | 小 | discoverability を上げるため |
| Collection breadth | 広い | 自己ホスト・compiler・Wasm tooling を標準で支える |
| Text/Binary | 厳密分離 | UTF-8 と raw bytes の混同を防ぐ |
| Lazy pipeline | `Seq<T>` 導入 | Vec を何でも屋にしない |
| Error model | `Result<T, Error>` | 分岐しやすく、ログしやすい |
| Wasm support | stdlib 本体に含める | Arukellt の主戦場だから |
| Trait exposure | 最小限 | LLM が壊しやすい解決規則を surface に出しすぎない |
| Docs policy | example-first + harness | ドキュメント腐敗を防ぐ |

## 付録 B. 最小 Stable セット

| 領域 | Stable に固定する面 |
|---|---|
| core | Option, Result, Ordering, Range, Error, panic, assert |
| text | String, string::{split, trim, replace, lines, starts_with, ends_with} |
| bytes | Bytes, ByteBuf, Cursor, endian, hex, base64, leb128 |
| collections | Vec, HashMap, HashSet, Deque, BTreeMap, IndexMap, BitSet |
| seq | map, filter, fold, zip, take, skip, collect_vec |
| io/path/fs | read/write, exists, create_dir_all, join, normalize, args, exit |
| time/random | now, instant, duration, next_u32, fill_bytes |
| test | assert_eq, assert_ne, expect_err, snapshot |

## 付録 C. 参考資料

外部調査は原則として公式ドキュメントを参照した。Arukellt 側の現状認識には、同梱された v3-review パッケージの `current-state.md`, `roadmap-v3.md`, `ADR-002`, `ADR-003`, `ADR-004`, `std/manifest.toml`, `std/prelude.ark`, `docs-stdlib/*` を用いた。

- Python Standard Library: <https://docs.python.org/3/library/index.html>
- Python collections: <https://docs.python.org/3/library/collections.html>
- Python collections.abc: <https://docs.python.org/3/library/collections.abc.html>
- Rust std: <https://doc.rust-lang.org/std/>
- Rust core: <https://doc.rust-lang.org/core/>
- Rust alloc: <https://doc.rust-lang.org/alloc/>
- Rust collections: <https://doc.rust-lang.org/std/collections/index.html>
- Go standard library index: <https://pkg.go.dev/std>
- Go builtin: <https://pkg.go.dev/builtin>
- Go os: <https://pkg.go.dev/os>
- Go strings: <https://pkg.go.dev/strings>
- Go errors: <https://pkg.go.dev/errors>
- Java SE API: <https://docs.oracle.com/en/java/javase/21/docs/api/index.html>
- Java core libraries guide: <https://docs.oracle.com/en/java/javase/24/core/java-core-libraries1.html>
- Kotlin stdlib: <https://kotlinlang.org/api/core/kotlin-stdlib/>
- Kotlin collections overview: <https://kotlinlang.org/docs/collections-overview.html>
- MDN JavaScript built-in objects: <https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects>
- MDN TypedArray: <https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/TypedArray>
- MDN JavaScript typed arrays guide: <https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Typed_arrays>
- cppreference standard library overview: <https://en.cppreference.com/w/cpp/standard_library.html>
- cppreference headers/categories: <https://en.cppreference.com/w/cpp/headers.html>
- OCaml Stdlib manual: <https://ocaml.org/manual/stdlib.html>
- OCaml Stdlib API: <https://ocaml.org/api/Stdlib.html>
- Zig docs: <https://ziglang.org/documentation/0.15.1/>
- Haskell hierarchical libraries: <https://downloads.haskell.org/ghc/9.12.1/docs/libraries/index.html>
- WebAssembly Component Model intro: <https://component-model.bytecodealliance.org/>
- Canonical ABI: <https://component-model.bytecodealliance.org/advanced/canonical-abi.html>
- WIT reference: <https://component-model.bytecodealliance.org/design/wit.html>
