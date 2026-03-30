# CLI / LSP / DAP Startup Contract

This document defines the startup interface between the `arukellt` CLI binary
and editor extensions. Any change to these contracts must be reflected in the
VS Code extension (`extensions/arukellt-all-in-one`) and verified by extension
tests.

## Binary discovery

The extension resolves the binary via:

1. `arukellt.server.path` setting (default: `"arukellt"`)
2. PATH lookup

Probe: `arukellt --version` — must exit 0 and print a version string.

## LSP launch

```
arukellt lsp [--stdio]
```

- Transport: **stdio** (only supported transport)
- `--stdio` is accepted for compatibility but is the default
- Extension passes `[...extraArgs, 'lsp']` with `TransportKind.stdio`
- `extraArgs` comes from `arukellt.server.args` setting (default: `[]`)

## DAP launch

```
arukellt debug-adapter
```

- Transport: **stdio**
- Extension registers a `DebugAdapterExecutable` with args `['debug-adapter']`
- DAP protocol: standard Debug Adapter Protocol over Content-Length framed JSON

### Supported DAP requests (current)

| Request             | Status    | Notes                                    |
|---------------------|-----------|------------------------------------------|
| initialize          | supported | Reports capabilities                     |
| launch              | supported | Sends initialized event                  |
| configurationDone   | scaffold  | Sends terminated (no real execution yet) |
| threads             | scaffold  | Returns single "main" thread             |
| setBreakpoints      | scaffold  | Accepts but marks all unverified         |
| disconnect          | supported | Clean shutdown                           |

## Extension settings

| Setting               | Type     | Default           | Used by         |
|-----------------------|----------|-------------------|-----------------|
| `arukellt.server.path` | string   | `"arukellt"`      | LSP, DAP, tasks |
| `arukellt.server.args` | string[] | `[]`              | LSP             |
| `arukellt.target`      | string   | `"wasm32-wasi-p1"`| tasks, commands |
| `arukellt.emit`        | string   | `"core-wasm"`     | compile command |

## Error reporting

- Binary not found: error message with guidance to set `arukellt.server.path`
- LSP crash: output channel logs + status bar indicator
- DAP crash: VS Code shows standard debug session error

## Version compatibility

The extension requires the CLI to support:

- `arukellt --version` (exit 0)
- `arukellt lsp` subcommand
- `arukellt debug-adapter` subcommand
- `arukellt check`, `arukellt compile`, `arukellt run` for task provider
