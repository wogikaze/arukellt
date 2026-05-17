# Diagnostics Worker and Parse Loop Performance Budget

**Status**: DRAFT
**Updated**: 2026-05-17
**Issue**: #430; updated by #631
**Scope**: Playground runtime performance, diagnostics pipeline, worker
round-trip, and measurement methodology

## Current Architecture

The browser playground uses `playground/src/engine.ts` for parse, format,
tokenize, and typecheck-shaped diagnostics. The same engine can run on the main
thread through `createPlayground` or in a dedicated worker through
`createWorkerPlayground`.

The former Rust playground Wasm bridge was retired in #631. Historical Wasm
parse latency and wasm-pack size budgets are no longer active CI gates.

## Runtime Loop

```text
Editor (main thread)
  user edits source
  debounce
  postMessage to worker
Worker
  parseSource(source)
  build JSON-compatible diagnostics response
  postMessage to main thread
Main thread
  render diagnostics panel and source overlay
```

## Target Latencies

Targets are measured on curated playground snippets.

| Stage | Budget p50 | Budget p95 | Notes |
|-------|------------|------------|-------|
| Tokenize | <= 2 ms | <= 8 ms | `tokenizeSource` full pass |
| Parse summary | <= 5 ms | <= 20 ms | includes delimiter diagnostics |
| Worker round-trip | <= 20 ms | <= 75 ms | includes structured clone |
| Render diagnostics | <= 8 ms | <= 16 ms | stay within one frame at p95 |

Large pasted sources should degrade gracefully: no main-thread parse work in
worker mode, and no unbounded retained source snapshots.

## CI and Manual Checks

- JS bundle size is checked by `.github/workflows/playground-ci.yml`.
- Lighthouse runs against the static playground page.
- Future performance automation should benchmark `playground/src/engine.ts`
  directly from Node and from a browser worker harness.

## Historical Retired Path

Earlier versions of this document budgeted a Rust Wasm module, Wasm compilation
time, and wasm-pack output size. Those entries are historical after #631 and
must not be treated as current requirements.
