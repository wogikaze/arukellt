# ADR-085: `std::io` の `Read` / `Write` / `BufRead` / `Seek` による I/O 統合

ステータス: **ACCEPTED** — `Vec<i32>` alias を型付きハンドルへ移行し、静的ディスパッチの I/O trait を `std::io` の正規 API とする

提案日: 2026-10-25

決定日: 2026-10-25

> 2026-10-30 追補: D9 が残した互換 free 9 件は
> [ADR-094](ADR-094-host-stdio-namespace-free-retirement.md) で退役した。現行 surface は
> `std::host::stdio::Stdio` と `std::io::Stdout` / `Stderr` であり、trait の決定自体は
> 変わらない。

関連: [ADR-014](ADR-014-stability-labels.md)、[ADR-036](ADR-036-trait-stdlib-redesign.md)、[ADR-044](ADR-044-trait-method-syntax-adopted.md)、[ADR-046](ADR-046-free-function-eradication.md)、[ADR-048](ADR-048-design-heuristics-application-order.md)
関連: [ADR-054](ADR-054-host-linker-and-rust-runtime-retirement.md)、[ADR-059](ADR-059-language-v2-ownership-copy-mutability.md)、[ADR-060](ADR-060-language-v2-owned-adts-box-slice.md)、[ADR-062](ADR-062-language-v2-trait-core-coherence.md)
関連: [ADR-064](ADR-064-error-trait-namespace-and-source-boundary.md)、[ADR-076](ADR-076-language-v2-iterator-consumer-boundary.md)、[ADR-078](ADR-078-language-v2-parametric-trait-impl.md)、[ADR-079](ADR-079-language-v2-method-level-generics.md)、[ADR-084](ADR-084-language-v2-fallible-conversion-dispatch.md)
関連: [#688](../../issues/done/688-trait-method-dispatch.md)、[#692](../../issues/done/692-clone-default-from-into-traits.md)、[#693](../../issues/open/693-read-write-traits-io-unification.md)、[#694](../../issues/done/694-error-trait-unified-ecosystem.md)、[#703](../../issues/open/703-monomorphic-api-bold-cutover.md)、[#718](../../issues/open/718-stdlib-free-function-method-migration.md)

---

## 文脈

`std::io` は `Reader` / `Writer` / `BufReader` / `BufWriter` を `Vec<i32>` の
レイアウト約束として扱っている（`[cursor, b0, ...]` / `[fd_tag, b0, ...]`）。
型としてはすべて `Vec<i32>` であり、同じ型に `Read` と `Write` の両方を
実装すると source と sink の意味が衝突する。`Seek` と `BufRead` も cursor の
解釈を共有できない。そのため file / memory / stdio を同じ抽象で扱えず、
`io::copy<R: Read, W: Write>` のような generic な合成も表現できない。

#693 の初期実装（`d4c5f09f6`）は `Read` / `Write` / `BufRead` / `Seek` の
宣言と `Vec<i32>` への impl、free な補助関数、monomorphic な `copy_trait` を
追加した。しかし trait は `pub` ではなく、公開面は引き続き free function で、
`copy_trait` は generic な `copy` の代替になっていない。本 ADR はこの形を
正規の設計として採用せず、公開 trait と型付きハンドルへ置き換える。

言語側は trait / `impl` / method と monomorphization による静的ディスパッチを
持つ（[ADR-044](ADR-044-trait-method-syntax-adopted.md)、
[ADR-062](ADR-062-language-v2-trait-core-coherence.md)、
[ADR-078](ADR-078-language-v2-parametric-trait-impl.md)、
[ADR-079](ADR-079-language-v2-method-level-generics.md)）。
`dyn Trait`、`&mut T`、default method は採択されていない。所有権は
[ADR-059](ADR-059-language-v2-ownership-copy-mutability.md)、owned ADT は
[ADR-060](ADR-060-language-v2-owned-adts-box-slice.md)、error 境界は
[ADR-064](ADR-064-error-trait-namespace-and-source-boundary.md) と
[ADR-084](ADR-084-language-v2-fallible-conversion-dispatch.md) に従う。

host 側は [ADR-054](ADR-054-host-linker-and-rust-runtime-retirement.md) により
公式 WASI 境界だけを使う。`std::host::fs` は path 全体の read / write と read
probe、実験的な `fd_seek` / `fd_tell` / `fd_fdstat_get` を公開するが、open /
read / write / close の fd surface は持たない。`File` を trait に接続するには、
この host capability の境界を先に決める必要がある。

さらに `src/compiler/lint/deprecated_table.ark` は `std::io` を
`std::host::stdio` へ移転済みとして案内しているが、`std/manifest.toml` は
`std::io` の関数を stable として公開しており、実際の import も解決される。
抽象 module と host stdio の責務が lint / docs / manifest で一致していない。

## 提案する決定

### D1: 抽象は `std::io`、host 実装は `std::host` に置く

- `std::io` は target-independent な trait、`SeekFrom`、`IoError`、memory 用の
  `Reader` / `Writer`、generic adapter `BufReader` / `BufWriter`、generic
  algorithm の名前空間 `Io` を持つ。
- host に依存する型は `std::host` 側に置く。`std::host::fs::File` は
  `std::io` の trait を実装する。新しい `std::io` 表面は `std::host` を
  import しない。既存の host-backed 関数は互換 wrapper として分離し、削除時に
  host 依存も一緒に外す。
- 既存の `std::io::stdout()` / `stderr()` / `stdin()` など host-backed な
  関数は、移行期間の compatibility wrapper として残す。新しい host stdio の
  正規形は `std::host::stdio` の型に対する `Write` / `Read` 実装であり、
  削除条件は #693 が所有する。
- `std::io` を「`std::host::stdio` へ移転済み」とする lint / docs の案内は
  誤りであり、実装時に削除する。抽象 module の名前は `std::io` のままとする。

### D2: trait signature と receiver

```text
pub trait Read {
    fn read(self: Self, max_bytes: i32) -> Result<Vec<i32>, IoError>
}

pub trait Write {
    fn write(self: Self, data: Vec<i32>) -> Result<i32, IoError>
    fn flush(self: Self) -> Result<(), IoError>
}

pub trait BufRead: Read {
    fn fill_buf(self: Self) -> Result<Vec<i32>, IoError>
    fn consume(self: Self, amount: i32) -> Result<(), IoError>
    fn read_line(self: Self) -> Result<Option<String>, IoError>
}

pub enum SeekFrom {
    Start(i64),
    Current(i64),
    End(i64),
}

pub trait Seek {
    fn seek(self: Self, from: SeekFrom) -> Result<i64, IoError>
    fn position(self: Self) -> Result<i64, IoError>
}
```

- trait 宣言の receiver は `self: Self`、`impl` では具象型を書く。
  `Iterator` / `IntoIterator` と同じ形である。
- receiver は by-value 表記だが、method 呼び出しは receiver を move しない。
  handle の可変状態は handle の同一性に属し、同じ handle への複数回の
  `read` / `write` が状態を共有する。これは `Vec::push` や
  `HashMap::insert` と同じ既存の method 意味論であり、ADR-059 の binding
  move とは独立である。
- handle 型は `Copy` にしない。adapter へ move すると所有権は adapter に移り、
  元の binding は使えない。`let mut` は binding の再代入にだけ必要で、handle の
  内部状態の更新には要求しない。
- default method は使わない（ADR-062 が範囲外としている）。trait の method は
  すべて required とし、複数型に共通の algorithm は `Io` の associated
  function に置く（D4）。
- `Read::read` は最大 `max_bytes` バイトを読み、実際に読んだバイト列を返す。
  `max_bytes` は非負でなければならず、負値は `IoError::Other` とする。
  EOF は空の `Vec` で表し、`UnexpectedEof` は `Io::read_exact` のように
  「ちょうど N バイト必要」な操作だけが返す。
- `Write::write` は受け付けたバイト数を返す。`flush` は buffer の確定境界である。
- `BufRead::fill_buf` は未消費バイトの owned snapshot を返す。borrow を返さない
  理由は D5 と代替案に記す。`consume` の `amount` は非負で、直前の
  `fill_buf` の範囲を超えてはならない。違反は `IoError::Other` とする。
  `read_line` は改行を含めない行を返し、EOF では `None` を返す。空行と EOF を
  区別する。
- `Seek` は絶対位置ではなく `SeekFrom` を取り、新しい絶対位置を返す。負の
  実効位置は `IoError::Other` とする。旧 `seek_to` の clamp は compatibility
  wrapper の挙動としてのみ残す。

### D3: `Vec<i32>` alias を newtype に置き換える

- `Reader` / `Writer` を `Vec<i32>` の alias ではなく newtype にする。
  - `Reader`: memory の読み取りハンドル。`Read` / `BufRead` / `Seek` を実装。
  - `Writer`: memory の書き込みハンドル。`Write` を実装。
- 内部表現は private とし、`[cursor, ...]` / `[fd_tag, ...]` のレイアウトを
  公開契約にしない。既存の `Vec<i32>` API との互換のため、wrapper の内部では
  同じレイアウトを使ってよい。
- `Reader::from_bytes(data)`、`Reader::new()`、`Reader::len()`、
  `Reader::is_eof()`、`Reader::into_bytes()`、`Writer::new()`、
  `Writer::into_bytes()`、`Writer::into_reader()` を用意する。
  `Writer::into_reader` は書き込み済みバイトを cursor 0 の `Reader` にする。
- `impl Read for Vec<i32>` / `impl Write for Vec<i32>` は削除する。
  `Vec<i32>` は複数の layout を同時に表せず、`Read` と `Write` の impl が
  同じ型で衝突する。raw の `Vec<i32>` を stream として使う場合は
  `Reader::from_bytes` で明示的に包む。
- 既存の `reader_*` / `writer_*` 関数は移行期間は同じ signature のまま残し、
  newtype への wrapper とする。stable の削除は ADR-014 に従う。

### D4: generic algorithm は `Io` の associated function にする

```text
pub struct Io {}

impl Io {
    fn copy<R: Read, W: Write>(from: R, to: W) -> Result<i64, IoError>
    fn read_exact<R: Read>(from: R, amount: i32) -> Result<Vec<i32>, IoError>
    fn read_to_end<R: Read>(from: R) -> Result<Vec<i32>, IoError>
    fn write_all<W: Write>(to: W, data: Vec<i32>) -> Result<(), IoError>
}
```

- `Io::copy` は `from.read(chunk)` が空を返すまで読み、`to.write` が全量を
  受け付けるまで繰り返す。途中で error が返ったらその error を返し、部分
  コピー数を成功値として返さない。非空の `data` に対して `write` が 0 を
  返した場合は、無限 loop にせず `IoError::Other` を返す。
- `read_line` は `BufRead` の method をそのまま使うため、`Io` には重複して
  置かない。`Io` は receiver を取らない generic algorithm だけを持つ。
- `io::copy` という free function は新設しない（ADR-046）。ADR-046 が求める
  型または module 名前空間の associated のうち、現行言語で直接書けるのは型の
  associated function であるため、`Io` を名前空間型とする。
- `Read::copy_to` を default method にしない。default method は ADR-062 の
  範囲外であり、required method にすると全 impl に copy loop が重複する。
- `dyn Read` / `dyn Write` は使わない。静的ディスパッチのみ（ADR-062）。

### D5: `BufReader` / `BufWriter` を generic adapter にする

- `BufReader<R: Read>` は inner handle、buffer、buffer 内位置、容量を持つ。
  `Read` と `BufRead` を実装し、`R: Seek` のとき `Seek` も実装する。
- `BufWriter<W: Write>` は inner handle、buffer、容量を持つ。`Write` を実装し、
  `flush` で buffer を inner に書き出す。`into_inner` は flush してから
  inner を返す。
- 既存の `buf_reader_new` / `buffered_reader` / `buf_writer_new` /
  `buffered_writer` は、対応する generic adapter の具象 instantiation
  （`BufReader<Reader>` など）を使う compatibility wrapper とする。
- `Writer::write_string` と `BufWriter::write_string` は `Write` の上に置く
  inherent convenience とする。trait には string 専用 method を追加しない。
- buffer の所有権は adapter が持つ。`fill_buf` が borrow ではなく owned
  `Vec<i32>` を返すのは、`&mut [u8]` と lifetime 付き view が言語に採択されて
  いないためである。zero-copy view は D2 の signature を変える別決定とする。

### D6: `File` は `std::host::fs` の whole-file handle とする

- `std::host::fs::File` を追加し、`Read` / `Write` / `Seek` を実装する。
- 構築:
  - `File::open(path) -> Result<File, IoError>`: 読み取り用。open 時に現在の
    ファイル内容を byte image として読み込む。
  - `File::create(path) -> Result<File, IoError>`: 書き込み用。空の image から
    始め、`flush` で全体を書き出す。最初の `flush` まで path の truncate は
    確定しない。
- 契約:
  - `read` / `seek` は image 上で動く。
  - `write` は image の cursor 位置へ書き、必要なら延長する。他 handle への
    可視性は `flush` が境界である。
  - `flush` は image 全体を path へ書き、成功時に dirty 状態を解消する。
  - `close` は flush して handle を閉じる。閉じた後の操作は error を返す。
  - read-only の `open` に対する `write`、write-only の `create` に対する
    `read` は `IoError::Other` を返す。
- この「whole-file image」は現在の host capability（path 全体の read / write）
  に合わせた意図的な契約である。fd 単位の streaming 可視性は promise しない。
  streaming / resource-backed な `File` が必要になった場合は、可視性意味論が
  変わるため本 ADR を置換する新 ADR で決める。
- 実装前提: 現在の `__runtime_abi_fs_read_file` は `String` を返すため、任意
  バイトの read には byte 指向の host primitive が必要である。この追加は
  #693 の実装フェーズが所有し、`std::io` の trait 契約には影響しない。
- 既存の `file_from_fd` / `file_read` / `file_write` は experimental な fd
  escape hatch であり、本 ADR の正規 surface には含めない。存続する場合は
  experimental のまま #693 の削除条件に従う。

### D7: Error 型の境界

- `Read` / `Write` / `BufRead` / `Seek` の error は既存の `IoError` に固定する。
  `IoError` は `UnexpectedEof(String)` と `Other(String)` のままとする。#693
  では variant を増やさない。variant の追加・分類は #694 の error taxonomy と
  future ADR の範囲である。
- `IoError` は `Display` と `std::error::Error` を実装済みであり、
  `Error::source` は typed source object ABI がないため `None` のまま
  （ADR-064）。
- `?` による伝播は、利用者が `From<IoError> for AppError` を実装する #694 の
  パターンを使う。trait signature に generic error 型や associated error 型を
  導入しない。ADR-084 の `TryFrom` の error 型固定とは独立である。
- `std::host::fs` の path 関数は現在 `Result<_, String>` を返す。`File` の
  impl は host error を `IoError::Other` に写像する。path 関数の error 型を
  一括変更することは #693 の決定に含めない。
- `From<IoError> for String` は移行用に残す。新しい API はこれを前提にせず、
  String error からの移行が完了した時点で削除条件を #693 に記録する。

### D8: 安定性

- 追加する trait、`SeekFrom`、`Reader`、`Writer`、`BufReader`、`BufWriter`、
  `Io`、`std::host::fs::File` は **experimental** から始める（ADR-014）。
- `IoError` と既存の `std::io` free function は、非推奨化するまでは stable の
  まま据え置く。置き換え対象の free function は `deprecated_by` と削除時期を
  持つ `deprecated` へ移し、stable は少なくとも 1 release の deprecation を
  置く（ADR-014、ADR-036 D2、ADR-046 D4）。
- manifest では trait を `[[types]]` の entry として登録する（`Iterator` と
  同じ扱い）。`std::io` の `[[modules]]` entry を追加し、stability を
  experimental / provisional の段階に合わせる。
- experimental から provisional へは、両 target の fixture と signature の
  安定を示した時点で上げる。provisional から stable へは変更なしの release
  1 回と migration guide を条件とする。
- 現在の partial #693 が追加した trait helper の free function は、実装
  scaffolding として扱う。manifest の stable 表示が残っている場合は、
  ADR-014 の downgrade 手順に従って分類を是正してから削除する。無言の stable
  削除はしない。

### D9: 後方互換と移行

- 既存の `Vec<i32>` API は移行期間中に signature を変えない。実装は newtype
  への wrapper とし、内部 layout の互換を保つ。
- 新しい型の内部 layout は公開契約ではない。旧 API が `Vec<i32>` を返す期間
  だけ、layout は compatibility artifact として維持される。
- 挙動差:
  - `Seek::seek` は負の実効位置を error にする。旧 `seek_to` の clamp は
    wrapper が旧挙動として維持する。
  - `BufRead::read_line` は EOF を `None` で返す。旧 `reader_read_line` /
    `buf_reader_read_line` の「EOF で空文字列」は wrapper が維持する。
  - `Write::flush` は memory writer では no-op、`File` では確定境界である。
- 主な移行対応:

| 旧 | 新 |
|----|----|
| `reader_from_bytes(d)` | `Reader::from_bytes(d)` |
| `reader_read(r, n)` | `r.read(n)` |
| `reader_read_exact(r, n)` | `Io::read_exact(r, n)` |
| `reader_read_all(r)` | `Io::read_to_end(r)` |
| `reader_read_line(r)` | `r.read_line()` |
| `reader_eof(r)` | `r.is_eof()` |
| `writer_new()` | `Writer::new()` |
| `writer_write(w, d)` | `w.write(d)` |
| `writer_write_str(w, s)` | `w.write_string(s)` |
| `write_all(w, d)` | `Io::write_all(w, d)` |
| `write_string(w, s)` | `w.write_string(s)` |
| `writer_flush(w)` / `flush(w)` | `w.flush()` |
| `writer_to_bytes(w)` | `w.into_bytes()` |
| `buf_reader_new(r, n)` / `buffered_reader` | `BufReader::new(r, n)` |
| `buf_reader_read_line(br)` | `br.read_line()` |
| `buf_writer_new(w, n)` / `buffered_writer` | `BufWriter::new(w, n)` |
| `buf_writer_write_str(bw, s)` | `bw.write_string(s)` |
| `buf_writer_flush(bw)` | `bw.flush()` |
| `copy_bytes(r, w)` / `copy(r, w)` | `Io::copy(r, w)` |
| `new_memory_buffer()` | `Reader::new()`（source） / `Writer::new()`（sink） |
| `write_bytes(b, d)` | `b.write(d)` |
| `read_bytes(b, n)` | `b.read(n)` |
| `seek_to(b, p)` | `b.seek(SeekFrom::Start(p))` |
| `position(b)` | `b.position()` |
| `buffer_len(b)` | `b.len()` |
| `fill_buffer(b)` | `b.fill_buf()` |
| `file_from_fd` / `file_read` / `file_write` | experimental fd lane。正規 surface は `File` |

- in-tree consumer（`std/host/stdio.ark`、fixtures、`src/compiler` の io 参照）は
  段階的に移行する。旧 API の削除は #703 の monomorphic cutover と揃える。
- `stdout()` / `stderr()` / `stdin()` / `read_stdin_line()` / `print_bytes()` は
  host-backed な互換関数であり、本 ADR の target 表面には含めない。
  `std::host::stdio` の handle 型への移行は host stdio の別作業で決め、#693 は
  これらの削除条件だけを記録する。

### D10: 実装順序（設計依存）

実装の進捗・PR 分割は issue #693 が所有する。ここでは設計上の依存順だけを固定する。

1. **Trait core と memory newtype**: D2 / D3 の signature、`Reader` / `Writer`、
   impl、manifest `[[types]]`、両 target の fixture。`Vec<i32>` への trait impl
   を外し、既存 free function はそのまま残す。
2. **Generic algorithm**: `Io::copy` / `read_exact` / `read_to_end` / `write_all`。
   2 つの trait bound を持つ generic function の monomorphization が前提。
3. **Buffered adapter**: `BufReader` / `BufWriter`。generic field 上の
   user-defined trait dispatch が前提。
4. **File**: byte 指向 host read primitive、`std::host::fs::File`、impl、
   file -> memory の generic copy fixture。host capability の availability を
   manifest に記録する。
5. **Compat / deprecation**: 旧 free function を newtype wrapper にし、
   deprecation と migration note を付け、fixture と docs を移行する。
   `std::io` の誤った deprecation 案内を削除する。
6. **Removal**: 1 release の deprecation 後に旧 free function を削除し、
   `std::io` の表面を trait / method / associated に揃える。

現行 artifact（2026-10-23 の selfhost runtime）では、2 つの trait bound を持つ
generic function の dispatch と、user-defined trait の generic field 経由の
dispatch が invalid core Wasm になった。また default method の body は runtime
trap になった。これらは ADR の決定ではなく実装前提の観測であり、実装フェーズの
開始時に current compiler で再確認する。default method は D2 の通り採用しない。

## 帰結

- file / memory / stdio の stream が同じ trait で扱える。`Io::copy` と
  `BufReader` / `BufWriter` は具象型に依存しない。
- `Reader` / `Writer` の型区別により、同じ `Vec<i32>` に `Read` と `Write` を
  実装する衝突がなくなる。layout は公開契約から外れる。
- 既存の stable free function は移行期間中にそのまま動く。削除は deprecation
  期間の後で、#703 と同期する。
- host 依存は `std::host::fs::File` に閉じ、`std::io` は target-independent に
  なる。`File` の whole-file 契約は fd streaming とは別の決定である。
- `IoError` の分類は #694 の境界に残り、#693 は signature への採用だけを行う。

## 却下した代替案

### `Vec<i32>` alias のまま trait impl を足す

`Reader` と `Writer` が同じ型になり、`Read` と `Write` の impl が同じ型で
衝突する。`BufRead` と `Seek` も cursor の解釈を共有できない。layout が公開
契約に残り、`File` を加えると同じ `Vec<i32>` に複数の意味が集まる。newtype
分離が最小の修正である。

### `read` を caller 提供の buffer へ書き込む形にする

Rust の `Read::read(&mut [u8])` に近いが、Arukellt には `&mut [u8]` がない。
`Vec<i32>` を out-param にすると、長さが capacity を兼ね、呼び出し側が事前に
zero で埋める必要がある。buffer の所有権と有効範囲が型に出ず、ADR-059 の
ownership 境界も曖昧になる。owned `Vec<i32>` を返す形を採る。

### `io::copy` を free function として追加する

ADR-046 がユーザー可達 free function を禁止している。module 名前空間に
associated function を置けないため、`Io` を名前空間型にする。

### `Read::copy_to` を default method にする

default method は ADR-062 の範囲外であり、現行 artifact の default body は
runtime trap になった。required method にすると全 impl に copy loop が重複する。
`Io::copy` が重複と free function の両方を避ける。

### `File` を fd streaming として先に設計する

現在の host surface には open / read / write / close がなく、WASI P2 の
resource も公開されていない。先に streaming 契約を決めると target ごとに
可視性意味論が分かれる。path 全体の capability に合わせた whole-file image を
現在の契約とし、streaming は別 ADR とする。

### trait を `std::host::stdio` に置く

`Read` / `Write` / `Seek` は host capability に依存しない抽象である。
`std::host` に置くと memory adapter まで host 名前空間に混ざり、ADR-054 の
境界も崩れる。抽象は `std::io`、host 実装は `std::host` とする。

### `dyn Read` / `dyn Write` を採用する

ADR-062 は静的ディスパッチを採用し、`dyn Trait` の ABI を採択していない。
generic copy と adapter は monomorphization で足りる。

## 再検討条件

- `&mut T` / lifetime 付き borrow が言語に採択された場合: `read` を fill-buffer、
  `fill_buf` を zero-copy view、receiver を borrow に変える。
- default method が採択された場合: `Read::copy_to`、`BufRead::read_line`、
  `Write::write_all` を provided method に移し、`Io` の associated function を
  縮小する。
- fd / resource-backed streaming `File` の host capability が公式 WASI 境界に
  現れた場合: whole-file image 契約を置換する ADR を作る。
- #694 が associated error 型や source chaining を採択した場合: `IoError` の
  境界を更新する。
- `dyn Trait` の ABI が採択された場合: object-safe な `Read` / `Write` を検討する。

## 関連文書

- [`std/io/mod.ark`](../../std/io/mod.ark) — 現行の alias / free function 実装
- [`std/host/fs.ark`](../../std/host/fs.ark) — host filesystem surface
- [`std/manifest.toml`](../../std/manifest.toml) — 公開 API と stability の正本
- [issue #693](../../issues/open/693-read-write-traits-io-unification.md) — 実装フェーズと進捗
- [ADR-000](ADR-000-process.md) — ADR プロセス
- [ADR-014](ADR-014-stability-labels.md) — stability label
- [ADR-036](ADR-036-trait-stdlib-redesign.md) — trait-based stdlib redesign
- [ADR-044](ADR-044-trait-method-syntax-adopted.md) / [ADR-046](ADR-046-free-function-eradication.md) — trait-first API
- [ADR-054](ADR-054-host-linker-and-rust-runtime-retirement.md) — host 境界
- [ADR-059](ADR-059-language-v2-ownership-copy-mutability.md) — ownership / move
- [ADR-062](ADR-062-language-v2-trait-core-coherence.md) — static dispatch / default method の境界
- [ADR-064](ADR-064-error-trait-namespace-and-source-boundary.md) — Error trait 境界
- [ADR-078](ADR-078-language-v2-parametric-trait-impl.md) / [ADR-079](ADR-079-language-v2-method-level-generics.md) — generic impl / method
- Rust `std::io`: <https://doc.rust-lang.org/std/io/index.html>
