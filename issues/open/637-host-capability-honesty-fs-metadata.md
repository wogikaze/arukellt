---
Status: open
Created: 2026-06-12
Updated: 2026-06-12
ID: 637
Track: stdlib
Depends on: 051, 076
Orchestration class: implementation-ready
Blocks v1 exit: none
Source: docs-to-issues audit — docs/process/docs-gap-inventory-2026-06-12.md
---

# 637 — Host capability honesty: fs metadata and read_dir surface

## Summary

docs/stdlib/modules/fs.md documents read_dir, metadata, and is_dir as future or error-returning placeholders. std/manifest.toml and capability docs must honestly reflect implemented vs unavailable surfaces after #051 and #076 land.

## Evidence source

docs/stdlib/modules/fs.md, docs/stdlib/604-contract-honesty-gap-ledger.md, docs/capability-surface.md

## Primary paths

std/host/fs.ark, std/manifest.toml, src/compiler/emit_intrinsic_io.ark, docs/stdlib/modules/fs.md

## Non-goals

Full wasi:filesystem/types parity (#076 scope), HTTPS (#446)

## Acceptance

- [ ] read_dir / metadata / is_dir either implemented with honest semantics or compile-time rejected with documented error codes
- [ ] std/manifest.toml stability labels match runtime behavior (no stub advertised as available)
- [ ] docs/stdlib/modules/fs.md and capability-surface.md synced with implementation truth
- [ ] At least one fixture proves honest behavior or honest rejection per surfaced API

## Required verification

```bash
python3 scripts/manager.py verify fixtures
python3 scripts/check/check-docs-consistency.py
```

## Close gate

Fixture evidence + manifest/docs alignment; #633 honesty gate not contradicted by fs module claims.
