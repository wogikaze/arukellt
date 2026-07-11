# CLI surface (structured)

> **Generated** from `docs/data/cli-surface.toml`.
> Binary: `arukellt` — alias policy: `none`.
> Wrapper: `scripts/run/arukellt-selfhost.sh`. Usage source: `src/compiler/main/usage.ark`.

| Command | Status | Presence stability | Implementation | Release guarantee IDs | Summary |
|---------|--------|--------------------|----------------|-----------------------|---------|
| `arukellt compile` | `stable` | `stable` | `functional` | `compile_wasm32_gc`, `compile_wasm32` | Compile an .ark file to Wasm |
| `arukellt run` | `stable` | `stable` | `functional` | `run_wasmtime` | Compile and run an .ark file |
| `arukellt check` | `stable` | `stable` | `functional` | `cli_check` | Type-check without compiling |
| `arukellt build` | `stable` | `stable` | `limited` | — | Build the project in the current directory (requires ark.toml) |
| `arukellt fmt` | `stable` | `provisional` | `functional` | — | Format .ark source files |
| `arukellt test` | `provisional` | `provisional` | `functional` | — | Discover and type-check in-file test declarations (ADR-041) |
| `arukellt lint` | `provisional` | `provisional` | `functional` | — | Run static analysis lints |
| `arukellt targets` | `stable` | `stable` | `unavailable` | — | List supported compilation targets |
| `arukellt analyze` | `provisional` | `provisional` | `functional` | — | Analyze a compiled Wasm binary |
| `arukellt init` | `stable` | `stable` | `functional` | `cli_init` | Initialize a new Arukellt project |
| `arukellt script` | `provisional` | `provisional` | `functional` | — | Manage and run project scripts from ark.toml |
| `arukellt doc` | `stable` | `stable` | `functional` | `cli_doc` | Look up stdlib documentation or generate HTML reference |
| `arukellt component` | `provisional` | `provisional` | `functional` | — | Build/inspect/validate components (build aliases compile --emit component) |
| `arukellt compose` | `provisional` | `provisional` | `functional` | — | Validate and plan component linking; binary compose via wac plug (ADR-034) |
| `arukellt lsp` | `provisional` | `provisional` | `functional` | — | Start the LSP server (stdio) |
| `arukellt debug-adapter` | `experimental` | `experimental` | `functional` | — | Start the DAP debug adapter (scaffold) |
| `arukellt help` | `stable` | `stable` | `functional` | `cli_help` | Print help |

Human guide: [`../cli-reference.md`](../cli-reference.md).
