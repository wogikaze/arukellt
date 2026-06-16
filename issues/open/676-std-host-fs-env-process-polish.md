---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 676
Track: stdlib-api
Depends on: "076 (wasi-p2-filesystem, done), 445 (std-host-process, done)"
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: P1 filesystem/env/process polish checklist audit 2026-06-17
---

# 676 — std::host fs / env / process capability completion

## Summary

`std::host::fs` exposes `read_dir`, `metadata`, `exists`, etc., but several APIs
return stub errors (`read_dir` / `metadata not yet supported`). Env/process polish
items (vars iterator, `current_dir`, `--deny-process`, path traversal tests) are
missing from fixtures and CLI.

## Acceptance

- [ ] `std::host::fs::read_dir` backed by runtime dispatch (T1/T3 as applicable)
- [ ] `std::host::fs::metadata` backed by runtime dispatch
- [ ] `std::host::fs::remove_file` and `create_dir_all` implemented or explicit
      `FsError` contract with fixtures
- [ ] `std::host::env::vars` iterator (or documented equivalent)
- [ ] `std::host::env::current_dir`
- [ ] `std::host::process::id` placeholder or stable rejection diagnostic
- [ ] `--deny-process` capability flag (Issue #448 follow-up)
- [ ] Tests: read-only directory grant enforcement, denied filesystem diagnostic
      snapshots, path traversal rejection for preopened dirs
- [ ] Process abort fixture (if not covered by #445)
- [ ] Document host capability defaults in CLI `--help` / `docs/cli-reference.md`
- [ ] Gate `scripts/check/gate-676-std-host-fs-env-process.py`
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `std/host/fs.ark`, `std/host/env.ark`, `std/host/process.ark`
- `issues/done/076-wasi-p2-filesystem.md`
- `issues/done/445-std-host-process-implementation.md`
