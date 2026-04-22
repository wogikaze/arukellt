# Optimization Uplift Plan (Compile / Run / Size) (Operational Guide)

> **Status:** Implementation Guide — ready for execution with verification checkpoints
> **For agentic workers:** Execute phase-by-phase. Each phase has mandatory verification steps.

**Goal:** Raise Arukellt by one optimization tier: from **“already strong on tiny binaries and reasonable on T1, but still conservative / partially unverified on T3”** to **“measured, cross-language-grounded, lowering-optimized, T3-safe, and further compacted by default.”**

**Work Streams (DO NOT MIX):**

1. Measurement / verification: `scripts/perf/*`, `tests/baselines/perf/*`, `docs/process/*`, `docs/current-state.md`
2. Compile-speed: current lowering / orchestration path (`src/compiler/*.ark`, compiler-adjacent lowering paths, `crates/ark-mir/*`)
3. Runtime optimization / T3 safety: `crates/ark-mir/src/passes/*`, target-specific gating paths, runtime/perf fixtures
4. Binary size / Wasm layout: emitter + reachability + section compaction paths, `docs/process/wasm-size-reduction.md`

**Key Constraint:** First goal is **NOT** “copy Rust/C flags or subsystems one-for-one”.
First goal is **“turn the current Arukellt bottlenecks into measured wins without regressing determinism, selfhost behavior, or T3 GC safety.”**
The external lesson is structural: mature compilers win by avoiding unnecessary recomputation, by making whole-program decisions when profitable, and by stripping dead/duplicate output late in the pipeline. Arukellt should adopt those principles in Arukellt terms. ([Rust Compiler Development Guide][1])

---

## Why this plan must exist

Current repo evidence shows that Arukellt already has meaningful optimization infrastructure, but the next bottlenecks are now concentrated and measurable:

* `tests/baselines/perf/baselines.json` (generated 2026-04-22, target `wasm32-wasi-p1`) shows:

  * compile median: **14.9065 ms**
  * run median: **11.9115 ms**
  * binary median: **1714 B**
* phase averages from the same baseline indicate:

  * `resolve`: **1.73 ms**
  * `typecheck`: **1.62 ms**
  * `lower`: **5.06 ms**
  * `opt`: **1.06 ms**
  * `emit`: **0.35 ms**
  * `lower` is therefore ~**49%** of measured compile phase time
* runtime is frequently startup-dominated:

  * `startup.ark` overhead in the baseline is **11.434 ms**
  * many workloads have tiny `guest_ms`, so “run ms” alone is not a trustworthy optimization target
* `docs/process/benchmark-results.md` still says **“No cross-language table embedded yet”**
  even though `scripts/compare-benchmarks.sh` exists and roadmap C-ratio gates are documented
* `docs/process/wasm-size-reduction.md` shows:

  * T1 `hello.ark`: **534 B**
  * T3 `hello.ark`: **918 B**
  * remaining opportunities explicitly documented:

    * dead type elimination: **~70 B**
    * merge identical code functions: **~59 B**
    * remove unused element section: **~40 B**
* `crates/ark-mir/src/passes/README.md` confirms T3 still keeps multiple O2/O3 passes gated, and dead function elimination remains disabled on T3 because the current export/reachability contract is not yet safe enough

**Interpretation:**
Arukellt does **not** need a random grab bag of “more optimizations.”
It needs a staged plan that first repairs measurement truth, then attacks the actual compile bottleneck (`lower`), then unlocks T3 safely, then squeezes the remaining T3 size overhead.

---

## Known gap vs ideal optimization stack

The web study suggests the “ideal” next layer above Arukellt’s current state is:

1. **Avoid rework first**
   Rust’s incremental query system and Clang/GCC’s modules/PCH reduce redundant frontend work instead of only trying to make every pass faster. ([Rust Compiler Development Guide][1])

2. **Use scalable whole-program information**
   ThinLTO / WHOPR show that whole-program optimization becomes practical when summary-based and scalable, not monolithic. ([Clang][2])

3. **Use profile information carefully**
   Rust/Clang/GCC all expose PGO, and official docs consistently frame it as something driven by representative workloads, not by guesswork. ([Rust ドキュメント][3])

4. **Strip late and aggressively, but safely**
   `--gc-sections` and ICF are late-stage output compaction mechanisms that shrink binaries after reachability is known. ([Sourceware][4])

**Arukellt gap:**
Today’s repo is strongest on **early/intermediate MIR optimization and T1 size wins**, but weaker on:

* persistent reuse / incremental invalidation
* trustworthy cross-language measurement in docs
* T3-safe late reachability / dead stripping
* T3 tiny-binary section overhead cleanup
* profile-guided or summary-guided global decisions

---

## Execution Phases

### Phase 0: Baseline Establishment (Observe only)

**Purpose:** Fix the optimization truth before changing implementation.

**Execution:**

```bash
python scripts/manager.py perf benchmarks --no-quick
python scripts/manager.py perf gate
bash scripts/compare-benchmarks.sh
python scripts/manager.py docs check
```

Also re-run the canonical tiny-binary measurements from `docs/process/wasm-size-reduction.md`:

```bash
./target/release/arukellt compile tests/fixtures/hello/hello.ark \
  --target wasm32-wasi-p1 --opt-level 2 -o /tmp/hello-t1.wasm
./target/release/arukellt compile tests/fixtures/hello/hello.ark \
  --target wasm32-wasi-p2 --opt-level 2 -o /tmp/hello-t3.wasm
wc -c /tmp/hello-t1.wasm /tmp/hello-t3.wasm
```

**Record:**

* current compile / run / guest / startup medians
* current phase breakdown averages (`resolve`, `typecheck`, `lower`, `opt`, `emit`)
* whether `docs/process/benchmark-results.md` still lacks an embedded cross-language table
* current T1 / T3 `hello.ark` sizes
* current T3 gated pass list from `crates/ark-mir/src/passes/README.md`
* any doc drift among:

  * `docs/current-state.md`
  * `docs/process/benchmark-results.md`
  * `docs/process/wasm-size-reduction.md`
  * `tests/baselines/perf/baselines.json`

**Phase 0 Exit Condition:**
Optimization baselines are re-recorded and any docs drift is explicitly written down before implementation work starts.

---

### Phase 1: Measurement Truth Repair (CRITICAL)

**Goal:** Make performance work judgeable on real evidence, not mixed or stale numbers.

#### 1-1. Restore cross-language visibility in docs

**Target:**

* `docs/process/benchmark-results.md`
* `scripts/compare-benchmarks.sh`
* underlying benchmark writer paths

**Implementation:**

* ensure the compare runner actually embeds the C / Rust / Go comparison block into the doc
* if embed cannot be made automatic, fail docs freshness when the section remains empty after a benchmark refresh
* keep the current HTML comment regeneration contract intact

#### 1-2. Promote `startup_ms` and `guest_ms` to first-class reporting

**Problem:** Current run medians can hide the real optimization effect because short workloads are dominated by Wasmtime startup.

**Implementation:**

* surface `startup_ms` and `guest_ms` prominently in perf docs, not only raw JSON
* publish both for T1 and T3 benchmark runs
* do not treat total wall-clock alone as the runtime source of truth for short-lived programs

#### 1-3. Publish phase attribution in docs

**Implementation:**

* add generated summary text or table for average phase cost
* make `lower` dominance visible in the human-readable report
* keep the machine-readable JSON as the source of truth

#### 1-4. Sync performance and size docs

**Implementation:**

* align `docs/current-state.md`
* align `docs/process/benchmark-results.md`
* align `docs/process/wasm-size-reduction.md`

**Verification (mandatory):**

```bash
python scripts/manager.py perf benchmarks --no-quick
bash scripts/compare-benchmarks.sh
python scripts/manager.py docs check
```

**Phase 1 Exit Condition:**
There is no longer an “embedded cross-language table missing” gap in the primary benchmark doc, and human-readable docs reflect current startup/guest/phase realities.

---

### Phase 2: Compile-Speed Uplift (Lowering First)

**Goal:** Reduce compile latency by attacking the measured bottleneck, not the most fashionable compiler idea.

**Repo evidence that justifies this phase:**
Current perf data shows `lower` is ~49% of measured compile-phase cost, while `opt` is only ~10%. This means the next compile-speed win should come from lowering/orchestration cleanup before large investments in parallel typecheck or allocator rewrites.

#### 2-1. Split lowering into stable sub-phases

**Target:**

* current lowering path
* adjacent perf instrumentation paths

**Implementation:**

* instrument lowering with deterministic sub-phase timing
* examples of acceptable cuts:

  * expression lowering
  * CFG/block construction
  * local allocation / local remap
  * function index / call planning
  * monomorph/specialization hookup
  * emitter handoff preparation

**Requirement:**
The split must be stable enough for benchmark baselines, not just ad-hoc logging.

#### 2-2. Eliminate duplicated or allocation-heavy work in lowering

**Implementation directions:**

* remove duplicated AST/CoreHIR traversals where possible
* reduce clone-heavy intermediate materialization
* avoid re-deriving stable symbol / literal / signature facts multiple times within one compilation session
* prefer deterministic in-session memoization before any persistent cross-build cache

#### 2-3. Re-evaluate deferred compile-speed ideas only after new evidence

**STOP_IF:**

* do **not** start parallel typecheck (`issues/reject/098-compile-parallel-typecheck.md`)
* do **not** start arena allocator rewrite (`issues/reject/097-compile-arena-allocator.md`)
* do **not** escalate incremental work beyond design (`issues/open/099-compile-incremental-parse.md`)
  unless new phase data proves `lower` is no longer the main bottleneck

**Verification (mandatory):**

```bash
python scripts/manager.py perf benchmarks --no-quick
python scripts/manager.py perf gate
python scripts/manager.py verify --full
python scripts/manager.py selfhost parity
```

**Phase 2 Exit Condition:**

* average `lower` phase time improves by **at least 15%** versus Phase 0 baseline
* benchmark-suite compile median improves by **at least 8%** versus Phase 0 baseline
* no determinism, selfhost, or correctness regressions are introduced

---

### Phase 3: Runtime Uplift (T3-Safe First, Not Marketing-First)

**Goal:** Convert T3 from “conservative but correct” to “measurably optimized and still GC-safe.”

#### 3-1. Define a T3 export / reachability root contract

**Problem:**
Dead function elimination is still disabled for T3 because the current reachability rule can incorrectly remove WASI-exported or externally reachable functions.

**Implementation:**

* define the root set explicitly for T3
* separate:

  * entry roots
  * exported roots
  * host-reachable roots
  * internal-call roots
* document the rule in compiler docs and code comments

#### 3-2. Re-enable T3 dead function elimination only after the contract is explicit

**Implementation:**

* land the reachability contract first
* then enable T3 dead function elimination
* add regression fixtures for host-reachable but not locally-called functions

#### 3-3. Unlock gated T3 passes one-by-one

**Implementation:**

* remove passes from the gated list only with a dedicated regression fixture and a written safety reason
* if a pass remains unsafe, leave it gated and create a dedicated blocker issue rather than hand-waving it open

**Priority candidates:**
Pick from the currently gated set only after export-root semantics are stable.

#### 3-4. Judge runtime by `guest_ms`, not startup noise

**Implementation:**

* use guest-dominated workloads when claiming optimizer wins
* prefer `binary_tree`, `parse_tree_distance`, or another explicitly guest-heavy benchmark over no-op or startup-bound programs

**Verification (mandatory):**

```bash
python scripts/manager.py perf benchmarks --no-quick
ARUKELLT_TARGET=wasm32-wasi-p2 cargo test -p arukellt --test harness -- --nocapture
python scripts/manager.py verify --full
```

**Phase 3 Exit Condition:**

* T3 dead function elimination is either enabled by default **or** replaced by a landed reachability contract + dedicated blocker issue
* at least **two** previously gated T3 passes are either:

  * safely enabled with regression coverage, or
  * conclusively deferred with explicit blocker issues
* runtime claims are backed by guest-time measurements, not only total wall-clock numbers

---

### Phase 4: Binary Size Uplift (T3 Tiny-Binary Compaction)

**Goal:** Keep T1 strength, but make T3 tiny binaries materially smaller.

**Repo evidence that justifies this phase:**
`docs/process/wasm-size-reduction.md` explicitly records the remaining high-value T3/Tiny opportunities:

* dead type elimination: ~70 B
* merge identical code functions: ~59 B
* remove unused element section: ~40 B

#### 4-1. Implement dead type elimination

**Target:** type section compaction for unreachable / unreferenced entries

**Requirement:**
Must preserve deterministic index assignment and validation correctness.

#### 4-2. Remove unused element/table payload in no-indirect-call outputs

**Implementation:**

* when the program has no indirect calls, do not emit the element/table baggage that is only supporting that mode
* keep validation and runtime compatibility intact

#### 4-3. Make custom section policy explicit

**Problem:**
For tiny outputs, custom sections become a noticeable share of total size.

**Implementation:**

* decide which custom sections are required in release / benchmark / debug outputs
* if name or hint sections are optional, gate them by mode/flag rather than always carrying them in smallest-output workflows

#### 4-4. Merge identical tiny helper functions only if deterministic

**Implementation:**

* only pursue identical-body merge if it preserves deterministic symbol/index behavior and does not compromise debugging/validation assumptions

**Verification (mandatory):**

```bash
python scripts/manager.py verify size
python scripts/manager.py perf benchmarks --no-quick
python scripts/manager.py docs check
```

**Phase 4 Exit Condition:**

* T3 `hello.ark` at `--opt-level 2` is **≤ 850 B**
* T1 `hello.ark` at `--opt-level 2` remains **≤ 534 B**
* benchmark-suite median binary size does not regress
* `docs/process/wasm-size-reduction.md` is updated with new section breakdown

---

### Phase 5: Advanced Groundwork (Design-Ready, not Default-On)

**Goal:** Convert lessons from Rust / C toolchains into Arukellt-native next-step designs, without prematurely shipping unstable infrastructure.

Rust and C-family toolchains do not rely on one miracle pass; they combine incremental reuse, scalable whole-program summary-based optimization, and profile-guided decisions. That pattern is the right long-term direction for Arukellt too, but only after current bottlenecks are measured and controlled. ([Rust Compiler Development Guide][1])

#### 5-1. Define persistent artifact / invalidation boundaries

**Output:**

* design note for what can be cached between runs:

  * parse artifacts
  * resolve facts
  * type facts
  * lowering summaries
* explicit invalidation rules

**Related issue:** `issues/open/099-compile-incremental-parse.md`

#### 5-2. Define whole-program summary requirements

**Output:**

* design note for future summary-guided optimizations:

  * reachability
  * inline candidacy
  * hot/cold layout
  * export retention
  * size-aware dedup opportunities

#### 5-3. Define profiling contract for future PGO-like work

**Output:**

* benchmark/profiling doc that explains:

  * which workloads are representative
  * how profiles are collected
  * which optimization decisions may consume them
* no default-on nondeterministic CI behavior in this phase

**Verification (mandatory):**

```bash
python scripts/manager.py docs check
python scripts/manager.py verify quick
```

**Phase 5 Exit Condition:**

* design docs are landed
* follow-up implementation issues are created
* no unstable cache / PGO system is forced on by default in this plan issue

---

## Non-goals

* Recreating Rust’s incremental system in one issue
* Recreating ThinLTO / WHOPR in one issue
* Parallel typecheck by default before lowering evidence says it is worth it
* T4 native backend work
* Marketing claims based on startup-dominated runtime numbers
* Any nondeterministic optimization strategy that weakens repro/selfhost verification

---

## Required follow-on issues to mint from this plan

1. **Perf truth repair:** embed cross-language table + expose startup/guest/phase summaries
2. **Compile-speed lowering slice:** sub-phase instrumentation + lowering cleanup
3. **T3 reachability contract:** export-root model for safe dead-function elimination
4. **T3 size compaction:** dead type elimination + unused element removal + custom section policy
5. **Design slice:** persistent artifact graph / invalidation
6. **Design slice:** summary-guided whole-program optimization / PGO contract

---

## Close gate

This plan is not done when “some optimization landed.”
It is done only when all of the following are true:

1. Arukellt performance docs show a **current embedded cross-language comparison**
2. compile-speed work has reduced the **measured lowering bottleneck**
3. T3 optimization is **less conservative in documented, tested ways**
4. T3 tiny binaries are **materially smaller**
5. next-step design issues for incremental / summary / profiling work are explicitly queued

[1]: https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation.html "https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation.html"
[2]: https://clang.llvm.org/docs/ThinLTO.html "https://clang.llvm.org/docs/ThinLTO.html"
[3]: https://doc.rust-lang.org/rustc/codegen-options/index.html "https://doc.rust-lang.org/rustc/codegen-options/index.html"
[4]: https://sourceware.org/binutils/docs/ld.html "https://sourceware.org/binutils/docs/ld.html"
