# Arukellt CLI Reference

> Binary name: **`arukellt`** (see `src/compiler/main/usage.ark`).  
> There is no documented `ark` alias. Wrapper: `scripts/run/arukellt-selfhost.sh`.

Full command list and release status also appear in [`current-state.md`](current-state.md).

## Compile target axes

`--target`, `--memory-width`, `--wasi-version`, and `--emit` are independent
configuration axes. The accepted target/host matrix is generated from
[`docs/data/project-state.toml`](data/project-state.toml).

| Option | Values | Rule |
|--------|--------|------|
| `--target <target>` | `wasm32`, `wasm32-gc`, `native-cpp`, `native-llvm` | Selects the backend target and its GC availability. |
| `--memory-width <width>` | `auto`, `memory32`, `memory64` | `auto` uses the target default; `wasm32` rejects `memory64`. |
| `--wasi-version <profile>` | `wasi-p1`, `wasi-p2`, `wasi-p3` | Selects a host profile; it is not a target name. |
| `--emit <kind>` | `wasm`, `wat`, `component`, `wit`, `all` | Selects an artifact kind allowed by the target. |

`wasm32-gc` defaults to Memory64 for core Wasm. It also accepts explicit
Memory32, so GC availability and memory width are not synonyms. Component
output uses the official WASI P2 canonical memory boundary and resolves
`auto` to Memory32; an explicit Memory64 request with `component` or `all`
is rejected before source compilation. The current WAT scaffold resolves
`auto` to Memory32 and rejects explicit Memory64. See [ADR-074](adr/ADR-074-language-v2-target-capability-configuration.md).

## Subcommands

### `doc`

Look up standard library metadata from `std/manifest.toml` or generate the
static standard library reference used by GitHub Pages.

```bash
arukellt doc <symbol> [--json] [--target <target>]
arukellt doc --html -o <output.html>
```

| Option | Description |
|--------|-------------|
| `--json` | Emit machine-readable JSON for a symbol lookup |
| `--target <target>` | Show availability warnings for the selected target |
| `--html` | Generate a rich static HTML stdlib reference |
| `-o`, `--output <path>` | HTML output path for `--html` |

```bash
arukellt doc println
arukellt doc --json std::host::fs::read_to_string
arukellt doc --html -o docs/docs/std/index.html
```

### `component`

Build, inspect, and validate WebAssembly components.
`component build` sets `--emit component`, the canonical `wasm32-gc` target,
and the official `wasi-p2` host profile on the compile pipeline
(`src/compiler/main/component_cmd.ark`). Prefer the explicit form:

`arukellt compile --target wasm32-gc --emit component`.

```bash
arukellt component build <file.ark> [options]
arukellt component <file.ark>          # build is the default
arukellt component inspect <file.wasm> # not implemented — use wasm-tools
arukellt component validate <file.wasm>
```

| Subcommand | Status |
|------------|--------|
| `build` | Implemented (delegates to compile + `--emit component`) |
| `inspect` | Not implemented (`wasm-tools component wit`) |
| `validate` | Not implemented (`wasm-tools validate`) |

```bash
arukellt component build my_app.ark -o out/my_app.component.wasm
# preferred public form:
arukellt compile my_app.ark --target wasm32-gc --emit component -o out/my_app.component.wasm
```

**Known limitation:** the pinned bootstrap wasm may return empty / non-invokable
library components for `export fn` declarations. Use a built s2 artifact
(`ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2.wasm`) for library component
work — see [`current-state.md`](current-state.md).

### `compose`

Validate and plan linking of multiple Wasm components (ADR-034 Phase 3).
Delegates binary composition to `wac plug` via the selfhost wrapper.

```bash
arukellt compose --plug <provider.component.wasm> <socket.component.wasm> -o <output.component.wasm>
arukellt compose --validate --plug <provider> <socket> -o <output>
```

### Other commands

See [`current-state.md`](current-state.md) for `compile`, `run`, `check`, `build`,
`fmt`, `test`, `lint`, `targets`, `analyze`, `init`, `script`, `lsp`, and
`debug-adapter`.


## Host capability defaults

WASI P2 host capabilities are explicit runtime authorities. Filesystem access is deny-by-default and requires a preopened directory (for example Wasmtime `--dir`). Environment access sees only values supplied by the runtime. The repository does not provide HTTP/socket facade APIs or network permission flags; networking must be imported through an official WASI Component and granted by the embedding runtime. Process `exit`/`abort` are available by default on supported host targets; `--deny-process` rejects programs that use those process-control intrinsics during compilation/checking. `--deny-clock` and `--deny-random` provide the analogous compile-time deny controls for clock and host randomness.

The compiler does not manufacture a process ID on WASI 0.2: `std::host::process::id()` returns `Err`. Runtime directory traversal outside a granted preopen is rejected by the WASI filesystem boundary.
