---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 677
Track: cli
Depends on: "475, 485 (done)"
Orchestration class: design-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: P2 developer experience checklist audit 2026-06-17
---

# 677 — Component developer experience (CLI, cookbooks, diagnostics explain)

## Summary

Issue #475 added `arukellt component build|inspect|validate` with wasm-tools delegation
stubs. P2 DX checklist items remain: `wit inspect`, `targets --json`, `capabilities`,
`doctor`, `--explain` docs for component diagnostics, cookbooks, troubleshooting,
and machine-readable build reports.

## Acceptance

- [ ] `arukellt wit inspect <file.wit>` (or documented alias) delegates to wasm-tools
      with actionable errors when missing
- [ ] `arukellt component inspect` / `validate` improved beyond stub (or documented
      permanent delegation contract in CLI reference)
- [ ] `arukellt targets --json` machine-readable target matrix
- [ ] `arukellt capabilities` lists host/WASI capability flags
- [ ] `arukellt doctor` reports wasm-tools + wasmtime component-model support versions
- [ ] Actionable diagnostics when wasm-tools or wasmtime lacks component support
- [ ] `--explain E0401`, `E0402`, `W0005` documentation pages linked from CLI
- [ ] Cookbook pages:
  - [ ] Exporting Arukellt function as WIT component
  - [ ] Importing Rust component into Arukellt
  - [ ] Using jco with Arukellt component
  - [ ] WASI P2 command component
- [ ] Migration guide: core wasm → component wasm
- [ ] Troubleshooting page for component validation failures
- [ ] Error snapshot tests for component diagnostic codes
- [ ] Machine-readable component build report (`--report json` or similar)
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `src/compiler/main/component_cmd.ark`
- `docs/cli-reference.md`
- `issues/done/475-arukellt-component-subcommand.md`
