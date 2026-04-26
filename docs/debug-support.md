# Debug Support

## Overview

Arukellt provides a Debug Adapter Protocol (DAP) server (`crates/ark-dap`)
that integrates with VS Code and any DAP-compatible editor. The debug adapter
supports source-level breakpoints, stepping, stack traces, and variable
inspection through static source analysis.

## Target Support Matrix

| Target | Debug Status | Breakpoints | Stepping | Variables |
|--------|-------------|-------------|---------|-----------|
| `wasm32-wasi-p1` (T1) | ✅ Supported | ✅ Source-level | ✅ Next/Continue | ✅ Static |
| `wasm32-wasi-p2` (T3) | ✅ Supported | ✅ Source-level | ✅ Next/Continue | ✅ Static |
| `wasm32-component` | ⚡ Best-effort | ✅ Source-level | ✅ Next/Continue | ✅ Static |
| T2/T4/T5 | 🔴 Not implemented | — | — | — |

**Canonical debug target**: Both T1 and T3 are supported for debugging.

### What "Supported" means

The DAP server provides a **source-level stepping model**:

1. Source file is loaded and parsed for executable lines
2. Program is compiled and run via `arukellt run`
3. Breakpoints can pause execution at specific source lines
4. `next` / `continue` advance through source lines
5. Stack trace shows current frame with enclosing function name
6. Variables pane shows visible `let` bindings and function parameters

## VS Code Usage

### Prerequisites

1. Install the `arukellt-all-in-one` VS Code extension
2. Build the `arukellt` binary: `cargo build`

### Launch configuration

Add to your project's `.vscode/launch.json`:

```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "arukellt",
            "request": "launch",
            "name": "Debug Arukellt Program",
            "program": "${workspaceFolder}/src/main.ark",
            "stopOnEntry": false
        }
    ]
}
```

Or use the **Run → Start Debugging** menu when an `.ark` file is open.

### Debug workflow

When you press F5:

1. VS Code sends `initialize` → `launch` → `setBreakpoints` → `configurationDone`
2. The DAP server loads the source and compiles/runs `arukellt run <program>`
3. If breakpoints are set, execution pauses at the first breakpoint
4. Stack trace, scopes, and variables are available in the debug sidebar
5. Use Continue (F5) to advance to next breakpoint, or Step Over (F10) for line-by-line
6. Program stdout/stderr appears in the **Debug Console**
7. When stepping completes or no more breakpoints, program output is emitted

### `stopOnEntry` option

Set `"stopOnEntry": true` in launch configuration to pause at the first
executable line of the program, before any breakpoints.

## DAP Server Architecture

The DAP server is a standalone binary (`arukellt debug-adapter`) that reads
from stdin and writes to stdout using the DAP protocol.

```text
VS Code ←─── DAP messages ───→ arukellt debug-adapter
                                       │
                                       ├─── loads source for stepping
                                       └─── spawns: arukellt run <file>
                                                    captures stdout/stderr
                                                    sends output events
```

Source: `crates/ark-dap/src/lib.rs`

### Supported DAP requests

| Request | Status |
|---------|--------|
| `initialize` | ✅ Full capabilities |
| `launch` | ✅ Loads source, supports stopOnEntry |
| `configurationDone` | ✅ Runs program, stops at breakpoints |
| `setBreakpoints` | ✅ Verified, line-adjusted to executable lines |
| `threads` | ✅ Returns main thread |
| `stackTrace` | ✅ Current frame with function name and source |
| `scopes` | ✅ Locals scope when stopped |
| `variables` | ✅ Visible let bindings and parameters |
| `continue` | ✅ Advances to next breakpoint or end |
| `next` | ✅ Steps to next executable line |
| `stepIn` | ✅ Same as next (no call-level granularity) |
| `stepOut` | ✅ Same as next (no call-level granularity) |
| `terminate` | ✅ Ends session |
| `disconnect` | ✅ Ends session |

## Source Location ↔ DAP Line/Column Correspondence

- **Line numbers**: 1-based (matching editor convention)
- **Column numbers**: 1-based (DAP convention)
- **Source paths**: absolute paths to `.ark` source files
- **Executable lines**: non-empty, non-comment, non-import, non-brace-only lines

Breakpoints set on non-executable lines are automatically adjusted to the next
executable line.

## Limitations

- Variables show source-text values, not runtime values
- Step In / Step Out behave identically to Step Over
- No watch expressions or evaluate support
- No multi-thread debugging (single "main" thread)
- Breakpoints are simulated at source level, not injected into Wasm runtime
- No conditional breakpoints or function breakpoints

## Future: Runtime-Level Debugging

A future enhancement will add Wasm-level breakpoint injection for true runtime
debugging with live variable values. This requires:

1. Compiler source map emission (Wasm offset → source line mapping)
2. Wasmtime debug hook API integration
3. Live variable inspection through Wasm locals

## Testing

DAP unit tests: `cargo test -p ark-dap` (6 tests covering function detection,
variable extraction, breakpoint hit, and stepping).

Extension E2E tests verify debug type registration, launch configuration
templates, and initial configurations.
