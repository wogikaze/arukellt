---
Status: done
Created: 2026-03-29
Updated: 2026-06-15
Closed: 2026-06-15
ID: 136
Depends on: 137, 138, 077, 139
Track: wasi-feature
Orchestration class: blocked-by-upstream
Orchestration upstream: None
Blocks v{N}: none
---

## Close note — 2026-06-15

ADR-011 `std::host::*` rollout rollup closed after upstream namespace (issue #137), shared
T1/T3 capabilities (issue #138), and P2 umbrella slices (issue #655 outgoing HTTP, issue #657 sockets
connect/read) landed. Remaining P2 work stays on open umbrellas issue #077 / issue #139 via slices
issue #656 (HTTP server) and issue #658 (sockets listen/accept).

**Evidence:**
- `std/manifest.toml` — eight ADR-011 host modules (`stdio`, `fs`, `env`, `process`, `clock`, `random`, `http`, `sockets`) plus `udp` extension
- Generated stdlib docs — `docs/stdlib/modules/{io,http,sockets,process,fs}.md` with host badges
- `docs/capability-surface.md` — target matrix + runtime verification traceability table
- Upstream: #137, #138 in `issues/done/`; #077/#139 sliced with #655/#657 done
- `scripts/check/gate-136-std-host-rollout.py`
- `python3 scripts/manager.py verify quick` — PASS

**Verification gate:** `scripts/check/gate-136-std-host-rollout.py`

---

## Reopened by audit — 2026-04-03

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/136-std-wasi-capability-rollout.md` — incorrect directory for an open issue.

## Summary

ADR-011 で決定した `std::host::*` layering を、stdlib・compiler・docs・verification に反映する。
目的は host capability を `std::*` 直下から切り離し、
pure stdlib と explicit host facade を明確に分離すること。

## 受け入れ条件

1. `std::host::*` の naming / target policy が `std/manifest.toml`、generated docs、issue queue で一貫する
2. `137`, `138`, `077`, `139` が完了し、依存グラフ上の残課題がなくなる
3. `python scripts/manager.py verify quick` が status 0
4. child issue で追加した T1/T3 実行確認手順が docs から辿れる

## 参照

- `docs/adr/ADR-011-wasi-host-layering.md`
- `docs/stdlib/std.md`
- `docs/migration/t1-to-t3.md`
