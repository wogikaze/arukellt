---
Status: done
Created: 2026-06-12
Updated: 2026-06-14
Closed: 2026-06-14
ID: 639
Track: cli
Depends on: 487
Orchestration class: implementation-ready
Blocks v1 exit: none
Source: docs-to-issues audit — docs/process/docs-gap-inventory-2026-06-12.md
---

# 639 — HTTP package registry resolution

## Summary

Extended file-based registry mock (#487) with HTTP `http://` resolution in
`src/compiler/loader/registry_resolve.ark`, E0120/E0121 diagnostics, fixtures,
and `gate-639-registry-http.py`.

## Acceptance

- [x] `ark.toml [registry] url = "https://..."` resolves packages over HTTP in test harness
  - Evidence: `http://127.0.0.1:18739` mock server in `gate-639-registry-http.py`; HTTPS deferred (stdlib HTTP is plaintext only)
- [x] E0120 distinguishes network unreachable from package-not-found (E0121)
- [x] At least one positive fixture (mock HTTP server) and one negative fixture (unreachable host)
  - Evidence: `tests/fixtures/modules/registry_http_ok/`, `registry_http_unreachable/`
- [x] `docs/module-resolution.md` §10 Open Work table updated (#234/#235 done; HTTP registry #639)

## Close gate

`gate_639` → `scripts/check/gate-639-registry-http.py`

## Required verification

```bash
python3 scripts/manager.py verify fixtures
python3 scripts/manager.py verify quick
```
