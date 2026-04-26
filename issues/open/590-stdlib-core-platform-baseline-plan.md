# Stdlib Core Platform Baseline Plan (Operational Guide)

> **Status:** Implementation Guide — ready for execution with verification checkpoints
> **For agentic workers:** Execute phase-by-phase. Each phase has mandatory verification steps.

> ⚠️ **DO NOT IMPLEMENT DIRECTLY.** This is an operational guide umbrella. Dispatch child issues:
> - **#604** `604-stdlib-baseline-contract-honesty.md` — contract honesty (implementation-ready)
> - **#605** `605-stdlib-baseline-host-platform.md` — host platform `std::host::*` (depends: 604)
> - **#606** `606-stdlib-baseline-structured-data.md` — JSON/TOML/structured data (depends: 604)
> - **#607** `607-stdlib-baseline-hash-hardening.md` — hash hardening (depends: 604)
> - **#608** `608-stdlib-baseline-docs-bench.md` — docs + bench rollout (depends: 604/605/606/607, closes #590)

**Goal:** Raise the Arukellt standard library by one full stage: from a selfhost-capable, partly experimental, partly raw-facing library into a **practical core-platform stdlib** for ordinary CLI tools, compiler tooling, and small applications.

This plan does **NOT** aim for immediate parity with Python/Go/Rust/Java breadth.  
It aims to close the largest current gaps that block day-to-day trust and usefulness:

- host filesystem surface is still whole-file / read-probe centric
- structured data parsers still have contract ambiguity
- text/time semantics are shallow compared to mature stdlibs
- collections/hash still have correctness/perf hazards
- generated docs are broad in coverage but thin in trustworthy module-level guidance

**Current repo evidence (root causes, not exhaustive):**

1. `docs/stdlib/modules/fs.md` explicitly says `std::host::fs` / `std::fs` are **not** a complete filesystem facade yet: no directory listing, no metadata, no streaming I/O.
2. `std/host/fs.ark` shows `exists(path)` is a **read probe** built on `__intrinsic_fs_read_file`, not general path existence.
3. `docs/stdlib/modules/http.md` says `std::host::http` is plain `http://` only, with **no HTTPS**.
4. `std/host/sockets.ark` documents `connect()` as a **minimum implementation** that returns fixed fd `3`; full read/write/close is future work.
5. `docs/stdlib/modules/time.md` shows `std::time` is only duration arithmetic; host time reads live elsewhere.
6. `std/text/mod.ark` and `docs/stdlib/modules/text.md` show many operations are byte/ASCII oriented (`trim_*`, `to_lower`, `to_upper`, best-effort `len_chars`).
7. `docs/stdlib/modules/json.md` and `docs/stdlib/modules/toml.md` still describe experimental, bounded surfaces.
8. `docs/stdlib/514-implementation-quality-audit.md` ranks JSON/TOML/hash as follow-up priority and flags contract risks in `std::fs`.
9. `docs/stdlib/516-raw-facade-boundary-policy.md` already defines the migration policy, but the current public surface still mixes raw helpers and user-facing facades.
10. generated docs under `docs/stdlib/modules/*.md` still show `_No module doc comment yet_` across module families.

**Work Streams (DO NOT MIX):**
1. Contract / facade honesty: `std::fs`, `std::host::fs`, `std::json`, `std::toml`, `std::collections::hash`, generated docs
2. Host core-platform baseline: filesystem, path, process/env, clock boundary
3. Structured data / semantics baseline: text, json, toml, time
4. Collections hardening: hash quality, no ambiguous primary APIs, no silent failure
5. Verification / docs / benchmark gates

**Key Constraint:** First goal is **NOT** “add every missing stdlib family”.  
First goal is: **make the boring core trustworthy**.

**Explicit Non-goals for Round 1:**
- Full batteries-included parity with Python/Java
- Async runtime / thread / sync family
- TLS / HTTPS / crypto / regex / URL full families
- Full TOML 1.0 compliance
- Full HTTP server/client expansion
- True generic collections before compiler support is ready (`#044`, `#312`, `#512`)
- Pretend implementations that over-claim capability without backend/runtime support

---

## Execution Phases

### Phase 0: Baseline Establishment

**Purpose:** Freeze current truth before changing APIs. Observe only. Do not implement.

**Execution:**

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python3 scripts/gen/generate-docs.py
```

**Record:**

- current targeted module docs that still show `_No module doc comment yet_`
- current targeted APIs whose names over-promise semantics
- current stdlib families that are stable/provisional/experimental/deprecated in `std/manifest.toml`
- current fixture pass/fail/skip counts for stdlib-heavy paths
- current benchmark coverage gaps already tracked by open issues

**Gap ledger to write down explicitly before implementation:**

- `std::host::fs::exists` is not path existence
- `std::json::parse` contract vs actual whole-document behavior
- `std::toml` supported subset vs user-facing name expectations
- `std::collections::hash` raw layout helpers vs user-facing facade
- `std::host::http` / `std::host::sockets` provisional minimum state
- `std::text` byte-vs-char-vs-ASCII semantics
- `std::time` pure math vs host clock boundary

**Phase 0 Exit Condition:** There is a single written baseline table of “current claim vs actual behavior” for all targeted families.

---

### Phase 1: Contract / Facade Honesty (CRITICAL)

**Goal:** No targeted stable- or provisional-looking API should promise a stronger contract than the implementation actually provides.

#### 1-1. Apply Raw / Facade / Adapter policy to current problem families

**Primary references:**

- `docs/stdlib/516-raw-facade-boundary-policy.md`
- `std/host/fs.ark`
- `std/json/mod.ark`
- `std/toml/mod.ark`
- `std/collections/hash.ark`

**Implementation direction:**

- raw representation helpers must be explicitly named or demoted
- facade names must reflect semantics the implementation can actually guarantee
- adapter-level runtime details must not leak as if they were stable user contracts

#### 1-2. Fix name / contract mismatches before adding breadth

**Examples that must be decided explicitly:**

- `std::host::fs::exists`

  - either deprecate the current name and replace with an honest read-probe facade
  - or reserve `exists` for future real path-existence semantics and mark current behavior transitional
- `std::json` accessors and parse entry points

  - do not imply a mature DOM or whole-document contract if current semantics are prefix/heuristic based
- `std::toml`

  - bounded subset must be prominent in API docs and errors
- `std::collections::hash`

  - raw layout helpers and unsafe assumptions must not read like a normal mature `HashMap`

#### 1-3. Align docs + manifest stability with truth

**Requirements:**

- add real `//!` module doc comments to targeted modules
- regenerate docs from source comments + `std/manifest.toml`
- keep unstable or heuristic surfaces `experimental` / `provisional` until contracts are real
- add migration guidance where names or semantics change

**Verification (mandatory):**

```bash
python3 scripts/gen/generate-docs.py
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
```

**Phase 1 Exit Condition:** For the targeted families, docs and names no longer overstate implementation reality.

---

### Phase 2: Host Core-Platform Baseline

**Goal:** Move from “can read/write a file somehow” to “has a trustworthy minimum host platform surface”.

#### 2-1. Harden the whole-file baseline first

**Targets:**

- `std::host::fs`
- `std::fs`
- `std::path`
- `std::host::process`
- `std::host::env`
- `std::host::clock`

**Implementation:**

- keep whole-file read/write as the stable base
- clarify exact error and availability semantics
- ensure `std::path` edge cases are fixture-backed (`join`, `normalize`, `parent`, `stem`, `with_extension`)
- keep process/env minimal, but make the contract explicit and documented

**Do NOT do yet:**

- fake streaming APIs
- fake directory handles
- fake metadata APIs with read-probe semantics

#### 2-2. Add the first real filesystem capability upgrade

**Target level for Round 1:**

- minimum directory / metadata surface on supported targets

**Desired minimum facade:**

- `read_dir(path)` or equivalent iterator/list facade
- `metadata(path)` or equivalent structured result
- `is_file(path)` / `is_dir(path)` or equivalent split
- true `exists(path)` only when backed by real path query semantics

**Upstream tie-in:**

- filesystem backend/runtime work must align with active WASI/P2 track (`#076`)
- do not over-claim support on targets that cannot implement it honestly

#### 2-3. Keep target gating explicit

**Requirement:**

- T1/T3 availability must be visible in docs and diagnostics
- provisional host capabilities must stay provisional until runtime + fixtures exist

**Verification (mandatory):**

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
mise bench
python3 scripts/gen/generate-docs.py
```

**Phase 2 Exit Condition:** Host filesystem/path/process surface is no longer “read-probe + string helpers only”; at least one real filesystem step beyond whole-file I/O has landed honestly.

---

### Phase 3: Structured Data + Text/Time Semantics Baseline

**Goal:** Stop treating text/time/data-format APIs as thin convenience wrappers; make them semantically trustworthy.

#### 3-1. `std::json`: whole-document truth first

**Requirements:**

- `parse` must have explicit whole-document semantics
- trailing non-whitespace garbage must be rejected if the API name remains `parse`
- accessor behavior must not rely on undocumented substring heuristics
- keep the family experimental until structure/access contracts are genuinely better

**Reference inputs:**

- `docs/stdlib/514-implementation-quality-audit.md`
- `docs/stdlib/modernization/514-parser-host-quality-audit.md`
- `issues/open/055-std-json-toml-csv.md`
- `issues/open/520-stdlib-performance-allocation-and-complexity-audit.md`

#### 3-2. `std::toml`: make the subset exact and predictable

**Requirements:**

- supported grammar must be narrow but exact
- unsupported forms must fail clearly and consistently
- user-facing docs must stop reading like “TOML in general” when the implementation is a bounded subset

#### 3-3. `std::text`: make semantics explicit

**Requirements:**

- byte-oriented APIs, char-count APIs, and ASCII-only transforms must be clearly separated in docs
- performance hotspots using repeated `concat` / `slice` should move toward builder/buffer patterns where already identified by `#520`
- do not silently imply full Unicode text semantics when only byte/ASCII behavior is implemented

#### 3-4. `std::time`: keep the wall-clock / monotonic / pure-math split clean

**Requirements:**

- pure duration helpers remain stable and clearly pure
- host clock helpers remain in `std::host::clock`
- naming/docs must make the boundary obvious
- align with `#051` without expanding scope beyond the current runtime truth

**Verification (mandatory):**

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
mise bench
python3 scripts/gen/generate-docs.py
```

**Phase 3 Exit Condition:** JSON/TOML/text/time are still smaller than mature-language stdlibs, but they are honest, predictable, and fixture-backed.

---

### Phase 4: Collections + Hash Hardening

**Goal:** Fix the highest-risk correctness/perf hazards in the current hash family without waiting for full generics.

**Important:** This phase is **NOT** the generic `HashMap<K,V>` project.
That remains blocked by upstream compiler work.

#### 4-1. Define one canonical hash policy

**Targets:**

- `std::core::hash`
- `std::collections::hash`

**Requirements:**

- one canonical integer/string/combine policy
- remove “same concept, different mix function” drift
- document stability vs quality expectations explicitly

#### 4-2. Remove misleading primary APIs

**Must fix:**

- primary `get`-style facade must not collapse “missing” and “stored zero”
- primary insert path must not silently fail when full
- if resize/rehash is not ready, return an explicit failure surface instead of pretending success

#### 4-3. Keep raw layout helpers out of the recommended path

**Requirements:**

- flat `Vec<i32>` layout knowledge must not be the default user path
- raw helpers remain possible, but the facade must be the recommended surface
- docs must make load-factor / complexity / failure behavior explicit

**Verification (mandatory):**

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
mise bench
```

**Phase 4 Exit Condition:** The monomorphic hash family may still be temporary, but it is no longer silently lossy or contract-ambiguous in its primary facade.

---

### Phase 5: Docs / Verification / Benchmark Closeout

**Goal:** Make the stdlib readable and governable, not just implemented.

#### 5-1. Generated docs must become trustworthy output

**Requirements:**

- all targeted modules have real module doc comments
- availability, stability, and deprecation are visible in generated docs
- examples match current behavior

#### 5-2. Benchmark coverage must follow the fixed families

**Tie-ins:**

- file I/O benchmark coverage (`#543`)
- parser/text builder hot paths (`#520`)
- hash-family occupancy / collision / regression measurements

#### 5-3. Eliminate stale progress surfaces

**Requirement:**

- any hand-maintained progress board must not contradict manifest-generated truth
- `std/manifest.toml` + generated docs must remain the authoritative surface description

**Final verification (mandatory):**

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
mise bench
python3 scripts/gen/generate-docs.py
```

**Then close.**

---

## Daily Operational Procedure

**Per work unit (single concern only):**

1. **Select one concern**

   - Example: `std::host::fs::exists` contract only
   - OR `std::json::parse` trailing-garbage rejection only
   - OR `std::collections::hash` missing-vs-zero ambiguity only

2. **Observe before change**

   ```bash
   python scripts/manager.py verify quick
   python scripts/manager.py verify fixtures
   ```

3. **Implement**

4. **Run minimal verification**

   ```bash
   python scripts/manager.py verify quick
   ```

5. **Run family-specific verification**

   - stdlib fixtures
   - relevant negative fixtures
   - benchmark if the change touches hot paths

6. **Regenerate docs**

   ```bash
   python3 scripts/gen/generate-docs.py
   ```

7. **Record deltas**

   - changed contract
   - changed stability label
   - newly added fixtures
   - benchmark delta
   - docs delta

8. **Stop there**

   - do not opportunistically expand into adjacent missing families

---

## Branch Naming Convention

One branch per concern:

- `plan/stdlib-core-platform-baseline`
- `fix/stdlib-fs-contract-honesty`
- `feat/stdlib-fs-metadata-readdir`
- `fix/stdlib-json-whole-document-parse`
- `fix/stdlib-toml-subset-contract`
- `fix/stdlib-text-byte-char-semantics`
- `fix/stdlib-hash-no-silent-insert-failure`
- `docs/stdlib-module-doc-comments`
- `bench/stdlib-parser-and-file-io`

---

## Completion Criteria

### Criterion A: Contract Honesty

- [ ] targeted misleading contracts are removed, deprecated, or renamed
- [ ] targeted docs match actual implementation
- [ ] raw/facade/adapter boundaries are visible in the public surface

### Criterion B: Host Core-Platform Baseline

- [ ] whole-file filesystem surface is stable and clearly documented
- [ ] at least one real step beyond read-probe semantics exists (directory and/or metadata baseline)
- [ ] path/process/env/clock boundaries are fixture-backed and documented

### Criterion C: Structured Data / Semantics Baseline

- [ ] `std::json` parse semantics are whole-document and negative-case tested
- [ ] `std::toml` subset is explicit and predictable
- [ ] `std::text` clearly distinguishes byte/char/ASCII semantics
- [ ] `std::time` vs `std::host::clock` split is explicit

### Criterion D: Collections Hardening

- [ ] primary hash-map access no longer conflates missing and zero
- [ ] primary insert path no longer silently loses writes
- [ ] canonical hash policy is documented and implemented

### Criterion E: Docs / Bench / Governance

- [ ] targeted module docs no longer show `_No module doc comment yet_`
- [ ] generated docs are authoritative for the targeted families
- [ ] benchmark coverage exists for the major fixed hot paths
- [ ] no stale progress surface contradicts `std/manifest.toml`

---

## Immediate Next Steps (First 5 Tasks)

**Do these first before expanding scope to new families:**

1. **Create the Phase 0 gap ledger**
   Record exact “API name vs actual behavior” mismatches for fs/json/toml/hash/text/time.

2. **Fix module doc comments for the targeted families**
   Make generated docs readable before further surface growth.

3. **Resolve `std::host::fs::exists` semantics**
   This is the clearest contract-honesty bug in the current host filesystem surface.

4. **Define `std::json::parse` whole-document behavior**
   Add negative fixtures for trailing garbage and malformed nested content.

5. **Remove silent failure / ambiguity from the primary hash facade**
   `hashmap_get` and `hashmap_set` must stop being misleading before broader collection work proceeds.

**Rule:** Do not chase breadth first.
Do not add “famous stdlib modules” until the current core is honest, documented, and benchmarked.
