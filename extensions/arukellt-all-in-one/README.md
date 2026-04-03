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

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `arukellt.server.path` | `string` | `"arukellt"` | Path to the `arukellt` binary used to launch the language server. |
| `arukellt.server.args` | `string[]` | `[]` | Extra arguments inserted before the built-in `lsp` subcommand. |
| `arukellt.target` | `string` | `"wasm32-wasi-p1"` | Default compilation target for check, compile, and run commands. |
| `arukellt.emit` | `string` | `"core-wasm"` | Default emit kind for compile commands. |
| `arukellt.playgroundUrl` | `string` | `"https://arukellt.dev/playground"` | Base URL of the Arukellt web playground. |
| `arukellt.enableCodeLens` | `boolean` | `true` | Show Run / Debug / Test CodeLens above functions in `.ark` files. Set to `false` to hide all CodeLens entries. |
| `arukellt.hoverDetailLevel` | `"minimal"` \| `"standard"` \| `"verbose"` | `"standard"` | Controls hover information verbosity. `minimal`: signature only. `standard`: signature + docs + availability. `verbose`: all details. |
| `arukellt.useSelfHostBackend` | `boolean` | `false` | Use the self-hosted (ark-compiled) compiler backend. Requires Stage 2 fixpoint (Issue 459). Before Stage 2, enabling this logs a warning and falls back to the Rust backend. |

## Notes

This is the bootstrap scaffold tracked by issue #189. It intentionally keeps the language client thin and uses the existing `arukellt lsp` command as the server entrypoint.
