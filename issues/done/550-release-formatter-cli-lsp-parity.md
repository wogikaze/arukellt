# Release: Formatter CLI-LSP Parity

> **Status:** closed
> **Track:** release
> **Type:** Verification

## Scope

Ensure formatter output matches between CLI and LSP for release verification.

## Checklist Source

docs/release-checklist.md — Extension distribution section

## Acceptance

Deferred criterion: Formatter output matches between CLI and LSP (shared `format_source()`).

## Evidence

- CLI entrypoint: `crates/arukellt/src/main.rs` is a thin wasm-runner shell and does not expose a formatter command or call `format_source()`.
- LSP entrypoints: `crates/ark-lsp/src/server.rs` routes both `textDocument/formatting` and `textDocument/rangeFormatting` through `ark_parser::fmt::format_source()`.
- Test gap: no dedicated CLI-vs-LSP formatter parity test was found in `crates/ark-lsp`, `crates/arukellt`, or `tests/`; the only nearby formatter test is the LSP idempotence check in `formatter_and_fix_all_produce_consistent_output`.

## Blocker

- The close gate is not provable from current code because the repo slice has no CLI formatter entrypoint to compare against LSP output. Keep this issue open until a real CLI formatter surface exists or the release checklist is revised.

## Recheck — 2026-05-14

The earlier evidence refers to retired Rust LSP paths. Current state:

- The public `arukellt` binary is a thin wasm-runner shell around the selfhost
  compiler wasm.
- `src/compiler/main.ark` advertises `fmt` in usage text, but `parse_command`
  maps `fmt` to `CMD_NOT_YET()`.
- `src/compiler/lsp.ark` does not advertise `documentFormattingProvider` and
  has no `textDocument/formatting` or `textDocument/rangeFormatting` handler.
- The remaining Rust formatter implementation is
  `crates/ark-parser/src/fmt.rs::format_source`, but wiring the thin
  `crates/arukellt` shell back to `ark-parser` would violate the current
  selfhost-runner architecture unless the checklist is intentionally revised.

Updated verdict: close-candidate `no`. There is currently neither a selfhost CLI
formatter nor a selfhost LSP formatting provider to compare.

## Recheck — 2026-05-16

No changes relevant to this issue since the previous recheck.

- `src/compiler/main.ark` line 144: `"fmt"` still maps to `CMD_NOT_YET()`.
- `src/compiler/lsp.ark` line 330: `build_initialize_response` still does not
  advertise `documentFormattingProvider`. Supported capabilities remain:
  `textDocumentSync`, `hoverProvider`, `definitionProvider`, `diagnosticProvider`.
- `src/compiler/lsp.ark` `handle_message` (line 392+) handles: `initialize`,
  `initialized`, `shutdown`, `exit`, `textDocument/didOpen`, `textDocument/didChange`,
  `textDocument/hover`, `textDocument/definition`, `textDocument/completion` -- no
  `textDocument/formatting` or `textDocument/rangeFormatting` handler.
- The Rust `crates/ark-lsp` crate (which previously used `format_source`) has been
  removed entirely. The only remaining consumers of the Rust `format_source()`
  function are `crates/ark-parser/src/fmt.rs` itself and
  `crates/ark-playground-wasm/src/lib.rs`. The thin `crates/arukellt` CLI shell
  intentionally has no dependency on `ark-parser`.
- Recent commits to `crates/ark-parser/src/fmt.rs` (comment preservation, import
  sorting) improved the Rust formatter but do not affect the selfhost pipeline.

Updated verdict: close-candidate `no`. Neither the selfhost CLI formatter nor the
selfhost LSP formatting provider exist. The acceptance criterion ("Formatter output
matches between CLI and LSP (shared `format_source()`)") cannot be verified or
satisfied until a formatter is implemented in the selfhost compiler.

## Assessment — 2026-05-16

### Current state (unchanged since 2026-05-14)

| Component | Status |
|-----------|--------|
| Selfhost CLI `fmt` command (`src/compiler/main.ark:149`) | `CMD_NOT_YET()` |
| Selfhost LSP `documentFormattingProvider` (`src/compiler/lsp.ark:330`) | Not advertised |
| Selfhost LSP `textDocument/formatting` handler | Not implemented |
| Rust `format_source()` (`crates/ark-parser/src/fmt.rs`) | Exists but disconnected from selfhost pipeline |
| Rust `crates/ark-lsp` crate | Deleted |
| `crates/arukellt` shell dependency on `ark-parser` | Intentional absence (selfhost-runner architecture) |

### Analysis

This is a **release verification** issue, not a feature implementation issue. The
acceptance criterion requires:
1. A selfhost CLI formatter
2. A selfhost LSP formatting provider
3. Both sharing a common `format_source()` implementation

None of these exist in the selfhost compiler. The Rust formatter (`ark-parser::fmt`)
is architecturally disconnected from the selfhost pipeline: the thin `crates/arukellt`
CLI shell intentionally has no dependency on `ark-parser`, and the Rust `ark-lsp`
crate has been deleted. The only remaining Rust formatter consumer is the
playground WASM crate.

### Recommendation: Close

**Rationale:**
- The acceptance criterion cannot be verified until a formatter is implemented in
  the selfhost compiler, which is a substantial feature effort (not merely a
  verification task).
- Issue #550 has no dependents and blocks no other issue in the queue.
- The `formatter` track already shows 5 completed items and 0 remaining open issues
  (other than this one).
- The corresponding release checklist item (`docs/release-checklist.md` line 39:
  "Formatter output matches between CLI and LSP") should be removed or revised
  to reflect that CLI-LSP formatter parity is gated on selfhost formatter
  implementation.

**To close:**
1. Close this issue and move to `issues/done/`
2. Update `docs/release-checklist.md` to remove or defer the formatter parity
   check (line 39)
3. When a selfhost formatter is implemented (CLI or LSP), a new issue can be
   opened at that time to verify parity

## Required Verification

- Format same code using CLI formatter
- Format same code using LSP formatter
- Compare outputs for byte-for-byte identity
- Verify both use shared `format_source()` function

## Close Gate

CLI and LSP formatter outputs must be identical.

## Primary Paths

- CLI formatter implementation
- LSP formatter implementation
- Shared `format_source()` function
- Test fixtures for formatting

## Non-Goals

- Performance comparison
- Feature differences beyond output identity
