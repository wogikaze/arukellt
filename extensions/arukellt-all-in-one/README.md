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

## Settings

- `arukellt.server.path` — path to the `arukellt` binary (default: `arukellt`)
- `arukellt.server.args` — extra args inserted before `lsp`

## Notes

This is the bootstrap scaffold tracked by issue #189. It intentionally keeps the language client thin and uses the existing `arukellt lsp` command as the server entrypoint.
