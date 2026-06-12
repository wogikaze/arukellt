---
Status: open
Created: 2026-06-11
Updated: 2026-06-11
ID: 633
Track: stdlib
Depends on: "446, 447"
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks: none
---

# Reconcile std::host::http / sockets / udp capability claims with the selfhost execution path

## Summary

The capability manifest and stdlib docs advertise host network/IO families as
available on real targets, but the backing was the Rust `arukellt_host` runtime
that was **deleted** during the selfhost-first migration (#559 / #583,
ADR-029). The current execution path is `wasmtime run <selfhost.wasm>`
(`scripts/run/arukellt-selfhost.sh`) and the selfhost compiler dispatches host
calls only for `env`, `fs`, `process::exit`, and `stdio`
(`src/compiler/wasm/call_host_io.ark`). No HTTP / sockets / UDP host functions
exist on the only compile/run path, so these availability claims are
`docs-ahead-of-reality`.

This is the **documentation/manifest honesty** track. The implementation of the
host bindings themselves is tracked by #446 (http) and #447 (sockets); native
WASI P2 paths are tracked by #077 (http) and #139 (sockets).

## Why it must exist

`prompts/research.md` requires that docs / manifest / capability surface not
advertise functionality the repo cannot back. The over-claim is user-visible
(it drives `docs/stdlib/reference.md`, `docs/capability-surface.md`, and IDE
capability badges via `scripts/gen/generate-docs.py`).

## Evidence source

- `std/manifest.toml` — `std::host::http` declares
  `availability = { t1 = true, t3 = true, note = "... arukellt_host::http_get ..." }`;
  `std::host::sockets`/`udp` declare `t3 = true`. `arukellt_host` is deleted.
- `src/compiler/wasm/call_host_io.ark` — no http/sockets/udp dispatch.
- `crates/` directory absent (Rust runtime removed).
- `docs/capability-surface.md`, `docs/stdlib/modules/http.md`,
  `docs/stdlib/modules/sockets.md`, `docs/current-state.md` — still reference
  `arukellt_host` / `register_http_host_fns`.

## Primary paths

- `std/manifest.toml` (host family `availability` notes)
- `docs/capability-surface.md`, `docs/stdlib/modules/{http,sockets}.md`,
  `docs/current-state.md`
- `scripts/gen/generate-docs.py` (regeneration)

## Non-goals

- Implementing the host bindings (that is #446 / #447 / #077 / #139).
- Changing the selfhost compiler's intrinsic dispatch.

## Acceptance

- [ ] `std/manifest.toml` host http/sockets/udp availability reflects the
      selfhost path reality (not "available via arukellt_host").
- [ ] No active (non-issue, non-historical) doc references `arukellt_host` /
      `register_http_host_fns` as a current backing.
- [ ] `docs/capability-surface.md` marks http/sockets/udp as not user-reachable
      on the current selfhost path, cross-linking #446/#447/#077/#139.
- [ ] Generated docs regenerated and consistent.

## Required verification

```bash
rg -n "arukellt_host|register_http_host_fns" --glob '!issues/**' --glob '!docs/process/**' --glob '!docs/adr/**'
python3 scripts/gen/generate-docs.py
python3 scripts/check/check-docs-consistency.py
python scripts/manager.py verify quick
```

## Close gate

- Manifest + docs no longer advertise unbacked host availability.
- `check-docs-consistency.py` rc=0; `manager.py verify quick` rc=0.
- Cross-links to #446/#447/#077/#139 present.
