# ADR-091: prelude の安定 free function 例外と method 移行

ステータス: **ACCEPTED** — method 代替可能な prelude free 27 件を退役し、残す安定 free を 4 件に限定する（2026-10-29 追記: 保留 23 件のうち 11 件を追加退役。末尾の追記を参照）

決定日: 2026-10-29

関連: [ADR-000](ADR-000-process.md)、[ADR-014](ADR-014-stability-labels.md)、[ADR-044](ADR-044-trait-method-syntax-adopted.md)、[ADR-046](ADR-046-free-function-eradication.md)、[ADR-072](ADR-072-language-v2-prelude-and-keyword-policy.md)、[ADR-087](ADR-087-0x-deprecation-exception.md)、[ADR-090](ADR-090-std-namespace-and-panic-association.md)、[#917](../../issues/open/917-stdlib-eradication-execution.md)

## 文脈

[ADR-046](ADR-046-free-function-eradication.md) D1 は、公開 API を trait / method /
associated function に限り、ユーザー可達の free function を残さないことを要求する。
prelude の free function も例外ではなく、D1 が名指しする `push(v, x)` や `eq(a, b)`
の綴りは method / associated へ移す対象である。例外は非公開 intrinsic のみで、それ以外は
個別 ADR または issue で明示許可した場合に限る（D2）。

`docs/stdlib/eradication.md` の scorecard は、`prelude frees (default)` 規則に
54 件を `associated-or-method` として残していた。この 54 件はすべて
`std/manifest.toml` で `stability = "stable"` であり、ADR-087 D2 の 0.x 例外を
適用するには、W0009、migration guide、changelog の 3 条件を満たす必要がある。

54 件の内訳は 3 つに分かれた。

- 既存の receiver method / associated function で置換できるもの
  （`s.trim()`, `(x).abs()`, `parts.join(sep)`, `Env::args()` など）。
- receiver method がまだ存在せず、この wave で追加するもの
  （`String` method / `i32`・`f64` method / `Env`・`Process` associated）。
- receiver を持たない identity helper `values`。これは置換先を持たず、
  `for x in v` のように値を直接使うべきものである。

[ADR-072](ADR-072-language-v2-prelude-and-keyword-policy.md) §1 は v2 の安定
Prelude 関数を `assert` / `String_from` / `String_new` / `char_to_string`
（`panic` は ADR-090 で `Std::panic` へ移動）に固定している。この 4 件は
コンパイラの起動面・アサーション・文字列構築の最小集合であり、単純な method
移行では同じ契約を保てない。

- `String_from` の associated spelling `String::from` は存在しない。
- `String_new` の `String::new` は現行の pin で core Wasm 検証に失敗する
  （`String_new()` は v2 の安定関数として auto-import される）。
- `char_to_string` は `char` を i32 codepoint として渡す既存の境界
  （`char_to_string(48 + n)`）を持ち、`c.to_string()` へ一律に置換すると
  数値の文字列化へ意味が変わる。
- `assert` はレシーバ型を持たない言語面のアサーションであり、
  test / compiler の呼び出しが安定 prelude の一部として自動解決される。

`unwrap` / `is_some` / `is_ok` のような Option / Result の free も同様に
受信型を持つが、generic な `impl<T> Option<T>` は現行 lowering で
compile は通るものの実行時に `unreachable` へ落ちる。concrete 型ごとの impl は
動くが、呼び出し側の全 instantiation を列挙する必要があり、この wave の
replacement-first な完了条件を満たさない。

## 決定

### 1. method / associated で置換できる 27 件を退役する

次の 27 件を `std/manifest.toml` と `std/prelude.ark` から削除し、bare call を
E0100 と `resolver::removed_names` の具体的な replacement hint で失敗させる。

| Free function | Replacement |
|---------------|-------------|
| `abs(x)` | `(x).abs()` |
| `args()` | `Env::args()`（`use std::host::env::Env`） |
| `byte_len(s)` | `s.len_bytes()` |
| `clamp_i32(x, lo, hi)` | `(x).clamp(lo, hi)` |
| `exit(code)` | `Process::exit(code)`（`use std::host::process::Process`） |
| `format_bool(b)` | `(b).to_string()` |
| `format_i32(n)` | `(n).to_string()` |
| `gcd(a, b)` | `(a).gcd(b)` |
| `is_empty(x)` | `x.is_empty()` |
| `join(parts, sep)` | `parts.join(sep)` |
| `lines(s)` | `s.lines()` |
| `max(a, b)` | `(a).max(b)` |
| `min(a, b)` | `(a).min(b)` |
| `pad_left(s, width)` | `s.pad_left(width)` |
| `pad_right(s, width)` | `s.pad_right(width)` |
| `repeat(s, n)` | `s.repeat(n)` |
| `replace(s, from, to)` | `s.replace(from, to)` |
| `split(s, sep)` | `s.split(sep)` |
| `sqrt(x)` | `(x).sqrt()` |
| `to_lower(s)` | `s.to_lower()` |
| `to_lowercase(s)` | `s.to_lower()`（ASCII case fold） |
| `to_upper(s)` | `s.to_upper()` |
| `to_uppercase(s)` | `s.to_upper()`（ASCII case fold） |
| `trim(s)` | `s.trim()` |
| `trim_end(s)` | `s.trim_end()` |
| `trim_start(s)` | `s.trim_start()` |
| `values(v)` | なし。identity helper を削除し、`for x in v` のように値を直接使う |

`parse_i32` / `parse_i64` / `parse_f64` は、compiler / std の `TryFrom`
呼び出しが non-entry module で安定するまで free を残す（§4）。
`byte_len` / `is_empty` / `join` / `lines` / `to_lower*` などは既存の
`String` / `Vec<String>` method を使う。`abs` / `min` / `max` / `clamp` /
`gcd` / `sqrt` は既存の `i32` / `f64` inherent method を使う。
`push_char` は receiver method の lowering が未完成のため退役しない（§4）。

同じ wave で、`std/collections/string.ark` にある対応する 13 個の
`pub fn`（`to_lower` / `to_upper` / `trim` / `trim_start` / `trim_end` /
`repeat` / `pad_left` / `pad_right` / `replace` / `split` / `join` / `lines` /
`push_char`）を非公開または削除する。prelude の entry だけでなく module の
free 綴りも残さないためである。`push_char` は prelude free を残すので、
string module 側の重複宣言だけを削除する。

### 2. ADR-014 / ADR-087 lifecycle の限定例外

27 件はすべて `stable` であるため、本来は ADR-087 D2 の W0009 による
deprecation window が必要である。本 ADR は [ADR-090](ADR-090-std-namespace-and-panic-association.md) §3
と同じ限定例外を、この 27 件にだけ適用する。

- **Migration diagnostic**: 旧名は W0009 ではなく E0100（compile error + 具体的な
  method hint）で止める。削除後の移行診断として W0009 より強い。
- **Migration guide**: `docs/stdlib/migration-guidance.md` に上表の before/after を
  記録する。
- **Changelog**: `docs/stdlib/changelog.md` に日付、ADR-087 / 本 ADR、対象シンボル、
  削除理由、replacement を記録する。

この例外は「prelude の stable を一括削除してよい」という許可ではない。対象を
上表の 27 件に限定し、0.x 期だけに適用する。1.0.0 以降は ADR-014 の通常の
deprecation window に戻る。

### 3. 残す prelude free を 4 件に限定する

ADR-072 §1 の安定集合のうち、次の 4 件だけを ADR-046 D2 の明示的な例外として
prelude free のまま残す。

| Retained free | 理由 |
|---------------|------|
| `assert(cond)` | 言語面の安定アサーション。レシーバ型を持たず、test / compiler が bare call で使う |
| `String_from(s)` | 安定 constructor。`String::from` は未提供 |
| `String_new()` | 安定 constructor。`String::new` は現行 pin で invalid core Wasm になる |
| `char_to_string(c)` | 安定 conversion。i32 codepoint を受ける既存境界を保つ |

この 4 件以外の prelude free function を新設・温存しない。将来 `String::from` /
`String::new` が両 target で実行可能になった場合、および `char_to_string` の
i32 codepoint 境界が解消した場合は、この例外を見直す。

### 4. この wave で退役しない残り

次の 23 件は `associated-or-method` のまま #917 に残す。本 ADR の例外には含めない。

- `push_char`: `s.push_char(c)` は compile・validate するが、String が GC ref で
  method body が新しい array を作るため caller の binding が更新されない。
  free の lowering は caller local を書き換えるので、mutating-receiver の
  lowering が入るまで free を残す（この wave で測定）。
- `parse_i32` / `parse_i64` / `parse_f64`: 呼び出しを `i32::try_from(s)` 等へ
  移すと、再構築した s2 が任意の入力を compile する際に `unreachable` で trap
  した（`compile_source` / `backend_resolve` の計測）。compiler / std module から
  `TryFrom` を使う経路が安定するまで free を残す。fixture 側の移行も同じ理由で
  この wave では行わない。
- `Box_new` / `unbox`: ownership checker が free 名を特別扱いしており、
  method 化には ownership / MIR の変更が必要。
- `unwrap` / `unwrap_or` / `is_some` / `is_none` / `is_ok` / `is_err`:
  generic な `impl<T> Option<T>` / `impl<T, E> Result<T, E>` の receiver method は
  現行 lowering で実行時に `unreachable` へ落ちる。concrete impl の網羅が前提。
- `f32_to_string`: `f32` の `.to_string()` は typechecker の primitive
  `to_string` 契約に含まれず、`f64` との暗黙変換もない（ADR-067）。
- `clone` / `eq` / `byte_at` / `substring` / `to_string` / `starts_with` /
  `ends_with` / `contains` / `index_of` / `slice`: method は存在するが、
  compiler / std 内の呼び出しが大規模で、この wave では移行しない。
  次の機械的移行 wave の対象である。

## 帰結

- prelude の `prelude frees (default)` 規則は 54 件から 27 件へ減る。
- `prelude_wrapper` entry は 64 件から 40 件へ減る。
- `parse_i32` / `parse_i64` / `parse_f64` の free は残り、bare call は gated prelude の
  import hint を返す。
- `values` は機能ごと削除され、comprehension の結果は `for x in v` で直接走査する。
- prelude free の例外は 4 件に固定され、scorecard の `associated-or-method` から
  27 件が外れる。
- 1.0.0 以降は ADR-087 の 0.x 例外が失効するため、この削除根拠を再利用しない。

## 却下した代替案

### 54 件をすべて残す

ADR-046 D1 の「prelude free を恒久 API としない」に反し、scorecard の根絶を止める。
method / associated の置換が存在する 27 件を残す理由がない。

### 54 件をすべて同じ wave で削除する

`clone` / `eq` / `byte_at` などは compiler / std 内の呼び出しが大規模で、
一括削除は review 可能な差分と検証を破壊する。また `String_from` / `assert` /
`char_to_string` は安定 Prelude の起動面であり、置換が同じ契約を満たさない。

### W0009 の deprecation window を先に置く

27 件はすべて既知の method へ機械的に移行でき、E0100 の方が強い診断になる。
0.x の削除実行をさらに 1 wave 遅らせるだけであり、ADR-090 §3 の前例とも
一貫しない。

### Option / Result の free を generic method へ移す

generic な `impl<T> Option<T>` は compile・validate までは通るが、実行時に
`unreachable` へ落ちることをこの wave で測定した。concrete 型ごとの impl を
網羅する別 wave に分ける。

## 再検討条件

- 1.0.0 を切り、ADR-087 の 0.x 例外が失効したとき（本 ADR の削除根拠を再確認する）。
- `String::from` / `String::new` が両 target で実行可能になったとき
  （`String_from` / `String_new` の例外を見直す）。
- `char_to_string` の i32 codepoint 境界が解消したとき。
- generic な Option / Result の receiver method が両 target で実行可能になったとき
  （残る 6 件の free を method 化する）。
- `String::push_char` の mutating-receiver lowering が両 target で使えるようになったとき
  （`push_char` free を退役する）。
- compiler / std module から `i32::try_from` / `i64::try_from` / `f64::try_from` が
  安定して動くようになったとき（`parse_*` free を退役する）。
- 4 件以外の prelude free function を追加したくなったとき
  （本 ADR の例外リストを拡張しない）。

## 追記: 2026-10-29 `wave/prelude-text` — 保留 23 件のうち 11 件の退役

§4 で保留した 23 件のうち、次の 11 件を追加で退役した。§3 の 4 件例外は不変である。

| Retired free | Replacement |
|--------------|-------------|
| `push_char(s, c)` | `s.push_char(c)`（mutating receiver の grow 結果を caller local / field へ書き戻す） |
| `unwrap(o)` / `unwrap_or(o, d)` | `o.unwrap()` / `o.unwrap_or(d)`（`impl Option<i32>`） |
| `is_some(o)` / `is_none(o)` | `o.is_some()` / `o.is_none()` |
| `is_ok(r)` / `is_err(r)` | `r.is_ok()` / `r.is_err()`（`impl Result<i32, i32>`） |
| `parse_i32(s)` / `parse_i64(s)` / `parse_f64(s)` | `i32::try_from(s)` / `i64::try_from(s)` / `f64::try_from(s)`（`use std::core::convert`） |
| `f32_to_string(x)` | `x.to_string()`（`impl f32 { to_string }`） |

`parse_*` の再構築 s2 trap の原因は、bootstrap flat overlay の stdlib closure が
`std/core/convert.ark` を ship しておらず、compiler 自身の `f64::try_from` が
MIR の unresolved-call fallback（`drop; unreachable`）に落ちていたことだった。
overlay に convert を追加し、`try_from` を import せずに呼ぶ経路は E0100 で止める。
`option:<T>` 形式の call-result 型は method 解決時に `Option_<T>` の impl owner へ
正規化する。compiler 自身の `pop(...).unwrap()` は `match` へ移し、pin が bridge を
持たない状態でも s2 / s3 が一致するようにした。

残る 12 件（`Box_new` / `unbox` と text 10 件）は §4 の `associated-or-method` の
まま残す。§4 の「この wave で退役しない残り」は、この追記の 11 件についてのみ
更新されたものとして読む。

## 追記: 2026-10-29 `wave/prelude-text2` — text 10 件のうち 8 件の退役

§4 の text 10 件のうち、receiver method の戻り型・lowering が bootstrap pin で
安定した 8 件を追加で退役した。§3 の 4 件例外は不変である。

| Retired free | Replacement |
|--------------|-------------|
| `byte_at(s, i)` | `s.byte_at(i)` |
| `slice(s, start, end)` | `s.slice(start, end)` |
| `substring(s, start, end)` | `s.slice(start, end)`（同一の `string.slice` CoreOp） |
| `to_string(x)` | `x.to_string()`（`Display` impl） |
| `starts_with(s, prefix)` | `s.starts_with(prefix)` |
| `ends_with(s, suffix)` | `s.ends_with(suffix)` |
| `contains(s, sub)` | `s.contains(sub)` |
| `index_of(s, sub)` | `s.index_of(sub)` |

`std::collections::string` の module-scoped frees は explicit-import の
compatibility 面として残る。prelude の bare call は E0100 で止まり、上の
receiver を指す hint を返す。

残る 4 件は次の理由で退役しない。

- `clone`: free `clone` は typechecker の polymorphic special case であり、
  `Vec<String>` / `Vec<TypeInfo>` / `AstNode` など `Clone` impl を持たない値の
  唯一の copy 経路である。`s.clone()` は String / scalar / 一部 collection にしか
  解決せず、compiler 自身の clone を method へ一括移行すると bootstrap が
  compile できない。
- `eq`: `String` の `==` は operator を `eq` method へ rewrite して lowering する。
  bootstrap flat overlay は `std::core::cmp`（`impl Eq for String`）を ship して
  いないため、free `eq` を消すと pin が `==` を `i32.eq`（参照比較）へ落とし、
  invalid core Wasm を生成する。overlay の Eq dispatch が安定するまで残す。
- `Box_new` / `unbox`: generic enum payload の associated function / method が
  pin の lowering で `T` を推論できず（`cannot infer type parameter T`）、
  `Box::new` は resolver が enum variant として解決する。ownership checker の
  名前 special-case だけでは置換できない。ADR-088 §6 の borrowed-Box 拒否も
  同じ名前 boundary に依存する。
- `to_string` は退役済みだが、`String` の `Display` impl は
  `std::core::convert`（overlay ship 済み）に依存する。

## 追記: 2026-10-30 `wave/poly-clone` — free `clone` の退役

§4 の `clone` を退役した。§3 の prelude free 例外は 4 件から 3 件
（`eq`、`Box_new` / `unbox`、`to_string` の Display 依存）へ減る。

- **Root cause**: `.clone()` の bare fallback は `(String) -> String` の free 宣言を
  拾っていた。generic な shim を置いても、`vec:String` receiver は共有 callee
  cache で `String::clone` に解決され、Vec struct を String ref 引数へ渡して
  `type mismatch: expected (ref null $type), found (ref null $type)` の invalid
  core Wasm になった（`wasm-tools validate` の func 単位の型不一致）。
- **Mechanism**: `std/prelude.ark` に private な generic identity shim
  `fn clone<T>(value: T) -> T` と、`impl<T> Vec<T> { fn clone(self) -> Vec<T> }`
  （shallow。header を共有し element storage は deep copy しない）を置いた。
  `clone` を module graph に常時存在させることで、compiler 自身の `.clone()` は
  pin と stage-2/3 で同じ lowering になる。typechecker / MIR のロジックは変えて
  いない。committed pin が polymorphic method 解決を持たないため、この境界が
  bootstrap の成立条件である。
- **Deleted**: `std::collections::string::clone` は retired CoreOp 経由で
  `wasm unreachable` に落ちていたため削除した。`s.clone()` が replacement。
- **Migration**: compiler / std / fixture の 761 ファイル、14,057 call site を
  receiver method へ移し、747 件の selective import から `clone` を外した。
- **Diagnostics**: bare `clone(x)` は E0100 + `use \`x.clone()\` on the value
  instead`、`use std::prelude::{clone}` も E0100 で拒否する。
  `diagnostics/removed_prelude_free_clone` / `removed_prelude_import_clone` の
  golden を追加した。`std::collections::string::clone` の qualified call も
  module member hint を返す。
- **Delete condition (shim)**: pin を refresh し、bare-name signature なしで
  `.clone()` が全 receiver に解決できるようになったら private な
  `fn clone<T>` を削除する。`Vec<T>::clone` の shallow 実装は、`T: Clone` を
  要求する deep impl に置き換えるまで残す（compiler の AST / MIR record は
  `Clone` impl を持たない）。
- **Evidence**: `verify fixtures` 1924/1924 PASS（38/38 `test:`）、`verify lane`
  PASS、`docs check` 6/6、ADR-029 fixpoint PASS（s2 == s3 at sha256
  `9cec2ac2523c567cd6b8d4d88d9ada2a7f3bb202fb12dca92062f1783cf9b0bd`、pinned base
  `471855ad9db035c1ada136852fc55e896b477cf35303be066f352d9d563a8895`）。

## 追記: 2026-10-30 `wave/deep-clone` — `Vec<T>` deep clone

`Vec<T>::clone` の shallow な prelude inherent 実装を、trait impl による deep
clone へ置き換えた。prelude の identity shim は pin 用に残す。

- **Deep impl**: `std/core/clone.ark` に
  `impl<T: Clone> Clone for Vec<T>` を追加した。新しい `Vec<T>` を確保して
  element ごとに `T::clone` を呼ぶため、要素配列も `String` 要素の buffer も
  共有しない。`use std::core::clone` が receiver の impl を登録する。
- **No-import fallback**: checked-in prelude から
  `impl<T> Vec<T> { fn clone }` を削除した。import なしの `.clone()` は
  MIR の `mir_emit_clone_identity`（`src/compiler/mir/lower/method_emit.ark`）が
  identity の local copy へ落とす。bare callee が共有 cache で
  `String::clone` に化けて invalid core Wasm になっていた #928 の Vec 側を
  ここで遮断する。意味は shallow（header 共有）だが module は valid。
- **Bootstrap overlay**: pin は上記 MIR fallback より前の compiler なので、
  overlay prelude にだけ旧 shallow inherent
  `impl<T> Vec<T> { fn clone(self) -> Vec<T> { self } }` を注入する
  （`_patch_bootstrap_prelude_vector_clone_shallow_text`）。checked-in prelude と
  user-visible semantics は bridge を持たない。`std/core/clone.ark` も overlay に
  ship する。
- **Residual**: compiler 自身の `Vec` clone は overlay の shallow bridge を
  通り、import なしの user code も shallow のまま。element の deep copy は
  要素型の `Clone` impl に依存する（`String` は deep、`Clone` impl のない
  user struct は handle copy）。pin の fixture-parity oracle は no-import の
  Vec clone で invalid wasm を出し得るが、current s2 の PASS 判定とは分離する。
- **Delete condition**: pin を refresh して MIR identity fallback が pin 側にも
  入ったら overlay bridge を削除する。`use std::core::clone` を prelude へ
  移せるようになったら no-import fallback も不要になる。
- **Evidence** (`ARUKELLT_BUILD_DIR=$PWD/.build-lane`、gc pin):
  `selfhost build-compiler` PASS（final source build 56.6s）、`verify fixtures`
  PASS=1928 FAIL=0（38/38 `test:`、111 runtime smoke、2 P2-unsupported。
  `vec_clone_deep` と `vec_clone_deep_wasm32` を含む）、`verify lane` PASS、
  `docs check` 6/6、ADR-029 fixpoint PASS（s2 == s3 at sha256
  `cf6e5f3c6ed2a9daef1c1df36644dec9259239a129ad6225a32179abf49c4f99`、
  pinned base `471855ad9db035c1ada136852fc55e896b477cf35303be066f352d9d563a8895`）。
  修正前は `vec_clone_deep` が `expected "alpha", got "changed"` で失敗し、
  clone の `set` が source を書き換えていた。

## 参照

- [ADR-014](ADR-014-stability-labels.md)
- [ADR-044](ADR-044-trait-method-syntax-adopted.md)
- [ADR-046](ADR-046-free-function-eradication.md)
- [ADR-072](ADR-072-language-v2-prelude-and-keyword-policy.md)
- [ADR-087](ADR-087-0x-deprecation-exception.md)
- [ADR-090](ADR-090-std-namespace-and-panic-association.md)
- [`std/prelude.ark`](../../std/prelude.ark)
- [`std/manifest.toml`](../../std/manifest.toml)
- [migration-guidance.md](../stdlib/migration-guidance.md)
- [changelog.md](../stdlib/changelog.md)
- [eradication.md](../stdlib/eradication.md)
