# Arukellt CLI Reference

## Subcommands

### `component`

Build, inspect, and validate WebAssembly components.

**Usage:**

```
ark component build <file.ark> [options]
ark component inspect <file.wasm>
ark component validate <file.wasm>
```

**Subcommands:**

| Subcommand  | Description                                        | Status         |
|-------------|----------------------------------------------------|----------------|
| `build`     | Compile an `.ark` file to a `.component.wasm`      | Implemented    |
| `inspect`   | Print the WIT interface of a component             | Not implemented (use `wasm-tools component wit`) |
| `validate`  | Validate a `.component.wasm` against its WIT world | Not implemented (use `wasm-tools validate`)       |

The `build` subcommand is the default — `ark component <file.ark>` is equivalent
to `ark component build <file.ark>`. It delegates to the compile pipeline with
`--emit component --target wasm32-wasi-p2`.

**Options:**

| Option          | Description                          |
|-----------------|--------------------------------------|
| `-o <path>`     | Write output to `<path>`             |
| `--target <t>`  | Set target (default: wasm32-wasi-p2) |
| `--wit <path>`  | Add a WIT import file                |
| `--json`        | Emit JSON diagnostics                |

**Examples:**

```bash
# Build a component from source
ark component build my_app.ark

# Build with shorthand (build is the default)
ark component my_app.ark

# Specify output path
ark component build my_app.ark -o out/my_app.component.wasm

# Inspect a component (requires wasm-tools)
wasm-tools component wit my_app.component.wasm

# Validate a component (requires wasm-tools)
wasm-tools validate my_app.component.wasm
```

### `compose`

Validate and plan linking of multiple Wasm components into one composed artifact.

**Usage:**

```
ark compose --plug <provider.component.wasm> <socket.component.wasm> -o <output.component.wasm>
ark compose --validate --plug <provider> <socket> -o <output>
```

**Status:** Phase 3 (#443) — validates paths and WIT sidecars, prints a dependency graph,
and delegates binary composition to `wac plug` via `arukellt-selfhost.sh` (ADR-034 Phase 3).
Native in-tree linking is not implemented yet.

| Flag / option | Description |
|---------------|-------------|
| `--plug <provider> <socket>` | Provider component to plug into the socket (consumer) component |
| `-o`, `--output <path>` | Output composed `.component.wasm` path |
| `--validate` | Validate the plan only (no binary output) |

**Examples:**

```bash
# Validate a two-component plug plan (no binary output)
ark compose --validate --plug math-lib.component.wasm runner.component.wasm -o app.component.wasm

# Validate + run wac plug to produce composed artifact (requires wac in PATH)
ark compose --plug math-lib.component.wasm runner.component.wasm -o app.component.wasm
```

See [adr/ADR-034-component-composition-linking.md](adr/ADR-034-component-composition-linking.md).
