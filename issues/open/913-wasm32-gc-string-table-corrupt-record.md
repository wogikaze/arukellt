---
Status: open
Created: 2026-10-10
Updated: 2026-10-10
ID: 913
Track: compiler-internal
Depends on: none
Orchestration class: investigation
Orchestration upstream: none
Blocks v{N}: language-v2
Priority: 1
Source: 908 lane fixture validation 2026-10-10 - emitted wasm32-gc module rejected by wasm-tools validate
---

# 913 - wasm32-gc emit intermittently inserts a corrupt string-table record (len = -1)

## Summary

Compiling `tests/fixtures/stdlib_seq/seq_generic_string_instantiation.ark`
to wasm32-gc produced a module whose data section is malformed: one extra
4-byte record `ff 00 00 00` (little-endian length -1) is inserted in the
middle of the string table (observed at constant index 25, between the
`""` and `"\\n"` std literals). `total_string_data_size` counts
`4 + (-1) = 3` bytes for that element while `emit_string_data_payload`
writes the 4-byte header plus zero payload bytes, so the section is one
byte longer than declared and validators reject it:

```
error: section size mismatch: unexpected data at the end of the section (at offset 0x38d5)
```

## Evidence

- Two byte-identical bad artifacts were produced by two separate compiles
  on the same host binary (deterministic per binary): the data segment
  declares 757 bytes while the payload is 758; decoding shows every real
  record intact and shifted by one slot after the injected record.
- On the same source rebuilt with trivial instrumentation (an
  `if slen < 0` eprintln in `emit_string_data_payload`, later push-side
  logging in `mir/module_strings.ark`), the same fixture compiles cleanly
  12/12 runs with no negative-length element anywhere - classic
  heap-layout-dependent heisenbug: adding code shifts the compiler's own
  linear-GC allocation history and changes what a stale/uninitialised read
  lands on.
- 140 fixtures (stdlib_seq + stdlib_string + generics + match_extensions)
  compiled and validated cleanly under the current uninstrumented binary.

## Suspected mechanism

The injected element sits between two `m.string_constants` entries that
are pushed consecutively by lowering, so an `intern_string` /
`MirModule_push_string_constant` call stored a `String` whose
`len = -1`. Ark code cannot construct a negative-length String through
the public surface; the plausible producers are a stale or out-of-bounds
read that fabricates a bogus (ptr, len) pair (e.g. landing on an adjacent
record field holding a `0 - 1` sentinel), or a substring/slice path that
propagates a negative length unchecked. Intern sites that feed
`string_constants`: `mir/lower/values.ark` (node_text),
`mir/lower/core_literals.ark` (expr_text),
`mir/lower/match_arms.ark` (pat_text).

## Acceptance criteria

1. Root cause identified: the read that produces the corrupt String is
   found and fixed (bounds checked, field initialised, or lifetime
   corrected). Do not mask with emit-side guards - per the semantic-debt
   policy the fix must be at the producer, not a filter that hides
   corruption.
2. A regression mechanism exists that would catch recurrence: e.g. a
   cheap invariant in `intern_string` / `emit_string_data_payload`
   that traps loudly instead of emitting malformed bytes, and/or fixture
   validation of emitted section sizes.
3. The offending compile is reproducible before the fix and clean after,
   under the same host binary.

## Notes

- Bad artifacts retained for forensics during the investigation:
  `.build/fx-str-gc.wasm`, `.build/fx-str-gc2.wasm` (lane-908-mono
  worktree, 2026-10-10 ~20:54-21:10).
- Blocks closing the `run:` manifest gate only if it resurfaces; the
  908 fixture itself is committed on the strength of 12/12 clean
  compiles plus a green wasm32 run of the emitted module.
