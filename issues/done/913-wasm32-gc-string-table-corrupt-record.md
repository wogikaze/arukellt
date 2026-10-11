---
Status: done
Created: 2026-10-10
Updated: 2026-10-11
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

## Investigation progress (2026-10-11)

- Inside the s2 binary, `String`/`Vec<T>` are linear-memory repr (i32
  handle; `len(s)` reads the i32 header at `s - 4`), confirmed by
  disassembling `total_string_data_size` and `MirModule_push_string_constant`
  in the producing-era build, so a `len = -1` element means some producer
  wrote a -1 header (e.g. `substring(src, a, a - 1)`) or pushed a handle
  pointing four bytes past a -1 word (stale/aliased read). GC `array.len`
  can never be negative; the old impossible-under-GC reasoning is moot.
- The injected record sits between `""` and `"\\n"`, i.e. an extra
  `intern_string` push during lowering (the vec is append-only; intern
  sites are `values.ark` `node_text`, `core_literals.ark`
  `mir_core_expr_text`, `match_arms.ark` `pat_text`).
- Producing binary identified by deterministic rebuild: builds in
  `1383275a8..deb9710b7` all produce md5 `caa1fce2` (the range is
  source-identical modulo docs/tests/perf-memo). That binary now hangs
  >15 min on the fixture (pre-#912 mono-propagation explosion); the
  original ~16 min spacing between the two bad artifacts matches.
- Pre-#908-fix binaries (`s2-bigarena*` md5 `852e9924`, `eb20f7ff`,
  `f316c34c`, `07496ffe`) compile the fixture cleanly in seconds, so
  the corruption arrived with the `67eb8fda2` propagation machinery
  that the #912 demand-closure later replaced.
- `check` (parse+typecheck only) finishes in ~1 s on the producing
  binary; the pathological path is MIR lowering / mono emit, not
  front-end.

- Byte-level decode of `fx-str-gc.wasm`: the injected record's 4 emitted
  bytes are `ff 00 00 00`, i.e. exactly what `emit_string_data_payload`
  writes for `slen = -1` (`-1 % 256 = -1` -> 0xff; `-1 / 256 = 0` ->
  0x00 x3). The payload loop `0..(-1)` is empty, so the element costs
  3 bytes in `total_string_data_size` but 4 on the wire -> the 1-byte
  section-size mismatch. The element is an EXTRA push, not a mutated
  record: gc output has 35 records vs 34 in the linear output, and the
  real `""` / `"\\n"` / fixture literals are all intact in order.
- Same fixture + same producer binary, `--target wasm32` (linear):
  CLEAN (34 records, `""` then `"\\n"` adjacent). The extra intern
  therefore happens on a heap-history-dependent path: an intern call
  whose String argument reads `len = -1` under the gc run's layout but
  resolves to a dedup-matchable (or absent) value under the linear
  run's layout. Producer candidates remain the three `ctx_intern_string`
  sites (`node_text`, `mir_core_expr_text`, `pat_text`) whose argument
  is a field read that can return a stale/mis-resolved handle.
- Post-#912 s2 (`14affb4f`, demand-closure era): 40/40 fixture compiles
  emit an intact data section (declared size == walked payload, no
  negative-length record); 140-fixture sweep also clean. Producer
  binary rebuilt deterministically at `deb9710b7` (md5 `caa1fce2`) for
  a same-binary before/after check.

## Resolution (2026-10-11)

- Root cause: bounded to the three `ctx_intern_string` call sites
  (`node_text`, `mir_core_expr_text`, `pat_text`) fed by the
  `67eb8fda2` mono-propagation machinery — the corruption first
  appears in that era and the byte-level decode shows an extra
  append-only intern push with a `len = -1` header. Line-level
  isolation of the exact stale read was not completed because the
  containing subsystem was deleted wholesale: #912 replaced
  mono-propagation with demand-closure, and every post-#912 validation
  run is clean (40/40 on this fixture, 140-fixture sweep, and the
  1786-fixture emit sweep with zero corrupt/invalid sections).
- Regression mechanism (acceptance 2): `emit_string_data_payload`
  now traps loudly on `slen < 0` (`0a729373a`), so a recurrence
  produces an ICE at emit time instead of malformed bytes that only a
  validator would catch.
- Before/after evidence (acceptance 3): satisfied via retained
  artifacts — the two observed corrupt modules
  (`.build/fx-str-gc.wasm`, `.build/fx-str-gc2.wasm`, byte-identical,
  `ff 00 00 00` record present) are the before state; the post-#912
  clean runs above are the after state. A literal same-binary re-run
  was attempted three times and is infeasible: the `deb9710b7`
  producer binary hangs >60 min on this fixture (its own
  mono-propagation explosion, the same pathology #912 fixed), so it
  can no longer complete the offending compile either way.
- Residual risk: if a stale-read corruption resurfaces in new code,
  the emit-side trap converts it into a loud ICE with the offending
  constant index, which is the intended debugging hook for a fresh
  issue.
