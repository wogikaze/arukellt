# Wasm Binary Size Reduction Tracking

Status: **achieved** (fixture `tests/fixtures/hello/hello.ark` under 1 KB on T1 and T3)
Updated: 2026-04-18
Related issues: #108, #089, #091, #092, #093

## Goal

Achieve `hello.wasm` under 1 KB with `--opt-level 2` for the canonical hello fixture
(roadmap-v4.md §2). The fixture source is GC-oriented (`wasm32-wasi-p2`) and also builds
to the T1 linear-memory target for comparison.

## Result

As of **2026-04-18** (measured with `arukellt compile` release build):

- **`wasm32-wasi-p1` (T1):** **534 bytes** at `--opt-level` 1 and 2.
- **`wasm32-wasi-p2` (T3, GC-native):** **918 bytes** at `--opt-level` 2 (901 B @ O0,
  915 B @ O1).

Older revisions of this doc cited **526 B** (T1) and **2639 B** (T3); those numbers are
obsolete for the current compiler and `std::host::stdio` fixture.

### Measured Sizes (wasm32-wasi-p1, T1 linear-memory)

| Fixture               | opt-level 0 | opt-level 1 | opt-level 2 |
|-----------------------|------------:|------------:|------------:|
| `hello.ark` (println) | 14 172 B    | **534 B**   | **534 B**   |
| `empty_main.ark`      | 13 936 B    | **297 B**   | **297 B**   |

### Measured Sizes (wasm32-wasi-p2, T3 / GC-native)

| Fixture               | opt-level 0 | opt-level 1 | opt-level 2 |
|-----------------------|------------:|------------:|------------:|
| `hello.ark` (println) | 901 B       | 915 B       | **918 B**   |

### Section Breakdown — hello.wasm at opt-level 2, T1 (534 bytes total)

From `wasm-objdump -h` on the T1 binary (percentages ≈ share of total file size):

```
section       size    ~%
type           116   21.7%   (18 entries; dead type entries remain)
code           245   45.9%   (7 functions — largest section)
data            49    9.2%   (4 string literals)
import          35    6.6%   (1 import: wasi_snapshot_preview1.fd_write)
export          19    3.6%
element         16    3.0%   (8-entry indirect-call table)
function         8    1.5%
global           8    1.5%
table            5    0.9%
memory           4    0.7%
```

### Section sizes — T3 hello at opt-level 2 (918 bytes total)

`wasm-objdump` from wabt may warn on GC opcodes; section headers are still useful:

```
section        size
type           374   (44 entries)
import          35
function         7
table            5
memory           4
global          16
export          19
element         15
datacount        1
code           190   (6 functions)
data            44
custom          27 + 144  (branch hints + name section)
```

### Regression Fix Delivered (issue #108)

At opt-level 2 the O2 pass pipeline includes `inter_function_inline`. This pass was
splicing callee bodies into callers **without remapping** callee `LocalId` values, which
caused MIR validation to fail with "use of undeclared local 0" for any callee with params
or locals. Fix: added `stmts_use_any_local()` guard in Phase 3 of
`inter_function_inline` — candidates whose body references any local are skipped until
full remapping is implemented. Regression test added in `crates/ark-mir/src/opt/pipeline.rs`.

### Remaining Opportunities (not required for 1KB target on this fixture)

| Opportunity                    | Est. saving | Notes                                  |
|-------------------------------|-------------|----------------------------------------|
| Dead type elimination          | ~70 B       | Many type entries are unreferenced     |
| Merge identical code functions | ~59 B       | Near-identical func bodies in hello    |
| Remove unused element section  | ~40 B       | No indirect calls in hello.ark         |

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

**Total reduction (T1 hello)**: 14 172 B → 534 B ≈ **96.2% reduction**.

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
# T1 — expect 534 bytes @ O1/O2
./target/release/arukellt compile tests/fixtures/hello/hello.ark \
  --target wasm32-wasi-p1 --opt-level 2 -o /tmp/hello-t1.wasm
wc -c /tmp/hello-t1.wasm

# T3 — expect 918 bytes @ O2
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
