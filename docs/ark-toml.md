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

### `[bin]` *(required for `ark build`)*

Identifies the entry-point source file for `ark build`.

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

Per-target build overrides. Keys are target IDs (e.g. `wasm32-wasi-p2`).

```toml
[targets.wasm32-wasi-p2]
opt_level = 2          # override --opt-level for this target
output    = "out.wasm" # override default output path
```

### `[dependencies]` *(optional)*

Project dependencies. Only path-based local dependencies are supported in the current release.

```toml
[dependencies]
my-lib = { path = "../my-lib" }
```

Version-string dependencies (`my-lib = "0.1.0"`) are accepted by the parser but not yet resolved.

### `[world]` *(optional)*

WIT world binding for component output with `--emit component`.

```toml
[world]
name = "wasi:cli/command"
```

## Validation

Schema violations produce actionable error messages:

| Situation | Message |
|-----------|---------|
| `ark.toml` not found | `error: ark.toml not found in current directory or any parent` + hint to run `init` |
| TOML parse error | `error: failed to parse ark.toml: <detail>` |
| Missing `[bin]` on `ark build` | `error: ark.toml must contain a [bin] section` + field hint |

## Single-file vs project mode

| Mode | How to invoke | `ark.toml` required? |
|------|---------------|----------------------|
| Single-file | `arukellt compile src/main.ark` | No |
| Project | `arukellt build` | Yes (searches upward) |

Both modes produce the same Wasm output. Use project mode when you need `[scripts]`, a stable
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

[targets.wasm32-wasi-p2]
opt_level = 2
```

## References

- `docs/cli-startup-contract.md` — CLI / LSP binary interface
- `docs/current-state.md` — current implementation state
- `crates/ark-manifest/src/lib.rs` — schema implementation
