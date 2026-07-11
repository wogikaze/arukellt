# Wasm Binary Size Reduction Tracking

Status: **achieved** (fixture `tests/fixtures/hello/hello.ark` under 1 KB on `wasm32` and `wasm32-gc`)
Updated: 2026-05-16
Related issues: #108, #089, #091, #092, #093, #611, #612

## Goal

Achieve `hello.wasm` under 1 KB with `--opt-level 2` for the canonical hello fixture
(roadmap-v4.md §2). The fixture source is GC-oriented (`wasm32-gc`; legacy alias
`wasm32-wasi-p2`) and also builds to the `wasm32` linear-memory target for comparison.

## Result

As of **2026-05-16** (measured with `arukellt compile` release build):

- **`wasm32` (legacy alias `wasm32-wasi-p1`):** **491 bytes** at `--opt-level` 1 and 2.
- **`wasm32-gc` (legacy alias `wasm32-wasi-p2`):** **491 bytes** at `--opt-level` 2.

Older revisions of this doc cited **526 B** / **534 B** (`wasm32`) and **918 B** / **2639 B** (`wasm32-gc`);
those numbers are **obsolete** for the current compiler and `std::host::stdio` fixture.

### Measured Sizes (`wasm32` linear-memory)

| Fixture               | opt-level 0 | opt-level 1 | opt-level 2 |
|-----------------------|------------:|------------:|------------:|
| `hello.ark` (println) | 14 172 B    | **491 B**   | **491 B**   |
| `empty_main.ark`      | 13 936 B    | **297 B**   | **297 B**   |

### Measured Sizes (`wasm32-gc`)

| Fixture               | opt-level 0 | opt-level 1 | opt-level 2 |
|-----------------------|------------:|------------:|------------:|
| `hello.ark` (println) | 901 B       | 915 B       | **491 B**   |

### Section Breakdown — hello.wasm at opt-level 2, T1 (491 bytes total)

From `wasm-objdump -h` on the T1 binary:

```
section       size    ~%
type            40    8.1%   (6 entries; deduplicated)
code           107   21.8%   (1 function)
data            41    8.4%   (string constants)
import         246   50.1%   (7 WASI imports)
export          19    3.9%
function         2    0.4%
global           7    1.4%
memory           4    0.8%
```

Note: table/element sections are not emitted when no indirect calls exist.

### Regression Fix Delivered (issue #108)

At opt-level 2 the O2 pass pipeline includes `inter_function_inline`. This pass was
splicing callee bodies into callers **without remapping** callee `LocalId` values, which
caused MIR validation to fail with "use of undeclared local 0" for any callee with params
or locals. Fix: added `stmts_use_any_local()` guard in Phase 3 of
`inter_function_inline` — candidates whose body references any local are skipped until
full remapping is implemented. Regression test added in `crates/ark-mir/src/opt/pipeline.rs`.

### Optimization History (issue #612)

| Opportunity                    | Est. saving | Status                                |
|-------------------------------|-------------|---------------------------------------|
| Dead type elimination          | ~70 B       | **Done** — type deduplication pass in `src/compiler/emitter.ark` |
| Merge identical code functions | ~59 B       | Deferred — no duplicate bodies in hello.ark; implement when MIR merging infrastructure exists |
| Remove unused element section  | ~40 B       | **Done** — no table/element section emitted when no indirect calls |

---

## Optimization Contributions

Each optimization pass and its measured impact on `hello.ark` T1 opt-level 2:

| Optimization | Issue | Impact |
|-------------|-------|--------|
| Dead function elimination | (built-in) | ~13 600 B (from ~30 stdlib fns to 7) |
| Peephole local.get/local.set  | #088 | ~200 B  |
| Type section deduplication    | #089 | ~40 B   |
| String literal deduplication  | #091 | ~30 B   |
| Dead import elimination        | #092 | removes unused imports |
| Dead function elimination (T3) | #611 | enables reachability pruning for GC targets |
| Type signature deduplication   | #612 | reduces type section when duplicate signatures exist |
| Remove unused element section  | #612 | ~40 B saved when no indirect calls |

**Total reduction (T1 hello)**: 14 172 B → 491 B ≈ **96.5% reduction**.

T3 passes (#089–#093) overlap conceptually with the same goals (type/data/code compaction);
see per-section notes in issues #089–#093 for GC-specific behavior.

---

## Source fixture

`tests/fixtures/hello/hello.ark`

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::host::stdio
fn main() {
    stdio::println("Hello, world!")
}
```

## Verification

```bash
# T1 — expect 491 bytes @ O1/O2
./target/release/arukellt compile tests/fixtures/hello/hello.ark \
  --target wasm32-wasi-p1 --opt-level 2 -o /tmp/hello-t1.wasm
wc -c /tmp/hello-t1.wasm

# T3 — expect 491 bytes @ O2 (core wasm; component wrapper adds ~221 B)
./target/release/arukellt compile tests/fixtures/hello/hello.ark \
  --target wasm32-wasi-p2 --opt-level 2 -o /tmp/hello-t3.wasm
wc -c /tmp/hello-t3.wasm

# T1 section breakdown
wasm-objdump -h /tmp/hello-t1.wasm
```

## References

- roadmap-v4.md §2 (hello.wasm 1 KB target)
- Issue #088 (peephole optimizations)
- Issue #089 (type deduplication)
- Issue #091 (string deduplication)
- Issue #092 (dead import elimination)
- Issue #093 (const-if elimination)
