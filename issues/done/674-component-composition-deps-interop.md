---
Status: done
Created: 2026-06-17
Updated: 2026-08-14
Closed: 2026-08-14
ID: 674
Track: component-composition
Parent: 443
Depends on: 443, 663, 665
Priority: 3
---

# 674 — Component composition: dependency wasm, lockfile, and external interop

## Close summary

`scripts/component-deps.py` resolves component `.wasm` artifacts from `ark.toml`, extracts WIT with `wasm-tools component wit` or a sidecar fallback, validates package/world metadata, stores content-addressed artifacts under `.build/components/`, and writes deterministic SHA-256-bound `ark.lock` metadata. Missing artifacts, package mismatches and incompatible worlds are hard diagnostics.

`arukellt compose --manifest ... --socket ... -o ...` is routed through the same resolver and performs deterministic multi-provider `wac plug` composition. Go, C and Zig provider lanes exercise this path when an external-language component artifact is injected; the Python-host lane executes an Arukellt component through wasmtime. External toolchains remain optional test inputs rather than compiler dependencies.

## Acceptance closure

- [x] Manifest dependencies resolve component wasm, not only WIT sidecars.
- [x] Lockfile/cache conventions are implemented and documented.
- [x] Missing component/package/world mismatches have diagnostics.
- [x] WIT can be extracted from component binaries for contract validation.
- [x] Go/C/Zig provider and Python-host interop lanes are present and executable when artifacts/toolchains are supplied.
- [x] Dedicated dependency/compose close gate is present.

## Verification

- `scripts/component-deps.py`
- `scripts/check/gate-674-component-composition-deps.py`
- `scripts/check/gate-component-wit-productization.py`
