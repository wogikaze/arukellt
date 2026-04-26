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

---

## Single-file mode vs project mode

Arukellt supports two operating modes:

### Single-file mode

Run `arukellt compile <file.ark>` or `arukellt run <file.ark>` with an explicit source file path.
No `ark.toml` is needed. The compiler processes only the specified file.

**Characteristics:**
- No `ark.toml` required
- Output defaults to `<file>.wasm` in the current directory (or `-o <output>` override)
- Target defaults to `wasm32-wasi-p1` unless `--target` is specified
- Import resolution is relative to the source file's directory
- `arukellt run` compiles in memory and executes immediately
- Suitable for scripts, experiments, and single-file programs

```bash
# single-file mode examples
arukellt compile hello.ark
arukellt compile hello.ark --target wasm32-wasi-p2 -o hello.wasm
arukellt run hello.ark
```

### Project mode

Run `arukellt build` from a directory that contains (or is a child of a directory containing) an `ark.toml`.
The manifest drives the build: entry point, target, emit kind, and world are all taken from `ark.toml`.

**Characteristics:**
- Requires `ark.toml` with a `[package]` and `[bin]` section
- Project root is determined by upward directory search from cwd
- Output is placed at `<project_root>/<bin.name>.wasm`
- Target is determined by `[target]` section (or `--target` override)
- `[world]` section passes the WIT world name for component builds
- LSP and task provider also resolve settings through `ark.toml`

```bash
# project mode examples
arukellt build              # reads ark.toml from cwd or parent
arukellt build --opt-level 2
```

### Mode detection

The CLI detects the mode by the subcommand used:
- `arukellt compile <file>` or `arukellt run <file>` → single-file mode
- `arukellt build` → project mode (requires `ark.toml`)

The LSP always operates in project mode if an `ark.toml` is found; otherwise it falls back to single-file mode for each open document.

### Upgrading from single-file to project mode

```bash
mkdir myproject && cd myproject
cat > ark.toml <<'EOF'
[package]
name = "myproject"
version = "0.1.0"

[bin]
name = "myproject"
path = "src/main.ark"
EOF
mkdir src
# move your .ark file to src/main.ark
arukellt build
```
