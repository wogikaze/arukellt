# stub host module (http, sockets) の使用を compile error にする

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-03
**ID**: 292
**Depends on**: —
**Track**: capability
**Blocks v1 exit**: no
**Priority**: 12


---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: E0500 (incompatible-target) in codes.rs, implemented in load.rs

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).


## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/292-stub-host-compile-error.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`std/host/http.ark` と `std/host/sockets.ark` は error stub を返すだけの実装。使用者はコンパイルは通るが実行時に常にエラーになる。未実装 module の使用を compile-time に検出して error にすべき。

## Current state

- `std/host/http.ark`: `request()` / `get()` が `Err("not implemented")` を返す
- `std/host/sockets.ark`: `connect()` が `Err("not implemented")` を返す
- `std/manifest.toml:1685-1720`: `kind = "host_stub"` で分類済み

## Acceptance

- [x] `kind = "host_stub"` の関数を呼び出すコードが compile error (E レベル) を出す
- [x] error メッセージに「この API は未実装です (host_stub)」と表示される
- [x] `std/manifest.toml` の `kind` 情報がコンパイラに伝搬する経路がある
- [x] テスト: http::get を呼ぶコードが compile error を出す fixture

## References

- `std/host/http.ark`
- `std/host/sockets.ark`
- `std/manifest.toml:1685-1720`
