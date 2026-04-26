# ADR-020: T2 (`wasm32-freestanding`) I/O Surface Design — Console/DOM Bridge Contract

ステータス: **DECIDED** — Import-based bridge（1ページlinear memory region）を採用
**Created**: 2026-04-20
**Scope**: T2 codegen target, playground v2 browser execution, selfhost emitter (`src/compiler/emitter.ark`), `docs/target-contract.md`

---

## Context

ADR-017 established the playground v1 product contract: **no code execution in v1**.
Full compile-and-run in the browser is a v2 concern.  When playground v2 eventually
ships execution, the natural execution surface is T2 (`wasm32-freestanding`): a Wasm
module that contains no WASI imports and can run natively in any browser's Wasm
runtime without a shim layer.

T2 is registered in `crates/ark-target/src/lib.rs` with `implemented: false` and
`run_supported: false`.  No codegen backend exists yet.  Before implementation begins,
the I/O surface must be decided — specifically, how a T2 module communicates output
(e.g. `println`) to the host environment.

This ADR decides **only the I/O surface contract**.  It does not implement the T2
emitter, add fixtures, or change any crate code.  Implementation is a separate
work order.

### The four candidate approaches

| Option | Description |
|--------|-------------|
| **A** | Import-based bridge: JS host provides `{ arukellt_io: { write(ptr, len), flush() } }` |
| **B** | Memory-mapped I/O: module writes to a fixed address; JS polls or traps on write |
| **C** | Custom section descriptor: host reads module exports to discover I/O function pointers |
| **D** | Compute-only: no I/O — T2 modules cannot print; playground shows return value only |

### Constraints from the existing target model

- T2 uses **Wasm GC** (per ADR-007), not linear-memory-only like T1.
- T2 has **no WASI imports** — the entire WASI host layer (ADR-011) is absent.
- T3 already uses a 1-page linear memory region exclusively for WASI I/O marshaling.
  A T2 I/O surface that follows the same convention avoids inventing a new allocation
  strategy in the emitter.
- Modern browsers (V8 ≥ 9.4, SpiderMonkey ≥ FF 90) support Wasm GC (`struct`, `array`,
  `ref`), but passing GC references across the Wasm/JS boundary requires either
  `externref` boxing (well-supported) or `stringref` (not universally available in
  stable engines as of 2026-04).  Both add emitter complexity that is disproportionate
  to the I/O need.

---

## Decision

**Chosen approach: Option A — Import-based bridge with a minimal 1-page linear
memory region for I/O marshaling.**

### The T2 I/O import contract

A T2 module imports a single namespace `arukellt_io` from the host:

```wat
;; required import — host must supply both functions
(import "arukellt_io" "write" (func $io_write (param i32 i32)))
(import "arukellt_io" "flush" (func $io_flush))
```

| Import | Signature | Semantics |
|--------|-----------|-----------|
| `arukellt_io.write` | `(i32 ptr, i32 len) → void` | Write `len` UTF-8 bytes starting at `ptr` in the module's linear memory to the output buffer. The host appends to its internal string buffer; no newline is assumed. |
| `arukellt_io.flush` | `() → void` | Flush the accumulated buffer to the console (e.g. `console.log`). Called by the emitter at each `println` call site after the final `write`. |

The module **must** export its linear memory as `"memory"` so the host can read the
bytes passed to `write`:

```wat
(export "memory" (memory 0))
```

### Linear memory allocation in T2

T2 retains one page (64 KB) of linear memory **exclusively for I/O string marshaling**,
mirroring T3's design (see `docs/current-state.md` §GC-Native Data Model):

> "Linear memory is retained only for WASI I/O marshaling (1 page, 64 KB)."

All other value storage uses Wasm GC (`struct`, `array`, `ref`).  The scratch buffer
region for I/O is fixed at offset `0x0000`–`0xFFFF`.  String data written to the
buffer is not required to be null-terminated; the `len` parameter carries the byte
count.

### `println` lowering (informative — not a codegen prescription)

At a call site like `println("hello")`, the emitter:

1. Copies the UTF-8 bytes of the string into the scratch linear memory buffer.
2. Calls `$io_write(0, len)` with the byte offset and length.
3. Calls `$io_flush()`.

For multi-argument `println` that builds a composite string, the emitter may either
materialise the full string first or issue multiple `write` calls followed by a single
`flush`.  The host treats `flush` as the newline signal.

### Minimal host stub (informative)

The JavaScript host that instantiates a T2 module must supply:

```js
const imports = {
  arukellt_io: {
    write(ptr, len) {
      const bytes = new Uint8Array(instance.exports.memory.buffer, ptr, len);
      outputBuffer += new TextDecoder().decode(bytes);
    },
    flush() {
      console.log(outputBuffer);
      outputBuffer = "";
    }
  }
};
let outputBuffer = "";
const { instance } = await WebAssembly.instantiate(bytes, imports);
```

This stub is intentionally minimal.  The playground v2 shell (tracked separately)
owns the full host implementation.

### Module entry point

T2 modules export a `"main"` function with signature `() → void` (or `() → i32`
for exit-code programs).  The host calls `instance.exports.main()` after
instantiation to run the program.

---

## Rationale for Option A over alternatives

### Why not Option B (memory-mapped I/O)?

Memory-mapped I/O (writing to a magic address and relying on the host to detect it)
is a pattern from embedded/MMIO environments.  In WebAssembly, there is no trap-on-write
mechanism for arbitrary addresses.  The host would need to poll the memory region at a
fixed interval or install a `SharedArrayBuffer`-based watcher — both are awkward in
browser event loops and require `SharedArrayBuffer` with COOP/COEP HTTP headers.
This constraint rules out simple static hosting for playground v2.

### Why not Option C (custom section descriptor)?

Reading a custom section at instantiation time to discover I/O function pointers
requires the host to parse Wasm binary sections before calling
`WebAssembly.instantiate`.  This couples the host to a custom binary format that is
not standardised and would require a bespoke parser in the playground shell.  It also
makes the module non-self-describing at the type level.

### Why not Option D (compute-only)?

Compute-only modules cannot produce user-visible output from `println`, which is the
canonical "hello world" interaction for any language playground.  A compute-only T2
target would force the playground shell to infer output by reading a return value or
inspecting exported memory — providing a much worse UX and requiring the playground
shell to understand the language's calling convention.  This option is explicitly
rejected for playground use.

### Why not pass GC string refs directly?

Passing a Wasm GC `(ref (array i8))` directly to a JS import is possible in some
engines but requires the host to call `WebAssembly.Memory` + GC ref access APIs that
are not uniformly available in stable browsers as of 2026-04.  Routing through linear
memory (Option A) is universally supported and is already the pattern used by T3.
When `stringref` (the `stringref` proposal) reaches broad stable support, the I/O
surface can be extended without breaking the linear-memory path.

---

## Scope and non-goals

### T2 I/O surface scope (this ADR)

- ✅ `println` / `print` → `arukellt_io.write` + `arukellt_io.flush`
- ✅ Panic output (the default panic handler uses the same I/O path)
- ✅ One-way output only — the host writes to the T2 module's stdout; no stdin

### Explicit non-goals (deferred to implementation or later ADRs)

- ❌ **stdin / interactive input** — T2 v2 scope does not include `read_line`.  No
  `arukellt_io.read` import is specified here.
- ❌ **DOM manipulation** — Direct DOM bridge (`document.querySelector`, event
  listeners) is not in scope for the T2 I/O surface.  DOM interop would require a
  richer `externref`-based import contract and is a separate design decision.
- ❌ **stderr** — A separate `arukellt_io.write_err` import is not specified; panic
  output uses the same `write`/`flush` path as stdout.
- ❌ **Return value inspection** — The playground shell may choose to display exit
  codes or `main` return values, but this is a shell UX concern, not part of this
  contract.
- ❌ **T2 emitter implementation** — No codegen changes; no fixtures.
- ❌ **Browser packaging** — Wasm module packaging for playground v2 is a separate
  work order.
- ❌ **WASI compatibility** — T2 is a strict superset of neither WASI P1 nor P2.
  The import namespace `arukellt_io` is Arukellt-specific.

---

## T2 as a v2-playground target

This ADR explicitly scopes T2 as a **v2 playground target**:

- **Playground v1** does not include T2 or any browser-side code execution.
  v1 scope is defined in ADR-017: edit + format + parse + check + diagnostics + share.
- **Playground v2** adds full compile-and-run in the browser.  T2 is the
  compilation target for v2 execution.  Implementation of the T2 emitter
  is tracked in issue 382 (separate work orders).
- `docs/target-contract.md` is updated to reflect "ADR written, emitter not
  started" for T2 (see Consequences below).

---

## Consequences

1. **`docs/target-contract.md`** T2 row is updated from "not-started" to
   "ADR written, emitter not started."  No other changes to that document.

2. **Selfhost emitter (`src/compiler/emitter.ark`)** — When the T2 emitter work order begins,
   the emitter must emit the two `arukellt_io` imports and export `"memory"`.
   The 1-page linear memory allocation must be consistent with T3's layout
   so that shared lower-level helpers (string serialisation, scratch buffers)
   can be reused.

3. **Playground v2 shell (future)** — The shell must supply the
   `arukellt_io` import object.  The minimal JS stub above is the normative
   reference; the actual implementation may buffer multiple `write` calls
   differently for performance, but `flush` must always flush to a visible
   output surface.

4. **ADR-017** is not modified — it remains the v1 contract.  T2 is
   correctly described there as decoupled from playground v1.

5. **DOM interop** — If a future work order specifies DOM bridge imports
   (e.g. for reactive examples), that requires a new ADR or an amendment to
   this one.  The `arukellt_io` namespace is reserved for standard I/O;
   DOM imports must use a separate namespace.

---

## Alternatives considered

### A′: Dual-mode `write` (GC ref + linear memory)

Emit both `arukellt_io.write_gc(ref s)` and `arukellt_io.write(ptr, len)` and let
the host choose.  Rejected: dual-mode adds emitter complexity and host complexity
simultaneously; GC ref passing is not universally stable.  Revisit when `stringref`
is stable.

### B′: WASI shim

Emit standard WASI P1 imports and provide a JS polyfill that maps them to
`console.log`.  Rejected: WASI shim libraries (e.g. `@bjorn3/browser_wasi_shim`)
add kilobytes of JS dependency and introduce WASI semantics (fd numbers,
`environ_get`, etc.) that are meaningless for a playground.  The T2 target is
explicitly "no WASI" per ADR-007; adding a WASI shim defeats this constraint.

---

## References

- [ADR-007](ADR-007-targets.md) — T2 target definition (Wasm GC, no WASI, browser)
- [ADR-011](ADR-011-wasi-host-layering.md) — WASI host layering (T3 only)
- [ADR-013](ADR-013-primary-target.md) — T3 as primary target
- [ADR-017](ADR-017-playground-execution-model.md) — Playground v1 contract; T2 deferred to v2
- `crates/ark-target/src/lib.rs` — T2: `implemented: false`, `run_supported: false`
- `docs/target-contract.md` — T2 status (updated by this ADR)
- `docs/current-state.md` — GC-Native Data Model and T3 1-page linear memory precedent
- Issue 382 — T2 freestanding implementation (this ADR is the design phase)
- Issue 378 / ADR-017 — Playground execution model decision
