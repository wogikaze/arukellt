---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 675
Track: capability
Depends on: "446, 447, 655, 656, 657, 658 (done); 633 (host-capability-honesty, done)"
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: P1 host capability checklist audit 2026-06-17 — reverses #633 docs-only stance
---

# 675 — Host capability user-reachability and runtime permission flags

## Summary

Issue #633 correctly marked `std::host::http`, `sockets`, and `udp` as **not
user-reachable** on the selfhost path. Subsequent work (#655–#658) added T3
dispatch (`call_host_network.ark`) and gate fixtures, but manifest/docs still
claim non-reachability and permission flags (`--allow-http`, `--deny-http`,
`--allow-net`, `--deny-net`) are missing.

This issue reconciles implementation, manifest, generated docs, and close gates so
host HTTP/sockets/UDP are honestly **user-reachable** where fixtures prove runtime
behavior.

## Acceptance

- [ ] `std::host::http::get` and `::request` callable from user Ark on T3 selfhost
      path without host_stub rejection
- [ ] `std::host::sockets::connect` (and read/write/listen/accept where implemented)
      user-reachable on T3 selfhost path
- [ ] `std::host::udp::send` compile-time host_stub rejection removed; runtime dispatch
      wired or explicit E0500 with migration note
- [ ] CLI flags: `--allow-http`, `--deny-http`, `--allow-net`, `--deny-net` (compile
      and/or run enforcement documented)
- [ ] HTTPS URLs rejected with stable diagnostic until TLS exists
- [ ] Fixtures + manifest:
  - [ ] `std_host_http_get_success` (or equivalent green GET)
  - [ ] `std_host_http_404`, `std_host_http_timeout`
  - [ ] UDP send success + invalid host diagnostic
- [ ] `std/manifest.toml` + `docs/capability-surface.md` + generated stdlib docs
      updated to **user-reachable** where gates pass
- [ ] `scripts/gen/generate-docs.py` fails when user-reachable status drifts from
      manifest `availability` blocks
- [ ] Runtime capability audit for `call_host.ark` / `call_host_network.ark`
- [ ] Close gate for user-reachable host capabilities (extend #138 or new gate)
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `issues/done/633-host-capability-surface-honesty-vs-selfhost-runtime.md`
- `src/compiler/wasm/call_host_network.ark`
- `scripts/check/gate-655-http-outgoing.py`, `gate-657-sockets-connect-read-write.py`
