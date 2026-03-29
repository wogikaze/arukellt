# Arukellt All-in-One

Minimal VS Code extension scaffold for Arukellt.

## Current scope

- Registers `.ark` as the `arukellt` language
- Provides a basic language configuration, grammar, and snippets
- Launches `arukellt lsp` using the configured CLI path
- Supports restarting the language server from the command palette

## Settings

- `arukellt.server.path` — path to the `arukellt` binary (default: `arukellt`)
- `arukellt.server.args` — extra args inserted before `lsp`

## Notes

This is the bootstrap scaffold tracked by issue #189. It intentionally keeps the language client thin and uses the existing `arukellt lsp` command as the server entrypoint.
