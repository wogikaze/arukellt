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

Canonical declarations live in
`extensions/arukellt-all-in-one/package.json` (`contributes.configuration.properties`).
The extension README lists the same keys, types, defaults, and descriptions.

| Setting | Type | Default | Used by |
|---------|------|---------|---------|
| `arukellt.server.path` | `string` | `"arukellt"` | LSP, DAP, tasks |
| `arukellt.server.args` | `string[]` | `[]` | LSP launch args |
| `arukellt.target` | `"wasm32-wasi-p1"` \| `"wasm32-wasi-p2"` \| `null` | `null` | LSP (`arkTarget`), tasks, commands |
| `arukellt.emit` | `string` | `"core-wasm"` | compile command |
| `arukellt.playgroundUrl` | `string` | `"https://wogikaze.github.io/arukellt/playground/"` | Open in Playground |
| `arukellt.enableCodeLens` | `boolean` | `true` | LSP (`enableCodeLens`) |
| `arukellt.hoverDetailLevel` | `"full"` \| `"minimal"` | `"full"` | LSP (`hoverDetailLevel`) |
| `arukellt.diagnostics.reportLevel` | `"errors"` \| `"warnings"` \| `"all"` | `"all"` | LSP (`diagnosticsReportLevel`) |
| `arukellt.useSelfHostBackend` | `boolean` | `false` | LSP (`useSelfHostBackend`; extension warns and falls back before Stage 2 fixpoint) |
| `arukellt.check.onSave` | `boolean` | `true` | LSP (`checkOnSave`) |

LSP behaviour settings are sent on startup via `initializationOptions` and on change
via `workspace/didChangeConfiguration` (`extensions/arukellt-all-in-one/src/extension.js`).
Server-side handling is implemented in the selfhost LSP (`lsp/lsp_config.ark`, Issue #479).

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

### Version output format

- Command: `arukellt --version` or `arukellt -V`
- Exit code: `0`
- stdout: single line `arukellt 0.1.0` (plain text; extension probes exit code only)
- stderr: empty on success

## stdio separation

| Stream | Content |
|--------|---------|
| stdout | LSP/DAP JSON-RPC frames only (`Content-Length` framed) |
| stderr | Human-readable errors, build logs, `eprintln` diagnostics |

Script replay (CI fixtures): `arukellt lsp <path/to/script.lsp-script>` reads a
fixture file and writes framed responses to stdout — same framing as stdio mode.

Verification: `python3 scripts/check/check-lsp-lifecycle.py` runs every
`tests/fixtures/selfhost/lsp_*.lsp-script` through both file-arg and stdin stdio
paths against the selfhost compiler wasm.
