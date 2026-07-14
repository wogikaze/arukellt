---
Status: done
Created: 2026-06-10
Updated: 2026-06-10
ID: 804
Track: cli
Severity: medium
---

# CLI subcommands implemented (init, fmt, targets, lint, analyze, doc, script, component)

## Summary

The CLI subcommand surface was rounded out by implementing the remaining
`CMD_NOT_YET` entries in the `ark` command dispatch. These subcommands cover
project scaffolding, source formatting, target listing, linting, static analysis,
documentation lookup, script management, and component output.

## Implemented subcommands

| Subcommand | Purpose | Notes |
|---|---|---|
| `targets` | List available compilation targets | Was already tracked in an earlier commit |
| `lint` | Run the linter over a source file | Reuses the analysis pipeline (`src/compiler/analysis/`) |
| `analyze` | Run static analysis over a source file | Reuses the analysis pipeline  |
| `doc` | Look up stdlib documentation | Resolves items from `std/manifest.toml` and prints docs |
| `init` | Scaffold a new `src/main.ark` and `ark.toml` | Project-level scaffolding |
| `fmt` | Format source code (tabs->spaces, trailing whitespace, blank lines) | Basic source formatter |
| `script` | List or look up scripts defined in `ark.toml` | Delegates to `[scripts]` table |
| `component` | Emit a WASM component | Delegates to `compile --emit component` |

## Resolution

All eight subcommands have been implemented and are available in the `ark` CLI.
No structural changes to the compiler, stdlib, or verification pipeline were
needed beyond wiring the dispatch and, where applicable, calling into existing
analysis or compilation paths.

## Evidence

- `ark targets` prints the target list and exits 0
- `ark lint <file>` runs lint and exits 0 (or reports findings)
- `ark analyze <file>` runs analysis and exits 0 (or reports findings)
- `ark doc <item>` prints documentation for the given stdlib item
- `ark init <dir>` scaffolds project structure with expected files
- `ark fmt <file>` formats and writes the file in place
- `ark script` lists scripts; `ark script <name>` prints the script command
- `ark compile --emit component` produces a WASM component output
