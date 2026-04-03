# Arukellt All-in-One

Minimal VS Code extension scaffold for Arukellt.

## Current scope

- Registers `.ark` as the `arukellt` language
- Provides a basic language configuration, grammar, and snippets
- Launches `arukellt lsp` using the configured CLI path
- Supports restarting the language server from the command palette
- Adds command palette actions for `check`, `compile`, and `run` on the active `.ark` file
- Adds a basic `arukellt` task provider and status bar state
- Adds setup doctor, command graph, and environment diff diagnostics in the output channel

## Commands

- `Arukellt: Restart Language Server`
- `Arukellt: Check Current File`
- `Arukellt: Compile Current File`
- `Arukellt: Run Current File`

## Extension Settings

The following five rationalized settings control LSP server behaviour. All settings have `window` scope and can be configured in `.vscode/settings.json` or VS Code's Settings UI.

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `arukellt.enableCodeLens` | `boolean` | `true` | Show Run / Debug / Test CodeLens above functions in `.ark` files. Set to `false` to hide all CodeLens entries. Forwarded to the LSP server as `enableCodeLens`. |
| `arukellt.hoverDetailLevel` | `"full"` \| `"minimal"` | `"full"` | Controls how much information is shown on hover. `"full"`: signature + docs + availability + usage examples. `"minimal"`: type signature only. Forwarded to the LSP server as `hoverDetailLevel`. |
| `arukellt.diagnostics.reportLevel` | `"errors"` \| `"warnings"` \| `"all"` | `"all"` | Controls which diagnostics appear in the Problems panel. `"errors"`: errors only. `"warnings"`: errors + warnings. `"all"`: all severities including hints. Forwarded to the LSP server as `diagnosticsReportLevel`. |
| `arukellt.target` | `"wasm32-wasi-p1"` \| `"wasm32-wasi-p2"` \| `null` | `null` | Compilation target for the LSP server and CLI commands. `null` means auto-detect from `ark.toml`. Forwarded to the LSP server as `arkTarget`. |
| `arukellt.useSelfHostBackend` | `boolean` | `false` | Use the self-hosted (ark-compiled) compiler backend. Requires Stage 2 fixpoint (Issue 459). When `true` before Stage 2, the extension logs a warning and falls back to the Rust backend silently. |

### Other Settings

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `arukellt.server.path` | `string` | `"arukellt"` | Path to the `arukellt` binary used to launch the language server. |
| `arukellt.server.args` | `string[]` | `[]` | Extra arguments inserted before the built-in `lsp` subcommand. |
| `arukellt.emit` | `string` | `"core-wasm"` | Default emit kind for compile commands. |
| `arukellt.playgroundUrl` | `string` | `""` | Optional URL used by `Open in Playground`. Set this only if you have a real playground endpoint to open. |

## Notes

This is the bootstrap scaffold tracked by issue #189. It intentionally keeps the language client thin and uses the existing `arukellt lsp` command as the server entrypoint.
