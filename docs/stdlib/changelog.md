# Stdlib API Changelog

> Stdlib の公開 API lifecycle（deprecation / removal / tier change）を記録する手動管理の changelog。
>
> ADR-014 と [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) が要求する changelog
> entry の置き場である。現在の stability と replacement は `std/manifest.toml` と生成された
> `reference.md` / `eradication.md` を正とし、このファイルは変更履歴を持つ。生成物では
> ない。entry は日付順に追記し、過去の entry を書き換えない。

## 記録ルール

- 1 entry = 1 日付 + 1 wave。ADR / issue、対象シンボル、変更、replacement を書く。
- `stable` の削除は [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) の 3 条件
  （W0009、migration guide、この changelog）を満たす。条件を満たさない削除を先に
  記録しない。
- このファイルの過去 entry は削除時点の履歴であり、現在の API 一覧ではない。現在の
  事実は manifest と生成 docs から取得する。

## 2026-10-30 — Slice constructor/accessor free の退役と method ABI の一本化 (ADR-046, ADR-087, ADR-091, #917, wave/slice-abi)

- **Removed**: prelude の free `as_slice(v, s, e)` と `slice_get(view, i)`。
  manifest entry と `pub fn` 宣言を削除した。bare call と
  `use std::prelude::{as_slice|slice_get}` は **E0100** と receiver method
  hint（`v.as_slice(start, end)` / `view.get(idx)`）で失敗する。
  `as_slice` 宣言は `Vec<T>` receiver method の signature anchor として
  private で残す。`Slice<T>::len` / `get` は変更しない。
- **ABI decision**: `Slice<T>` の表現は target ごとに 1 つだけ持つ。
  `wasm32` は `[tag=0, vec, start, end]` の 4 word record への pointer、
  `wasm32-gc` は同じ field 並びの GC struct ref である。構築経路は
  private prelude anchor と import された `Vec<T>` impl の 2 つだが、
  どちらも同じ `Slice::View` payload を作り、`Slice<T>::len` / `get` が
  その 1 表現だけを消費する。free を両方退役したことで、free 構築と
  method 構築を混在させる旧経路自体が surface から消える。
- **WAT evidence**: `wasm32` の `as_slice` は
  `(param i32 i32 i32) (result i32)` で 16 byte を確保し、tag 0 / vec /
  start / end を格納して pointer を返す。`Slice::len` は `(param i32)`、
  `Slice::get` は `(param i32 i32)` で同じ pointer を受ける。
  `wasm32-gc` でも method 構築（`Vec__as_slice__i32`）と anchor 構築の
  戻り値は同じ Slice struct ref で、`Slice__len__i32` / `Slice__get__i32`
  が同じ field を読む。退役前に method 構築 + free 消費、free 構築 +
  method 消費、function 境界を越える view の 4 通りを両 target で実行し、
  同じ出力になることを確認した。
- **Migration**: 最後の in-tree free caller `ownership/adt_probe` を
  `values.as_slice(0, 1)` / `view.len()` / `view.get(0)` へ移行した。
  `collections/slice_method_abi` を追加し、import された `Vec<T>` impl の
  method 構築と function から返る view を両 target で検証する。
- **Diagnostics / fixtures**: `resolver::removed_names` と
  `resolver::register_use` に E0100 hint を追加し、W0009 行を削除した。
  `diagnostics/removed_as_slice_free{,_wasm32}` /
  `removed_as_slice_import` / `removed_slice_get_free{,_wasm32}` /
  `removed_slice_get_import` を追加した。
- **Scorecard**: real std surface 13 → **11**、`pub fn` declarations
  13 → **11**、manifest public entries 13 → **11**、`must delete`
  6 → **4**、vec intrinsic-compat rule 5 → **3**、deprecated 5 → **3**。
- **Gates**: pin `56ff0808…` から build した s2 で `verify fixtures`
  1957/1957 PASS（112 runtime、43/43 test、0 FAIL、runtime-unsupported 2）、
  `verify lane` PASS、`docs check` 6/6、`selfhost diag-parity` PASS=279、
  ADR-029 fixpoint は s2 == s3 = sha256
  `7d29d63795f3d732952e66ce037edbe534d28e3affc6a3985385a45697f1fad8`。
  `check-stdlib-eradication` / `check-compat-orphans` / `check-comment-policy`
  / `check-ark-code-quality` / `check-semantic-debt` / `git diff --check` も
  clean。追加 fixture: `collections/slice_method_abi` と
  `diagnostics/removed_{as_slice,slice_get}_{free,import}`（各 wasm32 版を含む）。

## 2026-10-30 — prelude free `push` / `pop` / `set` の退役 (ADR-046, ADR-087, ADR-088, #917, wave/vec-push-pop-set)

- **Removed**: prelude の free `push(v, x)` / `pop(v)` / `set(v, i, x)`。
  manifest entry と `std/prelude.ark` の `pub fn` 宣言を削除した。bare call
  と `use std::prelude::{push|pop|set}` は **E0100** と receiver method hint
  （`v.push(x)` / `v.pop()` / `v.set(i, x)`）で失敗する。宣言自体は
  `Vec<T>` receiver method の signature anchor として private で残す
  （String / struct / nested element の method 型付けに必要）。
- **Migration**: compiler / std / fixtures / benchmarks の
  free call 4,065 箇所を receiver method へ移行した。codemod は
  `v.push(x)` / `v.pop()` / `v.set(i, x)` を生成し、`use std::prelude::{...}`
  から 3 名を除去する。
- **Root cause**: receiver method の call は引数を operand stack で渡すが、
  module 定数は accessor call の結果が local に materialize されるため
  stack に残らない。`v.set(0, K)` / `v.push(NIL)` は operand が 1 個不足し、
  pin で build した s2 が invalid core Wasm を出していた。AST / typed CoreHIR
  の両 method lowering で const 引数を reload し、CoreHIR direct call の
  staged const も二重 copy しないようにした。linear `wasm32` の
  `Stdio::println(x.to_string())` は printlike call の引数を stage して
  修正した（wave/to-string-linear の fix を前提として取り込む）。
- **Bootstrap**: pinned compiler は linear target で `Vec<String>` への
  field-access 引数 push を materialize するため、compiler source の
  `module_names.push(md.name)` は local へ hoist した。pin 更新時に戻す。
- **Diagnostics / fixtures**: `resolver::removed_names` と
  `resolver::register_use` に E0100 hint を追加し、W0009 行を削除した。
  `stdlib_vec/prelude_const_argument{,_wasm32}` を追加し、const 引数の
  `push` / `set` / `pop` を両 target で検証する。残差として
  `docs/sample/parser.ark` は len 退役時点から型検査が通らない stale sample
  のため、free `push` / `pop` 呼び出しを残す（sample 修復とは分離）。
- **Scorecard**: real std surface 16 → **13**、`pub fn` declarations
  16 → **13**、manifest public entries 16 → **13**、`must delete`
  9 → **6**、vec intrinsic-compat rule 8 → **5**。
- **Gates**: pin `56ff0808…` から build した s2 で `verify fixtures`
  1956/1956 PASS（112 runtime、43/43 test、0 FAIL、runtime-contract
  delegation 2）、`verify lane` PASS、`docs check` 6/6、
  `selfhost diag-parity` PASS=273、ADR-029 fixpoint は
  s2 == s3 = sha256 `6e0c6b60372cf82e2d17113c4bb529506fd2fefcaab614a6a77e213275198e6d`。
  追加 fixture: `stdlib_vec/prelude_const_argument{,_wasm32}` と
  `diagnostics/removed_vec_mutation_{free,import}`。

## 2026-10-30 — vec intrinsic-compat `len` free の退役 (ADR-046, ADR-087, ADR-091, #917, #936, wave/vec-intrinsic-len)

- **Removed**: prelude の signature-only free `len(s: String) -> i32`。
  `String` / `Vec<T>` の両 receiver を 1 つの free が担い、checker が
  receiver owner から `String::len` / `Vec::len` を選ぶ最後の
  intrinsic-compat member だった。bare call と
  `use std::prelude::{len}` は **E0100** と receiver method hint
  （`s.len()` / `v.len()`）で失敗する。manifest entry と
  `std/prelude.ark` 宣言を削除した。
- **Replacement**: `s.len()`（`String`）と `v.len()`（`Vec<T>`）。
  inline の `s.slice(start, end).len()` も同じ method spelling で型付け・
  lower される。
- **Migration**: compiler / std / fixtures / benchmarks の free `len(...)`
  呼び出し 5,701 箇所を receiver method へ移行した。`len` を列挙していた
  `use std::prelude::{...}` も整理した。`docs/sample/parser.ark` は HEAD で
  既に現行型検査を通らない stale sample のため対象外とし、122 箇所を残す
  （sample の型エラー修復とは分離）。
- **Bootstrap**: pinned compiler は `String` receiver の `.len()` を
  未だ bare `len` declaration 経由で lower するため、flat overlay の
  prelude に overlay-only の `len` 宣言を残す（`scripts/selfhost/checks.py`
  の `BOOTSTRAP_PRELUDE_LEN_SHIM`）。user program は退役後の surface
  （E0100 + receiver method）だけを見る。pin 更新時に削除する。
- **Diagnostics / fixtures**: `resolver::removed_names` と
  `resolver::register_use` に E0100 hint を追加し、W0009 行を削除した。
  `diagnostics/removed_len_free{,_wasm32}` と
  `diagnostics/removed_len_import` を追加し、`stdlib_vec/len_method_surface`
  （`s.len()`、`v.len()`、inline slice receiver）を両 target で追加した。
  旧 `prelude_len_*` diagnostics は退役に伴い削除した。
- **Scorecard**: real std surface 28 → **27**、`pub fn` declarations
  28 → **27**、manifest public entries 28 → **27**、`must delete`
  21 → **20**、vec intrinsic-compat rule 9 → **8**。`prelude_wrapper`
  5、monomorphic helpers 2、intrinsic-only 1 は不変。

## 2026-10-30 — `std::io` / `std::host::stdio` 互換 free 9 個の退役 (ADR-046, ADR-094, ADR-087, #693, wave/io-stdio)

- **Removed**: `std::host::stdio` の `read_stdin` / `print` / `println` /
  `eprintln` と、`std::io` の `stdout` / `stderr` / `print_bytes` /
  `stdin` / `read_stdin_line`。いずれも `stable` だったが、ADR-094 の
  限定例外により W0009 window を置かず E0100 へ移行した（ADR-087 D2 の
  migration diagnostic / migration guide / changelog の 3 条件を満たす）。
  manifest entry と std 宣言を削除した。
- **Replacement**: 文字列ストリームは `std::host::stdio::Stdio` の
  associated function（`Stdio::read_stdin()` / `print(s)` / `println(s)` /
  `eprintln(s)`）。タグ付き sink は typed handle
  （`io::Stdout::new()` / `io::Stderr::new()`）、`print_bytes` は
  `io::Stdout::new().write(data)`。`stdin` / `read_stdin_line` は実装の
  なかったプレースホルダーであり、置換なしで退役した（stdin 全体は
  `Stdio::read_stdin()`）。
- **Compiler**: `data/core-ops.toml` から `stdio::read_stdin` /
  `stdio::print` / `stdio::println` / `stdio::eprintln` の legacy alias を
  削除し、`core_op_binding_generated.ark` を再生成した（650 -> 646
  bindings）。`resolver::removed_names` は module 名 `stdio` /
  `host::stdio` / `std::host::stdio` と `io` の各 free に E0100 hint を返す。
  MIR の dead な `stdio::*` callee 特例も削除した。
- **Bootstrap**: flat overlay は `std/host/stdio.ark` を同梱し、entry module
  だけ pin が解決できる prelude `println` stub へ書き換える。pin 更新時に
  この bridge を削除する。
- **Migration**: `docs/stdlib/migration-guidance.md` の
  `std::host::stdio / std::io compatibility wave` に before/after を記録。
  compiler 自身、fixtures、benchmarks、docs examples の `stdio::*` 呼び出しを
  `Stdio::*`（または typed handle）へ移行した。
- **Fixtures**: `stdlib_host/stdio_namespace_surface{,_wasm32}` を追加し、
  `diagnostics/removed_stdio_free_function` / `removed_io_stdout_free` /
  `removed_io_stdin_free` を E0100 golden として追加した。
  `stdlib_io/read_stdin_line_empty` は退役し、`stdlib_io/read_stdin_empty`
  は `Stdio::read_stdin()` へ移行した。
- **Scorecard**: public free 41 -> **32**、`pub fn` declarations 41 -> **32**、
  manifest public entries 41 -> **32**、`associated-or-method` 16 -> **7**。
  `std::io and host stdio frees` 規則は live match を失ったため削除した。

## 2026-10-30 — 数値変換 free function 17 個の退役 (ADR-046, ADR-067, ADR-087, #917, wave/numeric-conv)

- **Removed**: prelude の数値変換 free function 17 個
  （`u8_to_i32`、`u16_to_i32`、`i8_to_i32`、`i16_to_i32`、`i32_to_u8`、
  `i32_to_u16`、`i32_to_i8`、`i32_to_i16`、`i32_to_u32`、`u32_to_u64`、
  `u64_to_u32`、`i32_to_i64`、`i64_to_i32`、`f32_to_f64`、`f64_to_f32`、
  `f64_bits_lo`、`f64_bits_hi`）。ADR-087 Wave 1 で deprecated だったものの
  Wave 2 削除。manifest entry と `std/prelude.ark` 宣言を削除し、bare call は
  W0009 ではなく **E0100** と receiver method hint を返す。明示的な
  `use std::prelude::{name}` も同じ hint で拒否する。
- **Replacement**: `std::core::convert` の receiver method
  （`u8::to_i32`、`i8::to_i32`、`u16::to_i32`、`i16::to_i32`、
  `i32::to_u8_wrapping` / `to_u16_wrapping` / `to_i8_wrapping` /
  `to_i16_wrapping` / `to_u32_bits`、`u32::to_u64`、
  `u64::to_u32_wrapping`、`i32::into`、`i64::to_i32_checked` /
  `to_i32_wrapping` / `to_i32_saturating`、`f32::to_f64`、`f64::to_f32`、
  `f64::to_bits_lo` / `to_bits_hi`）。method 本体は `__intrinsic_*`
  conversion bridge を直接 lower し、free への委譲を廃止した。
- **Compiler**: `data/core-ops.toml` の legacy alias を bare free 名から
  `__intrinsic_*` 名へ移し、`core_op_binding_generated.ark` を再生成した。
  `typechecker/builtins.ark` の scalar-conversion concrete-signature 表と
  `lint/deprecated_table.ark` の W0009 数値変換表を削除。MIR の
  `ctx_fn_return_vt` は f32→f64 / f64→f32 の intrinsic 結果型を返す。
  bootstrap pin の baked binding table のため、overlay
  （`scripts/selfhost/checks.py`）だけが退役 free 宣言と free 呼び出しを
  再導入する。pin 更新時に削除する。
- **Migration**: `docs/stdlib/migration-guidance.md` の 17 行を removed
  section に移し、`docs/language/guide.md` の parse 例を `TryFrom` と
  `.into()` に更新した。`std/bytes`、`std/io`、`std/host/fs`、
  `std/toml/parser`、`std/core/hash`、compiler 自身の呼び出しを
  receiver method / intrinsic へ移行した。
- **Fixtures**: `scalar/conversion`、`scalar/sign_extend`、
  `scalar/suffix_literals`、`scalar/const_i32`、
  `scalar/numeric_conversion_methods`、`scalar/numeric_explicit_conversions`、
  `simd_conformance/float_lane_ops*`、`component/export_*f32*`、
  `operators/arithmetic_edges`、`stdlib_bytes/*`、`stdlib_simd/*`、
  `stdlib_inline/abs_probe` を method surface へ移行した。
  `diagnostics/deprecated_numeric_conversion*` は
  `removed_numeric_conversion*` の E0100 golden に置換し、数値変換専用の
  引数型 diag 2 個を削除、`prelude_concrete_arg_type` は残存する
  `String_from` の concrete signature 検査へ置き換えた。
- **Scorecard**: `docs/stdlib/eradication.md` の生成値で public free
  symbols 69 → **52**（`must delete` 53 → **36**）、manifest public
  entries 69 → **52**、`pub fn` declarations 69 → **52**、
  `prelude_wrapper` 26 → **9**、monomorphic helpers 19 → **4**、
  deprecated entries 52 → **35**。
- **Verification**: `verify fixtures` 1949/1949 compile+validate PASS
  （112 runtime、43/43 test、0 FAIL）、`verify lane` PASS、`docs check`
  6/6、`selfhost diag-parity` PASS、ADR-029 fixpoint PASS（s2 == s3 sha256
  `947dd5a7…`、pinned base `56ff0808…`）。
- **Residual**: pinned bootstrap overlay の numeric-conversion bridge は
  pin refresh まで残る。native-c の `raw.f64_bits_*` capability は
  `planned` のままで、`f64::to_bits_*` は method 経由で scalar handler を
  使う。

## 2026-10-30 — Hash collection の `entry` / `iter` / `retain` / `FromIterator` (ADR-093, wave/borrowed-iter)

- **Added**: `HashMap<K, V>::iter()`、`HashMap<K, V>::entry(key)`、
  `HashMap<K, V>::retain(keep: fn((K, V)) -> bool)`、および 6 pair の
  `FromIterator<HashMapIter<...>> for HashMap<...>`。`HashSet<i32>` /
  `HashSet<String>` にも snapshot `iter()` と `retain(keep: fn(T) -> bool)` を
  追加した。公開型は `HashMapIter<K, V>`（snapshot cursor）と
  `Entry<K, V>`（owned handle）で、いずれも provisional。
- **Contract**: v2 に borrow がないため、`iter` は keys/values の snapshot を
  所有する cursor、`entry` は map と key を所有する handle である。
  `or_insert` / `or_insert_with` は消費時に lookup して absent なら書き戻す。
  `retain` の predicate は borrowed `&K` / `&V` ではなく owned pair 1 個を
  受け取る。`HashMap` の既存 `IntoIterator` は keys のままで、pairs は
  `m.iter()` で走査する。cursor の順序は bucket order（implementation-defined）。
- **Design**: 採用理由と却下案は ADR-093 に記録した。`VecIter<(K, V)>` は
  両 target で tuple payload を落とすため cursor にせず、専用の
  `HashMapIter<K, V>` が keys/values snapshot を保持する。`Entry` は pair ごとの
  concrete impl で map field の GC layout を確定する。`retain` は pair ごとの
  store target で snapshot 評価後に拒否 key を削除する。
- **Not changed**: 退役済み `hashmap_*` free function 群は非公開 MIR lowering
  target のままで、新しい method surface はそれらを置き換えない。E0100
  hint / migration scorecard の変更はない。
- **Fixtures**: `stdlib_hashmap/hashmap_entry_iter_retain` と `_wasm32`、
  `stdlib_hashset/hashset_iter_retain` と `_wasm32` を追加し、i32/String/fn pair の
  `iter` / `entry` / `retain` / `FromIterator` と set の `iter` / `retain` を
  両 target で実行する。
- **Verification**: `verify fixtures` 1949/1949 PASS、`verify lane` PASS、
  `docs check` 6/6、`selfhost diag-parity` PASS、ADR-029 fixpoint PASS
  （s2 == s3 sha256 `b4cd5a22…`、pinned base `56ff0808…`）。
- **Residual**: `HashMap::into_iter` の keys 契約は据え置き。borrow / lifetime
  導入時に `iter` / `entry` を参照ベースへ置き換えるか併存させるかを再検討する。
  `retain` の flat store は snapshot + 個別 remove のため最悪 O(n²)。

## 2026-10-30 — `Vec<T>::clone` は import なしで deep (ADR-091, wave/clone-noimport)

- **Changed**: `std/prelude.ark` が `std::core::clone` を import し、
  `impl<T> Clone for Vec<T>` から `T: Clone` bound を外した。`.clone()` は
  `use std::core::clone` なしで独立した element storage を確保する。element
  type に `Clone` impl があれば element-wise に clone し、なければ compiler の
  identity copy を使う。
- **Contract**: Vec の storage は常に独立。element の深さは element type の
  `Clone` impl に依存する。struct / enum / Option の `clone` は従来どおり
  compiler identity fallback である。
- **Fixed**: prelude の `Clone` 登録により `Option<i32>.clone()` が
  `i32::clone(Option ref)` へ lower されていた MIR 解決を修正した。`option:`
  receiver の bare `clone` は payload owner に再解決しない。また
  `std::core::clone` / `std::core::eq` の module short name が bare symbol と
  して登録されることで退役した `clone(x)` / `eq(a, b)` が解決されていたため、
  `resolve_ident` は use-alias を持たない bare module symbol に retirement
  hint を返す。明示 import（`use std::prelude::{filter_i32}` など）は alias を
  持つので影響を受けない。
- **Bootstrap**: overlay は prelude の clone import を strip し、pin 用の
  shallow inherent `Vec::clone` bridge を維持する。pin の clone emitter が HEAD に
  追いついたら両方を削除する。
- **Fixtures**: `clone_no_import/vec_clone_deep`（gc runtime）と
  `clone_no_import/vec_clone_deep_wasm32`（wasm32 `test:`）。`poly_clone` /
  `poly_clone_wasm32` は Option receiver の回帰。
- **Evidence**: `verify fixtures` PASS=1939 FAIL=0（112 runtime smoke、42/42
  `test:`、2 runtime-unsupported）、`verify lane` PASS、`docs check` 6/6 PASS、
  `selfhost diag-parity` PASS。ADR-029 fixpoint PASS（s2 == s3 at sha256
  `6c3aa713bc392c9630b74829020bfc8ace13cfd364771f96ffff0dbc623b7983`）。

## 2026-10-30 — `HashSet<String>` の bucket capacity 表面 (#917, wave/hashset-capacity)

- **Changed**: `HashSet<String>` を独立した `Vec<String>` facade から、
  String-keyed `HashMap<String, i32>` bucket store の key view へ移した。
  membership は同じ keys/flags 配列を使い、value slot は常に 0、capacity 群は
  map の counters と 0.75 load-factor rehash を共有する。`capacity` /
  `reserve` / `try_reserve` / `shrink_to_fit` を `HashSet<String>` に追加し、
  既存の `clear` / `extend` / set algebra も同じ store 上で lower する。
  `HashSet<i32>` にも `try_reserve` を追加し、両 element family の capacity
  表面を揃えた。
- **Contract**: `try_reserve` の `true` は reserve 完了を意味する。String set の
  `to_vec` / `IntoIterator` / `Clone` / `Debug` は bucket order
  （implementation-defined）で snapshot し、順序は保証しない。
- **Removed**: `HashSet<String>` の capacity に対する E0200 境界と、診断
  fixture `diagnostics/hashset_string_capacity*` /
  `hashset_string_reserve` / `hashset_string_shrink_to_fit`。family 分岐の
  拒否機構自体は将来の family 固有 method 用に残る。
- **Fixtures**: `stdlib_hashset/hashset_string_capacity` と
  `_wasm32` variant を追加し、`hashset_family_split` / `hashset_methods` を
  両 family の capacity coverage へ更新した。
- **Residual**: borrow-based `entry` / `iter` / `retain` /
  `FromIterator` parity と、`with_capacity` 付き `HashSet` 構築は未対応。

## 2026-10-30 — `HashMap<K, V>` capacity / bulk surface の全 pair 展開 (#917, wave/hashmap-generic)

- **Changed**: `with_capacity` / `capacity` / `reserve` / `try_reserve` /
  `shrink_to_fit` / `clear` / `drain` / `remove_entry` / `extend` を
  `HashMap<i32, i32>` 限定から、サポートする全 pair
  （`i32/String` キー × `i32`/`String`/`fn(i32) -> i32` 値）へ広げた。
  宣言は flat pair の `impl HashMap<i32, i32>` shell と typechecker の HashMap
  method table が担い、MIR が receiver の pair ごとの store 実装へ rewrite する。
  非 flat 宣言を generic shell へ置かないのは、pinned bootstrap が
  `std/collections/hash_map.ark` を型検査し続けられるようにするためである。
  `HashMap<i32, i32>` は flat `Vec<i32>`、String/fn pair は compiler 管理
  bucket 配列を `__hm_*` bridge 経由で操作する。
- **Contract**: `drain` は `Vec<(K, V)>`、`remove_entry` は `Option<(K, V)>` を
  返す。`try_reserve` の `true` は reserve 完了を意味する。flat pair の
  `with_capacity(0) -> capacity 1` を含め、0 以下の容量は 1 に clamp する。
- **Removed**: 非 flat pair の capacity 呼び出しに対する E0200 境界。診断 fixture
  `diagnostics/hashmap_string_capacity*` と
  `diagnostics/hashmap_string_with_capacity*` を削除し、positive fixture
  `stdlib_hashmap/hashmap_string_capacity` / `hashmap_pair_capacity` /
  `hashmap_capacity_pairs` を追加した。
- **Not changed**: `HashSet<String>` の capacity 境界は据え置き。String set は
  bucket layout を持たない `Vec<String>` facade のままである。
- **Residual**: borrow-based `entry` / `iter` / `retain` /
  `FromIterator` parity は collection iterator 設計待ちのまま。

## 2026-10-30 — `std::collections::string::eq` の退役 (#917 / ADR-046, wave/string-eq-qualified)

- **Removed**: `std/collections/string.ark` の public `pub fn eq(a, b)` を
  退役した。replacement は `a.eq(b)`。qualified `string::eq(...)` は E0100 と
  `use \`a.eq(b)\` on the \`String\` receiver instead` の hint を返す。public
  surface は `impl String` / `impl Vec<String>` の receiver surface のみで、
  compiler が使う `__core_string_eq_impl` CoreOp fallback は非公開のまま残る。
  ただし下記 Residual の resolver-stability shim として private `fn eq` は残る。
- **Migration**: module_import fixture の qualified-call coverage を
  `std::io::read_stdin_line` / `std::host::stdio::{println}` へ移した。
  `data/core-ops.toml` から `string::eq` / `std::collections::string::eq` の
  legacy CoreOp alias を削除し、generated binding を再生成した（650 -> 648
  bindings）。`scripts/gen/ark_fnv_index.py` と registry/bindings generator が
  生成する bare `eq(...)` import / 呼び出しも `.eq()` に同期し、生成物と
  generator の drift を解消した。
- **Fixtures**: `module_import/use_{qualified_call_typed,std_string,destructure_multi,destructure_typed,prelude_coexist}`、
  selective import の `use_func_destructure`、check-only の
  `stdlib_inline/abs_probe`。E0100 hint の golden は
  `diagnostics/removed_string_eq_free`。`use_func_destructure.expected` は
  receiver 化後の出力 `true` に対し `1` のままだった既存 golden を `true` に
  直した。
- **Scorecard**: eradication scope 84 -> **83**、`pub fn` declarations 84 -> **83**、
  unregistered 2 -> **1**、`must delete` 67 -> **66**。`std::io::io_error_message`
  が最後の unregistered declaration として残る（`associated-or-method` / #693）。
- **Evidence**: 変更した 6 run fixture は pinned / current ×
  wasm32-gc（`--memory-width memory32` の P2 component）/ `--target wasm32`
  の全 24 legs が byte-exact PASS。`verify fixtures` **1929/1929**
  compile+validate PASS（111 runtime executed、38/38 `test:` PASS、2
  runtime-unsupported、0 FAIL）、`selfhost diag-parity` PASS（新規 golden
  含む）、`docs check` 6/6、`verify lane` PASS、ADR-029 fixpoint PASS
  （s2 == s3 = `35913e9d…`、pinned base `56ff0808…`）。
- **Residual**: 非公開の `fn eq` resolver-stability shim は残る。宣言ごと削除
  すると `stdlib_deque/deque_from_iter_eq` の generic `i32.eq` が
  `Deque::eq` に解決され、`wasm-tools validate` が type mismatch で落ちる
  （pinned / current の両 compiler で再現）。generic method candidate selection
  が symbol set に依存しなくなった時点で削除する。public surface ではないため
  scorecard は 83 / must delete 66 のまま。

## 2026-10-30 — prelude free `eq` の退役 (#917 / ADR-091, wave/eq-retire)

- **Removed**: `std/prelude.ark` の `eq(a, b)` と `std/manifest.toml` の
  prelude entry を削除した。replacement は `a.eq(b)`。bare call は E0100 と
  `use \`a.eq(b)\` on the \`String\` value instead` の hint を返す。
- **Migration**: compiler / std / tests の free `eq` call site 5,729 件を
  `.eq()` へ移した。codemod が付けた不要な grouping parens は postfix
  receiver から落とした（`(x).eq(y)` → `x.eq(y)`）。括弧付き receiver は
  `if` 文の `END` 直後の最初の式で composed receiver と認識されず、receiver の
  `LOCAL_GET` が落ちて unresolved-call stub / `i32::eq` になるため（詳細は
  [ADR-091](../adr/ADR-091-prelude-free-function-exception.md) 追記）。
- **Module split**: `std/core/cmp.ark` の `trait Eq` と impl を
  `std/core/eq.ark` へ分離し、`cmp` は `pub use std::core::eq` で再公開する。
  prelude は `use std::core::eq` を持つ。bootstrap overlay は
  `std/core/eq.ark` を ship し、free `eq` shim を削除した。埋め込み
  static-dispatch stub と target-contract generator も `.eq()` へ移した。
- **Fixtures**: `operators/string_eq`（gc / wasm32、32/32 行）と
  `stdlib_trait/eq_*`。qualified `string::eq` は module_import fixture 用に
  public のまま残し、`docs/data/stdlib-api-eradication.toml` で
  `must delete` / #917 に分類した。
- **Scorecard**: `docs/stdlib/eradication.md` の生成値で `prelude_wrapper`
  entries は 27 → **26**、`prelude frees (default)` は 5 → **4**、
  `associated-or-method` は 18 → **17**。
- **Evidence**: `verify fixtures` **1928/1928** compile+validate PASS
  （111 runtime smoke、38/38 `test:` PASS、2 runtime-unsupported、0 FAIL）、
  `verify lane` PASS、`docs check` 6/6、ADR-029 fixpoint PASS
  （s2 == s3 at sha256
  `f2b2159d70b3adad954ce9746e888b558f4607001a5ee1fae8ec4da7e16fac2c`、
  pinned base `56ff0808171dcc3c521b0e85d07f2b16a349a59419483ad71148a5ee928d23e8`）。

## 2026-10-30 — prelude `Box_new` / `unbox` の退役 (#917 / ADR-091, wave/box-new)

- **Removed**: `Box_new(x)` / `unbox(b)` を `std/prelude.ark` と
  `std/manifest.toml` から削除した。replacement は `Box::new(x)`（`use
  std::prelude::Box`）と `b.take()`。どちらも `Box<T>` enum と同じ prelude に
  おける `impl<T> Box<T>` の associated function / receiver method である。
- **Migration**: compiler / std / fixture の call site を移行した。bare call は
  E0100 と `use \`Box::new(value)\`` / `use \`boxed.take()\`` の hint を返す。
  ownership checker の free 名 special case を `Box::new` / `.take()` の
  E0227 sink に置き換え、borrowed `Box<T>` の規則を保った。
- **Root fix**: generic enum の associated constructor が return 型を mono key
  （`i32` / `String`）へ置換していた MIR signature 解決を、enum instantiation
  を保持する `enum:Wrap<i32>` で修正した。あわせて let annotation と call
  result が generic enum の type argument を落とさないようにし、generic ref
  payload の match が宣言側 `T` layout を読む問題を直した。詳細は
  [ADR-091](../adr/ADR-091-prelude-free-function-exception.md) 追記。
- **Fixtures**: `generic_enum/impl_constructor`（annotated / inferred /
  turbofish）と `ownership/box_methods`、E0100 golden
  `removed_prelude_free_box_new` / `removed_prelude_free_unbox` /
  `removed_prelude_import_box_new` / `removed_prelude_import_unbox` を追加。
  `box_methods` は wasm32-gc と wasm32 の両 target で compile+validate+実行
  PASS。
- **Evidence**: `verify fixtures` 1926/1926 PASS（111 runtime smoke、
  38/38 `test:` PASS）、`verify lane` PASS、`docs check` 6/6、ADR-029 fixpoint
  PASS（sha256 `ddf46a2ff3f9a758d4020f0f656667855e719217fc89ef0288f37e4d4dada166`）。

## 2026-10-30 — HashMap の容量境界と wide literal 引数の残件 (#917, wave/hashmap-remain)

- **E0200 boundary**: 容量・一括操作 group は引き続き flat `HashMap<i32, i32>`
  store 専用。String / fn pair では `with_capacity` を含む 9 名を型検査で拒否する。
  診断は呼び出された method、受信 pair、group 全名、その pair で利用できる
  `len` / `is_empty` / `insert` / `get` / `contains_key` / `remove` / `keys` /
  `values` / `entries` を列挙する。汎用 `HashMap<K, V>` の容量 surface は、
  non-flat store の compiler-managed bucket に対する per-pair の `drain` /
  `remove_entry` / `extend` lowering が必要なため先送りし、理由を module docs と
  `collections-hashmap-rust-diff.md` / migration guide に記録した。退役した 21
  free は i32 専用だったため capability の欠落はない。
- **Literal lowering**: method 引数の i64 temp を、direct call と同じ i64→i32
  narrowing allowlist で i32 parameter に合わせるようにした。memory32 component
  でも `m.insert(2147483648, v)` / `0 - 2147483648` が検証を通る。
  `hashmap_remove` / `hashmap_reserve` / `hashmap_try_reserve` も allowlist に
  加えた。memory64 の i64 parameter は従来どおり wide local を保つ。
- **Fixtures**: `stdlib_hashmap/insert_max_int` / `insert_negative_keys` を wide
  literal 表記へ戻し、`insert_wide_literal_arg`（key / get / contains_key /
  remove / reserve / try_reserve）を追加。E0200 の
  `hashmap_string_with_capacity*` を両 target で追加し、
  `hashmap_string_capacity*` の golden を新診断文へ更新した。
  `removed_hashmap_adapter*` は golden 変更なしで両 target の diag parity を確認。
  registry 3556 -> **3559**。
- **Evidence**: `verify fixtures` **1923/1923** compile+validate PASS（111
  runtime smoke、38/38 `test:` PASS、2 runtime-unsupported、0 FAIL）。変更した
  run fixture 3 件は wasm32-gc component と `--target wasm32` の両方で
  compile+validate+実行 PASS。diag parity **260/260** PASS。`verify lane` PASS。
  `docs check` 6/6。ADR-029 fixpoint PASS: s2 == s3 sha256
  `d5bddc8811b482e3dbc0474f111cbabb385b33f3e1281fd61488ba8a10adeead`
  （pinned base `471855ad9db035c1ada136852fc55e896b477cf35303be066f352d9d563a8895`）。

## 2026-10-29 — text free 10 件のうち 8 件を退役 (#917, wave/prelude-text2)

- **Basis**: [ADR-091](../adr/ADR-091-prelude-free-function-exception.md) §4 が
  保留した text 10 件のうち、receiver method の戻り型と lowering が bootstrap pin で
  安定した 8 件を、同じ 0.x 例外（E0100 + method hint）で退役した。ADR-091 に
  追記した。
- **Retired (8)**: `byte_at` / `slice` / `substring` / `to_string` /
  `starts_with` / `ends_with` / `contains` / `index_of` の manifest entry と
  `std/prelude.ark` 宣言を削除した。`std::collections::string` に残っていた
  同名の module-scoped `pub fn`（`slice` / `starts_with` / `ends_with` /
  `contains` / `index_of` / `byte_at`）も削除し、二重の free 綴りを残さない。
- **Replacement**: `s.byte_at(i)`、`s.slice(start, end)`（`substring` も同じ
  `string.slice` CoreOp）、`x.to_string()`、`s.starts_with(prefix)` /
  `s.ends_with(suffix)` / `s.contains(sub)` / `s.index_of(sub)`。
- **Compiler migration**: compiler / std / fixture の 541 ファイル、
  byte_at 1,025・substring 901・starts_with 1,598・to_string 513・
  ends_with 269・contains 231・index_of 123・slice 76 の call site を
  receiver method へ移した。bootstrap overlay は `std::collections::string` を
  ship しないため、`.index_of(...)` の戻り値が Copy と推論されるよう
  `p2_struct_name_from_type` に `let first: i32` の注釈を置いた。
- **Diagnostics**: `resolver::removed_names` に 8 件の hint を追加し、
  `diagnostics/removed_prelude_text_free*` の E0100 golden を追加した。
- **Scorecard**: eradication scope 95 → **87**; manifest public entries
  94 → **86**; `pub fn` declarations 103 → **89**; `prelude_wrapper` entries
  35 → **28**; `prelude frees (default)` 16 → **8**; `associated-or-method`
  29 → **21**; `must delete` 66、unregistered 1、intrinsic-only 1 は不変。
- **Deferred (4)**: `clone`（typechecker の polymorphic copy 経路で、
  `Clone` impl を持たない Vec / struct / enum の唯一の clone）、`eq`
  （bootstrap overlay に `impl Eq for String` がなく、free `eq` を消すと
  String `==` が `i32.eq` へ落ちて invalid core Wasm になる）、
  `Box_new` / `unbox`（generic enum payload の associated/method lowering と
  ownership checker の名前 boundary が未解決）。3 family は §4 の
  `associated-or-method` に残す。
- **Evidence**: `verify fixtures` **1918/1918** compile+validate PASS（110 runtime
  smoke、38/38 `test:` PASS、2 runtime-unsupported、0 FAIL）、`verify lane` PASS、
  `docs check` 6/6、ADR-029 fixpoint PASS（s2 == s3 at sha256
  `4c2caf00ac0ad9a0f79cd4ff5a921b01d07248c41fd983d6dafaf750ae457f05`、pinned base
  `471855ad9db035c1ada136852fc55e896b477cf35303be066f352d9d563a8895`）。

## 2026-10-29 — prelude 保留 23 件のうち 11 件を退役 (#917, wave/prelude-text)

- **Basis**: [ADR-091](../adr/ADR-091-prelude-free-function-exception.md) §4 が
  保留した 23 件のうち、replacement の lowering が安定した 11 件を同じ 0.x 例外
  （E0100 + method / associated hint）で退役した。ADR-091 に追記した。
- **Root cause (`parse_*`)**: 先行 wave で `parse_*` を `TryFrom` へ移すと再構築 s2 が
  任意の入力で trap した原因は、bootstrap flat overlay の stdlib closure が
  `std/core/convert.ark` を ship しておらず、compiler 自身の `f64::try_from` が
  `drop; unreachable` の unresolved-call fallback に落ちていたこと。overlay に
  convert を追加し、import なしの `try_from` は E0100 にした。
- **Retired (11)**: `push_char`、`unwrap` / `unwrap_or` / `is_some` / `is_none` /
  `is_ok` / `is_err`、`parse_i32` / `parse_i64` / `parse_f64`、`f32_to_string` の
  manifest entry と `std/prelude.ark` 宣言を削除した。
- **Replacement**: `s.push_char(c)`（mutating-receiver の grow 結果を caller local /
  field へ書き戻す）、`o.unwrap()` / `o.unwrap_or(fallback)` と
  `r.unwrap()` / `r.unwrap_or(fallback)`（prelude の concrete
  `impl Option<i32>` / `impl Result<i32, i32>`）、`o.is_some()` / `o.is_none()` /
  `r.is_ok()` / `r.is_err()`、`i32::try_from(s)` / `i64::try_from(s)` /
  `f64::try_from(s)`（`use std::core::convert`）、`x.to_string()`（
  `impl f32 { to_string }` と typechecker の f32 mangling）。
- **Resolution**: call result の `option:<T>` 型を method 解決時に `Option_<T>` の
  impl owner へ橋渡しし、`pop(v).unwrap()` のような receiver でも解決するように
  した。compiler 自身の `pop(...).unwrap()` 5 箇所は `match` へ移した（pin が
  bridge を持たないため fixpoint の s2 / s3 一致を保つ）。
- **Diagnostics**: `resolver::removed_names` に 11 件の hint を追加し、
  `diagnostics/removed_prelude_push_char*` / `removed_prelude_try_from*` の
  E0100 golden を追加した。
- **Scorecard**: eradication scope 149 → **138**; manifest public entries
  147 → **136**; `pub fn` declarations 157 → **146**; `prelude_wrapper` entries
  40 → **35**; `prelude frees (default)` 27 → **16**; `associated-or-method`
  83 → **72**; `must delete` 66、unregistered 2、intrinsic-only 1 は不変。
- **Deferred (12)**: `Box_new` / `unbox` と text 10 件（`clone` / `eq` /
  `byte_at` / `substring` / `to_string` / `starts_with` / `ends_with` /
  `contains` / `index_of` / `slice`）は `associated-or-method` のまま残す。
- **Evidence**: `verify fixtures` **1914/1914** compile+validate PASS（108 runtime
  smoke、38/38 `test:` PASS、2 runtime-unsupported、0 FAIL）、`verify lane` PASS、
  `docs check` 6/6、ADR-029 fixpoint PASS（s2 == s3 at sha256
  `ffeec82861363baccba2a8a3084a6047723336bce1341b08af975852b32fba18`、pinned base
  `471855ad9db035c1ada136852fc55e896b477cf35303be066f352d9d563a8895`）。

## 2026-10-29 — prelude free 27 件の退役と安定 free 4 件の限定例外 (#917, wave/prelude-remain)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D1（trait / method /
  associated を正規形とする）と D2（例外は個別 ADR）、
  [ADR-091](../adr/ADR-091-prelude-free-function-exception.md)。ADR-091 は
  [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) D2 の 0.x 例外の限定版として、
  対象 27 件を W0009 ではなく E0100 + 具体的な method hint で削除する。
- **Retired (27)**: prelude の `abs` / `args` / `byte_len` / `clamp_i32` / `exit` /
  `format_bool` / `format_i32` / `gcd` / `is_empty` / `join` / `lines` / `max` /
  `min` / `pad_left` / `pad_right` / `repeat` / `replace` / `split` /
  `sqrt` / `to_lower` / `to_lowercase` / `to_upper` / `to_uppercase` / `trim` /
  `trim_end` / `trim_start` / `values` の manifest entry と
  `std/prelude.ark` 宣言を削除した。`values` は identity helper であり
  replacement を持たない。他の 26 件は receiver method / associated function
  へ移行する。
- **Underlying module frees (13)**: `std/collections/string.ark` の同名
  `pub fn`（`to_lower` / `to_upper` / `trim` / `trim_start` / `trim_end` /
  `repeat` / `pad_left` / `pad_right` / `replace` / `split` / `join` / `lines` /
  `push_char`）を非公開または削除した。`push_char` の prelude free は残すため、
  string module 側の重複宣言だけを削除した。他の 12 名は private 実装とし、
  公開綴りは `String` / `Vec<String>` method だけになる。
- **Replacement**: 既存の `s.len_bytes()` / `s.is_empty()` / `s.trim()` /
  `s.split(sep)` / `parts.join(sep)`、`i32` / `f64` の `abs` / `min` /
  `max` / `clamp` / `gcd` / `sqrt`、`Env::args()` / `Process::exit()`、
  primitive `to_string()` を使う。全 27 件の before/after は
  [migration-guidance.md](migration-guidance.md#prelude-free-function-retirement-adr-091)
  に記録した。旧名は E0100 と `resolver::removed_names` の hint で失敗する。
  `push_char` と `parse_i32` / `parse_i64` / `parse_f64` は free のまま残す
  （下記 Deferred）。
- **Retained exception (4)**: ADR-072 §1 の安定 Prelude 関数 `assert` /
  `String_from` / `String_new` / `char_to_string` は prelude free のまま残す。
  `String::from` は未提供で、`String::new` は現行 pin で invalid core Wasm になり、
  `char_to_string` は i32 codepoint 境界を持つ。ADR-091 §3 の正確な例外リストであり、
  他の prelude free には適用しない。
- **Deferred (23)**: `push_char`、`parse_i32` / `parse_i64` / `parse_f64`、
  `Box_new` / `unbox`、`unwrap` / `unwrap_or` / `is_some` / `is_none` /
  `is_ok` / `is_err`、`f32_to_string`、および `clone` / `eq` / `byte_at` /
  `substring` / `to_string` / `starts_with` / `ends_with` / `contains` /
  `index_of` / `slice` は `associated-or-method` のまま残す。
  `s.push_char(c)` は compile・validate するが、String が GC ref で method body の
  新しい array が caller binding に伝播しない（この wave で測定）。`parse_*` を
  `TryFrom` へ移すと、再構築した s2 が任意の入力を compile する際に `unreachable`
  で trap した（`compile_source` / `backend_resolve` の計測）。generic な
  `impl<T> Option<T>` の method も実行時 `unreachable` へ落ちる。
- **Scorecard**: eradication scope 195 → **168**; `associated-or-method`
  129 → **102**; manifest public entries 193 → **166**; `pub fn` declarations
  216 → **176**; `prelude_wrapper` entries 64 → **40**; monomorphic helpers
  50 → **47**; unregistered declarations 2、`must delete` 66、
  `intrinsic-only` 1 は不変。`prelude frees (default)` family は 54 → **27**。
- **Evidence**: `verify fixtures` **1910/1910** compile+validate PASS（110 runtime
  smoke、36/36 `test:` PASS、2 runtime-unsupported、0 FAIL）、`verify lane` PASS、
  `docs check` 6/6、ADR-029 fixpoint PASS（s2 == s3 at sha256
  `304624037927d797311cd09cc42bb26cb85fc08ee59a0da14586cbe9c0c2315e`、
  pinned base `471855ad9db035c1ada136852fc55e896b477cf35303be066f352d9d563a8895`）。

## 2026-10-29 — `std::seq` の provisional 汎用アルゴリズム削除 (#917, wave/associated-next)

- **Basis**: [ADR-014](../adr/ADR-014-stability-labels.md) の provisional 削除規則
  （changelog と migration guide の告知。ADR-087 の stable window と W0009 は不要）。
  対象は `provisional` で、`remove_in` の対象ではない。
- **Retired (8)**: `binary_search_by` / `binary_search_by_key` / `lower_bound` /
  `upper_bound` / `min_by` / `max_by` / `min_by_key` / `max_by_key` の manifest
  entry と `std/seq/mod.ark` の公開宣言を削除した。qualified な旧 call は E0100 と
  `resolver::removed_names` の method hint（`use std::collections::vec`）を返す。
- **Replacement**: `Vec<T: Ord>` の receiver method
  `v.binary_search(target)` / `v.binary_search_by_key(target_key, key)` /
  `v.lower_bound(target)` / `v.upper_bound(target)` / `v.min()` / `v.max()` /
  `v.min_by_key(key)` / `v.max_by_key(key)`。`min` / `max` 系は空 vector で
  従来同様に trap する。
- **Kept**: `sort_by` は `provisional` のまま残す。method 置換の `v.sort()` /
  `v.sort_by(cmp)` は i32 / String / gc component 経路では正しいが、`Vec<i64>` の
  5 要素以上で linear / core-emit 経路が誤った値を生成する（#965）。scratch
  layout が直るまで allocation-free な free を到達可能な generic `Ord` sort とする。
  13 の deprecated i32 wrapper も #703 の Wave 2 まで callable のままで、
  そのうち `binary_search` / `min_i32` / `max_i32` は新しい method へ委譲する。
- **Scorecard**: public surface 203 → **195**; manifest public entries 201 →
  **193**; `pub fn` declarations 224 → **216**; `associated-or-method` 137 →
  **129**; `must delete` 66、unregistered 2、intrinsic 1 は不変。
- **Evidence**: 詳細は issue #917 の `wave/associated-next` landed slice を参照。

## 2026-10-29 — `hash::hashmap_*` の Wave 2 削除 (#917, wave/hashmap-surface)

- **Basis**: [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) D2 の
  W0009 / migration guide / changelog の 3 条件。Wave 1 の deprecation は同じ
  branch の `50da61673`。
- **Retired (21)**: `hash::hashmap_new` / `get` / `get_option` / `contains` /
  `set` / `insert` / `size` / `capacity` / `with_capacity` / `reserve` /
  `try_reserve` / `shrink_to_fit` / `is_empty` / `clear` / `keys` / `values` /
  `entries` / `drain` / `remove` / `remove_entry` / `extend` の manifest entry と
  公開宣言を削除した。宣言は `std/collections/hash.ark` に非公開の MIR lowering
  target として残るため、旧 qualified call は
  `resolver::removed_private_member_hint` 経由で E0100 と method surface の hint
  を返す。`hashmap_entry_pairs` も非公開化した。
- **Replacement**: `HashMap::new<i32, i32>()` /
  `HashMap::with_capacity<i32, i32>(n)` と `m.get(k)` / `m.insert(k, v)` /
  `m.len()` / `m.keys()` / `m.values()` / `m.entries()` / `m.remove(k)` /
  `m.capacity()` / `m.reserve(n)` / `m.try_reserve(n)` / `m.shrink_to_fit()` /
  `m.clear()` / `m.drain()` / `m.remove_entry(k)` / `m.extend(other)`。flat
  free の `Vec<i32>` 契約は method では `Option` / typed pair に変わる。
- **E0200**: 容量・一括操作の 9 メソッドを String / fn pair で呼ぶと型検査エラー。
  hidden bucket array を持つ store を i32 helper で処理しないための明示的な
  capability 境界で、`HashSet<String>` の capacity と同じ扱い。
- **Evidence**: `stdlib_hashmap/` の 58 fixture は Wave 1 で method surface へ
  移行済み。`removed_hashmap_free*`（両 target、E0100）と
  `hashmap_string_capacity*`（両 target、E0200）を追加し、W0009 の
  `deprecated_hashmap_free*` を削除した。registry 3495 -> 3497。
- **Scorecard**: eradication scope 225 -> **203**、`associated-or-method`
  159 -> **137**、manifest 公開 entry 222 -> **201**、`pub fn` 宣言
  246 -> **224**、unregistered 3 -> **2**、`std::collections frees` rule
  38+2 -> **17+1**。scoreboard は hash family 21 -> **0** APIs（stable 112、
  deprecated 75 に戻る）。

## 2026-10-29 — `HashMap<i32, i32>` の容量・一括操作メソッド面と `hash::hashmap_*` の deprecation (#917, wave/hashmap-surface)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) の free-function
  根絶と [ADR-086](../adr/ADR-086-linear-collection-adts.md) の型・メソッド面。
- **Surface (9)**: `HashMap<i32, i32>` に `with_capacity` / `capacity` / `reserve` /
  `try_reserve` / `shrink_to_fit` / `clear` / `drain` / `remove_entry` / `extend` を
  追加した。`drain` は `Vec<(i32, i32)>`、`remove_entry` は
  `Option<(i32, i32)>` を返す。`try_reserve` の `true` は「予約が完了した」を意味し、
  flat store には回復可能な allocation error channel がないことを doc に明記する。
- **Deprecated (21)**: `hash::hashmap_*` の flat-store free を `stable` から
  `deprecated` へ移し、W0009 の qualified-call table と
  `diagnostics/deprecated_hashmap_free*` の両 target golden を追加した。
  `deprecated_by` は `HashMap::` の method / associated function を指す。
- **Contract**: `get` は sentinel `0` ではなく `Option<i32>`、`set` は `bool` では
  なく前の `Option<i32>` を返す `insert`、`entries` / `drain` は typed pair、
  `remove_entry` は `Option<(i32, i32)>` になる。W0009 のメッセージにもこの差分を
  記載する。
- **E0200**: 容量・一括操作の 9 メソッドは `HashMap<i32, i32>` 専用。String / fn
  値の store は bucket array を非公開の lowering 詳細として持つため、他の pair では
  `E0200` で reject し、i32 helper を誤った layout に適用しない。
- **Evidence**: `tests/fixtures/stdlib_hashmap/` の 58 fixture を method surface へ
  移行し、deprecated diag 2 件を registry に追加した。Wave 2 の削除は次の entry で
  記録する。

## 2026-10-29 — `Vec<T: Ord>` の i64 / String sort が wasm32-gc で動作 (#965)

- **Basis**: #917 / [ADR-046](../adr/ADR-046-free-function-eradication.md) で追加した
  `Vec<T: Ord>` の `sort` / `sort_unstable` / `is_sorted` / `select_nth`。
- **Fixed**: monomorphized impl method の signature registry が宣言時の `vec:T` を
  そのまま登録し、callee の receiver が `Vec<i32>` layout になっていた。generic body
  内の `Vec::with_capacity<T>` / `Vec::new<T>` も AST の型名 `T` を callee 名へ
  埋め込んで enclosing instance の型束縛を無視していた。両方とも enclosing mono
  instance の型引数で具体化する。
- **Evidence**: `tests/fixtures/stdlib_vec/vec_sort_ord_i64.ark` /
  `vec_sort_ord_string.ark` と各 `_wasm32` 版が wasm32-gc component と
  `--target wasm32` core の両方で compile・validate・execute して同一 stdout を
  返す。cross-backend parity 2/2。`T: Eq` bound は `std::core::cmp` の import 下で
  `impl Eq for i32` に解決する。

## 2026-10-29 — `Std::panic` と Vec no-argument sort (#917, wave/sort-panic)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) の free-function
  根絶と [ADR-090](../adr/ADR-090-std-namespace-and-panic-association.md)。ADR-090 は
  [ADR-072](../adr/ADR-072-language-v2-prelude-and-keyword-policy.md) §1 の安定集合を
  `Std` 型の追加と `panic` の associated 化へ改訂する。
- **Retired (5)**: `std::collections::sort` の `sort` / `sort_unstable` /
  `is_sorted` / `select_nth` と、prelude の `panic`。
- **Replacement**: `v.sort()` / `v.sort_unstable()` / `v.is_sorted()` /
  `v.select_nth(k)`（`impl<T: Ord> Vec<T>`）と、`Std::panic(message: String)`。
  `Std` は prelude の安定型として auto-import される。comparator を受ける
  `v.sort_by(cmp)` 系は `f64` のような非 `Ord` 要素型のために残る。
- **Lifecycle**: `panic` は `stable` のため [ADR-087](../adr/ADR-087-0x-deprecation-exception.md)
  D2 の 0.x 例外の対象だが、W0009 の代わりに E0100（compile error + replacement
  hint）で移行させる限定例外を ADR-090 §3 に記録した。対象は `panic` 1 件のみで、
  他の stable entry の削除条件は変更しない。
- **E0100**: 旧 `panic(...)` は `Std::panic(message)`、旧
  `sort(v)` / `sort_unstable(v)` / `is_sorted(v)` / `select_nth(v, k)` は
  `resolver::removed_names` が `v.sort()` 系の method 名を案内する。
- **Evidence**: `tests/fixtures/stdlib_vec/vec_sort_noarg.ark` が 4 method を
  wasm32-gc component と `--target wasm32` の両方で compile・validate・execute する。
  std / fixtures の panic call site 921 行を `Std::panic` へ移行し、両 target の
  panic fixture で stderr と trap を確認する。`Vec<String>` の method 解決は
  typechecker の bare `Vec` owner fallback で修正した（#966）。
- **Scorecard**: eradication scope 296 → **291**、`associated-or-method`
  230 → **225**、manifest 公開 entry 271 → **270**、`pub fn` 宣言 323 → **318**、
  unregistered 25 → **21**、`prelude_wrapper` 65 → **64**。`must delete` 66、
  `intrinsic-only` 1 は不変。

## 2026-10-29 — compiler collections type surface (#917, wave/associated-remain)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) の
  free-function 根絶。対象は `std::collections::compiler` の **14** free
  functions（`arena_*` 4、`slotmap_*` 6、`interner_*` 4）。全 entry が
  `experimental` なので、[ADR-014](../adr/ADR-014-stability-labels.md) D4 に
  より deprecation window なしで同時削除した（W0009 は対象外）。
- **Replacement**: `Arena::new()` / `a.alloc(value)` / `a.get(idx)` /
  `a.len()`、`SlotMap::new()` / `m.insert(value)` / `m.get(key)` /
  `m.remove(key)` / `m.contains(key)` / `m.len()`、`Interner::new()` /
  `i.intern(value)` / `i.lookup(sym)` / `i.len()`。backing store は各型の
  private field に閉じ、`HandleTableI32` も `SlotMap` を保持する。
- **E0100**: 旧 `arena_*` / `slotmap_*` / `interner_*` 名は
  `resolver::removed_names::removed_compiler_member_hint` が
  `compiler` / `collections::compiler` / `std::collections::compiler` の
  各綴りで新しい associated / method 名を案内する。
- **Evidence**: `stdlib_collections_compiler/{arena,slotmap,slotmap_remove,interner}_basic`
  と `stdlib_compiler/compiler_basic` を method surface へ移行し、
  wasm32-gc component と `--target wasm32` の両方で compile・validate・
  execute した。`diagnostics/removed_compiler_{arena,slotmap,interner}_free*`
  は両 target の E0100 golden として追加した。
- **Scorecard**: eradication scope 310 → **296**、`associated-or-method`
  244 → **230**、manifest 公開 entry 285 → **271**、`pub fn` 宣言
  337 → **323**。unregistered 25、`prelude_wrapper` 65 は不変。

## 2026-10-28 — `__intrinsic_arg_at` indexes real argv on wasm32-gc (#807, ADR-046)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) retired
  the `std::host::env` free functions behind `Env`; the `wave/env-args` slice
  recorded a residual — `Env::arg_at` indexed the `Env::args` vector because
  the direct `__intrinsic_arg_at` lowering was believed to be a constant
  `None` on `wasm32-gc`.
- **Root cause**: the direct intrinsic had two gaps.
  `__intrinsic_arg_at` was never registered in
  `resolver/builtins_intrinsics.ark`, so a direct call failed with **E0100**
  before reaching the emitter, and the `wasm32-gc` branch in
  `wasm/runtime_abi_env_args.ark` was the stopgap `emit_env_arg_at_gc_none`,
  which dropped the index operand and returned the `Option::None` variant.
  The `wasm32` linear emitter (`wasm/runtime_abi_env_arg_at_body.ark`) was
  already complete.
- **Emitter**: `wasm/runtime_abi_env_args_gc.ark` adds
  `emit_env_arg_at_gc`. It fetches the canonical argv
  (`wasi:cli/environment@0.2` `get-arguments` on wasi-p2,
  `args_sizes_get` / `args_get` on Preview 1), bounds-checks
  `0 <= index < argc - 1`, reads element `index + 1` because argv[0] holds
  the program name, copies the `(ptr, len)` payload into a GC string, and
  stores `Option::Some(string)` with discriminant 0; out-of-range and
  negative indices keep the existing `Option::None` variant (discriminant 1).
  The index is consumed from the MIR call operands, like the linear emitter.
- **Surface**: `__intrinsic_arg_at` is registered as a private intrinsic
  (ADR-044 / ADR-046 exception) and `Env::arg_at` calls it directly;
  `Env::args` and the other `Env` methods are unchanged. The host intrinsic
  GC adapter body for `arg_at` reuses the same emitter, so the deferred stub
  path also returns real arguments.
- **Fixtures**: `tests/fixtures/wasi_p2_native/arg_at.ark` (+ `.expected`,
  `.flags`) prints `arg_count`, two real arguments, an out-of-range index,
  and a negative index. `gate-668-p2-native-polish` runs the wasm32-gc P2
  component, and `gate-668-p2-args-env` runs the same source as a P2
  component and as a `wasm32` Preview 1 core with the same `alpha beta`
  argv, requiring the shared golden.
- **Verification**: `arg_at.expected` stdout on both targets
  (`arg_count=2`, `arg0=alpha`, `arg1=beta`, `arg2=none`, `arg_neg=none`);
  `gate-668-p2-args-env` PASS; `gate-668-p2-native-polish` PASS;
  `verify fixtures` **1913/1913 PASS**, 0 FAIL (110 runtime smoke, 2
  runtime-unsupported, 1 `test:` fixture PASS); `docs check` 6/6;
  `verify lane` PASS; `selfhost build-compiler` 2169.6 s; ADR-029 fixpoint
  PASS at s2 == s3 sha256
  `1aa4077a29a8f224315b3778aa01b3685cffee23d723e34b4caa82617683aa7e`
  (pinned base `8df2698c…`; s3 build 32m16s).
- **Residual**: each `Env::arg_at` call re-fetches the argument list through
  the canonical ABI, the same lifetime behavior as `Env::args`; caching the
  argv vector remains follow-up work. The `std::cli` free functions still
  index `Env::args`.

## 2026-10-28 — wrapping numeric conversion CoreOps bind the i32→u16/i16 and u64→u32 shims (#917, ADR-042, ADR-046, ADR-067, ADR-087)

- **Basis**: [ADR-042](../adr/ADR-042-intrinsic-layer-separation.md) CoreOp
  dispatch, [ADR-067](../adr/ADR-067-language-v2-numeric-conversion.md)
  canonical scalar conversion methods, and
  [ADR-046](../adr/ADR-046-free-function-eradication.md) free-function
  eradication. This closes the three numeric residuals recorded by the
  preceding `i8::to_i32` entry.
- **Root cause**: the prelude wrappers `i32_to_u16`, `i32_to_i16`, and
  `u64_to_u32` were listed as unbound intrinsic shims in
  `mir/lower/stdlib_body_defer.ark`. Their call sites were deferred from body
  lowering, but no `data/core-ops.toml` operation, binding alias, or emitter
  handler existed for them, so Wasm emission fell through to the unresolved
  fallback. WAT for `i32::to_u16_wrapping` was
  `local.get $self; drop; unreachable; local.get $_t1; return`; the free call
  sites and the wasm32 linear target showed the same `drop; unreachable`.
- **CoreOps**: `data/core-ops.toml` registers `runtime.i32_to_u16`
  (`i32.and 0xFFFF`), `runtime.i32_to_i16` (`i32/i64.extend16_s` by Memory64),
  and `runtime.u64_to_u32` (`i32.wrap_i64`, zero-extended when the MIR i32
  result slot is i64 under Memory64), each with a `legacy_bindings` alias for
  the prelude name. Handlers live in `wasm/intrinsic_cast.ark` behind
  `wasm/call_runtime_convert.ark`; the intrinsic-name fallback also resolves
  the three handlers. `native_c/core_call.ark` and
  `data/native-cpp-capabilities.toml` mark them supported so the native-c
  scaffold does not regress.
- **Surface**: the `impl i32 { to_u16_wrapping / to_i16_wrapping }` and
  `impl u64 { to_u32_wrapping }` methods already existed from the stable-64
  wave; they now execute instead of trapping. The free wrappers remain
  `deprecated` with W0009 coverage and now lower through the same CoreOps.
- **Fixtures**: `scalar/numeric_conversion_methods.ark` now covers `300`,
  `-1`, and `40000` i32 receivers plus `2^32` and `2^32 + 5` u64 receivers
  through both the methods and the deprecated frees. The fixture gate
  compiles and validates it on wasm32-gc; `check-cross-backend-parity.py`
  executes it on the wasm32-gc component path and the wasm32 linear path, and
  a standalone `--target wasm32-gc` Memory64 core compile validates.
- **Residual**: none for these three conversions. Unannotated narrow literal
  bindings (`let x = 5i8`) still carry no local type name, and i8 enum/tuple
  payload bindings still have independent typing gaps; those are unchanged.

## 2026-10-28 — `i8::to_i32` and the annotated-i8 MIR typing fix (#917, ADR-046, ADR-067, ADR-087)

- **Basis**: [ADR-067](../adr/ADR-067-language-v2-numeric-conversion.md)
  canonical scalar conversion methods and
  [ADR-046](../adr/ADR-046-free-function-eradication.md) free-function
  eradication. `i8_to_i32` was the one numeric deferral from the ADR-087
  stable-64 wave.
- **Compiler fix**: `corehir/type_ann_local_name.ark` now keeps `i8` in the
  annotated-let scalar type-name set. Without it, `let x: i8 = -5` produced a
  receiver local with no type name; MIR method resolution fell back to the
  bare `to_i32` callee, and the unresolved call emitted `drop; unreachable`
  even though `impl i8` existed. The same scalar-name classification now
  recognizes `i8` for struct fields, `Vec<i8>` elements, call-result typing,
  and print arguments; i8 enum and tuple payload bindings still have
  independent typing gaps.
- **Memory64**: `wasm/intrinsic_cast.ark` sign-extends `i8`/`i16` results with
  `i64.extend8_s` / `i64.extend16_s` when the scalar width is Memory64; the
  i32-only op left an i32 on an i64 result slot.
- **Surface**: `std/core/convert.ark` adds
  `impl i8 { fn to_i32(self: i8) -> i32 }`. `i8_to_i32` moves to
  `deprecated` (`deprecated_by = "i8::to_i32"`) with W0009 coverage, so all
  17 numeric conversions now have their canonical method.
- **Fixtures**: `scalar/numeric_conversion_methods.ark` covers `-5i8`,
  `-128i8`, and `127i8` receivers. The run fixture executes on the wasm32-gc
  component path and the wasm32 linear path via
  `check-cross-backend-parity.py`; a standalone `--target wasm32-gc`
  Memory64 core compile also validates.
- **Residual**: `i32::to_u16_wrapping`, `i32::to_i16_wrapping`, and
  `u64::to_u32_wrapping` still trap. Their intrinsic handlers are unbound
  prelude shims in `mir/lower/stdlib_body_defer.ark`, a separate Wave-2
  blocker. Unannotated narrow literal bindings (`let x = 5i8`) still carry no
  local type name; that affects `u8`/`i16` as well and is unchanged. An i8
  enum or tuple payload binding still hits a scalar/ref signature mismatch,
  which is separate from the annotated-let receiver path fixed here.

## 2026-10-28 — env frees retired behind `Env` and the MIR env call-result fix (#917, ADR-046, ADR-087)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2. The
  preceding env slices retired `std::env::get_var` and shipped the `Env`
  namespace (`Env::var` / `Env::arg_count` / `Env::arg_at` /
  `Env::var_or_default`), but left the free functions and `std::env` callable
  because a direct `__intrinsic_args` call inside a namespace method lowered
  its MIR result as a scalar. This wave fixes that lowering and retires the
  free surface.
- **Compiler fix**: `mir/lower/call_type_fallback_stdlib.ark` now types the env
  intrinsic call results like the fs file intrinsics: `__intrinsic_args` ->
  `vec:String`, `__runtime_abi_env_var` / `__intrinsic_arg_at` ->
  `Option_String`. `mir/lower/call_type_vec.ark` also marks an `Env::args`
  call site as `vec:String`, because a namespace method has no registered
  free-call spine entry to copy the layout from. The method frame no longer
  stores a Vec reference into an i64 local, and
  `wasm-tools validate --features gc,function-references,memory64` accepts the
  module on both targets.
- **Surface**: `std::host::env::Env` (`provisional`) now exposes
  `Env::args`, `Env::var`, `Env::arg_count`, `Env::arg_at`,
  `Env::var_or_default`, and `Env::has_flag`. The temporary `Option<String>`
  bindings in `Env::var` / `Env::arg_at` are gone. `Env::arg_at` indexes the
  vector from `Env::args`, so it returns real arguments on `wasm32-gc` (the
  direct `__intrinsic_arg_at` lowering still returns `None` there).
- **Removed (11 declarations, 10 manifest entries)**: the
  `std::host::env::{args, var, arg_count, arg_at, has_flag}` manifest entries
  plus the unregistered `var_or_default` declaration, and the whole legacy
  `std::env` module (`args`, `arg_count`, `arg_at`, `var`, `var_or_default`).
  `std::env` no longer resolves as a module path, so its W0002 entry is removed
  from `lint::deprecated_table`. Qualified old calls fail with **E0100**;
  `resolver::removed_names::removed_host_env_member_hint` names the matching
  `Env` method for the `env`, `host::env`, `std::env`, and `std::host::env`
  spellings, and the bare-name hints cover the retired members (the prelude
  `args()` binding is a separate entry and stays).
- **Bootstrap overlay**: the committed pin cannot lower `Env::args` (it has no
  env call-result typing and does not ship the namespace source), so
  `scripts/selfhost/checks.py::_patch_bootstrap_host_env_paths` rewrites the
  in-tree compiler's `Env::args` / `Env::arg_count` calls to the free builtin
  spelling, and `_patch_bootstrap_main_host_to_prelude` accepts the
  `Env::args()` main form. Checked-in source keeps the namespace; delete the
  patched spelling when the pin is refreshed (ADR-029).
- **Fixtures**: new dual-target `stdlib_host/env_args_surface*`
  (`wasm32-gc`/P2 component and `--target wasm32`) covers `Env::args`,
  `Env::var`, `Env::arg_count`, and `Env::has_flag`. The four free-function
  `stdlib_env/*` fixtures and their `t3-*` entries were retired with the
  surface; the call sites in `stdlib_host`, `stdlib_cli`, `host`,
  `wasi_p2_native`, `native_cpp_*`, and `lsp_perf` moved to `Env::`, and the
  cookbook/recipe manifest point at the new fixture.
- **Scorecard**: eradication scope 320 -> **310** (`pub fn` declarations
  348 -> **337**, manifest public entries 295 -> **285**); the `host/core
  namespace frees` family drops 254 -> **244**; `must delete` stays 66,
  `prelude_wrapper` stays 65, monomorphic helpers stay 50, and unregistered
  declarations stay 25. The fixture registry moves 3379 -> **3375** (four
  `run` and three `t3-*` entries removed, two `run` entries added).
- **Verification**: both targets compile, validate, and run
  `stdlib_host/env_args_surface*` with the same golden (`0`, `0`,
  `env-var-unset`, `env-args-surface-ok`); `wasm-tools validate --features
  gc,function-references,memory64` accepts the `wasm32-gc` core and plain
  `wasm-tools validate` accepts the `--target wasm32` core. A P2 component
  probe with a real argument reports `arg_count=1`, `has_flag=true`, and
  `arg_at(0)` = the argument; `gate-668-p2-args-env` PASS (CLI args and
  `--env` values through `Env`). `verify fixtures` **1904/1904 PASS**, 0 FAIL
  (109 runtime smoke, 2 runtime-unsupported, 1 `test:` fixture PASS);
  `docs check` 6/6; `verify lane` PASS; `check-stdlib-eradication` (286
  manifest functions, 25 unregistered), `check-compat-orphans`,
  `check-comment-policy`, `check-ark-code-quality`, `check-semantic-debt`,
  `check-legacy-dispatch-inventory`, `check-docs-consistency`,
  `check-docs-freshness`, and `check-deprecated-api-in-docs --strict` clean.
  ADR-029 fixpoint PASS: s2 == s3 at sha256
  `aa39ccd602d9b98cd87ec72ee7344ac1cbd9c2d69527ac125508651ebc2e2e45`
  (pinned base `8df2698c…`); `build-compiler` produced the s2 in 1657.7 s.
- **Residual**: the overlay rewrite stays pin-refresh debt (ADR-029). The
  direct `__intrinsic_arg_at` lowering still returns `None` on `wasm32-gc`, so
  `Env::arg_at` indexes the `Env::args` vector. `std::cli` free functions and
  the prelude `args()` binding are outside this wave.

## 2026-10-28 — Typed stdio handles + raw stderr write (#693, ADR-085 D9)

- **Basis**: [ADR-085](../adr/ADR-085-io-trait-unification.md) D9 kept the
  tagged `writer_write` / `writer_write_str` pair callable only until
  `std::host::stdio` exposed byte-faithful typed `Write` handles. The compiler
  also lacked a raw stderr write: `__intrinsic_eprintln` always appends `\n`.
- **Added**: `std::io::Stdout` and `Stderr` (`experimental` zero-field
  handles) implement `Write`. `write(Vec<i32>)` emits the
  byte image unchanged and returns its exact length, `write_string(text)`
  emits UTF-8 bytes, and `flush` is synchronous. `Stderr::write` never appends
  a newline. Constructors are `Stdout::new()` / `Stderr::new()`.
- **Compiler**: `__intrinsic_eprint` is registered as the `runtime.eprint`
  CoreOp (`stdio_eprint`) and lowers to a raw stderr write on `wasm32-gc`/P2,
  `wasm32`/P1, and the native C runtime. It does not append a newline.
- **Compiler: zero-field GC structs**: the `struct.new`-adjacent failure was a
  named-type registration gap. `ctx_gc_struct` skipped `field_count == 0`, so a
  zero-sized handle lowered as the legacy byte-size struct shell
  (`SubF_GS_f0_i32`, type 2) while the caller's local had no layout entry and
  fell back to the first GC type (the `i8` array, type 0). Zero-field user
  structs now register like any other struct, the bare `GS` sig becomes final
  `SubF_GS`, and that sig parses back to a zero-field struct.
- **Removed (2)**: `std::io::writer_write` and `writer_write_str` (manifest
  entries and `std/io/mod.ark` declarations). Qualified old calls fail
  resolution with **E0100** hints that name
  `Stdout::new().write(...)` / `Stderr::new().write(...)`; the W0009 rows and
  the `deprecated_io_free_function` golden were retired with them.
- **Fixtures**: `stdlib_io_traits/typed_stdout_write` (byte image, string,
  empty write counts) and `stdlib_io_traits/typed_stderr_raw` (two stderr
  fragments plus counts; an appended newline would break the golden) compile,
  validate, and run on `wasm32-gc` memory32/P2 and `wasm32`/P1. The core
  lowering regression is `structs/empty_struct_handle` (construction, return,
  by-value parameter, and inherent method on a zero-field struct).
- **Scorecard**: eradication scope 323 -> **321** (`pub fn` declarations
  352 -> 350, manifest public entries 298 -> 296); `must delete` 69 -> **67**;
  monomorphic helpers 51 -> **50**; deprecated manifest functions 67 -> **65**.
  The fixture registry moves 3365 -> **3375** (nine run/t3 entries plus the
  two E0100 goldens replacing one W0009 golden).
- **Verification**: `verify fixtures` **1904/1904 PASS**, 0 FAIL (110 runtime
  smoke, 2 runtime-unsupported, 1 `test:` fixture PASS);
  `check-t3-wasm-validate` 585/585 PASS; `selfhost diag-parity` PASS;
  `docs check` 6/6; `verify lane` PASS; ADR-029 fixpoint reached
  (`sha256(arukellt-s2.wasm) == sha256(arukellt-s3.wasm)` =
  `84c5b987ce8b6b6cd0b88534e3e2d614fa327a8af1a6d93f7ab86da0c6b2e49d`).
  The new fixtures also compile and run on `--target wasm32`/P1 with the same
  goldens, and the P2 harness packages and runs them on `wasm32-gc`.
- **Residual (separate)**: `compile --emit component` still emits
  `record Name {}` for a *user-defined* zero-field struct, which the
  Component Model WIT parser rejects. The pinned compiler reproduces the same
  WIT, so this predates the lowering fix; core lowering and the official P2
  harness packaging are unaffected, and std-declared handles are not emitted
  as user WIT records.

## 2026-10-28 — ADR-087 Wave 2: stable host fs / clock frees (#917, ADR-046, ADR-087)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2 and the
  [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) 0.x deprecation window.
  The seven `stable` entries deprecated in the 2026-10-27 host fs / clock waves
  completed their **W0009** window (migration rows and changelog entries in
  place), so this Wave 2 step deletes the manifest entries, the `pub fn`
  declarations, and the W0009 rows. Qualified old calls now fail with
  **E0100**.
- **Removed (7)**: `std::host::fs::is_readable_file`;
  `std::fs::{read_string, write_string, exists, is_readable_file}`;
  `std::host::clock::{monotonic_now, now_ms}`.
- **Replacement**: `Fs::read_to_string` / `Fs::write_string` /
  `Fs::is_readable_file` from `use std::host::fs::Fs`, and
  `Clock::monotonic_now()` / `Clock::now_ms()` from
  `use std::host::clock::Clock`. `std::fs` stays as a deprecated facade that
  re-exports `Fs` and warns on import.
- **E0100**: `resolver::removed_names::removed_host_fs_member_hint` already
  named the `Fs` replacements; this wave adds
  `removed_host_clock_member_hint` for the `clock::`, `host::clock::`, and
  `std::host::clock::` spellings. The W0009 rows and their unit assertions were
  removed from `lint::deprecated_table`; tests now assert that no W0009 entry
  remains for the seven names.
- **Goldens**: `diagnostics/deprecated_host_fs_free*` and
  `diagnostics/deprecated_host_clock_free*` became
  `diagnostics/removed_host_fs_stable_free*` and
  `diagnostics/removed_host_clock_free*`, with E0100 expectations on
  `wasm32-gc` and `--target wasm32`.
- **Scorecard**: eradication scope 330 -> **323** (`pub fn` declarations
  359 -> 352, manifest public entries 305 -> 298); `must delete` 76 -> **69**;
  `deprecated module frees` 8 -> **1** (`std::env::get_var`); `deprecated`
  entries 74 -> **67**; `associated-or-method` stays 254, `prelude_wrapper`
  stays 65, and unregistered declarations stay 25. The fixture registry stays
  3365 (four goldens renamed, none added). The monomorphic-helper metric drops
  53 -> 51 only because `read_string` / `write_string` matched its
  `*_string` name suffix.
- **Verification**: `verify fixtures` **1901/1901 PASS**, 0 FAIL (110 runtime
  smoke, 2 runtime-unsupported, 1 `test:` fixture PASS); `selfhost diag-parity`
  PASS; `docs check` 6/6; `verify lane` PASS; the four E0100 goldens match the
  compiler output byte-for-byte on both targets.

## 2026-10-27 — std::host::clock `Clock` namespace (#917, ADR-046, ADR-087)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2 and the
  [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) 0.x deprecation window.
  The two `stable` host clock reads become `deprecated` with live **W0009**;
  they stay callable until the Wave 2 deletion.
- **Replacement**: the `Clock` namespace type in `std::host::clock`.
  `Clock::monotonic_now()` returns nanoseconds from the monotonic clock and
  `Clock::now_ms()` returns milliseconds since the Unix epoch. Import it with
  `use std::host::clock::Clock`; the WASI clock intrinsics stay behind the impl.
- **W0009**: `deprecated_host_clock_qualified_message` covers the `clock::`,
  `host::clock::`, and `std::host::clock::` qualified spellings with unit
  assertions. The clock frees are not deleted in this wave, so no E0100 hint
  applies yet.
- **Compiler**: the in-tree driver, wasm, and MIR timing call sites use
  `Clock::`. The committed pin predates loading the namespace source into its
  flat overlay and stubs driver timing, so
  `scripts/selfhost/checks.py::_patch_bootstrap_host_clock_paths` rewrites the
  overlay calls (including the i32 stub / keep-clock form). Checked-in source
  keeps the namespace; delete the adapter when the pin is refreshed (ADR-029).
- **Fixtures**: dual-target `stdlib_host/clock_namespace_surface*` compile,
  validate, and run the namespace on `wasm32-gc` and `--target wasm32`; the
  W0009 goldens are `diagnostics/deprecated_host_clock_free*`. The migrated
  `stdlib_host`, `stdlib_time`, `stdlib_io`, `deny_clock` and benchmark call
  sites use `Clock::`.
- **Scorecard**: eradication scope stays **330** symbols (`pub fn` declarations
  359, manifest public entries 305, unregistered 25); the `host/core` family
  drops 40 -> **38** because the two frees are reclassified under
  `deprecated module frees` (6 -> **8**). Stability moves `stable`
  178 → **176** and `deprecated` 72 → **74**; `must delete` 74 → **76** and
  `associated-or-method` 256 → **254**; the fixture registry grows
  3361 → **3365**.
- **Lowering probe**: the same namespace-method shape is blocked for
  `std::host::env` on `wasm32-gc`: `Env::args()` (Vec<String>) and
  `Env::var()` (Option<String>) fail validation because
  `__intrinsic_args` / `__runtime_abi_env_var` only lower at the registered
  free-call site. That family remains free and is reported as a residual.
- **Verification**: `verify fixtures` **1901/1901 PASS**, 0 FAIL (110 runtime
  smoke, 2 runtime-unsupported, 1 `test:` fixture PASS); `selfhost diag-parity`
  PASS; `docs check` 6/6; `verify lane` PASS; ADR-029 fixpoint PASS:
  s2 == s3 at sha256
  `802144555fc4a42ad43eff6ea6c227ac0e128bf9d6e7085a621691c7d48ac506`
  (pinned base `04d4446b…`).

## 2026-10-27 — std::json / std::toml document entry points and parse-step helpers (#917, ADR-046)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2 and the
  ADR-014 D4 exception for `experimental` entries. This wave closes the
  `std::json` / `std::toml` / `std::csv` associated-or-method family; no public
  free function remains in those modules.
- **Removed**: `json::parse`, `toml::toml_parse`, `toml::toml_stringify`, and
  the unregistered `JsonParseStep_ok` / `JsonParseStep_err` /
  `json_parse_step_value` / `json_parse_step_next` / `TomlValStep_ok` /
  `TomlValStep_err` / `TomlKeyStep_ok` / `TomlKeyStep_err`.
- **Replacement**: `JsonValue::parse(doc)` / `v.to_text()`,
  `TomlValue::parse(doc)` / `v.to_text()`, `JsonParseStep::ok` / `err` with
  `step.value()` / `step.next()`, and the `TomlValStep` / `TomlKeyStep`
  associated constructors. Qualified calls get E0100 hints from
  `resolver::removed_names::removed_json_member_hint` /
  `removed_toml_member_hint`.
- **Compiler fix**: direct `TomlValue::Integer(42)` construction stored an
  unannotated `i32` literal into the declared `i64` payload
  (`struct.set` type mismatch on wasm32-gc). `emit_gc_variant_payload_fields`
  now widens an `i32` payload when the resolved variant layout declares
  `i64`; parsed TOML integers already used i64 and were unaffected. The
  unannotated associated `Result` match now infers its payload under the
  rebuilt s2.
- **Name-collision fix**: the JSON and TOML parsers each declared private
  `parse_document` / `stringify_value` / `stringify_array`. A non-main module
  using both `JsonValue::to_text` and `TomlValue::to_text` bound the bare
  helper to the wrong module (blank output on wasm32, `ref.cast` trap on
  wasm32-gc). The private helpers are now `json_*` / `toml_*` prefixed and no
  duplicate top-level name remains; the two-module regression covers both
  value types.
- **Fixtures**: `stdlib_json/json_associated_surface*.ark`,
  `stdlib_toml/toml_text_surface*.ark`, and
  `modules/non_main_associated_call/` cover both parsers, `to_text` on an
  integer payload, and the two-module call path on `wasm32-gc` and
  `--target wasm32`. The E0100 goldens are
  `diagnostics/removed_json_toml_csv_free*.ark`.
- **Scorecard**: 358 -> **347** public symbols; `associated-or-method`
  288 -> **277**; manifest functions 325 -> **322**; unregistered declarations
  34 -> **26**. `must delete` 70 and `intrinsic-only` 1 unchanged.

## 2026-10-27 — std::simd lane namespace completion (#917, ADR-046)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2 and the
  ADR-014 D4 exception for `experimental` entries. The first lane wave moved
  132 operations to `F32x4` … `V128`; this wave fixes the emitter and moves the
  remaining 130, so no `std::simd` top-level free function remains.
- **Emitter fixes**: corrected the 0xfd sub-opcode table (i8x16/i16x8/i32x4/
  i64x2/f32x4/f64x2 arithmetic, neg, and load/store-lane values), registered
  the missing comparison/min/max operations and dispatch branches, added the
  unsigned-lane aliases plus `U8x16`/`U16x8`/`U32x4`/`U64x2` namespaces,
  emulated `i8x16::mul` and the four unsigned `i64x2` compares, kept the i64x2
  shift amount at wasm `i32`, and rebuilt `v128::new` from `i8x16` lanes.
- **Removed**: the 130 remaining `pub fn` declarations and the 37 `std::simd`
  `[[functions]]` manifest entries. `U8x16`, `U16x8`, `U32x4`, and `U64x2`
  are new `[[types]]` entries; the existing type method lists now cover every
  lane operation.
- **Replacement**: the namespace associated functions on `F32x4`, `F64x2`,
  `I8x16`, `I16x8`, `I32x4`, `I64x2`, `U8x16`, `U16x8`, `U32x4`, `U64x2`, and
  `V128`. Qualified old calls keep the E0100 hint from
  `resolver::removed_names`.
- **Evidence**: `simd_conformance/{u8x16,u16x8,u32x4,u64x2,signed,float,v128_new}`
  `_lane_ops`/`_new` fixtures plus their `_wasm32` twins execute on
  `wasm32-gc` components and `--target wasm32`; the full fixture harness,
  `selfhost fixpoint`, and the ADR-029 gate are recorded in the wave receipt.

## 2026-10-27 — std::core range/error/hash receiver surface (#917, ADR-087 Wave 2)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2 and the
  [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) 0.x exception. The
  eight `stable` targets were converted to `deprecated` in Wave 1 (commit
  `3e1b307d8`) with `deprecated_since = "0.1.0"`, `remove_in = "1.0.0"`,
  live **W0009** qualified-call warnings, and dual-target diag goldens. Wave 2
  deletes the manifest entries and `pub fn` declarations after the migration
  rows and this entry landed; the unregistered `std::core::identity` has no
  replacement.
- **Removed (8 manifest + 1 unregistered)**: `range_new`, `range_contains`,
  `range_len`, `range_inclusive_new`, `range_inclusive_contains`, `cmp_i32`,
  `error_message`, `combine`, and `identity`.
- **Replacement**: `Range::new(start, end)` / `r.contains(value)` / `r.len()`,
  `RangeInclusive::new(start, end)` / `r.contains(value)`, `a.cmp(b)` through
  the `Ord` trait, `e.message()`, and `h1.combine_hash(h2)`. `Range::new` is
  bound to the `core.range_new` CoreOp, so the GC struct constructor and the
  linear-memory 8-byte record path keep their existing lowering. Qualified
  old calls now fail with **E0100** plus the receiver hint from
  `resolver::removed_names`; bare names carry the same hint.
- **Compiler fix**: an enum-variant struct literal
  (`Error::IndexOutOfBounds { .. }`) keeps the variant payload layout as its
  local type, so `e.message()` resolved `IndexOutOfBounds::message`; the
  ambiguous bare-name fallback (`message` is also `JsonParseError::message`)
  emitted `drop; unreachable` and failed Wasm validation. Method-call
  resolution now maps a registered payload-variant receiver to the owning
  enum when that enum declares the method, keeping the payload layout for
  emit (stamping the local as the enum resolved the method but trapped on the
  linear target). A value returned from a function or bound through an
  explicit annotation already lowered correctly.
- **Fixtures**: dual-target `stdlib_core/core_method_surface*` pair (compile,
  validate, run), `diagnostics/removed_core_free*` E0100 goldens, and the
  migrated `stdlib_core`, `stdlib_trait`, `stdlib_inline`,
  `stdlib_component`, and `generics_v1` call sites. The #914 shadow fixture
  is retargeted from the removed `std::core::identity` to the live
  `std::collections::string::to_lower`.
- **Scorecard**: 515 → **506** public symbols; `associated-or-method`
  445 → **436**; manifest public entries 369 → **361**; unregistered
  declarations 146 → **145**; `pub fn` declarations 569 → **560**;
  monomorphic `*_i32` helpers 57 → **56** (`cmp_i32`); `must delete` 70,
  `intrinsic-only` 1, and `prelude_wrapper` 65 unchanged. The
  `host/core namespace frees` family drops from 63/21 to **55/20**.
- **Residuals**: the 20 `std::core::raw` representation bridges still need a
  `RawArray` / `RawString` namespace surface; the other host families
  (`std::host::{env,fs,clock,random,process}`, `std::fs`, `std::env`,
  `std::cli`, `std::process`, `std::signal`) are untouched.

## 2026-10-27 — std::json / std::toml / std::csv associated conversions (#917, ADR-046)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2 and the
  [ADR-014](../adr/ADR-014-stability.md) D4 experimental rule. Every removed
  manifest entry was `experimental`; unregistered declarations were never in
  the stable 64, so no ADR-087 W0009 window applies. E0100 hints come from
  `removed_json_member_hint` / `removed_toml_member_hint` /
  `removed_csv_member_hint` and the bare-name fallback.
- **Compiler unblock**: a `module::Type::method` call in a loaded module kept
  the module prefix (the resolver only canonicalizes the entry module), while
  the signature spine registers `Type::method`. Lowering left `fid=-1`; the
  fused in-loop emitter emitted `drop; unreachable` instead of deferring, and
  the reachability worklist could drop the impl body. `callee_spelling_variants`
  now canonicalizes the spelling in lowering, the typechecker, edge discovery,
  the in-loop oracle, and the drain-time name fallback.
- **Removed (32 manifest + 25 unregistered)**: the JSON text helpers
  (`quote_string`, `json_escape`, `skip_ws`, `parse_int_at`,
  `parse_content_length`, `find_key_pos`, `json_hex_byte`, `json_hex_value`,
  `json_parse_string_at`, `json_decode_escape[_at]`,
  `json_decode_unicode_ascii`, `json_get_str[_after]`, `json_get_int`), the
  JSON primitive constructors/serializers (`json_stringify_*`, `json_null`,
  `json_parse_*`, `stringify_pretty`, `json_field`, `json_str`, `json_i32`,
  `json_bool`, `json_null_val`, `json_raw`, `json_obj_new`, `json_arr_new`,
  `json_obj_string`, `json_arr_string`, `json_encode_string`), the TOML text
  scanners (`toml_parse_line`, `toml_find_*`, `find_toml_*`,
  `toml_parse_string_array`), and the CSV frees (`csv_split_line`,
  `csv_parse_row`, `csv_stringify_row`, `csv_count_rows`, `csv_get_row_raw`,
  `csv_parse_row_at`, `csv_parse`, `csv_stringify`, `csv_parse_with_header`).
- **Replacement**: `JsonText` (`quote`, `escape`, `skip_ws`, `parse_int_at`,
  `content_length`, `find_key_pos`, `hex_byte`, `hex_value`,
  `parse_string_at`, `decode_escape_at`, `decode_unicode_ascii`, `get_str`,
  `get_int`), `JsonValue` constructors (`null`, `bool`, `int`, `string`,
  `raw`, `array`, `object`), `JsonField::new`, `JsonValue::pretty_text`,
  the `JsonValue` primitive helpers (`stringify_*`, `parse_*`,
  `encode_string`, `null_text`), `JsonStrResult` / `JsonEscapeResult`
  accessors, `TomlText` (`parse_line`, `find_section`, `find_value`,
  `find_raw_value`, `parse_string_array`), and `Csv` (`split_line`,
  `parse_row`, `stringify_row`, `count_rows`, `get_row_raw`, `parse_row_at`,
  `parse`, `stringify`, `parse_with_header`). The full before/after table is
  in the [migration guide](migration-guidance.md#associated-conversion-wave-2026-10-27-917--wavenon-main-module).
- **Fixtures**: `stdlib_json/json_associated_surface*.ark`,
  `stdlib_toml/toml_text_surface*.ark`,
  `stdlib_csv/csv_namespace_surface*.ark`, and the two-module regression
  `modules/non_main_associated_call/`. The pairs compile and run on
  `wasm32-gc` (P2 component) and `--target wasm32` (linear).
- **Scorecard**: 588 → **531** public symbols; `associated-or-method`
  518 → **461**; manifest functions 417 → **385**; unregistered declarations
  172 → **147**. `must delete` 70 and `intrinsic-only` 1 unchanged.
- **Residuals**: `json::parse` / `toml::toml_parse` / `toml::toml_stringify`
  stay free (unannotated associated `parse` loses the Result payload type;
  `TomlValue::to_text` still emits an i64 return on wasm32-gc);
  `Vec<String>::concat` remains invalid on wasm32-gc independently of this
  wave (join with an explicit loop / use `len` / `get_unchecked`). The
  committed pin predates the fix, so the generated flat overlay routes the
  compiler's own manifest scans through free `bootstrap_toml_*` shims
  (`scripts/selfhost/checks.py::_patch_bootstrap_manifest_scanner_paths`);
  delete the adapter when the pin is refreshed.

## 2026-10-27 — std::json / std::toml value methods (#917, ADR-046)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2 and the
  [ADR-014](../adr/ADR-014-stability.md) D4 experimental rule. All removed
  manifest entries were `experimental`; no ADR-087 W0009 window applies.
- **Removed (18 manifest + 9 unregistered)**: `json_get`, `json_get_index`,
  `json_as_bool/string/i32/f64/array`, `is_null/bool/number/string/array/object`,
  `stringify`, `parse_error_message`, `toml_get`, `toml_table_keys`, and
  `toml_as_bool/int/string/float/array/table`.
- **Replacement**: `JsonValue` methods (`v.is_*()`, `v.as_*()`,
  `v.get_field(key)`, `v.get_index(i)`, `v.to_text()`,
  `e.message()` on `JsonParseError`) and `TomlValue` methods
  (`v.as_*()`, `v.get_field(key)`, `v.table_keys()`). See the
  [migration guide](migration-guidance.md#stdjson--stdtoml-value-method-wave-adr-046-2026-10-27).
- **Retained by lowering blocks**: the primitive helpers, DOM constructors,
  `JsonText` / `TomlText` scanner, CSV frees, `json::parse`,
  `toml::toml_parse`, and `toml::toml_stringify`. Associated calls from a
  non-main module emit an unresolved-call fallback, and associated `parse`
  loses its `Result` payload type in an unannotated `match`.
- **Scorecard**: 728 → **701**; `associated-or-method` 658 → **631**;
  manifest functions 507 → **489**; unregistered declarations 222 → **213**.

## 2026-10-27 — std::bytes free surface removed (#917, ADR-087 Wave 2)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2/D4, the
  [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) 0.x exception, and the
  [#917](https://github.com/wogikaze/arukellt/issues/917) execution lane. The
  45 entries were `stable` and completed their Wave 1 window in the
  2026-10-27 bytes surface wave (`deprecated` + W0009,
  `deprecated_since = "0.1.0"`, `remove_in = "1.0.0"`).
- **Removed (45)**: `bytes_new`, `bytes_len`, `bytes_get`, `bytes_push`,
  `bytes_eq`, `bytes_from_string`, `string_from_bytes`, `bytes_concat`,
  `bytes_slice`, `byte_length`, `hex_encode`, `hex_decode`,
  `leb128_encode_u32`, `leb128_encode_i32`, `u16_to_le_bytes`,
  `u16_from_le_bytes`, `u32_to_le_bytes`, `u32_from_le_bytes`,
  `u32_to_be_bytes`, `u32_from_be_bytes`, `bytes_from_array`, `buf_new`,
  `buf_with_capacity`, `buf_push_u8`, `buf_push_u16_le`, `buf_push_u32_le`,
  `buf_push_u64_le`, `buf_extend`, `buf_freeze`, `buf_len`, `cursor_new`,
  `cursor_pos`, `cursor_remaining`, `read_u8`, `read_u16_le`, `read_u32_le`,
  `read_u64_le`, `read_u32_be`, `read_bytes`, `base64_encode`,
  `base64_decode`, `leb128_decode_u32`, `leb128_decode_i32`,
  `leb128_encode_u64`, `leb128_decode_u64`. The manifest entries and the
  `pub fn` declarations are gone.
- **Replacement**: `Vec<i32>` byte-vector methods (`b.push_byte`, `b.len`,
  `b.get_unchecked`, `a.equals_bytes`, `a.concat_bytes`, `b.slice_bytes`,
  `b.to_hex`, `b.to_base64`, the endian `to_*` / `push_*` pairs,
  `b.extend_bytes`, `b.freeze`, `data.byte_cursor()` with the `c.read_*` /
  `c.try_read_*` / `c.read_leb128_*` readers) plus the `Bytes` namespace for
  receiver-less constructors/decoders (`Bytes::from_hex`,
  `Bytes::from_base64`, `Bytes::from_array`, `Bytes::from_u16_le`,
  `Bytes::from_u32_le`, `Bytes::from_u32_be`, `Bytes::from_leb128_*`).
  Import `use std::bytes` for the methods and `use std::bytes::Bytes` for the
  associated functions.
- **Diagnostics**: the 45 W0009 rows became **E0100** hints from
  `resolver::removed_names::removed_bytes_member_hint`; the eight
  unregistered helpers retired in Wave 1 (`hex_char`, `hex_val_char`,
  `try_hex_val_char`, and the five `try_read_*` readers) keep their hints in
  the same table (the earlier merge dropped the table, so this wave restores
  it with the 45 additions). The W0009 goldens were replaced by
  `diagnostics/removed_bytes_free*.ark` on both targets.
- **Call sites**: 150 fixture files (869 `bytes::*` calls) and the
  `modules/use_qualified_call` qualifier fixture moved to the method /
  associated surface; the cookbook `wasm-binary` code blocks and the language
  spec section were updated.
- **Scorecard**: 858 → **813** public symbols; `must delete` 115 → **70**;
  manifest functions 626 → **581**; `pub fn` declarations 981 → **936**;
  `associated-or-method` 743 and `intrinsic-only` 1 unchanged. The
  `deprecated std::bytes frees` rule is removed from the eradication TOML (no
  remaining symbol matches it).
- **Verification**: `verify fixtures` **1845/1845 PASS**, `selfhost
  diag-parity` 124 PASS / 0 FAIL, both targets (wasm32-gc P2 and linear
  wasm32 P1) compile+validate for every changed fixture, `docs check`,
  `check-stdlib-eradication`, and `check-compat-orphans` clean.
- **Residual**: `Bytes::from_base64` still skips invalid quartets
  (compatibility behavior), `try_read_u64_le` stays deferred (constructing
  `Some(i64)` emits invalid Wasm), and the `std::io` /
  `std::host::fs` byte-layout adapters remain under their own lanes.

## 2026-10-27 — concrete `HashSet<i32>` membership surface (#917)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2 and
  [ADR-086](../adr/ADR-086-linear-collection-adts.md) §2/§4: public collection
  types use associated constructors and receiver methods. This is an
  incremental slice of the [#917](https://github.com/wogikaze/arukellt/issues/917)
  execution lane; the aggregate and `String` lanes are not part of it.
- **Added**: `HashSet<i32>` (`std::collections::hash_set::HashSet`) with
  `HashSet::new<i32>()` and the receiver methods `insert`, `contains`,
  `remove`, `len`, `is_empty`. They lower through the compiler method rewrite
  to the flat `hash::hashset_*` store.
- **Retained, deprecated (20)**: the 15 `hash_set::hashset_i32_*` adapters and
  the 5 `hash_set::hashset_str_*` `Vec<String>` facade adapters. They stay for
  the aggregate i32 and `String` lanes, whose GC lowering is not fixed yet;
  no manifest lifecycle or E0100 removal applies.
- **Fixtures**: `stdlib_hashset/hashset_methods.ark` and
  `stdlib_hashset/hashset_methods_wasm32.ark` (both targets, compile + run).
  No aggregate, `String`, or `union`/`reserve`/`extend` fixture is registered.
- **Residual**: `HashSet<T>` has no aggregate methods (`to_vec`, `union`,
  `intersection`, `difference`, `capacity`, `reserve`, `shrink_to_fit`,
  `clear`, `extend`) or `HashSet<String>` methods until the GC method
  lowering is fixed; those users keep the deprecated adapters. No
  `IntoIterator` / `FromIterator` / `Clone` / `Eq` / `Debug` impls yet.

## 2026-10-27 — iterator collect lane fixes (#917, #691)

- **Basis**: the iterator-collect residuals reported by the
  [#917](https://github.com/wogikaze/arukellt/issues/917) `next1054` lane and
  the lazy-adapter lane of [#691](https://github.com/wogikaze/arukellt/issues/691).
  No public signature is removed or renamed in this entry.
- **Fixed**: `UniqueIter` / `RevIter` `.collect()` now emits the method body;
  the AST edge worklist could infer the receiver of
  `a.iter().unique().collect()` as the inner `VecIter`, keep
  `VecIter::collect`, and drop `UniqueIter::collect`, so the call lowered to
  the unresolved `drop; unreachable` fallback while `.next()` / `.count()`
  worked.
- **Fixed**: `TakeIter` / `SkipIter` `.collect()` instantiated inside a std
  module resolved its `for item in self` desugar to the first concrete
  `into_iter` variant (`TakeIter<MapIter<..>>`), which does not match the
  emitted `TakeIter<VecIter<i32>>` signature. Both adapters now drain through
  an explicit `self.next()` loop in their collect hook (the same item
  protocol, without the `for` desugar), so `I::Item` resolves per instance.
  The deprecated
  `std::seq::take_i32` / `skip_i32` shims are lazy again
  (`v.iter().take(n).collect()` / `v.iter().skip(n).collect()`) with the same
  signatures and boundary results.
- **Fixed**: `std::core::iter` method calls in a module that omits
  `use std::core::iter` now fail type checking with
  ``method `<name>` requires `use std::core::iter` `` instead of silently
  lowering to the unresolved `drop; unreachable` fallback. Auto-loading the
  module from the prelude is not viable yet: its `Pair<A, B>` / `Range`
  declarations collide with same-named user types in the shared type registry,
  so the diagnostic keeps the explicit-import contract that the migration
  docs already use.
- **Fixtures**: `stdlib_trait/iterator_unique_rev_collect*.ark`,
  `stdlib_seq/seq_take_skip_collect*.ark`, and the
  `diagnostics/iterator_methods_no_import.ark` E0200 golden; the
  `iterator_method_surface*.ark` pair now asserts the `unique` / `rev`
  `collect` values instead of only `next` / `count`. Each pair compiles,
  validates, and runs on `wasm32-gc` (memory32 / P2) and `--target wasm32`
  (wasi-p1).
- **Residuals**: a negative `n` passed to the `std::seq` take shim still traps
  on linear-memory `wasm32`; the same call traps with the previous eager
  implementation, so it is a separate pre-existing linear lowering bug. The
  new fixtures pin the supported 0 / positive / over-length boundaries. The
  underlying generic-`for` site selection still pins the first concrete
  `into_iter` instance for other adapter bodies; fixing that needs
  per-enclosing-instance call-site records and is not required for the two
  collect hooks fixed here.

## 2026-10-27 — eager iterator constructor/collector frees removed (#917)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2/D4 and
  the [#917](https://github.com/wogikaze/arukellt/issues/917) execution lane.
  All 44 symbols were `provisional` manifest entries with top-level `pub fn`
  declarations; the provisional lifecycle allows a minor-version change with
  changelog notice, so no ADR-087 `stable` exception and no W0009 deprecation
  window applied.
- **Removed (44)**: the eager `std::core::iter` constructors (`vec_iter`,
  `range_iter`, `range_inclusive_iter`, `map_iter`, `filter_iter`, `take_iter`,
  `skip_iter`, `chain_iter`, `enumerate_iter`, `zip_iter`, `flat_map_iter`,
  `peek_iter`, `step_by_iter`, `scan_iter`, `unique_iter`, `rev_iter`) and the
  collectors/consumers (`collect_vec_iter` … `collect_range_iter`,
  `collect_map_i32`, `collect_filter_i32`, `collect_take_i32`,
  `collect_skip_i32`, `collect_chain_i32`, `collect_range_i32`,
  `collect_map_filter_i32`, `collect_map_filter_take_i32`, `iter_fold_i32`,
  `iter_count_i32`, `iter_sum_i32`, `iter_any_i32`, `iter_all_i32`,
  `iter_contains_i32`, `iter_count_eq_i32`, `iter_unique_i32`,
  `iter_reverse_i32`).
- **Replacement**: the lazy adapter method surface on `VecIter<T>`.
  `chain`, `enumerate`, `zip`, and `all` landed in the same change so every
  removed constructor has a method spelling; the collectors are
  `collect` / `count` and the numeric capabilities
  (`sum`, `fold`, `contains`, `count_eq`) are trait methods. `iter::*`
  qualified calls now fail resolution with an **E0100** hint from
  `resolver::removed_names`; every before/after row is in
  [migration-guidance.md](migration-guidance.md).
- **In-tree callers**: the 10 deprecated `std::seq` compatibility helpers were
  migrated too; map / filter / fold now use the methods, while take / skip
  keep an eager bounded copy because the `TakeIter` / `SkipIter` collector
  mis-resolves its item type when instantiated from a std module (#691; the
  same chain compiles from user code).
- **Scorecard**: public surface 1104 → **1060** symbols; manifest public
  entries 729 → **685**; `pub fn` declarations 1279 → **1235**;
  `associated-or-method` 1021 → **977**; monomorphic `*_i32` manifest helpers
  114 → **97**. `must delete` 83, `intrinsic-only` 1, `prelude_wrapper` 65,
  and the 375 unregistered declarations are unchanged. The
  `std::core::iter frees` classification rule retired with the family, and the
  module's 44-API row dropped from the coverage scoreboard.
- **Evidence**: stage-2 rebuild; full fixture parity **1839/1839**
  compile+validate PASS with 105 runtime smoke fixtures (the two new
  `stdlib_trait/iterator_method_surface*.ark` fixtures cover the method
  surface on `wasm32-gc` and linear `wasm32`); the 56 migrated
  `stdlib_core/iter_*` and `stdlib_trait/iterator_*` fixtures compile and
  validate on both targets; `selfhost diag-parity` **117/117** PASS including
  the two new `diagnostics/removed_iter_constructor*.ark` E0100 goldens;
  cross-backend parity **2/2** for the new pair; `verify lane` PASS;
  `check-stdlib-eradication`, `check-compat-orphans`, and `docs check` 6/6
  clean.
- **Residuals**: the `.unique().collect()` / `.rev().collect()` runtime lane
  traps inside the shared generic `collect_iterator` on both targets
  (pre-existing: `.next()` / `.count()` work, and the retired
  `iter_unique_i32` / `iter_reverse_i32` frees used the same trapping collect
  path), so the new fixture pins the reachable next/count paths. Methods of
  `std::core::iter` resolve without importing the module but miscompile, so
  migrated call sites keep `use std::core::iter`; this loader/lowering gap is
  separate from the eradication wave.

## 2026-10-27 — stable seq/combinator frees removed (ADR-087 Wave 2, #703 / #917)

- **Basis**: [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) D2/D3,
  [ADR-046](../adr/ADR-046-free-function-eradication.md), and the
  [#703](https://github.com/wogikaze/arukellt/issues/703) /
  [#917](https://github.com/wogikaze/arukellt/issues/917) execution lanes.
- **Conditions**: (1) the 31 entries carried `deprecated_since = "0.1.0"` and
  `remove_in = "1.0.0"` from the Wave 1 deprecation on 2026-10-26, with live
  W0009 coverage and diagnostic fixtures; (2) `docs/stdlib/migration-guidance.md`
  records the before/after and the reachable replacement for every symbol;
  (3) this entry. The 31 bare names now fail resolution with **E0100** and the
  `resolver::removed_names` hint, replacing the W0009 warning.
- **Removed (31)**: `sort_i32`, `sort_i64`, `sort_f64`, `sort_String`,
  `map_i32_i32`, `map_i64_i64`, `map_f64_f64`, `map_String_String`,
  `filter_i64`, `filter_f64`, `filter_String`, `fold_i32_i32`,
  `fold_i64_i64`, `fold_f64_f64`, `any_i32`, `any_i64`, `any_f64`,
  `any_String`, `find_i32`, `find_i64`, `find_f64`, `find_String`,
  `contains_i32`, `contains_String`, `reverse_i32`, `reverse_String`,
  `remove_i32`, `sum_i32`, `product_i32`, `map_option_i32_i32`, and
  `contains_string` (manifest entries and `std/prelude.ark` declarations).
- **Reason**: ADR-046 eradication of the user-reachable monomorphic prelude
  surface; the replacement is the `Vec` / iterator method surface, with the
  concrete reachable fallback recorded per row in the migration guide.
  `filter_i32` remains `provisional` (individual deferral) and was not
  removed.
- **Replacement**: `v.sort_by(cmp)` for `Vec<i32>` / `Vec<String>` and
  `std::seq::sort_by(v)` for the `i64` / `f64` sort lanes;
  `v.iter().map(f).collect()`, `v.iter().filter(f).collect()`,
  `v.iter().fold(init, f)`, `v.iter().sum()`, `v.iter().any(f)`,
  `v.iter().find(f)`, `v.iter().contains(x)`, and
  `v.iter().rev().collect()` for the verified item lanes; `v.drain(i, i + 1)`
  for `remove_i32`; `o.map(f)` via `std::core::container` for
  `map_option_i32_i32`; and `s.contains(sub)` for `contains_string`. The
  `i64` / `f64` numeric consumers, the `Vec<String>` reverse/contains lanes,
  and the generic `Vec::sort_by` receiver for `i64` / `f64` still emit invalid
  Wasm, so those rows document an explicit loop / `std::seq::sort_by`
  fallback until the lane lands.
- **Scorecard**: public surface 1168 → **1137** symbols; manifest public
  entries 776 → **745**; `prelude_wrapper` entries 96 → **65**; `pub fn`
  declarations 1359 → **1328**; `must delete` 114 → **83**
  (`associated-or-method` unchanged at 1054). The family count is 32 → **1**
  (`filter_i32` only).
- **Evidence**: stage-2 rebuild; full fixture parity **1836/1836**
  compile+validate PASS (105 runtime smoke, 1 `test:` fixture PASS);
  cross-backend parity **81/81** for the changed run fixtures
  (`wasm32-gc` component vs `wasm32` linear core, stdout identical);
  `selfhost diag-parity` PASS with the new E0100 goldens;
  `check-stdlib-eradication`, `check-compat-orphans`, and `docs check` 6/6
  clean. `verify lane` PASS after forcing single-file lint with the pinned
  bootstrap CLI: the stage-2 stub host traps in its `lint` `parse_args` path
  before per-file fallback (pre-existing tooling residual, not a source
  failure). The ADR-029 fixpoint result is recorded in the wave receipt.

## 2026-10-27 — std::io Phase F completion (ADR-085 Phase F, #693)

- **Basis**: [ADR-085](../adr/ADR-085-io-trait-unification.md) D2/D3/D5/D8/D9,
  [ADR-046](../adr/ADR-046-free-function-eradication.md) D3/D4, and the
  [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) 0.x removal exception.
  The BufReader dispatch residual that blocked the buffered wrapper migration
  was fixed by `ebfb2f8be` (calls inside `if` conditions now record their mono
  demand edge).
- **Conditions**: (1) all 13 removed entries carried
  `deprecated_since = "0.1.0"` / `remove_in = "1.0.0"` from the
  2026-10-27 Phase E entry, with live W0009 coverage and the
  `deprecated_io_free_function` diagnostic fixture; (2) the before/after rows in
  [migration-guidance.md](migration-guidance.md) name the replacement and the
  deleted `buf_*` layout behavior; (3) this entry. The removed names now fail
  resolution with **E0100** and a `resolver::removed_names` hint, replacing the
  W0009 warning.
- **Removed (13)**: `reader_from_bytes`, `buf_reader_new`, `buffered_reader`,
  `buf_reader_read_line`, `buf_writer_new`, `buffered_writer`,
  `buf_writer_write_str`, `buf_writer_flush`, `writer_new`, `write_all`,
  `write_string`, `writer_flush`, and `writer_to_bytes` (manifest entries and
  `std/io/mod.ark` declarations).
- **Reason**: the generic `BufReader<R: Read>` / `BufWriter<W: Write>`
  adapters and the `Reader` / `Writer` / `Io` methods fully replace these
  wrappers. Keeping the legacy `[cursor, ...]` / `[fd_tag, ...]` layouts would
  keep a second public representation of the same handles alive after its
  consumers migrated.
- **Replacement**: `Reader::from_bytes`, `BufReader::new` / `br.read_line()`,
  `BufWriter::new` / `bw.write_string(s)` / `bw.flush()`, `Writer::new` /
  `w.write(data)` / `w.write_string(s)` / `w.flush()` / `w.into_bytes()`, and
  `Io::write_all`.
- **Retained (2)**: `writer_write` and `writer_write_str` stay `deprecated`
  under the same window. Exact blocker: `stdout()` and `stderr()` return tagged
  `Vec<i32>` handles, not `Writer` values, and `std::host::stdio` does not yet
  expose typed handles with `Write` implementations, so there is no receiver to
  migrate these two calls to (ADR-085 D9). Deletion condition: those typed
  handles land.
- **Scorecard**: deprecated std::io compat frees 15 → **2**; `pub fn`
  declarations under `std/**/*.ark` 1279 → **1266**; manifest public entries
  729 → **716**; eradication scope 1104 → **1091**; `must delete` 83 → **70**
  (all 13 removed entries were already in that class). `std/manifest.toml` and
  the generated docs are the current source of truth.
- **Evidence**: stage-2 rebuild from `ebfb2f8be`; the 25 migrated `stdlib_io`
  / `stdlib_io_rw` / `stdlib_trait` run fixtures compile, validate, and run on
  both `wasm32-gc` (memory32, WASI P2 component) and `--target wasm32` (core),
  with identical stdout and matching `.expected` goldens (25/25). Full
  `verify fixtures`: **1837/1837** compile+validate PASS, 0 failed, 105 runtime
  smoke, 1 `test:` fixture PASS. `selfhost diag-parity` PASS with the updated
  W0009 golden and the new E0100 golden
  (`tests/fixtures/diagnostics/removed_io_compat_free_function.ark`).
  `check-compat-orphans` clean, `check-stdlib-eradication`
  717/717 manifest functions + 375/375 unregistered declarations classified,
  `docs check` **6/6**, `verify lane` PASS, and the ADR-029 fixpoint reached
  (`sha256(arukellt-s2.wasm) == sha256(arukellt-s3.wasm)` =
  `27fe9ef5c9e55ea1edfed3b2422159353ea3f7c112c26b51e3a95ced36521474`).

## 2026-10-27 — std::io trait cutover (ADR-085 Phase E/F, #693)

- **Basis**: [ADR-085](../adr/ADR-085-io-trait-unification.md) D1–D10,
  [ADR-046](../adr/ADR-046-free-function-eradication.md) D3/D4, and the
  [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) 0.x removal exception.
  The canonical surface is `Reader` / `Writer`, `Read` / `Write` / `BufRead` /
  `Seek`, the `Io` algorithms, the generic `BufReader<R>` / `BufWriter<W>`
  adapters, and `std::host::fs::File`.
- **Conditions**: (1) W0009 is live for the retained compatibility family in
  `src/compiler/lint/deprecated_table.ark` with unit coverage and a live diag
  fixture; (2) [migration-guidance.md](migration-guidance.md) records every
  before/after and the behavior differences; (3) this entry.
- **Converted to `deprecated` (15)** with
  `deprecated_since = "0.1.0"` and `remove_in = "1.0.0"`:
  `reader_from_bytes` → `Reader::from_bytes`; `buf_reader_new` /
  `buffered_reader` → `BufReader::new`; `buf_reader_read_line` →
  `BufReader::read_line`; `buf_writer_new` / `buffered_writer` →
  `BufWriter::new`; `buf_writer_write_str` → `BufWriter::write_string`;
  `buf_writer_flush` → `BufWriter::flush`; `writer_new` → `Writer::new`;
  `writer_write` → `w.write`; `writer_write_str` / `write_string` →
  `w.write_string`; `write_all` → `Io::write_all`; `writer_flush` →
  `w.flush`; `writer_to_bytes` → `w.into_bytes`.
  Two reasons keep this window open instead of deleting the family now:
  `reader_from_bytes` / `buf_*` retain the legacy `Vec<i32>` layout until the
  generic-adapter dispatch residual (#693) allows their fixtures to move, and
  `writer_*` keeps the tagged `stdout()` / `stderr()` handles writable until
  `std::host::stdio` exposes typed `Write` handles.
- **Removed (18)** with E0100 replacement hints in
  `src/compiler/resolver/removed_names.ark`:
  `reader_eof`, `reader_read`, `reader_read_exact`, `reader_read_all`,
  `reader_read_line`, `copy_bytes`, `copy`, `copy_trait`, `new_memory_buffer`,
  `write_bytes`, `read_bytes`, `seek_to`, `buffer_position`, `buffer_len`,
  `fill_buffer`, `file_from_fd`, `file_read`, `file_write` (the last three were
  the experimental fd lane; ADR-014 allowed direct retirement). The 17
  manifest entries were removed and `copy_trait` was source-only.
- **Guidance fix**: `use std::io` no longer emits the retired
  "moved to `std::host::stdio`" W0009; `std::io` is the canonical abstraction
  module (ADR-085 D1). `docs/compiler/error-codes.md` E0104 now uses the
  `std::time` → `std::host::clock` example.
- **Scorecard**: eradication scope 1226 → **1208** (manifest public entries
  813 → **796**, unregistered std declarations 413 → **412**); `must delete`
  119 → **134**, `associated-or-method` 1107 → **1074**. Stability:
  `stable` 315 → **287**, `experimental` 318 → **314**, `deprecated`
  97 → **112**.
- **Evidence**: W0009
  `tests/fixtures/diagnostics/deprecated_io_free_function.ark`, E0100
  `tests/fixtures/diagnostics/removed_io_free_function.ark`, and the migrated
  `stdlib_io*` / `stdlib_trait` / `question_mark` fixtures compiled and run on
  both `wasm32-gc` and `wasm32`. `python3 scripts/manager.py verify fixtures`
  reports 1835/1835 compile+validate, 0 failed, 105 runtime smoke; the 68
  changed io fixtures additionally produced identical `wasm32-gc` component and
  `wasm32` core output.

## 2026-10-26 — 0.x deprecation exception (ADR-014 addendum)

- **Policy**: [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) を採択した。
  ADR-014 は維持し、0.x 期に限り `stable` の削除窓を明示的例外で短縮する。
- **Scope**: 0.x 期の `stable` 公開 stdlib エントリのみ。language spec、CLI、ABI、
  target、compiler phase、stdlib 以外の API には適用しない。
- **Conditions**: 削除 wave ごとに (1) W0009 が対象 API に対して実装・発火する、
  (2) `migration-guidance.md` に before/after と到達可能な replacement がある、
  (3) この changelog に対象シンボル・削除理由・replacement を記録する。
- **`remove_in`**: 0.x 例外の下では上限として扱う。`remove_in = "1.0.0"` の
  `stable` エントリを 0.x の minor で削除してよいが、`remove_in` より後には削除しない。
- **1.0.0**: 今は切らない。release-blocking な 3 項目（fixture harness、selfhost CLI
  parity、selfhost diag parity）の再証跡が揃うまでタグを切らない。1.0.0 以降は
  ADR-014 の通常方針に戻り、この例外は失効する。
- **Immediate wave**: `experimental` の `must delete` 2 件、`provisional` の
  `must delete` 21 件（個別判断）、`associated-or-method` 1129 件の replacement 追加。
- **Not in this wave**: `stable` の `must delete` 64 件。特に prelude monomorphic
  seq/combinator の 33 件は W0009 実装（別 lane）まで削除しない。314 件の `stable`
  free→method 置換は replacement の追加として進める。

## 2026-10-26 — stable 64 + seq 33 wave（ADR-087）

- **Basis**: [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) D2（0.x 期の
  `stable` 削除例外）。条件 1 の W0009 は全対象シンボルの bare 名に対して
  `src/compiler/lint/deprecated_table.ark` に実装し、単体テストで 61 名を
  カバーする。条件 2 は [migration-guidance.md](migration-guidance.md)、
  条件 3 は本 entry。`remove_in = "1.0.0"` は上限であり、削除は Wave 2。
- **Converted to `deprecated` (60)**: `stability = "deprecated"`、
  `deprecated_by`、`deprecated_since = "0.1.0"`、`remove_in = "1.0.0"` を
  `std/manifest.toml` に設定した。理由は ADR-046 の free-function 根絶と
  ADR-044 の trait / method 正規化。replacement は次の各 family のとおり。
  - **Numeric conversions (16)**: `u8_to_i32`, `u16_to_i32`,
    `i16_to_i32`, `i32_to_u8`, `i32_to_u16`, `i32_to_i8`, `i32_to_i16`,
    `u32_to_u64`, `u64_to_u32`, `i32_to_i64`, `i64_to_i32`, `f32_to_f64`,
    `f64_to_f32`, `i32_to_u32`, `f64_bits_lo`, `f64_bits_hi` →
    `u8/i8/u16/i16::to_i32()`、`i32::to_u8_wrapping()` ほか narrow methods、
    `u32::to_u64()`、`u64::to_u32_wrapping()`、`i32::into()`、
    `i64::to_i32_checked()`、`f32::to_f64()`、`f64::to_f32()`、
    `f64::to_bits_lo()` / `to_bits_hi()`。新しい scalar conversion method は
    `std/core/convert.ark` に追加した。`i8_to_i32` は個別 deferral: 現行 GC
    emitter では method frame からの sign-extension が trap するため stable の
    まま保持し、`i8::to_i32` を追加しない。`i32_to_u16` / `i32_to_i16` /
    `u64_to_u32` の method は free と同じ deferred intrinsic body を共有して
    おり、Wave 2 まで runtime trap が残る。
  - **Vec intrinsic-compat (9)**: `push`, `pop`, `len`, `get`, `set`,
    `get_unchecked` → `Vec<T>` methods。`as_slice`, `slice_len`, `slice_get`
    → `v.as_slice(s, e)`, `view.len()`, `view.get(i)`。
  - **Assertion frees (4)**: `assert_eq` → `std::test::assert_eq_i32`、
    `assert_ne` → `assert_ne_i32`、`assert_eq_i64` → `assert_eq_i64`、
    `assert_eq_str` → `assert_eq_string`。
  - **Seq/combinator frees (31)**: `sort_i32/i64/f64/String` →
    `v.sort_by(cmp)`; `map_i32_i32`, `map_i64_i64`, `map_f64_f64`,
    `map_String_String` → `v.iter().map(f).collect()`; `filter_i64`,
    `filter_f64`, `filter_String` → `v.iter().filter(f).collect()`;
    `fold_i32_i32`, `fold_i64_i64`, `fold_f64_f64` →
    `v.iter().fold(init, f)`; `any_i32/i64/f64/String` → `v.iter().any(f)`;
    `find_i32/i64/f64/String` → `v.iter().find(f)`; `contains_i32`,
    `contains_String` → `v.iter().contains(x)`; `reverse_i32` →
    `v.iter().rev().collect()`; `reverse_String` → 同形（String rev lane は
    planned）; `remove_i32` → `v.drain(i, i + 1)`; `sum_i32` →
    `v.iter().sum()`; `product_i32` → `v.iter().fold(1, mul)`;
    `map_option_i32_i32` → `Option::map`（`std::core::container`）;
    `contains_string` → `s.contains(sub)`。`filter_i32` は provisional の
    個別判断として据え置く。i64/f64/String の consumer lane は planned で
    あり、Wave 2 前に到達可能化を完了させる。
- **Removed (3)**: `clear`, `map_option_String_String`,
  `map_result_i32_i32` は宣言も resolver 登録も持たない stale な builtin
  entry だったため、deprecated 化せず削除した。`clear` は `v.clear()`、
  残りは `match` / `std::core::container` の `Option::map` が replacement。
  E0100 の removed-name hint を `resolver::removed_names` に追加した。
- **Scorecard**: eradication scope 1246 → **1243**（`must delete`
  122 → 119、`associated-or-method` 1124 のまま）。stability は
  `stable` 378 → **315**、`deprecated` 37 → **97**、manifest 公開 entry
  815 → **812**。`prelude_wrapper` entry は 96 のまま（deprecated 化のみで、物理削除は Wave 2）。
- **Evidence**: W0009 の live diag fixture を wasm32-gc / wasm32 の両 target
  で追加し、replacement の run fixture を `scalar/`, `collections/`,
  `iterator/`, `stdlib_test/` に追加した。`check-cross-backend-parity.py` で
  両 target の compile + execute を確認する。
- **Wave 2**: 60 件の物理削除は W0009 window の後。`i8_to_i32` の deferral と
  3 件の deferred-body trap も Wave 2 の前提。vec intrinsic-compat は
  compiler / std 内の約 9k 呼び出しの移行、i64/f64/String consumer lane の
  実装、call-site migration を前提とする。

## 2026-10-27 — HashMap builtin retirement（#917）

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) の free-function
  根絶と #917 の実行 wave。対象は `provisional` の monomorphic `HashMap_*` builtin 20 件で
  あり、`stable` ではないため ADR-087 の deprecation window は適用されない。移行は
  W0009 ではなく、未解決名に対する **E0100** の removed-name hint で案内する。
- **Removed (20 manifest builtins)**: `HashMap_i32_i32_{new,insert,get,contains_key,len}`、
  `HashMap_new_String_i32` / `HashMap_String_i32_{insert,get,contains_key,len}`、
  `HashMap_new_i32_String` / `HashMap_i32_String_{insert,get,contains_key,len}`、
  `HashMap_new_String_String` / `HashMap_String_String_{insert,get,contains_key,len}`。
  manifest entry だけでなく、`resolver::builtins_hash` / `typechecker::builtins_hash` /
  `mir::lower::call_type_hash` / `wasm::hash_aliases` / `native_c::core_call` の lowering
  登録と、`std/collections/hash_map.ark` の i32→i32 adapter 10 件を同時に削除した。
- **Replacement**: `impl HashMap<K,V>` の `new` / `insert` / `get` / `contains_key` /
  `remove` / `len` / `is_empty` / `keys` / `values` / `entries` のみが公開綴りである。
  `HashMap<i32, i32>` は `std::collections::hash` の flat store 実装へ直接 lower し、
  他の pair は `hash_map::` typed adapter を維持する。旧 spelling は
  `src/compiler/resolver/removed_names.ark` の `HashMap_` prefix hint で
  `HashMap::new<K, V>()` と receiver methods を案内する。
- **Evidence**: `hashmap_basic`, `hashmap_insert_return`, 6 pair family fixture、
  `hashmap_fn_growth`, `typed_i32_i32_remove`, `collections/hashmap_entries_values`、
  `collections/hashmap_*_method` を wasm32-gc / `--target wasm32` の両方で
  compile + execute し、stdout を比較した。t3 compile fixtures、native-cpp scenario、
  benchmark も method 綴りへ移行した。
- **Scorecard**: eradication scope 1226 → **1196**（`must delete` 119 → 99、
  `associated-or-method` 1107 → 1097）。manifest 公開 entry 813 → **793**、
  `pub fn` 宣言 1397 → **1387**、unregistered 413 → **403**。
  `prelude_wrapper` 96 は不変。`HashMap` methods の `keys` / `values` / `entries` は
  引き続き typechecker が直接型付けする。

## 2026-10-27 — WIT builder wave（#917、wave/917-1106 の記録復元）

- **Note**: この節は wave/917-1106 で追加された changelog 行を、
  #693 Phase E/F のマージで失われた後に復元したものである。コード側の
  `std::wit::world` と `WitParam::new` は既に method / associated 形で
  land している。
- **対象**: `std::wit::world` の builder free 9 件 (`world_new`,
  `world_import`, `world_export`, `world_name`, `world_exports_len`,
  `interface_new`, `interface_add_func`, `wit_func_new`, `wit_param_new`)
  と、その forwarder だった `std::wit::types::wit_param`。
- **変更**: `World::new` / `Interface::new` / `WitFunc::new` /
  `WitParam::new`、`w.add_import(iface)` / `w.add_export(iface)` /
  `w.name()` / `w.exports_len()` / `iface.add_func(func)`。
- **Evidence**: `stdlib_wit/world_build.ark` と
  `stdlib_wit/wit_print.ark` を method surface へ移行し、
  `stdlib_wit/wit_methods_wasm32.ark` が linear target の runtime を担う。

## 2026-10-27 — WIT type / text / printer method wave（#917）

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) の
  free-function 根絶と [ADR-044](../adr/ADR-044-trait-method-syntax-adopted.md)
  の method-first surface。対象は `std::wit` の非 AST 面 **33 記号**。
  16 件は `experimental` の manifest entry なので ADR-014 D4 により
  deprecation window なしで直接削除した。残り 17 件は未登録 source 宣言で
  W0009 の対象外。全 call が module-qualified だったため、E0100 は
  `resolver::removed_names::removed_module_member_hint` の `wit` /
  `types` / `scan` / `names` / `printer` alias にだけ追加した。
- **Replacement (types 20)**: `WitType::Bool` … `WitType::String` の enum
  variant、`WitType::list(inner)` / `WitType::option(inner)` の associated
  constructor、`ty.id()` / `ty.name()` / `ty.wit_text()` の receiver
  method、`WitType::from_id(id)` / `WitType::name_from_ark(name)` の
  associated function。`std::wit` の 16 個の forwarding wrapper も同じ
  wave で削除し、`std::wit` は module doc だけを持つルートになった。
- **Replacement (scan 8)**: `WitText` namespace type の
  `find_char` / `trim` / `strip_version` / `before_colon` / `split_param` /
  `split_csv_items` / `line_start` / `line_end`。`std::wit::scan` は
  `std::wit::types` に依存しないまま（bootstrap の module loading 制約）。
- **Replacement (names 4)**: `WitName` namespace type の
  `to_kebab` / `to_snake` / `to_pascal` / `capitalize`。
- **Replacement (printer 1)**: `print_wit(w)` → `w.to_wit()`。private の
  `wit_type_print` は `WitType::wit_text` と重複していたため削除し、
  `wit_func_print` / `wit_iface_print` は `WitFunc::to_wit` /
  `Interface::to_wit` へ移した。
- **Proof**: `WitType` の recursive `List` / `Option` payload bind を含む
  method を wasm32-gc で compile し、`wasm-tools validate
  --features gc,function-references,memory64` と公式 WASI P2 WIT での
  component packaging 後の実行で linear と同一 stdout を確認した。
  `std::wit::ast` の deferral は `WitNode` 固有で、`WitType` には及ばない。
- **Scorecard**: eradication scope 1168 → **1135**（`associated-or-method`
  1054 → **1021**、`must delete` 114 と `intrinsic-only` 1 は不変）。
  manifest 公開 entry 776 → **760**、`pub fn` 宣言 1359 → **1310**、
  unregistered 392 → **375**。`prelude_wrapper` 96 は不変。
- **Evidence**: `stdlib_wit/wit_basic.expected` と `wit_types.expected` は
  wasm32-gc component と `--target wasm32` の両方で一致。
  `world_methods_wasm32` は `wit_methods_wasm32` に改名し、types / scan /
  names / printer の method surface を linear target で実行する。
  全 8 個の `stdlib_wit` fixture は wasm32-gc で compile + validate 済み。
- **Residual**: `std::wit::ast` の 23 宣言、`parser` の 3 free
  (`parse_full` / `parse_full_import_text` / `parse_wit`)、
  `std::component` と `std::component::handle` の family は #917 に残る。
