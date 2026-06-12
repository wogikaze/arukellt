---
Status: open
Created: 2026-03-31
Updated: 2026-06-12
Closed: 2026-06-12
ID: 292
Track: capability
Depends on: —
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 12
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Evidence: "E0500 (incompatible-target) in codes.rs, implemented in load.rs"
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
---

- `std/host/http.ark`: "`request()` / `get()` が `Err("not implemented")` を返す"
- `std/host/sockets.ark`: "`connect()` が `Err("not implemented")` を返す"
- `std/manifest.toml: 1685-1720`
- [x] テスト: "http::get を呼ぶコードが compile error を出す fixture"

# stub host module (http, sockets) の使用を compile error にする

## Reopened by audit — 2026-06-12 (slice D)

**Classification**: `must-reopen` / `acceptance-not-actually-met`

**Reopen reason**: Acceptance requires compile-time rejection of `kind = "host_stub"` calls. Manifest no longer uses `host_stub` (http/sockets are `intrinsic_wrapper` with false availability claims), and the selfhost compiler has no stub-kind compile gate — programs compile and fail only at missing intrinsic lowering.

**Repo evidence**:

- `rg 'kind = \"host_stub\"' std/manifest.toml` → 0 matches.
- `std/host/http.ark` calls `__intrinsic_http_get` without compile-time rejection; no handler in `src/compiler/wasm/`.
- `tests/fixtures/host_stub_sockets.ark` expects E0500 but selfhost `src/compiler/` has no target-gating / host_stub diagnostic path (E0500 is emit-failed only).

**Violated acceptance**: all four checkboxes (host_stub compile error, message text, manifest→compiler propagation, http compile-error fixture).

**Evidence files**: `std/manifest.toml`, `std/host/http.ark`, `std/host/sockets.ark`, `tests/fixtures/host_stub_sockets.ark`, `src/compiler/diagnostics/codes.ark`

**Follow-up split**: overlap with #446/#447 reopen; compile-time deny vs runtime honesty tracked via #633

---

# stub host module (http, sockets) の使用を compile error にする

---

## Closed by audit — 2026-04-03

## Reopened by audit — 2026-04-03

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/292-stub-host-compile-error.md` — incorrect directory for an open issue.

## Summary

`std/host/http.ark` と `std/host/sockets.ark` は error stub を返すだけの実装。使用者はコンパイルは通るが実行時に常にエラーになる。未実装 module の使用を compile-time に検出して error にすべき。

## Current state

- `std/host/http.ark`: `request()` / `get()` が `Err("not implemented")` を返す
- `std/host/sockets.ark`: `connect()` が `Err("not implemented")` を返す
- `std/manifest.toml:1685-1720`: `kind = "host_stub"` で分類済み

## Acceptance

- [ ] `kind = "host_stub"` の関数を呼び出すコードが compile error (E レベル) を出す
- [ ] error メッセージに「この API は未実装です (host_stub)」と表示される
- [ ] `std/manifest.toml` の `kind` 情報がコンパイラに伝搬する経路がある
- [ ] テスト: http::get を呼ぶコードが compile error を出す fixture

## References

- `std/host/http.ark`
- `std/host/sockets.ark`
- `std/manifest.toml:1685-1720`
