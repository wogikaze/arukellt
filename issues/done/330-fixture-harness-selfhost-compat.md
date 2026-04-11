# Fixture harness を selfhost binary 対応にする

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-05
**ID**: 330
**Depends on**: 328
**Track**: selfhost-retirement
**Blocks v1 exit**: no
**Priority**: 23

---

## Decomposition note — 2026-04-03

この issue を 2 層に分解した。

| Layer | Issue | Scope |
|-------|-------|-------|
| harness implementation | #482 | `harness.rs` が ARUKELLT_BIN env var を読む |
| CI artifact / regression | **#330 (this issue)** | pass/fail リスト生成 + regression 追跡 |

**#330 の acceptance を絞り込む**: acceptance 1 (ARUKELLT_BIN が harness で使われる) は #482 担当。
この issue (#330) は acceptance 2-4 (pass/fail リスト・regression・CI artifact) のみを担当する。
**Depends on #482**。

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

---

## Closed by orchestrator — 2026-04-05

Close gate satisfied (commits `912dbb2`, `e9b1d62`):
- scripts/gen/gen-harness-report.sh created (executable, JSON/text modes, --baseline regression tracking)
- .github/workflows/ci.yml modified: ARUKELLT_BIN env var + harness artifact upload
- verify-harness.sh --quick 19/19
