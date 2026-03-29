# Compiler Determinism

This document defines the rules that keep Arukellt's `.wasm` output
**bit-exact reproducible** — the same source must always produce the same
binary, byte-for-byte.

## Why It Matters

Reproducible builds let users, CI, and package registries verify that a
published `.wasm` was produced from claimed sources. Any non-determinism
turns binary verification into a coin flip and makes caching unreliable.

## Determinism Rules

### 1. Function Emission Order

Functions must be emitted in a deterministic order — typically the order
they appear in the source, or a stable sort by fully-qualified name.
The emitter must never rely on iteration order of an unordered collection
(e.g. `HashMap`) to decide function index assignment.

### 2. Type Section Order

Wasm type entries (function signatures, struct types) must be emitted in a
stable, predictable order. Use insertion-order or sorted-by-key collections
when building the type index.

### 3. HashMap / HashSet Avoidance

Rust's `HashMap` and `HashSet` use randomised hashing by default.
Iterating over them produces a different order on every run.

**Policy:**

- Prefer `BTreeMap` / `BTreeSet` for any map whose iteration order can
  influence emitted output (type indices, function indices, export names,
  symbol tables).
- If `HashMap` is used for performance in a hot path, **sort the entries**
  before writing them to the output.
- Use `IndexMap` (from the `indexmap` crate) when insertion-order
  preservation is sufficient.

### 4. No Timestamps or Random Values

The compiler must not embed timestamps, UUIDs, random nonces, or any
non-deterministic metadata into the output binary. Build-info sections,
if added, must be opt-in and contain only reproducible data (e.g. source
hash, compiler version).

### 5. Stable Import / Export Ordering

Import and export entries must follow a deterministic order — typically
alphabetical by module+name, or source-declaration order.

### 6. Constant-Pool and Data-Segment Order

String literals, numeric constants, and data segments must be emitted in
a stable order (e.g. first-occurrence in AST walk).

## Verification

The reproducible-build gate is enforced by `scripts/verify-harness.sh`. To
verify manually, compile the same fixture twice and compare:

```bash
arukellt compile fixture.ark -o out1.wasm
arukellt compile fixture.ark -o out2.wasm
cmp out1.wasm out2.wasm
```

On failure, compare WAT output (`wasm-tools print out1.wasm > a.wat`, etc.)
to locate the non-deterministic section.

## Debugging a Failure

1. Run the failing fixture with `--emit-phases` (if available) to dump
   intermediate representations.
2. Diff the two WAT outputs (`wasm-tools print build1.wasm > a.wat`, etc.).
3. The diff usually points to a reordered index — trace back to the
   collection that produced it and replace with an ordered alternative.
