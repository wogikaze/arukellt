# Stdlib Baseline: Host Core-Platform Baseline

**Status**: open
**Created**: 2026-04-22
**Updated**: 2026-04-22
**ID**: 605
**Parent**: #590
**Depends on**: 604
**Track**: stdlib / wasi-feature
**Orchestration class**: blocked-by-upstream

---

## Summary

Child issue for #590 Phase 2 — Host Core-Platform Baseline.

Move from "can read/write a file somehow" to "has a trustworthy minimum host platform
surface." This includes hardening the whole-file baseline, adding the first real
filesystem capability upgrade (directory / metadata), and keeping target gating explicit.

---

## Scope

**In scope:**
- Harden `std::host::fs` and `std::fs` whole-file read/write baseline with explicit error semantics
- `std::path` edge cases fixture-backed: `join`, `normalize`, `parent`, `stem`, `with_extension`
- Minimum directory / metadata surface on supported targets:
  - `read_dir` / directory listing facade
  - `metadata(path)` structured result
  - `is_file` / `is_dir` split
  - true `exists(path)` backed by real path query semantics (not read probe)
- `std::host::process`, `std::host::env`, `std::host::clock` contracts explicit and fixture-backed
- T1/T3 availability visible in docs and diagnostics

**Out of scope:**
- Streaming I/O, file handles, mmap
- HTTPS / TLS
- Full socket surface beyond current provisional state
- Any capability that cannot be honestly supported on the current WASI P1/P2 targets

---

## Primary paths

- `std/host/fs.ark`
- `std/fs.ark`
- `std/path.ark`
- `std/host/process.ark`
- `std/host/env.ark`
- `std/host/clock.ark`
- `tests/fixtures/` (stdlib host fixtures)

## Allowed adjacent paths

- `std/manifest.toml`
- WASI P2 host capability track (#076) — read-only coordination, no blocking

---

## Upstream / Depends on

604 (contract honesty must come first — do not add capabilities on top of misleading facades)

## Blocks

- #608 (docs/bench closeout)

---

## Acceptance

1. `std::path` edge cases are fixture-backed for all listed operations
2. At least one real directory capability (read_dir or metadata) is implemented and fixture-tested
3. True `exists(path)` based on real path semantics is available (or old probe-based version is clearly renamed)
4. T1/T3 target availability is documented for new capabilities

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python3 scripts/gen/generate-docs.py
```

---

## STOP_IF

- Do not fake streaming APIs or directory handles without backend support
- Do not over-claim support on targets that cannot implement the capability
- Do not expand `std::host::http` or `std::host::sockets` in this issue

---

## Close gate

Close when: path edge cases have fixtures, at least one real directory capability is
implemented and documented, and T1/T3 availability is visible in generated docs.
