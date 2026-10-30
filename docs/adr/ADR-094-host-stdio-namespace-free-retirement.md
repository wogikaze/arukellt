# ADR-094: host stdio の `Stdio` 名前空間移行と free function 退役

ステータス: **ACCEPTED** — `std::io` / `std::host::stdio` の stable free 9 件を退役し、`Stdio` 名前空間型へ移す

決定日: 2026-10-30

関連: [ADR-000](ADR-000-process.md)、[ADR-014](ADR-014-stability-labels.md)、[ADR-044](ADR-044-trait-method-syntax-adopted.md)、[ADR-046](ADR-046-free-function-eradication.md)、[ADR-085](ADR-085-io-trait-unification.md)、[ADR-087](ADR-087-0x-deprecation-exception.md)、[ADR-092](ADR-092-host-core-namespace-free-exception.md)、[#693](../../issues/open/693-read-write-traits-io-unification.md)、[#917](../../issues/open/917-stdlib-eradication-execution.md)

## 文脈

[ADR-046](ADR-046) D1 は、ユーザー可達の free function を残さず、レシーバを持たない
操作も型またはモジュール名前空間の associated function へ寄せることを要求する。例外は
非公開 intrinsic のみである（D2）。

[ADR-085](ADR-085) は `Read` / `Write` / `BufRead` / `Seek` trait と typed な
`std::io::Stdout` / `Stderr` を導入したが、Phase E/F の互換面として次の 9 件の stable
free function を残した。

| module | free | 現行の実装 |
|--------|------|-----------|
| `std::host::stdio` | `read_stdin` | `__intrinsic_stdin_read_to_string` |
| `std::host::stdio` | `print` | `__intrinsic_print` |
| `std::host::stdio` | `println` | `__intrinsic_println` |
| `std::host::stdio` | `eprintln` | `__intrinsic_eprintln` |
| `std::io` | `stdout` | タグ `1` の `Vec<i32>` |
| `std::io` | `stderr` | タグ `2` の `Vec<i32>` |
| `std::io` | `print_bytes` | バイト列を文字列化して `__intrinsic_print` |
| `std::io` | `stdin` | 空の `Vec<i32>`（WASI stdin 未接続） |
| `std::io` | `read_stdin_line` | 常に `Ok("")`（WASI stdin 未接続） |

`docs/stdlib/eradication.md` の `std::io and host stdio frees` 規則は、この 9 件を
`associated-or-method` として残していた。9 件はすべて `stable` であり、[ADR-014](ADR-014)
と [ADR-087](ADR-087) D2 は通常、削除前に W0009 の deprecation window、migration guide、
changelog の 3 条件を要求する。

## 決定

### 1. `Stdio` 名前空間型を正規 surface にする

`std::host::stdio` に `Stdio`（`kind = "struct"`、`provisional`）を追加し、次の
associated function を公開する。

| 型 | associated functions |
|----|----------------------|
| `Stdio` | `read_stdin` / `print` / `println` / `eprintln` |

`read_stdin` は stdin 全体を `String` として読む。`print` / `println` /
`eprintln` は文字列を stdout / stderr へ書く（`println` / `eprintln` は改行を付ける）。
バイト忠実な sink は既存の `std::io::Stdout` / `Stderr` のままとし、この型へ統合しない。

### 2. 9 件の free function を退役させる

次の 9 件を `std/manifest.toml` と std 宣言から削除する。

| module | 退役する free | replacement |
|--------|----------------|-------------|
| `std::host::stdio` | `read_stdin` | `Stdio::read_stdin()` |
| `std::host::stdio` | `print` | `Stdio::print(s)` |
| `std::host::stdio` | `println` | `Stdio::println(s)` |
| `std::host::stdio` | `eprintln` | `Stdio::eprintln(s)` |
| `std::io` | `stdout` | `io::Stdout::new()` |
| `std::io` | `stderr` | `io::Stderr::new()` |
| `std::io` | `print_bytes` | `io::Stdout::new().write(data)` |
| `std::io` | `stdin` | 置換なし。旧 API は空のタグ付き `Vec<i32>` で、byte handle として lower していなかった。stdin 全体は `Stdio::read_stdin()` で読む |
| `std::io` | `read_stdin_line` | 置換なし。行読みは lower しておらず、旧 API は常に `Ok("")` を返すプレースホルダーだった。stdin 全体は `Stdio::read_stdin()` で読む |

`stdin` / `read_stdin_line` は「置換先の都合で残す」のではなく、存在しなかった動作を
約束していた surface として削除する。ADR-046 D1 が禁じる偽の成功 API を温存しない。

旧 qualified call は E0100 と `resolver::removed_names` の具体的 hint で失敗させる。
bare `println` / `print` / `eprintln` の prelude 退役 hint も `Stdio::*` へ向ける。

### 3. ADR-014 / ADR-087 lifecycle の限定例外

9 件の削除は、W0009 の deprecation window を置かずに同じ wave で行う。
[ADR-087](ADR-087) D2 の 0.x 例外のうち、次を明示的に満たすものとして扱う。

- **Migration diagnostic**: 旧名の利用は W0009（warning）ではなく E0100（compile error +
  replacement hint）で止まる。削除後の移行診断として W0009 より強い。
- **Migration guide**: `docs/stdlib/migration-guidance.md` に全 9 件の before/after と
  replacement（`stdin` / `read_stdin_line` は置換なしの理由を含む）を記録する。
- **Changelog**: `docs/stdlib/changelog.md` に日付、ADR-087 / 本 ADR、対象シンボル、
  削除理由、replacement を記録する。

この例外は「host I/O の stable を一括削除してよい」という許可ではない。対象は上表の
9 件に限定し、0.x 期だけに適用する。1.0.0 以降は ADR-014 の通常の deprecation window に戻る。

### 4. 保持する surface

`std::io` の `IoError`、`Read` / `Write` / `BufRead` / `Seek` trait、`Reader` /
`Writer` / `BufReader` / `BufWriter` / `Io`、`Stdout` / `Stderr` は変更しない。
`Stdio` は `Stdout` / `Stderr` の置換ではなく、レシーバを持たない文字列ストリーム操作の
名前空間である。

### 5. bootstrap overlay の扱い

flat bootstrap overlay は pin が `std::host::stdio` を builtin として解決していたため、
これまで `std/host/stdio.ark` を同梱していなかった。`Stdio::*` は builtin ではないため、
overlay は `std/host/stdio.ark` を同梱し、entry module（`main.ark`）だけは pin が解決
できる prelude の `println` stub へ書き換える。この bridge は pin 更新時に削除する。

## 帰結

- `std::io` / `std::host::stdio` にユーザー可達の free function は残らない。
  `Stdio` が host 標準ストリームの唯一の文字列 API になる。
- scorecard の public free は 41 件から 32 件へ減り、`associated-or-method` は 16 件から
  7 件へ減る。`std::io and host stdio frees` 規則は live match を失うため削除する。
- `stdin` / `read_stdin_line` の偽の成功 surface が消える。stdin 全体の読み取りは
  `Stdio::read_stdin()` に一本化される。
- 旧 call site は E0100 の hint で機械的に移行できる。
- `stdout()` / `stderr()` のタグ付き `Vec<i32>` 前提コードは、typed な
  `Stdout` / `Stderr` へ移行するまでコンパイルできない。これは意図した削除である。

## 却下した代替案

### `print` / `println` / `eprintln` を `Stdout` / `Stderr` の method にする

`println` はストリーム値ではなく出力先の指定を伴わない操作であり、呼び出しごとに
`Stdout::new()` を要求すると receiver の所有権と無関係な一時値を強いる。ADR-092 が
`Process` / `Env` をモジュール名前空間型にしたのと同じ理由で、`Stdio` の associated
function を採る。

### `stdin` / `read_stdin_line` を `Stdio` へ移す

両者は WASI stdin の byte handle / 行読みを lower しておらず、`Stdio::stdin()` を
追加すると「lower しない surface を公開する」ことになる。ADR-046 D1 の偽の成功 API
禁止に反するため、置換なしで退役する。

### `stdin()` を `Reader::new()` に置き換える

`Reader::new()` はメモリ上の EOF reader であり、stdin ではない。契約の異なる型を
同じ名前で提示すると、利用者は stdin からの読み取りを期待し続ける。採らない。

### 9 件を ADR 例外として free のまま残す

`std::host::fs` の experimental `fd_*` 3 件は wasm32 / WASI P1 専用で wasm32-gc の
parity fixture を持たないため残す（[ADR-092](ADR-092) §4）。本 9 件は wasm32-gc と
wasm32 の両方で lower するか、もともと lower しないプレースホルダーであり、同じ例外
理由は当てはまらない。採らない。

## 再検討条件

- WASI stdin の byte handle / 行読みが lower し、`Stdio` か専用型へ honest に移せるように
  なったとき。
- 1.0.0 を切り、ADR-087 の 0.x 例外が失効したとき（本 ADR の削除根拠を再確認する）。
- `Stdio` へ無関係な操作を追加したくなったとき（型の責務を再検討する）。
- bootstrap overlay の stdio bridge が不要になったとき（pin 更新時に削除する）。

## 参照

- [ADR-014](ADR-014-stability-labels.md)
- [ADR-044](ADR-044-trait-method-syntax-adopted.md)
- [ADR-046](ADR-046-free-function-eradication.md)
- [ADR-085](ADR-085-io-trait-unification.md)
- [ADR-087](ADR-087-0x-deprecation-exception.md)
- [ADR-092](ADR-092-host-core-namespace-free-exception.md)
- [#693](../../issues/open/693-read-write-traits-io-unification.md)
- [#917](../../issues/open/917-stdlib-eradication-execution.md)
- [`std/manifest.toml`](../../std/manifest.toml)
- [`docs/data/stdlib-api-eradication.toml`](../data/stdlib-api-eradication.toml)
- [eradication.md](../stdlib/eradication.md)
- [migration-guidance.md](../stdlib/migration-guidance.md)
- [changelog.md](../stdlib/changelog.md)
