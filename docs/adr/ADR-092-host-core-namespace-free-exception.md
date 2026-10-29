# ADR-092: host/core 名前空間型への free function 退役と E0100 移行例外

ステータス: **ACCEPTED** — 25 件の stable host/core free function を名前空間型の associated function へ移し、E0100 で退役させる

決定日: 2026-10-29

関連: [ADR-000](ADR-000-process.md)、[ADR-014](ADR-014-stability-labels.md)、[ADR-044](ADR-044-trait-method-syntax-adopted.md)、[ADR-046](ADR-046-free-function-eradication.md)、[ADR-087](ADR-087-0x-deprecation-exception.md)、[ADR-090](ADR-090-std-namespace-and-panic-association.md)、[ADR-091](ADR-091-prelude-free-function-exception.md)、[#917](../../issues/open/917-stdlib-eradication-execution.md)

## 文脈

[ADR-046](ADR-046) D1 は、レシーバを持たない公開操作も free function のまま残さず、型または
モジュール名前空間の associated function へ寄せることを要求している。例外は非公開 intrinsic
のみである（D2）。`docs/stdlib/eradication.md` の `host/core namespace frees` 規則は、
`std::signal` / `std::cli` / `std::host::random` / `std::host::process` / `std::process` の
28 件を `associated-or-method` として残していた。

28 件のうち 25 件は `stable` の公開エントリであり、[ADR-014](ADR-014) と
[ADR-087](ADR-087) D2 は通常、削除前に W0009 の deprecation window、migration guide、
changelog の 3 条件を要求する。残る 3 件（`std::host::fs` の `fd_seek` / `fd_tell` /
`fd_fdstat_errno`）は `experimental` で、`wasm32` / WASI P1 専用であり、wasm32-gc の
parity fixture を持たない。これらを `Fs` へ移すと primary target で lower しない表面を
約束することになるため、本 ADR の対象外として据え置く。

25 件の置換先はすべて既に callable な名前空間型として実装できる。

- `std::host::process` の 3 件と legacy `std::process` の 2 件は `Process`。
- `std::host::random` の 4 件は `Random`。
- `std::cli` の 6 件は、引数ベクトル読み取りが既存の `Env`、明示ベクトル走査が新しい `Cli`。
- `std::signal` の 10 件は `Signal`。

[ADR-090](ADR-090) §3 と [ADR-091](ADR-091) §2 は、replacement が callable で検証済みの
場合に限り、ADR-087 D2 の W0009 条件をより強い E0100 + 具体的 replacement hint で置き換える
前例を作った。本 ADR は同じ限定例外を、この 25 件にだけ適用する。

## 決定

### 1. 名前空間型を正規 surface にする

次の 4 つの名前空間型を stable 相当の置換 surface として実装し、manifest の `[[types]]` に
登録する。stability は既存の `Env` / `Clock` / `Fs` に合わせて `provisional` とする。

| 型 | module | associated functions |
|----|--------|----------------------|
| `Process` | `std::host::process` | `exit` / `abort` / `id` |
| `Random` | `std::host::random` | `random_i32` / `random_i32_range` / `random_bool` / `next_f64` |
| `Cli` | `std::cli` | `flag` / `option` |
| `Signal` | `std::signal` | `pi` / `tau` / `sin_approx` / `cos_approx` / `clone_f64` / `zeros` / `fft` / `ifft` / `magnitude` / `approx_eq` |

`std::cli` の引数ベクトル読み取りは複製せず、既存の `Env::args` / `Env::arg_count` /
`Env::arg_at` / `Env::has_flag` を canonical とする。`std::process` は
`pub use std::host::process::Process` の compatibility facade として残し、free `exit` /
`abort` は持たない。

### 2. 25 件の free function を退役させる

次の 25 件を `std/manifest.toml` と std 宣言から削除する。

| module | 退役する free | replacement |
|--------|----------------|-------------|
| `std::host::process` | `exit` / `abort` / `id` | `Process::exit(code)` / `Process::abort()` / `Process::id()` |
| `std::process` | `exit` / `abort` | `Process::exit(code)` / `Process::abort()` |
| `std::host::random` | `random_i32` / `random_i32_range` / `random_bool` / `next_f64` | `Random::random_i32()` / `Random::random_i32_range(lo, hi)` / `Random::random_bool()` / `Random::next_f64()` |
| `std::cli` | `parse_args` / `arg_count` / `arg_at` / `has_flag` | `Env::args()` / `Env::arg_count()` / `Env::arg_at(index)` / `Env::has_flag(flag)` |
| `std::cli` | `flag` / `option` | `Cli::flag(args, name)` / `Cli::option(args, name)` |
| `std::signal` | `pi` / `tau` / `sin_approx` / `cos_approx` / `clone_f64` / `zeros` / `fft` / `ifft` / `magnitude` / `approx_eq` | 同名の `Signal::*` associated function |

旧 qualified call は E0100 と `resolver::removed_names` の具体的 hint で失敗させる。
`std::process` の warning（W0002）は facade の移動告知として維持する。

### 3. ADR-014 / ADR-087 lifecycle の限定例外

25 件の削除は、W0009 の deprecation window を置かずに同じ wave で行う。
[ADR-087](ADR-087) D2 の 0.x 例外のうち、次を明示的に満たすものとして扱う。

- **Migration diagnostic**: 旧名の利用は W0009（warning）ではなく E0100（compile error +
  replacement hint）で止まる。削除後の移行診断として W0009 より強い。
- **Migration guide**: `docs/stdlib/migration-guidance.md` に全 25 件の before/after と
  replacement を記録する。
- **Changelog**: `docs/stdlib/changelog.md` に日付、ADR-087 / 本 ADR、対象シンボル、
  削除理由、replacement を記録する。

この例外は「host/core の stable を一括削除してよい」という許可ではない。対象は上表の
25 件に限定し、0.x 期だけに適用する。1.0.0 以降は ADR-014 の通常の deprecation window に戻る。

### 4. 保持する正確な例外（3 件）

`std::host::fs` の `fd_seek` / `fd_tell` / `fd_fdstat_errno` は `experimental` のまま
free function として残す。これらは `wasm32` / WASI P1 専用で wasm32-gc の parity fixture を
持たず、`Fs` へ移すと primary target で lower しない表面を約束することになる。非公開の
`__intrinsic_fd_*` bridge は維持する。本 ADR の例外リストはこの 3 件であり、他の
host/core free function には適用しない。

## 帰結

- `Process` / `Random` / `Cli` / `Signal` が host/core の唯一の公開 surface になり、
  `std::cli` の引数読み取りは `Env` に一本化される。
- scorecard の `host/core namespace frees` は 28 件から 3 件（fd 例外）に減り、
  `associated-or-method` は 25 件減る。
- 旧 call site は E0100 の hint で機械的に移行できる。`std::process` の facade は既存の
  W0002 移動告知を保つ。
- 3 件の experimental fd helper は残るため、host/core の根絶は完了しない。これは
  target-parity fixture が揃うまで維持する残余である。

## 却下した代替案

### W0009 の deprecation window を先に置き、削除を次 wave に送る

置換先がすべて callable で、E0100 の方が強い移行診断になるため採らない。ADR-090 / ADR-091
と同じ限定例外の前例に揃える。

### 3 件の fd helper も `Fs` へ移す

`wasm32-gc` で lower しない surface を primary target の namespace に置くことになり、
target-parity の主張を偽るため採らない。experimental のまま残余として記録する。

### `Cli::flag` / `Cli::option` を `Vec<String>` の receiver method にする

`Vec<String>` の method surface は `join` など既に大きく、引数走査の追加は
`std::cli` のモジュール責務と一致しない。明示ベクトルを取る名前空間型の方が契約が一意になる。

### `Signal` の FFT を `Vec<f64>` の mutating method にする

`fft` / `ifft` は real / imag の 2 本の平行バッファを同時に更新する。片方を receiver に
すると他方が暗黙の第 2 バッファになり、ADR-088 の mut 引数境界を曖昧にする。名前空間型の
associated function として 2 本を対等に渡す形を維持する。

## 再検討条件

- 3 件の fd helper に wasm32-gc の parity fixture が付き、`Fs` か専用型へ移せるようになったとき。
- 1.0.0 を切り、ADR-087 の 0.x 例外が失効したとき（本 ADR の削除根拠を再確認する）。
- E0100 の移行診断が不十分で、W0009 の deprecation window が必要と判断されたとき。
- 名前空間型へ無関係な操作を追加したくなったとき（型の責務を再検討する）。

## 参照

- [ADR-014](ADR-014-stability-labels.md)
- [ADR-044](ADR-044-trait-method-syntax-adopted.md)
- [ADR-046](ADR-046-free-function-eradication.md)
- [ADR-087](ADR-087-0x-deprecation-exception.md)
- [ADR-090](ADR-090-std-namespace-and-panic-association.md)
- [ADR-091](ADR-091-prelude-free-function-exception.md)
- [#917](../../issues/open/917-stdlib-eradication-execution.md)
- [`std/manifest.toml`](../../std/manifest.toml)
- [`docs/data/stdlib-api-eradication.toml`](../data/stdlib-api-eradication.toml)
- [eradication.md](../stdlib/eradication.md)
- [migration-guidance.md](../stdlib/migration-guidance.md)
- [changelog.md](../stdlib/changelog.md)
