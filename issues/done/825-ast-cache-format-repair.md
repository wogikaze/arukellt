---
Status: done
Created: 2026-07-17
Updated: 2026-10-15
ID: 825
Parent: 823
Track: selfhost-infra
Depends on: "823"
Related: "#823, #824, docs/research/selfhost-compile-latency-root-cause.md"
Orchestration class: implementation
Blocks v4 exit: False
---

# AST cache format repair (not “re-enable as-is”)

## Summary

AST cache is effectively disabled (`AST cache disabled - needs heap investigation`).
This issue is **format/contract repair**, not a blind re-enable. Do not raise
implementation priority until frontend-time effect is measured against #823/#824.

## Acceptance

- [x] Fix `deserialize_node` pos initialization bug
- [x] Repair missing f64 handling in the cache codec
- [x] Document binary I/O contract (endianness, lengths, node tags)
- [x] Distinguish corrupt cache from empty AST
- [x] Atomic write (temp + rename) for cache files — substituted, see below
- [x] Schema / version field; reject mismatched versions
- [x] Revisit the 500-node limit policy (document keep / raise / remove)
- [x] On cache hit: measure heap / peak RSS delta
- [x] Measure frontend (lex/parse) wall vs cold before prioritizing over reachability work
- [x] No product enablement that silently returns empty trees on corrupt input

## Update 2026-10-15 — codec repaired (format v2), measured, still disabled

### Bugs found and fixed

The codec was more broken than "pos bug": several layered defects meant the
cache could never have produced a correct tree.

- **`deserialize_node` shadowed `pos` with `let pos = 0`**, so every node
  parsed from offset 0.
- **Infix `|` is not an expression operator in Ark.**  The lexer emits
  TK_PIPE, but in a `let` RHS the first `|` silently terminates the
  statement — `let val = b0 | (b1 << 8) | ...` bound `val = b0` and left
  the shifts as dead garbage statements.  Every i32 read returned the low
  byte only.  Replaced with plain little-endian arithmetic
  (`b0 + b1*256 + b2*65536 + b3*16777216`); `&`/`>>`/`^` infix are
  verified working, `|` and chained `.bitor((x).shl(n))` are not (see #922
  note below).
- **Bare tuple literal in tail position mis-emits on the wasm32 linear
  backend** — the elements are dropped and `unreachable` is emitted, so any
  `fn -> (a, b)` ending in a bare `(x, y)` traps at runtime (and can emit
  invalid stack shapes).  `return (x, y)` emits a correct tuple build.  All
  codec tails now use explicit `return`.  This alone made every reader
  function a trap.
- **f64 was serialized as text but never parsed** — `float_val` always came
  back 0.0.  Now parsed via `parse_f64`; unparseable payload fails closed.
- **`char_to_string(byte)` reconstruction corrupted multi-byte UTF-8**
  text (each byte >= 0x80 became its own codepoint).  Replaced with
  `text::try_slice_bytes`, which is byte-faithful and fails closed on a
  mid-codepoint boundary.
- **No bounds checks anywhere** — every reader now returns pos = -1 on
  bounds violation; the sentinel propagates so each level checks once.

### Format v2 (documented in ast_serialize.ark header)

  magic "ASTC" | version:i32 = 2 | count:i32 | payload_len:i32 | crc:i32
  | payload[payload_len]   (node encoding unchanged)

CRC is FNV-1a-style (basis 216613626, prime 16777619, abs-normalized),
matching std::core::hash policy.  Any torn/stale write fails validation
before parsing.

**Atomic write**: true temp+rename is impossible — the host fs facade has
no rename (mutation APIs were deliberately removed; official WASI
filesystem WIT is the sanctioned path).  The checksum envelope provides
the required guarantee — readers never consume a partial file — and the
limitation is documented in the codec header.  Substituted deliberately,
not silently.

**500-node policy**: kept as named `ASTC_MAX_NODES`.  Measurement shows
break-even at the cap boundary (below), so raising it is the only
interesting direction — deferred until re-enablement is measured-worthy.

### Measurement (wasm32 linear target, selfhost s2)

Source: src/compiler/loader/module_local_decls.ark (4,976 B, 485 nodes —
just under the cap; serialized cache file is 26,588 B = 5.3x source).

- cold parse x20: 71 ms  (~3.6 ms/parse)
- cache hit x20: 79 ms  (~4.0 ms/hit: read + validate + decode)
- peak RSS (60-iter loops, `/usr/bin/time -v`): parse 56,200 KB vs hit
  53,804 KB — delta is baseline-dominated, no meaningful separation
- at the 500-node cap the hit path is break-even at best; larger ASTs
  could benefit but exceed the cap.  **Verdict: keep disabled.**

### Regression coverage

`test mod "astc_codec"` in ast_serialize.ark (4 tests, executed):
field-level round-trip incl. f64 2.5 and non-ASCII text, header
corruption (magic/version/checksum), truncation, empty forest Ok-vs-Err.

Plus a live probe (temporary, .build/astc-probe.wasm) that exercised
read/write through real fs on wasm32: all cases passed.

### Still open / deliberately not done

- Re-enablement: out of scope per "not re-enable as-is"; the measured
  effect at the cap does not justify it.  module_parse keeps the
  disabled-path comment updated.
- `ast_serialize_buf.ark` deleted (dead duplicate of the serializer half).
- bare `AstNode`/`Span` names resolve program-wide in the full compiler
  build but not under `arukellt test` or standalone roots; test helpers
  use qualified `ast_node_record::AstNode` literals.  Bare-name scoping
  rules deserve their own issue if they bite elsewhere.

## Resolution

Closed 2026-10-15 (commit 010c6e4bb).  The codec is repaired to format
v2 and verified end-to-end on the real wasm32 target; the product path
stays disabled by measurement (hit ~= cold at the 500-node cap) rather
than by suspicion.  The temp+rename acceptance item was satisfied by
substitution — the host fs facade exposes no rename, so the checksummed
envelope carries the "never consume a partial file" guarantee — and the
substitution is documented in the codec header and above.  The
linear-backend tuple-tail miscompile and infix-`|` statement truncation
found here are recorded in #922 (deterministic small repro).

## Non-goals

- Early body lowering (#824) — done / wontfix (decl_emit gate missed)
- Phase arena (#827)
- Raising priority above measured frontend benefit

## Notes

- Parent #823 explicitly deferred AST cache product work in the reachability slice.
- Prefer failing closed (miss / rebuild) over shipping a broken hit path.

## References

- `issues/done/823-selfhost-compile-latency-quadratic-mir.md`
- `docs/research/selfhost-compile-latency-root-cause.md` (原因6)
