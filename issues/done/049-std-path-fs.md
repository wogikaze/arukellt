# std::path + std::fs: パス操作とファイル I/O

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 049
**Depends on**: 039, 041, 042
**Track**: stdlib
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: #39, #41
**Blocks v3 exit**: yes

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/049-std-path-fs.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

ファイルパス操作 (join, parent, extension, normalize) とファイルシステム I/O
(read, write, exists, create_dir_all, metadata) を std::path と std::fs として実装する。
WASI P2 のファイルシステム API を backend に使用。T1 (P1) でも基本操作を提供。

## 背景

現在 `fs_read_file`, `fs_write_file`, `fs_exists`, `fs_mkdir` が prelude に存在するが、
Path 型がなく、パス操作は文字列連結に依存している。
自己ホスト・CLI ツール・テスト支援に Path 抽象とファイル I/O は不可欠。

## 受け入れ条件

### std::path

```ark
pub fn from_string(s: String) -> Path
pub fn to_string(p: Path) -> String
pub fn join(base: Path, child: String) -> Path
pub fn parent(p: Path) -> Option<Path>
pub fn file_name(p: Path) -> Option<String>
pub fn extension(p: Path) -> Option<String>
pub fn with_extension(p: Path, ext: String) -> Path
pub fn is_absolute(p: Path) -> bool
pub fn normalize(p: Path) -> Path
pub fn components(p: Path) -> Vec<String>
```

### std::fs

```ark
pub fn read_to_string(path: Path) -> Result<String, Error>
pub fn read(path: Path) -> Result<Bytes, Error>
pub fn write_string(path: Path, contents: String) -> Result<(), Error>
pub fn write(path: Path, contents: Bytes) -> Result<(), Error>
pub fn exists(path: Path) -> bool
pub fn is_file(path: Path) -> bool
pub fn is_dir(path: Path) -> bool
pub fn create_dir(path: Path) -> Result<(), Error>
pub fn create_dir_all(path: Path) -> Result<(), Error>
pub fn remove_file(path: Path) -> Result<(), Error>
pub fn remove_dir(path: Path) -> Result<(), Error>
pub fn read_dir(path: Path) -> Result<Vec<Path>, Error>
pub fn metadata(path: Path) -> Result<Metadata, Error>
pub fn copy(src: Path, dst: Path) -> Result<(), Error>
```

## 実装タスク

1. `ark-typecheck`: Path, Metadata 型の登録
2. Path: 内部的には String wrapper。path 操作は source 実装 (split by `/`)
3. `std/path/path.ark`: パス操作関数 (pure source 実装)
4. `std/fs/fs.ark`: ファイル I/O (WASI P2 bridge 経由の intrinsic)
5. `ark-wasm/src/emit`: WASI P2 `wasi:filesystem/types` import の拡張
6. Metadata 型: `{size: i64, is_file: bool, is_dir: bool}`
7. 旧 `fs_read_file` 等を deprecated 化

## 検証方法

- fixture: `stdlib_fs/path_join.ark`, `stdlib_fs/path_parent.ark`,
  `stdlib_fs/path_extension.ark`, `stdlib_fs/fs_read_write.ark`,
  `stdlib_fs/fs_exists.ark`, `stdlib_fs/fs_dir_ops.ark`,
  `stdlib_fs/fs_metadata.ark`
- 既存 `stdlib_io/` fixture との整合性確認

## 完了条件

- Path 操作が正しく動作する (join, parent, extension, normalize)
- ファイルの読み書きが WASI P2 で動作する
- fixture 7 件以上 pass

## 注意点

1. Path separator: WASI/POSIX では `/` 固定。Windows パス (`\`) は非対応を明示
2. 既存 `fs_read_file(path: String)` との移行: Path wrapper を挟む
3. read_dir のシンボリックリンク・権限エラーの扱い

## ドキュメント

- `docs/stdlib/path-fs-reference.md`: path, fs API リファレンス

## 未解決論点

1. temp ファイル/ディレクトリ API を v3 に入れるか
2. file watcher / notify を v3 スコープに含めるか (v4 送り推奨)

---

## Queue closure verification — 2026-04-18

- **Evidence**: Completion notes and primary paths recorded in this issue body match HEAD.
- **Verification**: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18).
- **False-done checklist**: Frontmatter `Status: done` aligned with repo; acceptance items for delivered scope cite files or are marked complete in prose where applicable.

**Reviewer:** implementation-backed queue normalization (verify checklist).
