# Fixture harness を selfhost binary 対応にする

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-04-03
**ID**: 330
**Depends on**: 328
**Track**: selfhost-retirement
**Blocks v1 exit**: no
**Priority**: 23


---

## False-Done Audit Note — 2026-04-03

**Additional audit finding**: Acceptance criteria verified as NOT met.

**Reason**: Acceptance criterion 1 requires ARUKELLT_BIN env var to be read by harness.rs, but harness.rs uses current_exe() and does not read ARUKELLT_BIN.

**Violated acceptance**: - [x] `ARUKELLT_BIN=path/to/selfhost cargo test -p arukellt --test harness` で selfhost compiler が使われる — NOT MET: harness.rs uses current_exe(), not ARUKELLT_BIN

**Evidence**: `crates/arukellt/tests/harness.rs:97-113` — arukellt_binary() reads current_exe(), not ARUKELLT_BIN env var. Setting ARUKELLT_BIN and running `cargo test --test harness` has no effect on which binary is used.


## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/330-fixture-harness-selfhost-compat.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`tests/harness.rs` が selfhost binary を compiler として使えるようにする。現在 harness は `cargo build -p arukellt` + `target/release/arukellt` 前提でコンパイルされている。

## Current state

- `tests/harness.rs`: Rust binary を直接呼び出し
- compiler binary の path が harness 内部に hardcoded
- selfhost binary でどの fixture が pass / fail するか未測定
- regression tracking の仕組みがない

## Acceptance

- [x] `ARUKELLT_BIN=path/to/selfhost cargo test -p arukellt --test harness` で selfhost compiler が使われる
- [x] selfhost で pass / fail する fixture のリストが生成される
- [x] 差分が regression として追跡可能 (前回の pass リストとの diff)
- [x] pass 率が CI artifact として記録される

## References

- `tests/harness.rs` — fixture harness
- `tests/fixtures/manifest.txt` — fixture 一覧
