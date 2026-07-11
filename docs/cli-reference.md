# Arukellt CLI Reference

> Binary name: **`arukellt`** (see `src/compiler/main/usage.ark`).  
> There is no documented `ark` alias. Wrapper: `scripts/run/arukellt-selfhost.sh`.

Full command list and release status also appear in [`current-state.md`](current-state.md).

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
arukellt doc --json std::host::http::get
arukellt doc --html -o docs/docs/std/index.html
```

### `component`

Build, inspect, and validate WebAssembly components.
`component build` sets `--emit component` on the compile pipeline
(`src/compiler/main/component_cmd.ark`). Equivalent core path:
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
# equivalent:
arukellt compile my_app.ark --target wasm32-gc --emit component -o out/my_app.component.wasm
```

**Known limitation:** the pinned bootstrap wasm may return empty / non-invokable
library components for `pub fn` exports. Use a built s2 artifact
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
