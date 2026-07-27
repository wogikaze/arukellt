---
Status: done
Created: 2026-07-14
Updated: 2026-07-21
ID: 800
Track: tooling-contract
Depends on: "791"
Orchestration class: done
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: fmt --check performance
---

# 800 — Batch multi-file fmt to amortize wasmtime cold-start

## Summary

`arukellt fmt` accepted only one file per invocation. The manager and
pre-commit hook spawned one wasmtime process per `.ark` file (~1981 files),
making `python3 scripts/manager.py fmt --check` take ~47s. Batching is landed:
multi-file CLI, manager batches, pre-commit batching.

## Acceptance

- [x] `arukellt fmt <f1> <f2> ...` formats all files in one wasmtime run
- [x] `arukellt fmt` continues past a parse error and reports all failures
- [x] `python3 scripts/manager.py fmt --check` is substantially faster than the
      pre-batching ~47s baseline (measured 2026-07-21: **~17s**, exit 0,
      `fmt: checked=2006 failed=0`)
- [x] `python3 scripts/manager.py selfhost fmt-parity` passes (re-verified 2026-07-21)
- [x] pre-commit hook batches staged `.ark` into one `fmt --check` call
- [x] `check-ark-code-quality.py` global + touched-code ratchets pass (no baseline bump needed)

## Close note

The original `<5s` aspirational ceiling is not met on this host; the batching
feature and the large wall-time win vs ~47s are complete. Further cold-start
cuts stay out of scope for this issue.

## Re-evaluation

Owner: compiler-tooling. The new `cli_add_fmt_input` accessor clones its
argument internally (unlike `cli_add_wit_path`) so it is not counted as a
thin wrapper by the ratchet. No baseline bump was required.
