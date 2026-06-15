---
Status: done
Created: 2026-06-15
ID: 663
Track: language-design
Parent: 124
Depends on: 652, 653, 654
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v4 exit: no
ADR candidate: yes
Implementation target: "Use Ark (src/compiler/*.ark) per #529 selfhost transition."
Status note: Closed — ark.toml WIT package dependencies merge into wit_paths via driver/file; gate-663 fixture passes without --wit.
---

# 663 — ark.toml WIT package resolution for component imports

## Summary

Declare external WIT packages in `ark.toml` `[dependencies]` and resolve vendor `mod.wit`
files into the compile pipeline without requiring `--wit` on the CLI.

## Parent

Umbrella: [#124 WIT component import syntax](../done/124-wit-component-import-syntax.md)

## Acceptance

- [x] `ark.toml` accepts quoted WIT package keys with `{ path = "vendor/..." }`
- [x] `merge_wit_paths_for_source` collects vendor `mod.wit` paths from manifest
- [x] Resolver/typecheck use parsed WIT documents via `wit_register` hook (no `--wit` required)
- [x] Fixture `tests/fixtures/wit_import/ark_manifest/` typechecks without `--wit`
- [x] `python3 scripts/manager.py verify quick` exits 0

## Out of scope

- #664 — general record/enum import bindings expansion
- #665 — compose + wasmtime round-trip E2E

## Close gate

`python3 scripts/check/gate-663-ark-toml-wit-package.py`

## References

- `src/compiler/loader/wit_manifest.ark` — manifest WIT dependency scan
- `src/compiler/driver/file.ark` — merge hook before compile
- `src/compiler/resolver/wit_register.ark` — dynamic WIT doc registration
- `tests/fixtures/wit_import/ark_manifest/`
