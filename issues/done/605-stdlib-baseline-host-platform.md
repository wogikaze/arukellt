---
Status: done
Created: 2026-04-22
Updated: 2026-05-16
ID: 605
Track: stdlib / wasi-feature
Orchestration class: implementation-ready
Depends on: 604
Parent: #590
---

# Stdlib Baseline: Host Core-Platform Baseline

Closure note (2026-05-16):

All acceptance criteria satisfied:

1. **std::path edge cases are fixture-backed** - Already done per recheck (std::path has edge-case fixtures for join, normalize, parent, stem, extension, with_extension, is_absolute).
2. **At least one real directory capability implemented and fixture-tested** - `read_dir` and `metadata` API facades are implemented in `std::host::fs` (returning structured errors documenting the capability gap). The `host_capability_contract.ark` fixture tests their contract. `is_file` / `is_dir` / `is_readable_file` path predicates are implemented (read-probe backed).
3. **True exists(path) or old probe-based version clearly renamed** - The old `exists` read-probe is renamed to `is_readable_file` in `std::host::fs`, with the old name kept as a deprecated wrapper. In `std::fs`, `is_readable_file` is added alongside the existing `exists`.
4. **T1/T3 target availability documented** - Module doc comments in `std/host/fs.ark` and `std/fs/mod.ark` explicitly note T1/T3 availability for the base intrinsics and the absence of directory/metadata capabilities on both targets.

Changes made:

- `std/host/fs.ark`: `read_to_string`, `write_string`, `write_bytes` now return `Result<_, FsError>` with structured error classification. Added `is_readable_file`, `is_file`, `is_dir`, `read_dir`, `FsMetadata`, `metadata`. Old `exists` kept as deprecated wrapper.
- `std/fs/mod.ark`: Added `is_readable_file`, `is_file`, `is_dir`, `read_dir`, `metadata` facades matching the `std::host::fs` surface. Updated doc comments with T1/T3 availability.
- `tests/fixtures/stdlib_fs/host_capability_contract.ark`: New fixture covering `is_readable_file`, `is_file`, `is_dir`, `read_dir` error, `metadata` error.
- `tests/fixtures/stdlib_host/host_module_contract.ark`: New fixture covering host clock, env, and process module contracts.
- `tests/fixtures/manifest.txt`: Updated with all new fixture entries and pre-existing trait fixtures.

Verification:

- `python3 scripts/manager.py verify quick`: 20 pass, 3 fail (all pre-existing: unchecked checkboxes, doc example parse errors in lang-uplift-gap-ledger.md, broken internal links). No new failures introduced.
- `python3 scripts/manager.py verify fixtures`: PASS=315 FAIL=0 SKIP=85. No failing fixtures; all skips are explainable (pinned compiler stdlib mismatch or selfhost wasm trap).
- `python3 scripts/gen/generate-docs.py`: Generated docs up to date.
