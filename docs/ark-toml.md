# ark.toml — Project Manifest Reference

`ark.toml` is the entry point for Arukellt project configuration. The CLI, LSP, and task provider
all resolve project settings through this file.

## Discovery

The CLI searches for `ark.toml` starting from the current working directory and walking upward
until the filesystem root. The first directory containing `ark.toml` is the project root.

## Sections

### `[package]` *(required)*

```toml
[package]
name = "my-project"    # required; used for output naming and diagnostics
version = "0.1.0"      # required; semver string
authors = ["Alice"]    # optional; list of author strings
description = "..."    # optional; one-line project description
```

### `[bin]` *(required for `arukellt build`)*

Identifies the entry-point source file for `arukellt build`.

```toml
[bin]
name = "my-app"          # output file will be <name>.wasm
path = "src/main.ark"    # relative to the project root (ark.toml location)
```

### `[scripts]` *(optional)*

Named shell commands runnable with `arukellt script run <name>`.

```toml
[scripts]
test = "arukellt compile src/main.ark && wasmtime main.wasm"
fmt  = "..."
```

### `[targets]` *(optional)*

Per-target build overrides. Keys are target IDs (e.g. `wasm32-gc`).

```toml
[targets.wasm32-gc]
opt_level = 2          # override --opt-level for this target
output    = "out.wasm" # override default output path
```

### `[dependencies]` *(optional)*

Project dependencies. Path-based local `.ark` packages use simple names; WIT component
packages use Layer C identifiers (`namespace:package`) per [ADR-031](adr/ADR-031-import-syntax-wit-unification.md).

```toml
[dependencies]
my-lib = { path = "../my-lib" }
"test:host" = { path = "vendor/host" }   # resolves vendor/host/mod.wit for WIT imports
```

Version-string dependencies (`my-lib = "0.1.0"`) are accepted by the parser but not yet resolved.
WIT package entries resolve `mod.wit` (or `interface.wit`) under the vendor path into `--wit` paths automatically.

### `[world]` *(optional)*

WIT world binding for component output with `--emit component`.

```toml
[world]
name = "wasi:cli/command"
```

### `[lint]` *(optional)*

Per-project lint rule severity overrides. Rules are identified by their
diagnostic code (e.g. `W0006`) or a human-readable alias.

```toml
[lint]
allow = ["W0006"]   # suppress unused import warnings
warn  = ["W0007"]   # unused bindings as warnings (default)
deny  = ["W0001"]   # escalate to hard error
```

Three severity levels are supported:

| Level | Effect |
|-------|--------|
| `allow` | Diagnostic is suppressed entirely |
| `warn` | Diagnostic is reported as a warning (default for most lint rules) |
| `deny` | Diagnostic is escalated to a hard error; compilation fails |

Use `arukellt lint --list` to see all available lint rule codes.

## Validation

Schema violations produce actionable error messages:

| Situation | Message |
|-----------|---------|
| `ark.toml` not found | `error: ark.toml not found in current directory or any parent` + hint to run `init` |
| TOML parse error | `error: failed to parse ark.toml: <detail>` |
| Missing `[bin]` on `arukellt build` | `error: ark.toml must contain a [bin] section` + field hint |

## Single-file vs project mode

| Mode | How to invoke | `ark.toml` required? |
|------|---------------|----------------------|
| Single-file | `arukellt compile src/main.ark` | No |
| Project | `arukellt build` | Yes (searches upward) |

Both modes use the same compiler pipeline. Single-file output defaults to the source basename;
project output defaults to `[bin].name`. Use project mode when you need `[scripts]`, a stable
output name, or per-target configuration.

## Example

```toml
[package]
name = "hello-ark"
version = "0.1.0"
description = "Hello world in Arukellt"

[bin]
name = "hello"
path = "src/main.ark"

[scripts]
run = "arukellt compile src/main.ark && wasmtime hello.wasm"

[targets.wasm32-gc]
opt_level = 2
```

## References

- `docs/cli-startup-contract.md` — CLI / LSP binary interface
- `docs/current-state.md` — current implementation state
- `src/compiler/main.ark` — schema implementation
