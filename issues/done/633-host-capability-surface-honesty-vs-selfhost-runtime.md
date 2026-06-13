---
Status: done
Created: 2026-06-11
Updated: 2026-06-14
ID: 633
Track: stdlib
Depends on: "446, 447"
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks: none
---

# Reconcile std::host::http / sockets / udp capability claims with the selfhost execution path

## Acceptance

- [x] `std/manifest.toml` host http/sockets/udp availability reflects the
      selfhost path reality (not "available via arukellt_host").
- [x] No active (non-issue, non-historical) doc references `arukellt_host` /
      `register_http_host_fns` as a current backing.
- [x] `docs/capability-surface.md` marks http/sockets/udp as not user-reachable
      on the current selfhost path, cross-linking #446/#447/#077/#139.
- [x] Generated docs regenerated and consistent.
- [x] `docs/current-state.md` and `docs/capability-surface.md` state HTTPS is not supported for `std::host::http` (docs-to-issues audit 2026-06-12).
