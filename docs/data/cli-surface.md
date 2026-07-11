# CLI surface (structured)

> **Generated** from `docs/data/cli-surface.toml`.
> Binary: `arukellt` — alias policy: `none`.
> Wrapper: `scripts/run/arukellt-selfhost.sh`. Usage source: `src/compiler/main/usage.ark`.

> Axes (do not overload a single `status` field):
> - **Presence**: `present` | `absent` — whether the subcommand exists in the binary
> - **Contract stability**: `stable` | `provisional` | `experimental` — CLI contract maturity
> - **Implementation**: `functional` | `limited` | `scaffold` | `unavailable` | `unknown` — runtime behavior

| Command | Presence | Contract stability | Implementation | Guarantee IDs | Summary |
|---------|----------|--------------------|----------------|---------------|---------|
| `arukellt compile` | `present` | `stable` | `functional` | `compile_wasm32_gc`, `compile_wasm32` | Compile an .ark file to Wasm |
| `arukellt run` | `present` | `stable` | `functional` | `run_wasmtime` | Compile and run an .ark file |
| `arukellt check` | `present` | `stable` | `functional` | `cli_check` | Type-check without compiling |
| `arukellt build` | `present` | `stable` | `limited` | — | Build the project in the current directory (requires ark.toml) |
| `arukellt fmt` | `present` | `provisional` | `functional` | — | Format .ark source files |
| `arukellt test` | `present` | `provisional` | `functional` | — | Discover and type-check in-file test declarations (ADR-041) |
| `arukellt lint` | `present` | `provisional` | `functional` | — | Run static analysis lints |
| `arukellt targets` | `present` | `stable` | `unavailable` | — | List supported compilation targets |
| `arukellt analyze` | `present` | `provisional` | `functional` | — | Analyze a compiled Wasm binary |
| `arukellt init` | `present` | `stable` | `functional` | `cli_init` | Initialize a new Arukellt project |
| `arukellt script` | `present` | `provisional` | `functional` | — | Manage and run project scripts from ark.toml |
| `arukellt doc` | `present` | `stable` | `functional` | `cli_doc` | Look up stdlib documentation or generate HTML reference |
| `arukellt component` | `present` | `provisional` | `limited` | — | Build/inspect/validate components (build aliases compile --emit component) |
| `arukellt compose` | `present` | `provisional` | `limited` | — | Validate and plan component linking; binary compose via wac plug (ADR-034) |
| `arukellt lsp` | `present` | `provisional` | `functional` | — | Start the LSP server (stdio) |
| `arukellt debug-adapter` | `present` | `experimental` | `scaffold` | — | Start the DAP debug adapter (scaffold) |
| `arukellt help` | `present` | `stable` | `functional` | `cli_help` | Print help |

Human guide: [`../cli-reference.md`](../cli-reference.md).
