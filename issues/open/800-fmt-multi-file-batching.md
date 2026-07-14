---
Status: open
Created: 2026-07-14
Updated: 2026-07-14
ID: 800
Track: tooling-contract
Depends on: "791"
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: fmt --check performance
---

# 800 — Batch multi-file fmt to amortize wasmtime cold-start

## Summary

`arukellt fmt` accepted only one file per invocation. The manager and
pre-commit hook spawned one wasmtime process per `.ark` file (1981 files),
making `python3 scripts/manager.py fmt --check` take ~47s. The CLI now
accepts multiple positional file paths, processes them in one process, and
emits per-file status lines plus a `fmt: checked=N failed=M` summary. The
manager batches non-baseline files into one `arukellt fmt` call per batch
(~80 files) and deinterleaves the output back into per-file results so the
parser-failure baseline still applies per file.

## Acceptance

- [x] `arukellt fmt <f1> <f2> ...` formats all files in one wasmtime run
- [x] `arukellt fmt` continues past a parse error and reports all failures
- [x] `python3 scripts/manager.py fmt --check` completes in <5s (was ~47s)
- [x] `python3 scripts/manager.py selfhost fmt-parity` passes
- [x] pre-commit hook batches staged `.ark` into one `fmt --check` call
- [x] `check-ark-code-quality.py` global + touched-code ratchets pass (no baseline bump needed)

## Re-evaluation

Owner: compiler-tooling. The new `cli_add_fmt_input` accessor clones its
argument internally (unlike `cli_add_wit_path`) so it is not counted as a
thin wrapper by the ratchet. No baseline bump was required.
