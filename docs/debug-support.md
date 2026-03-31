# Debug Support

> **Status**: Scaffold — DAP server exists and handles initialize/launch/basic
> protocol, but source-level breakpoints require runtime hooks not yet available.
> See issues #277 and #280 for the complete debug implementation plan.

## Overview

Arukellt provides a Debug Adapter Protocol (DAP) server (`crates/ark-dap`)
that integrates with VS Code and any DAP-compatible editor. This document
describes the current scope, what is and is not supported per target, and
how to use the debug adapter.

## Target Support Matrix

| Target | Debug Status | Breakpoints | Stepping | Variables |
|--------|-------------|-------------|---------|-----------|
| `wasm32-wasi-p1` (T1) | ⚡ Run-only | ❌ Not available | ❌ Not available | ❌ Not available |
| `wasm32-wasi-p2` (T3) | ⚡ Run-only | ❌ Not available | ❌ Not available | ❌ Not available |
| T2/T4/T5 | 🔴 Not implemented | — | — | — |

**Canonical debug target**: `wasm32-wasi-p1` (T1) is the intended first target
for full debug support. T3 (GC-native) adds complexity due to reference types.

### What "Run-only" means

The DAP server can launch an Arukellt program and:
- Capture stdout/stderr as DAP output events
- Report program exit via `exited` + `terminated` events
- Support `disconnect` to terminate the session cleanly

What it **cannot** do yet:
- Stop at a source-level breakpoint
- Inspect variables or stack frames
- Step through source lines

This means `F5` (start debugging) works and shows program output in the Debug
Console, but `F9` (toggle breakpoint) has no effect on execution.

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
            "program": "${workspaceFolder}/src/main.ark"
        }
    ]
}
```

Or use the **Run → Start Debugging** menu when an `.ark` file is open.

### Expected behavior (current)

When you press F5:
1. VS Code sends `initialize` → `launch` → `configurationDone` to the DAP server
2. The DAP server runs `arukellt run <program>`
3. Program stdout/stderr appears in the **Debug Console**
4. When the program exits, VS Code reports "Process exited with exit code N"

### Known limitations

- Setting breakpoints does nothing (they are acknowledged but not enforced)
- The call stack shows empty frames
- Variables pane shows no variables
- The `pause` button has no effect (program runs to completion)

## DAP Server Architecture

The DAP server is a standalone binary (`arukellt debug-adapter`) that reads
from stdin and writes to stdout using the DAP protocol.

```
VS Code ←─── DAP messages ───→ arukellt debug-adapter
                                       │
                                       └─── spawns: arukellt run <file>
                                                    captures stdout/stderr
                                                    sends output events
```

Source: `crates/ark-dap/src/lib.rs`

### Supported DAP requests (current)

| Request | Status |
|---------|--------|
| `initialize` | ✅ Full |
| `launch` | ✅ Records source path |
| `configurationDone` | ✅ Runs program, streams output |
| `setBreakpoints` | ⚡ Acknowledged, not enforced |
| `threads` | ⚡ Returns single dummy thread |
| `stackTrace` | ⚡ Returns empty frames |
| `scopes` | ⚡ Returns empty scopes |
| `variables` | ⚡ Returns empty variables |
| `continue` | ⚡ Returns success (no-op) |
| `next` | ⚡ Returns success (no-op) |
| `stepIn` | ⚡ Returns success (no-op) |
| `stepOut` | ⚡ Returns success (no-op) |
| `terminate` | ✅ Kills spawned process |
| `disconnect` | ✅ Ends session |

Legend: ✅ = functional, ⚡ = stub/acknowledged, ❌ = not implemented

## Roadmap

### Phase 1: Source location tracking (prerequisite for breakpoints)

The compiler must emit source maps — a mapping from Wasm instruction offsets
back to source file + line + column. This is not yet implemented.

### Phase 2: Breakpoint injection (wasmtime debug hook)

Once source maps are available, the DAP server needs to inject breakpoints
via wasmtime's debug API. The wasmtime debug hook API is under development
upstream.

### Phase 3: Variable inspection

Variable inspection requires access to Wasm locals and the call stack at
a breakpoint. This is the most complex phase and depends on Phase 2.

See issues #277 (breakpoint implementation) and #280 (DAP test wiring) for
the tracked work.

## Source Location ↔ DAP Line/Column Correspondence

When source maps are implemented, Arukellt will use:
- **Line numbers**: 1-based (matching editor convention)
- **Column numbers**: 0-based (DAP convention)
- **Source paths**: absolute paths to `.ark` source files

This matches the DAP specification's `Source` structure.

## Testing

See `tests/dap/` for DAP protocol smoke tests (planned in #280).
The test runner will send the `initialize → launch → configurationDone → disconnect`
sequence and verify that output events arrive correctly.
