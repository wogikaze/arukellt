# T3 Reachability Root Contract

> Defines the root set for dead function elimination on T3 (wasm32-wasi-p2)
> component-model output. This contract governs which functions are preserved
> during MIR-level reachability pruning.

## Status

**Decided**. Implemented as part of issue #611.

## Motivation

Dead function elimination (pruning) removes functions that are not reachable
from any root. For T1 core Wasm modules, the root set is simply `main` and
`_start` -- the standard WASI entry points. For T3 component-model output,
WIT-exported functions can also serve as entry points, and must be preserved
even if nothing in the user's `main` path calls them.

Prior to this contract, T3 component output disabled dead function elimination
entirely (`lower_to_mir_no_prune`), because the reachability analysis was not
aware of exported functions.

## Root Categories

Functions are preserved if they are reachable (transitive call-graph closure)
from any of the following root categories:

### 1. Entry Roots

The canonical WASI entry points:

| Name      | Description                                      |
|-----------|--------------------------------------------------|
| `main`    | Standard program entry point (returns i32 exit code) |
| `_start`  | Low-level WASI entry point (used by some runtimes) |

These are always roots, regardless of target or emit mode.

### 2. Exported Roots

Functions exported through the WIT interface (component model `export`):

- Every `pub fn` declared at module scope that is not a special builtin
  (`main`, `_start`, `print`, `println`, `eprintln`, or containing `::`).
- These functions are callable by the host through the component's export
  section and must survive pruning.

### 3. Host-Reachable Roots

Functions that can be invoked by the host runtime:

- WASI imported functions (e.g. `fd_write`, `path_open`, `args_sizes_get`)
  are reachability roots only when the reachable code references them.
- The T3 backend emitter conditionally includes WASI imports based on the
  reachable function set.

### 4. Internal-Call Roots (Transitive Closure)

Any function reachable from another root via a `MIR_CALL` instruction:

- The transitive closure is computed by BFS: starting from all identified
  roots, follow every `MIR_CALL` operand to its callee and mark it reachable.
- Iteration continues until a fixed point is reached (no new functions
  marked in a pass).

## Implementation

### MIR-level pruning (`src/compiler/mir_lower.ark`)

The function `mir_prune_unreachable_with_roots(m, extra_roots)` performs
the BFS-based reachability analysis:

1. Mark `main` and `_start` as roots unconditionally.
2. Mark each name in `extra_roots` as a root (these are collected from
   pub function declarations).
3. BFS-follow all `MIR_CALL` instructions to mark transitive callees.
4. Remove all unmarked functions from the module.

### Driver integration (`src/compiler/driver.ark`)

For T3 component/wit emit mode:

1. Lower to MIR without initial pruning (`lower_to_mir_no_prune`).
2. Scan all declarations for `pub fn` entries that pass
   `driver_should_export_func` -- these become `extra_roots`.
3. Call `mir_prune_unreachable_for_t3(m, export_roots)`.

For T1 core wasm mode, the original `lower_to_mir` (with built-in
`main`/`_start` pruning) is used unchanged.

### Backend-level safety net (`src/compiler/emitter.ark`)

The T3 backend emitter emits every function present in the MIR module.
It does not perform a secondary reachability pass -- the MIR-level
pruning is the sole gate. This means the correctness of the root contract
is critical: any function that should be reachable but is not in the
root set will be incorrectly pruned.

## Verification

- **Regression fixture**: `tests/fixtures/component/export_dead_fn_elim.ark`
  verifies that an exported function not called from `main` survives
  pruning while a truly dead (unexported, uncalled) function is removed.
- **Existing component-compile fixtures**: All `component-compile:` entries
  in the manifest continue to pass, confirming that exported functions
  with various signatures are preserved.
- **t3-compile and t3-run fixtures**: Existing T3 tests confirm that
  reachability pruning does not break normal compilation or execution.
