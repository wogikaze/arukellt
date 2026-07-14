---
Status: open
Created: 2026-07-14
Updated: 2026-07-14
ID: 814
Track: code-quality
Depends on: "791"
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: CQ-18 audit — unresolved exceptions need open owner
---

# 814 — Formatter/parser exceptions (23 files)

## Summary

23 Ark files are exempted from formatter and parser checks via
content-addressed SHA256 exceptions. These files cannot be parsed by the
canonical formatter, preventing format enforcement.

## Exact failure scope

23 files in `src/compiler/` are listed in the formatter/parser exception
list. Each file has a SHA256 hash recorded in the exception baseline.

## Machine-readable baseline

The exception list is in `docs/data/ark-code-quality-baseline.toml` under
the formatter/parser exception section. Each entry has a file path and
SHA256 hash.

## Owner

compiler-tooling team

## Removal condition

Each file is removed from the exception list when the canonical formatter
can parse it without error. The SHA256 hash is removed and `fmt --check`
passes for that file.

## Validation command

```bash
python3 scripts/manager.py fmt --check
```

## Current count

23 exempted files

## New-failure ratchet

No new files may be added to the exception list. The count must only
decrease. Any new addition is a regression and blocks merge.
