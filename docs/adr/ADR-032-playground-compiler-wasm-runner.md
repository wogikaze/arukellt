# ADR-032: Playground v2 Compiler Wasm and Browser Runner Model

**Status**: DECIDED — ship the selfhost compiler Wasm to the browser and run compiled T2 Wasm in a sandboxed host
**Date**: 2026-05-17
**Track**: playground
**Issue**: [#632](../../issues/open/632-playground-compiler-wasm-build-run-loop.md)
**Supersedes**: none
**Refines**: [ADR-017](ADR-017-playground-execution-model.md), [ADR-020](ADR-020-t2-io-surface.md)

---

## Context

The playground must eventually compile and run real Arukellt programs in the
browser. A TypeScript interpreter or per-feature execution shim is the wrong
model: it duplicates language semantics outside the compiler, diverges on
features such as `Result`, `?`, `match`, generics, traits, and stdlib lowering,
and turns every language feature into a second implementation obligation.

The repo now has a committed selfhost compiler artifact:
`bootstrap/arukellt-selfhost.wasm`. That artifact is a WASI Preview 1 CLI
compiler. The browser cannot execute it directly as plain core Wasm because it
expects argv, environment, stdio, and file operations. The right boundary is
therefore a browser worker that provides a small WASI-like virtual host for the
compiler process.

For executing the user's compiled program, the browser target remains T2
(`wasm32-freestanding`). T2 is intentionally WASI-free and should run in the
browser's native `WebAssembly.instantiate` path with explicit Arukellt host
imports for stdio.

---

## Decision

Playground v2 compile/run uses a two-stage browser pipeline:

1. **Compile stage** — run the selfhost compiler Wasm inside a dedicated Web
   Worker with an in-memory WASI P1 host.
2. **Run stage** — instantiate the compiler-produced T2 Wasm module in the
   browser and provide an `arukellt_io` stdio host import object.

No browser playground code may implement Arukellt language execution semantics
directly. The TypeScript layer owns process orchestration, virtual files,
timeouts, stdio buffers, diagnostics transport, and UI state only.

### Compile Stage

The playground build publishes a compiler asset derived from the selfhost
artifact:

```text
bootstrap/arukellt-selfhost.wasm
  -> docs/playground/assets/arukellt-selfhost.wasm
```

The browser worker loads that asset and runs it as a command process:

```text
arukellt compile /work/main.ark --target wasm32-freestanding -o /work/out.wasm
```

The worker host provides:

- argv and env
- stdin, stdout, stderr capture
- an in-memory filesystem containing `/work/main.ark`
- an in-memory output file `/work/out.wasm`
- no network access
- no host filesystem access outside the virtual filesystem
- timeout and input/output size limits

The compile API returns compiler stdout/stderr, exit code, diagnostics when
available, and the output Wasm bytes when compilation succeeds.

### Run Stage

The run stage receives the compiled T2 bytes and instantiates them with an
`arukellt_io` import object. ADR-020's output-only surface is the baseline, but
playground v2 requires a stdio-shaped contract so the host can capture stdout
and stderr and later supply stdin without changing the runner shape.

The v2 stdio import surface is:

```wat
(import "arukellt_io" "write"     (func $write_stdout (param i32 i32)))
(import "arukellt_io" "write_err" (func $write_stderr (param i32 i32)))
(import "arukellt_io" "flush"     (func $flush_stdout))
(import "arukellt_io" "flush_err" (func $flush_stderr))
(import "arukellt_io" "read"      (func $read_stdin (param i32 i32) (result i32)))
```

The module must export linear memory as `"memory"` so the host can read and
write UTF-8 byte ranges for stdio marshalling.

`read(ptr, len) -> i32` copies up to `len` bytes from the playground stdin
buffer into module memory at `ptr` and returns the number of bytes copied.
EOF returns `0`. Higher-level `read_line` behavior belongs in stdlib/compiler
lowering; the JS host only provides bytes.

The run API returns:

- stdout
- stderr
- exit code
- trap/runtime error text when instantiation or execution fails
- elapsed compile/run timing for UI display

### Node-like Model

"Node-like" means command-process semantics, not Node API exposure. The browser
worker behaves like a tiny process host:

- files are virtual
- stdio is buffered
- command args are explicit
- execution is isolated from DOM and network
- the same request/response contract can be tested in Node using a Node WASI
  harness

Node tests are required for deterministic CI. Browser smoke tests remain
required for asset loading and UI integration.

---

## Consequences

1. The existing TypeScript playground engine can remain for fast parse,
   formatting, tokenization, and diagnostics, but it is not the execution
   engine.
2. The previous `crates/ark-playground-wasm` path stays retired. The compiler
   asset is the selfhost compiler Wasm, not a Rust frontend wasm-pack package.
3. `docs/playground/index.html` may expose Build/Run only when the compiler
   asset and worker host are present.
4. `wasm32-freestanding` must grow from scaffold to a runnable browser target
   for useful execution. Until T2 lowers stdio to `arukellt_io`, the run stage
   can only prove empty or compute-only programs.
5. Stdin support requires compiler/stdlib lowering to the byte-level
   `arukellt_io.read` import. The playground host must not fake `read_line`
   in TypeScript.
6. Security policy lives at the worker/process boundary: size caps, timeouts,
   no network, no persistent storage by default, and no access to the user's
   local filesystem.

---

## Non-goals

- Reimplementing the Arukellt language interpreter in TypeScript.
- Adding TypeScript support for individual syntax features such as `match`,
  `Result`, `?`, generics, or traits.
- Exposing Node APIs, DOM APIs, fetch, filesystem, or network to user programs.
- Running T3 (`wasm32-wasi-p2`) directly in the browser.
- Reintroducing the retired Rust `ark-playground-wasm` crate.

---

## Implementation Slices

1. **Compiler asset packaging**: copy the pinned selfhost compiler Wasm into the
   playground docs asset tree during `npm run build:app`; enforce a size budget.
2. **Compiler worker host**: implement the request/response API and virtual
   WASI process host for `compile`.
3. **T2 stdio lowering**: teach the selfhost T2 emitter to emit `arukellt_io`
   imports and wire `std::host::stdio` output to them.
4. **T2 browser runner**: instantiate compiled T2 bytes and capture stdout,
   stderr, exit code, and traps.
5. **UI integration**: wire Build/Run controls to the worker and runner, with
   disabled states when assets or browser Wasm features are unavailable.
6. **Node + browser proof**: CI tests compile a fixture through the compiler
   Wasm host and run a stdio fixture through the T2 runner.

---

## Open Questions

- Whether the compiler worker should use a small maintained WASI P1 shim
  dependency or a repo-local minimal WASI implementation tailored to the
  compiler's observed syscalls.
- Whether `read` is sufficient for all future stdio needs or whether a separate
  `poll`/`isatty` style import is needed for interactive examples.
- Whether long-running programs should use cooperative fuel instrumentation,
  worker termination, or both.
