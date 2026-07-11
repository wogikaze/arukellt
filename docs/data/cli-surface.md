# CLI surface (structured)

> **Generated** from `docs/data/cli-surface.toml`.
> Binary: `arukellt` — alias policy: `none`.
> Wrapper: `scripts/run/arukellt-selfhost.sh`. Usage source: `src/compiler/main/usage.ark`.

| Command | Status | Guarantee tier | Summary |
|---------|--------|----------------|---------|
| `arukellt compile` | `stable` | `guaranteed` | Compile an .ark file to Wasm |
| `arukellt run` | `stable` | `guaranteed` | Compile and run an .ark file |
| `arukellt check` | `stable` | `guaranteed` | Type-check without compiling |
| `arukellt build` | `stable` | `guaranteed` | Build the project in the current directory (requires ark.toml) |
| `arukellt fmt` | `stable` | `provisional` | Format .ark source files |
| `arukellt test` | `provisional` | `provisional` | Discover and type-check in-file test declarations (ADR-041) |
| `arukellt lint` | `provisional` | `provisional` | Run static analysis lints |
| `arukellt targets` | `stable` | `guaranteed` | List supported compilation targets |
| `arukellt analyze` | `provisional` | `provisional` | Analyze a compiled Wasm binary |
| `arukellt init` | `stable` | `guaranteed` | Initialize a new Arukellt project |
| `arukellt script` | `provisional` | `provisional` | Manage and run project scripts from ark.toml |
| `arukellt doc` | `stable` | `guaranteed` | Look up stdlib documentation or generate HTML reference |
| `arukellt component` | `provisional` | `provisional` | Build/inspect/validate components (build aliases compile --emit component) |
| `arukellt compose` | `provisional` | `provisional` | Validate and plan component linking; binary compose via wac plug (ADR-034) |
| `arukellt lsp` | `provisional` | `provisional` | Start the LSP server (stdio) |
| `arukellt debug-adapter` | `experimental` | `experimental` | Start the DAP debug adapter (scaffold) |
| `arukellt help` | `stable` | `guaranteed` | Print help |

Human guide: [`../cli-reference.md`](../cli-reference.md).
