# ADR-042: Intrinsic Layer Separation — 意味と実装の分離

ステータス: **PROPOSED**

決定日: 2026-07-10

---

## 文脈

### 現状の構造

Arukellt コンパイラは約 164 個の一般 intrinsic と約 256 個の SIMD intrinsic を持つ。
`src/compiler/wasm/intrinsic_*.ark` (135 ファイル, 15,811 行) と
`call_*.ark` (28 ファイル, 3,302 行) を合わせると、Wasm emitter 内の
intrinsic 関連コードは約 2 万行に達する。これはコンパイラ全体 (104,714 行) の
約 19% である。

現在の構造には以下の問題がある:

1. **callee 文字列による dispatch**: `call_dispatch.ark` が MIR に保存された
   callee 文字列を見て `host → seq → text → scalar → parse → vec → simd` の
   順で振り分ける。各層で `eq(clone(callee), "starts_with")` のような文字列比較を
   418 箇所行っている。`func_id_raw` が既に存在するにもかかわらず、intrinsic 判定
   は文字列ベースである。

2. **emitter が実装を持っている**: `starts_with`、`trim`、`sort`、`map` 等、
   Arukellt コードで書ける操作の実装が Wasm emitter 内にある。これらは
   言語プリミティブではなく、単に「今インライン化がないから emitter に書いた」
   だけのものである。

3. **GC / LM 二重実装**: `intrinsic_string_split.ark` と
   `intrinsic_string_split_gc.ark` のように、多くの intrinsic に GC 版と
   linear-memory 版の二つの実装がある。stdlib 操作が representation-specific
   lowering まで抱えているために起きている。

4. **prelude がスタブ**: `std/prelude.ark` L519-524 に
   "function bodies are never compiled (prelude decls are excluded from
   all_decls during typecheck/lower/emit)" と明記されている。
   `combine_loaded_and_main_decls_skip_prelude` で prelude 全体を backend から
   除外している。一部の関数は Ark で書かれているように見えても、実際には
   ドキュメント兼シグネチャ用スタブである。これが std が形骸化している
   直接的な原因である。

5. **effect 情報不足**: 現在の LICM は opcode を列挙して pure かどうかを判定
   している。CALL の effect が SignatureRegistry にあれば、pure な stdlib 関数の
   ループ外移動、DCE、CSE が可能になる。

6. **ADR-040 との不整合**: ADR-040 は「emitter を馬鹿にする」方向で
   `FunctionId`、`SignatureRegistry`、Typed MIR を導入した。しかし実際の
   intrinsic dispatch はまだ `func_id_raw` ではなく callee 名を使っており、
   ADR-040 の spine が intrinsic dispatch まで到達していない。

### 根本的な認識の誤り

現在の設計では「コンパイラが知っているか／知らないか」の二択で境界を引いている。
これは間違いである。分離すべきは 3 つの軸である:

1. **コンパイラが意味を知っているか** (semantic knowledge)
2. **実装本体を誰が持つか** (implementation ownership)
3. **最終的なコード生成を誰が行うか** (code generation)

Arukellt では、**コンパイラは多数の標準ライブラリ操作の意味を知っていてよいが、
その実装まで Wasm emitter が持つべきではない**。

## 決定事項

### D1: 5 層構造の採用

| 層 | 実装場所 | コンパイラの知識 | 例 |
|----|----------|-----------------|-----|
| 言語プリミティブ | MIR/backend | 完全に知る | GC 参照操作, raw array access, trap, indirect call |
| ランタイム ABI | runtime/host | ABI と effect だけ知る | allocator, panic handler, WASI/WIT imports |
| semantic stdlib | Ark 標準ライブラリ | 意味だけ知る | `Vec.len`, bounds check, String byte length |
| 通常 stdlib | Ark 標準ライブラリ | 通常関数として扱う | `split`, `trim`, `sort`, `map`, `parse` |
| target intrinsic | target 固有 module | 対象 backend だけ知る | Wasm SIMD, relaxed SIMD, 特殊命令 |

### D2: semantic stdlib 層の導入

Swift が Array、String、Dictionary を Swift 標準ライブラリ自身で実装しつつ
`@_semantics("array.count")` のような意味タグを付けるのと同様に、
Arukellt でも `Vec.len` は Ark 関数として本体を持たせながら、
SignatureRegistry 上では `SemanticId::VecLen` を持たせる。

通常時は普通に Ark 関数を呼ぶ。最適化時だけ:

- `VecLen(VecNew(...)) → 0`
- 同じ Vec への変更がなければ len 呼び出しを共通部分式除去
- bounds check と get の関係を解析
- 必要なら MIR の `ARRAY_LEN` へ置換
- 後段で小さな関数をインライン化

と処理する。「std に実装がある」と「コンパイラが意味を知っている」を両立する。

### D3: 真の primitive の判定条件

compiler intrinsic にしてよいのは、原則として次のどれかに該当するものだけ:

- それ自身なしでは定義できず、循環する
- Ark の型や値では保存できない意味がある
- GC、provenance、atomic、volatile など、最適化時にも保持すべき特殊な意味がある
- コンパイラがユーザーコード中に自動挿入する
- 特定ターゲット命令との 1 対 1 対応を明示的に提供する

逆に、次の理由だけでは intrinsic にしない:

- 現在インライン化がない
- Ark 実装より手書き Wasm の方が速い
- ホストアクセスが必要 (runtime/import でよい)
- emitter で書く方が簡単
- いつか最適化するかもしれない

### D4: Vec/String の内部表現は sealed raw API 経由のみ

Vec/String の内部表現は一般ユーザーコードには公開しない。
ただし stdlib 専用の sealed raw API (`core::raw` / `core::rt` / `core::intrinsics`)
からは触れるようにする。

例えば Vec は概念的には:

- `raw_array_new<T>`
- `raw_array_len<T>`
- `raw_array_get_unchecked<T>`
- `raw_array_set_unchecked<T>`
- `raw_array_grow<T>`

だけに依存し、その上に Ark で `Vec::new`、`Vec::push`、`Vec::pop`、`Vec::get`、
`Vec::set`、`Vec::reserve`、Iterator 操作を実装する。

`len`/`push`/`get` そのものを言語構文や真の primitive にはしない。

### D5: SignatureEntry の拡張

ADR-040 の SignatureEntry に以下を追加する:

| 情報 | 例 |
|------|-----|
| semantic ID | `VecLen`, `VecGetUnchecked`, `StringByteLen` |
| effect | pure, read, write, allocate, IO, noreturn |
| may trap | bounds check, parse 等 |
| const evaluable | 可否 |
| inline policy | never, hint, always |
| lowering policy | normal call, MIR op, runtime call, target intrinsic |
| fallback body | Ark 関数の FunctionId |

この情報は `std/manifest.toml` または別の `core-ops.toml` を単一の正本にして、
resolver、typechecker、MIR、docs、runtime ABI 表を生成する。
GHC の `primops.txt.pp` + `genprimopcode` と同じ手法である。

現在の `std/manifest.toml` は「single source of truth」と書かれている一方、
冒頭コメントで resolver/typechecker 間の不一致を列挙している。
正本として参照されてはいるものの、まだ生成元にはなっていない。
ここを本当に生成元にする。

### D6: callee 文字列 dispatch の廃止

intrinsic 判定を callee 名から `func_id_raw` + `SemanticId` へ移行する。
`SemanticId` は FunctionId から取得し、名前は診断表示にだけ使う。

### D7: prelude のコンパイル対象復帰

`combine_loaded_and_main_decls_skip_prelude` による prelude 除外を廃止し、
prelude 本体を本当にコンパイル対象に戻す。偽の関数本体を廃止する。

## 各機能の移動先

### Compiler/MIR に残すもの (言語プリミティブ)

- GC object/array の生成
- array length
- unchecked array get/set
- GC cast/test
- 必要な write barrier
- raw linear-memory load/store
- `memory.copy`, `memory.fill` (MIR 命令として)
- trap/unreachable
- function reference, indirect call
- 本当に表現不可能な bit cast
- target-specific SIMD

`memory.copy` 等は「名前付き関数を emitter で検出」するのではなく、
MIR 命令にする。

### Runtime ABI へ移すもの

- allocation/reallocation
- process exit/abort
- panic handler
- stdin/stdout/stderr
- filesystem
- clocks
- random
- HTTP
- sockets
- streams
- environment variables

WASI 系は、コンパイラが `fs_read_file` や `http_get` を個別に知るのではなく、
汎用的な WIT/import lowering だけを知るべき。
`std::fs::read_to_string` は `Ark stdlib → WIT binding/import → host` 経路にする。

`panic` はコンパイラが bounds check 失敗などから呼ぶため、Rust の lang item に
近い扱いとする。実装は runtime に置き、コンパイラは `PanicHandler` という
FunctionId だけを知る。

### Ark stdlib へ移すもの

- `starts_with`, `ends_with`, `contains`, `index_of`
- `trim`, `replace`, `split`, `join`, `repeat`, `pad_left/right`, `lines`
- `to_upper/lower`
- `sort`, `reverse`
- `map/filter/fold/find/any`
- `range_new/contains/len`
- `gcd`
- HashMap/HashSet 本体
- 数値 parse/format

`map_i32_i32`, `map_i64_i64`, `map_f64_f64` のようなモノモーフィック intrinsic 群は
generic/trait 実装へ統合する。これらが個別 intrinsic になっていること自体が、
型システムまたはモノモーフ化の不足を backend で補っている兆候である。

### Target intrinsic として残すもの

- Wasm SIMD 命令
- relaxed SIMD
- atomics
- target feature detection
- Wasm-specific reference/table operations

公開場所は `core::arch::wasm` / `std::arch::wasm` のような target-specific module。
一般的な `Vec::push` や `String::split` が Wasm API に依存してはならない。

## 移行段階

### 第 1 段階: intrinsic 追加を凍結

新規の文字列 dispatch を禁止する。例外は target-specific SIMD のみ。
同時に、すべての呼び出し判定を callee 名ではなく `func_id_raw` と
`SemanticId` へ移す。

### 第 2 段階: host intrinsic を runtime ABI へ分離

HTTP、fs、socket、clock、random、process、stdio を emitter から外す。
インライン化とは無関係なので先にできる。
WIT/import lowering の汎用機構に統合する。

### 第 3 段階: semantic registry を作る

SignatureEntry に semantic ID、effect、may trap、const evaluable、
inline policy、lowering policy、fallback body を追加する。
`core-ops.toml` を単一の正本にして、各種 compiler データを生成する。

### 第 4 段階: 小さな stdlib 専用 inliner

最初は一般的な高度 inliner でなくてよい:

- compiler-shipped core/std だけ対象
- 再帰なし
- MIR 命令数が小さい
- 単一 basic block または単純 CFG
- `@inline(always)` または cost threshold 以下
- target ごとの code size 上限あり

Swift と同様に、semantic operation は早期にはインライン化せず高水準最適化に使い、
後段でインライン化する設計が理想。

### 第 5 段階: pure operation を Ark へ移す

まず `gcd`、range 操作、trim start/end、starts/ends with、contains/index_of、
reverse、any/find/fold、sort から移す。
split、replace、format、HashMap などは、allocation や representation の設計が
安定してから移す。

### 第 6 段階: prelude のコンパイル対象復帰

prelude 本体を本当にコンパイル対象に戻し、偽の関数本体を廃止する。

## 目標規模

最終的な目標は個数ではなく責務で決める。目安:

- 真の target-independent primitive: 20〜40 種類
- runtime ABI 分類: 5〜15 種類
- compiler-known semantic ID: 20〜50 種類
- target SIMD: 多数でもよいが、表から自動生成
- 通常 stdlib: 個数制限なし

semantic ID が 50 個あっても問題は小さい。実装本体は stdlib にあり、
compiler 側には型・effect・最適化規則だけがあるため。

## 等価性検証

semantic lowering には必ず Ark fallback body を残す。
最適化 ON/OFF、GC/LM、各 target について、Ark fallback 版と
optimized lowering 版を同じ入力で実行し、結果と副作用が一致する
differential test を置く。compiler 特殊化が std の意味から乖離するのを防ぐ。

## ADR-040 との関係

本 ADR は ADR-040 の拡張である。ADR-040 は型情報の spine 化を行った。
本 ADR はそれを呼び出し意味・effect・intrinsic lowering まで拡張する。
実質的に ADR-040 を型情報だけで終わらせず、semantic spine を完成させる。

## 参考文献

- Swift: `@_semantics` attribute と HighLevelSILOptimizations
  ([swift/docs/HighLevelSILOptimizations.rst](https://github.com/swiftlang/swift/blob/main/docs/HighLevelSILOptimizations.rst))
- LLVM: Intrinsic の意味・型・メモリ効果定義
  ([LLVM Language Reference Manual](https://llvm.org/docs/LangRef.html))
- Rust: MIR における binary/unary/index 操作と lang items
  ([Rust Compiler Development Guide](https://rustc-dev-guide.rust-lang.org/mir/construction.html),
  [Lang Items](https://rustc-dev-guide.rust-lang.org/lang-items.html))
- Go: 高水準構文から runtime call への lowering
  ([Go Compiler README](https://go.dev/src/cmd/compile/README))
- GHC: `primops.txt.pp` + `genprimopcode` による primitive 宣言の表化
  ([GHC primops.txt.pp](https://gitlab.haskell.org/ghc/ghc/-/blob/master/compiler/GHC/Builtin/primops.txt.pp))
- Rust `core::intrinsics`: 標準ライブラリ実装詳細としての intrinsic
  ([core::intrinsics](https://doc.rust-lang.org/core/intrinsics/index.html))
