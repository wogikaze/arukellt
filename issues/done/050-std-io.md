# std::io: Reader、Writer、stdin/stdout/stderr、buffered I/O

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 050
**Depends on**: 039, 041, 043
**Track**: stdlib
**Blocks v3 exit**: yes

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: done` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/050-std-io.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

汎用 I/O 抽象 (Reader/Writer) と標準入出力 (stdin/stdout/stderr) を実装する。
Bytes を入出力単位とし、buffered I/O とコピーユーティリティを提供する。

## 受け入れ条件

### Reader / Writer

```ark
pub fn read_all(r: Reader) -> Result<Bytes, Error>
pub fn read_exact(r: Reader, n: i32) -> Result<Bytes, Error>
pub fn read_line(r: Reader) -> Result<String, Error>
pub fn write_all(w: Writer, data: Bytes) -> Result<(), Error>
pub fn write_string(w: Writer, s: String) -> Result<(), Error>
pub fn flush(w: Writer) -> Result<(), Error>
pub fn copy(r: Reader, w: Writer) -> Result<i64, Error>
```

### 標準入出力

```ark
pub fn stdin() -> Reader
pub fn stdout() -> Writer
pub fn stderr() -> Writer
pub fn read_stdin_line() -> Result<String, Error>
```

### buffered I/O

```ark
pub fn buffered_reader(r: Reader, buf_size: i32) -> Reader
pub fn buffered_writer(w: Writer, buf_size: i32) -> Writer
```

## 実装タスク

1. `ark-typecheck`: Reader, Writer 型の登録
2. Reader/Writer: GC struct wrapping WASI fd (i32)
3. `std/io/reader.ark`, `std/io/writer.ark`: 基本操作
4. `std/io/stdio.ark`: stdin/stdout/stderr
5. `std/io/buffered.ark`: buffered wrapper (内部 ByteBuf)
6. `std/io/copy.ark`: stream コピー
7. 既存の `println`/`print`/`eprintln` を Writer ベースに内部リファクタ

## 検証方法

- fixture: `stdlib_io/stdout_write.ark`, `stdlib_io/stderr_write.ark`,
  `stdlib_io/read_write_bytes.ark`, `stdlib_io/buffered_basic.ark`,
  `stdlib_io/copy_stream.ark`

## 完了条件

- stdin/stdout/stderr が Reader/Writer として使える
- Bytes の読み書きが正しく動作する
- fixture 5 件以上 pass

## 注意点

1. Reader/Writer は trait ではなく具象型 — v3 では fd-backed のみ
2. buffered_writer の flush を忘れるとデータ損失 — ドキュメントで明記
3. WASI P1 と P2 で fd_write の呼び出し規約が異なる点に注意

## ドキュメント

- `docs/stdlib/io-reference.md`: Reader, Writer, stdin/stdout/stderr, buffered I/O

## 未解決論点

1. Reader/Writer を interface/trait にするか concrete type にするか
2. async I/O を v3 で考慮するか (v4 送り推奨)
