# Diagnostics Worker and Parse Loop Performance Budget

**Status**: DRAFT
**Created**: 2026-06-22
**Issue**: #430
**Scope**: Playground runtime performance — parse loop, diagnostics pipeline,
worker round-trip, incremental parse strategy, and measurement methodology
**Related**:
  ADR-017 (execution model),
  ADR-022 (deployment & caching, load-time budgets),
  `docs/playground/deployment-strategy.md` §7 (asset size & load budgets)

---

## 1. Purpose and Scope

`docs/playground/deployment-strategy.md` §7 defines **load-time** performance
budgets: asset transfer sizes, Time to Interactive, and Wasm compilation time.
This document defines **runtime operational** performance budgets — the
latencies a user experiences *after* the playground is loaded and interactive,
while editing code and observing diagnostics.

The playground architecture (per ADR-017) runs parse, format, and diagnostics
via a `wasm32-unknown-unknown` module inside a dedicated Web Worker. Every
keystroke-triggered parse flows through this pipeline:

```
  Editor (main thread)
    │  user edits source
    ▼
  Debounce (main thread)
    │  wait for typing pause
    ▼
  postMessage to worker
    │  serialize source string
    ▼
  Worker: wasm parse(source)
    │  lexer → parser → diagnostics
    ▼
  Worker: JSON.stringify(result)
    │  serialize AST + diagnostics
    ▼
  postMessage to main thread
    │  structured clone transfer
    ▼
  Main thread: render diagnostics
    │  panel update + overlay rebuild
    ▼
  User sees updated diagnostics
```

Each stage has a latency cost. This document budgets each stage, defines the
v1 targets, and describes how to measure them.

---

## 2. Definitions

| Term | Definition |
|------|-----------|
| **Parse latency** | Wall-clock time from `wasm parse()` entry to return inside the Worker, excluding serialization |
| **Diagnostics emit** | Time to convert the internal diagnostic representation to the JSON wire format inside Wasm |
| **Worker round-trip** | Wall-clock time from `postMessage` (main → worker) to the corresponding `message` event (worker → main), including parse, serialization, and structured clone |
| **Render latency** | Time to update the diagnostics panel and source overlay on the main thread after receiving the worker response |
| **End-to-end latency** | Time from user keystroke to visible diagnostics update (debounce + worker round-trip + render) |
| **Input size** | Source code length in bytes (UTF-8 encoded) |
| **p50 / p95 / p99** | Percentile latencies measured over repeated runs on the reference hardware |

---

## 3. Target Latencies

### 3.1 v1 Targets (current architecture: full reparse per edit)

All latencies measured on the **reference hardware** defined in §6.

| Stage | Budget (p50) | Budget (p95) | Input size | Notes |
|-------|-------------|-------------|------------|-------|
| **Parse (Wasm)** | ≤ 5 ms | ≤ 15 ms | ≤ 1 KB (typical playground snippet) | Lexer + parser + diagnostic collection |
| **Parse (Wasm)** | ≤ 20 ms | ≤ 50 ms | ≤ 10 KB (large example) | Upper bound for v1 curated examples |
| **Parse (Wasm)** | ≤ 100 ms | ≤ 200 ms | ≤ 50 KB (stress test) | Graceful degradation; not a v1 gate |
| **Diagnostics emit** | ≤ 2 ms | ≤ 5 ms | Any | JSON serialization of ≤ 200 diagnostics |
| **Worker round-trip** | ≤ 10 ms | ≤ 25 ms | ≤ 1 KB | postMessage + parse + serialize + postMessage |
| **Worker round-trip** | ≤ 30 ms | ≤ 60 ms | ≤ 10 KB | Same, larger input |
| **Render (main thread)** | ≤ 5 ms | ≤ 16 ms | ≤ 100 diagnostics | Panel update + overlay rebuild; must not drop frames (16 ms = 60 fps) |
| **Debounce interval** | 150 ms | — | — | Tunable; balances responsiveness vs. redundant parses |
| **End-to-end** | ≤ 200 ms | ≤ 300 ms | ≤ 1 KB | Total: debounce (150) + round-trip (10–25) + render (5–16) |

**Key constraint**: The main thread must never be blocked by Wasm execution.
All Wasm calls happen in the Worker. The render budget (≤ 16 ms p95) ensures
that diagnostics updates do not cause jank or dropped frames.

### 3.2 Diagnostic Count Scaling

The diagnostics panel and overlay must handle the following diagnostic counts
without degrading render performance below the budgets in §3.1:

| Diagnostic count | Render budget (p95) | Notes |
|-----------------|-------------------|-------|
| ≤ 50 | ≤ 8 ms | Typical well-formed or small file |
| ≤ 200 | ≤ 16 ms | Dense error cases; v1 hard ceiling |
| ≤ 500 | ≤ 50 ms | Future/stress; acceptable degradation |
| > 500 | Truncate + show count | "Showing 500 of N diagnostics" |

Rationale: `diagnostics.ts` builds a per-character severity map for the
overlay (`buildDiagnosticOverlay`). With > 200 diagnostics on large source,
the O(source_len × diag_count) loop becomes measurable. Truncation at 500
prevents runaway render times.

### 3.3 Future Targets (v2+, with incremental parse)

These targets are aspirational and depend on the incremental parse strategy
described in §5. They are documented here for planning purposes and are
**not v1 gates**.

| Stage | Budget (p50) | Budget (p95) | Input size | Notes |
|-------|-------------|-------------|------------|-------|
| **Incremental parse** | ≤ 2 ms | ≤ 5 ms | ≤ 10 KB, single edit | Re-lex + reparse changed region only |
| **Worker round-trip (incremental)** | ≤ 5 ms | ≤ 15 ms | ≤ 10 KB | Smaller serialized delta |
| **End-to-end (incremental)** | ≤ 100 ms | ≤ 170 ms | ≤ 10 KB | Debounce (100 ms, reduced) + round-trip + render |

---

## 4. Wasm Module Size Budget

This section supplements `deployment-strategy.md` §7 with runtime-relevant
size constraints. The module size directly affects Wasm compilation time
(cold start) and code cache efficiency (warm start).

| Metric | Current | v1 Budget | Action on exceed |
|--------|---------|-----------|-----------------|
| `.wasm` (post `wasm-opt -Oz`) | 247 KB | ≤ 300 KB | CI fails (per ADR-022) |
| `.wasm` (gzipped) | ~100 KB | ≤ 150 KB | CI fails |
| Wasm compile time (cold) | ~200 ms est. | ≤ 500 ms | Advisory; log via `performance.measure()` |
| Wasm compile time (warm, code cache) | ~10 ms est. | ≤ 50 ms | Advisory |

### 4.1 Size growth projections

| Feature addition | Est. size impact | v1? |
|-----------------|-----------------|-----|
| Type checker (check-only) | +50–100 KB | ✅ planned (ADR-017) |
| Formatter improvements | +5–10 KB | ✅ planned |
| Full codegen backend | +100–200 KB | ❌ v2+ |
| Stdlib bundling | +20–50 KB | ❌ v2+ |

The type checker addition is the most likely v1 size impact. If it pushes the
module past 300 KB, the team must either optimize the type checker Wasm output
(dead-code elimination, `wasm-opt` tuning) or request a budget increase with
justification. Budget increases require updating ADR-022 and this document.

---

## 5. Incremental Parse Strategy

### 5.1 v1 Decision: Full Reparse

**v1 uses full reparse on every edit.** The entire source string is sent to
the Worker and parsed from scratch by the Wasm module.

**Rationale**:

1. **Simplicity**: The current `parse(source: &str) -> String` Wasm API
   accepts a complete source string and returns a complete result. No change
   tracking, tree diffing, or edit protocol is needed.

2. **Adequate performance**: For v1 playground inputs (≤ 10 KB curated
   examples), full reparse is budgeted at ≤ 20 ms p50. Combined with the
   150 ms debounce, the user perceives near-instant feedback.

3. **No parser modification required**: The `ark-parser` crate's public API
   is a batch `parse(source) → AST` function. An incremental API would
   require significant parser refactoring that is out of scope for v1.

4. **Correctness**: Full reparse eliminates the class of bugs where
   incremental parse state becomes inconsistent after complex edits
   (multi-cursor, find-replace, paste).

### 5.2 When to Move to Incremental

Full reparse becomes insufficient when *any* of these thresholds are crossed:

| Trigger | Threshold | Measurement |
|---------|-----------|-------------|
| Parse latency exceeds budget | p95 > 50 ms on reference hardware | `performance.measure()` in Worker |
| Source size routinely exceeds 10 KB | > 25% of sessions use > 10 KB input | Client-side analytics (v2) |
| User-reported lag | Qualitative feedback | User reports / issue tracker |
| Debounce must increase past 250 ms | Required to hide parse latency | Indicates parse is too slow |

If none of these triggers fire, full reparse remains the correct choice.
Do not pre-optimize.

### 5.3 Future Incremental Architecture (v2+ Design Sketch)

When incremental parsing is warranted, the following architecture is
recommended. This is a **design sketch**, not an implementation plan.

```
  Editor (main thread)
    │  user edits source
    │  editor reports change delta: { offset, deletedLen, insertedText }
    ▼
  postMessage to worker
    │  send delta (not full source)
    ▼
  Worker: incremental_parse(delta)
    │  1. Re-lex affected span (± context window)
    │  2. Identify invalidated parse tree nodes
    │  3. Reparse invalidated region
    │  4. Splice new nodes into existing tree
    │  5. Re-run diagnostics on affected nodes
    ▼
  Worker: serialize changed diagnostics only
    ▼
  postMessage to main thread
    │  send diagnostic diff: { added: [], removed: [], changed: [] }
    ▼
  Main thread: patch diagnostics panel & overlay
```

**Key design choices for incremental (v2)**:

| Choice | Recommendation | Rationale |
|--------|---------------|-----------|
| Edit protocol | TextEdit deltas (offset + delete + insert) | Matches CodeMirror/Monaco change events; avoids full source copy |
| Re-lex scope | Changed span ± 1 token context | Most lex errors are local; context window catches multi-char tokens spanning the edit boundary |
| Reparse scope | Smallest enclosing item (fn, struct, etc.) | Item-level granularity balances correctness with incremental savings |
| Tree representation | Persistent (immutable nodes, shared subtrees) | Enables O(changed_nodes) serialization instead of O(tree_size) |
| Diagnostic diff | Add/remove/change sets | Avoids re-rendering unchanged diagnostics |

**Wasm API change required for incremental**:

```rust
// Current v1 API (full reparse)
#[wasm_bindgen]
pub fn parse(source: &str) -> String;

// Future v2 API (incremental) — sketch only
#[wasm_bindgen]
pub fn create_session() -> SessionId;

#[wasm_bindgen]
pub fn apply_edit(
    session: SessionId,
    offset: u32,
    deleted_len: u32,
    inserted_text: &str,
) -> String; // returns diagnostic diff JSON
```

This requires parser-level changes in `ark-parser` and is explicitly **not
in scope for v1**.

---

## 6. Measurement Methodology

### 6.1 Reference Hardware

Performance targets are defined against two reference platforms:

| Platform | Spec | Represents |
|----------|------|-----------|
| **Desktop (primary)** | Modern x86-64, ≥ 4 cores, ≥ 8 GB RAM, Chrome/Firefox latest | Developer workstation |
| **Mobile (secondary)** | Mid-tier Android (e.g., Pixel 6a level), Chrome latest | Conference demo, "try it on your phone" |

All p50/p95/p99 budgets in §3 apply to the **desktop** reference. Mobile
targets are 2–3× the desktop budget and are advisory (not CI-enforced).

### 6.2 Instrumentation Points

Each stage in the pipeline should be instrumented with `performance.mark()`
and `performance.measure()`. The following marks are defined:

| Mark name | Location | Description |
|-----------|----------|-------------|
| `ark:edit` | Main thread, editor change handler | User edit event fires |
| `ark:debounce-end` | Main thread, after debounce timer | Debounce period elapsed; dispatching to worker |
| `ark:worker-send` | Main thread, before `postMessage` | Message sent to worker |
| `ark:worker-recv` | Worker, `message` event handler entry | Message received in worker |
| `ark:parse-start` | Worker, before `wasmExports.parse()` | Wasm parse begins |
| `ark:parse-end` | Worker, after `wasmExports.parse()` | Wasm parse returns (includes JSON serialization inside Wasm) |
| `ark:worker-respond` | Worker, before `postMessage` response | Worker posts result back |
| `ark:main-recv` | Main thread, `message` event handler | Result received from worker |
| `ark:render-start` | Main thread, before diagnostics panel update | Render begins |
| `ark:render-end` | Main thread, after diagnostics panel + overlay update | Render complete |

**Derived measures**:

| Measure | Calculation | Maps to budget |
|---------|------------|---------------|
| Parse latency | `ark:parse-end` − `ark:parse-start` | §3.1 Parse (Wasm) |
| Worker round-trip | `ark:main-recv` − `ark:worker-send` | §3.1 Worker round-trip |
| Render latency | `ark:render-end` − `ark:render-start` | §3.1 Render |
| End-to-end | `ark:render-end` − `ark:debounce-end` | §3.1 End-to-end |
| Debounce delay | `ark:debounce-end` − `ark:edit` | §3.1 Debounce interval |

### 6.3 Benchmark Protocol

To produce reproducible measurements:

1. **Warm-up**: Run 5 parse cycles before measuring (ensures Wasm is JIT-compiled
   and code-cached).

2. **Iterations**: Run ≥ 100 parse cycles per input for p50/p95/p99 calculation.

3. **Inputs**: Use standardized benchmark inputs:

   | Input name | Size | Description |
   |-----------|------|-------------|
   | `trivial.ark` | ~50 bytes | `fn main() {}` |
   | `small.ark` | ~500 bytes | Typical playground snippet (2–3 functions) |
   | `medium.ark` | ~2 KB | Realistic module (structs, enums, impls) |
   | `large.ark` | ~10 KB | Largest v1 curated example |
   | `stress.ark` | ~50 KB | Stress test (synthetic, many items) |
   | `errors.ark` | ~2 KB | Intentionally malformed; many diagnostics |

4. **Isolation**: Close other tabs/apps. Use `--disable-background-timer-throttling`
   in Chrome to prevent worker throttling.

5. **Reporting**: Record p50, p95, p99, min, max for each measure × input pair.

### 6.4 CI Integration (v1: advisory, future: gating)

**v1 (advisory)**: Parse latency is logged in CI builds but does not block
merge. The Wasm size gate (§4) is the only CI-blocking performance check.

Rationale: Runtime latency depends on the CI runner's CPU, which varies.
Stable CI-enforced latency gating requires dedicated benchmark hardware or
a noise-aware regression detection system (e.g., criterion.rs with
significance thresholds). This is a v2 concern.

**Future (gating)**:

```yaml
# .github/workflows/playground-perf.yml (sketch)
- name: Parse benchmark
  run: |
    # Build Wasm, run benchmark script, compare to baseline
    cargo bench -p ark-playground-wasm --bench parse_latency
    # Fail if p95 regresses > 20% from baseline
```

Using `criterion.rs` for Rust-side benchmarks (native, not Wasm) provides
a proxy signal: if native parse p95 regresses significantly, Wasm will too.
This avoids the CI runner variability problem.

### 6.5 Local Profiling Workflow

For developers investigating performance:

```bash
# 1. Build with profiling symbols
cargo build --target wasm32-unknown-unknown \
  -p ark-playground-wasm --release

# 2. Optimize (keep name section for profiling)
wasm-opt -O2 --debuginfo \
  target/wasm32-unknown-unknown/release/ark_playground_wasm.wasm \
  -o playground/dist/ark-playground-debug.wasm

# 3. Start local dev server
cd playground && npm run dev

# 4. Open Chrome DevTools → Performance tab
# 5. Record a session with typing
# 6. Look for:
#    - "ark:parse-start" → "ark:parse-end" measures in User Timing
#    - Long tasks on main thread (should be none from Wasm)
#    - Worker thread activity alignment with debounce
```

---

## 7. v1 vs Future Summary

| Concern | v1 | Future (v2+) |
|---------|-----|-------------|
| Parse strategy | Full reparse every edit | Incremental (re-lex + reparse changed region) |
| Edit protocol | Full source string via `postMessage` | TextEdit deltas (offset + delete + insert) |
| Diagnostic serialization | Full diagnostic array each time | Diagnostic diff (add/remove/change) |
| Parse budget (1 KB, p50) | ≤ 5 ms | ≤ 2 ms (incremental) |
| Worker round-trip (1 KB, p50) | ≤ 10 ms | ≤ 5 ms (smaller payloads) |
| End-to-end budget (p50) | ≤ 200 ms | ≤ 100 ms (reduced debounce possible) |
| Debounce | 150 ms (fixed) | 100 ms or adaptive (if parse is fast enough) |
| Wasm module size | ≤ 300 KB raw | ≤ 400 KB raw (budget increase for codegen) |
| Diagnostic cap | 500 (truncate beyond) | 500 (same; UI constraint) |
| CI latency gating | Advisory (size gate only) | Blocking (criterion.rs regression detection) |
| Measurement | `performance.measure()` manual | Automated benchmark suite + CI reporting |

---

## 8. Open Questions (Non-Blocking for v1)

These questions do not need answers for v1 but should be resolved before
implementing incremental parse (v2):

1. **Parser re-entrancy**: Can `ark-parser` be extended with a session-based
   incremental API without breaking the batch `parse()` contract? This
   depends on internal parser state management.

2. **Wasm memory management**: Incremental parse requires persistent state
   across calls (the session owns the tree). How does this interact with
   Wasm linear memory limits and garbage collection?

3. **Adaptive debounce**: Should the debounce interval auto-tune based on
   measured parse latency? (e.g., if parse < 5 ms, reduce debounce to
   100 ms; if parse > 30 ms, increase to 200 ms.)

4. **Worker transfer vs. structured clone**: For large results, should the
   worker use `Transferable` objects (e.g., `ArrayBuffer`) instead of
   structured clone to reduce postMessage overhead?

5. **Diagnostic deduplication**: If the same diagnostic appears in
   consecutive parses (common during typing), should the worker diff
   diagnostics before sending, or should the renderer diff on receive?

---

## 9. Relationship to Existing Budgets

This document does **not** replace or modify the performance budgets in
`docs/playground/deployment-strategy.md` §7 or ADR-022 §D4. Those budgets
cover **load-time** concerns (asset sizes, TTI, transfer budgets). This
document covers **runtime operational** concerns (parse latency, worker
round-trip, render jank) that apply after the playground is loaded and
interactive.

| Document | Scope |
|----------|-------|
| ADR-022 §D4 | Wasm ≤ 300 KB, total ≤ 250 KB gzip, TTI ≤ 3 s — **CI-enforced** |
| `deployment-strategy.md` §7 | Load budgets, cache budgets, Wasm compile time — **CI-enforced (size), advisory (TTI)** |
| **This document** | Parse latency, worker round-trip, render jank, incremental strategy — **advisory (v1), future CI** |

---

## References

- [ADR-017: Playground Execution Model](../adr/ADR-017-playground-execution-model.md)
- [ADR-022: Playground Deployment and Caching](../adr/ADR-022-playground-deployment-and-caching.md)
- [Deployment Strategy §7: Performance Budget](deployment-strategy.md)
- [Privacy & Telemetry Policy](privacy-telemetry-policy.md)
- `playground/src/worker.ts` — Web Worker implementation
- `playground/src/worker-client.ts` — Worker RPC client
- `playground/src/diagnostics.ts` — Diagnostics panel and overlay renderer
- `playground/src/types.ts` — Worker message protocol types
- `crates/ark-playground-wasm/src/lib.rs` — Wasm exports (parse, format, tokenize, version)
