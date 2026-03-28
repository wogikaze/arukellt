# Wasm Binary Size Reduction Tracking

Status: active
Updated: 2026-03-28
Related issues: #108, #089, #091, #092, #093

## Goal

Achieve `hello.wasm` (GC-native, `wasm32-wasi-p2`) under 1 KB with `--opt-level 2`
(roadmap-v4.md §2).

## Current Measurements

| Target | Opt Level | Size (bytes) | Notes |
|--------|-----------|-------------|-------|
| wasm32-wasi-p2 (T3/GC) | 0 | 2639 | No optimizations |
| wasm32-wasi-p2 (T3/GC) | 1 | 2639 | Safe optimizations |
| wasm32-wasi-p2 (T3/GC) | 2 | 2639 | All optimizations |
| wasm32-wasi-p1 (T1/linear) | 1 | 12222 | Baseline comparison |

**Source fixture:** `tests/fixtures/hello/hello.ark`

```ark
fn main() {
    println("Hello, world!")
}
```

## Optimization Contributions

Each optimization pass and its measured impact on `hello.wasm` (T3, GC-native):

| Optimization | Issue | Category | Impact |
|-------------|-------|----------|--------|
| Type deduplication | #089 | Type section | Deduplicates identical `(rec ...)` type definitions |
| String deduplication | #091 | Data section | Merges duplicate string constants |
| Dead import elimination | #092 | Import section | Removes unused WASI/host imports |
| Const-if elimination | #093 | Code section | Folds constant-condition branches |

### Breakdown by section (T3, opt-level 1)

Use `arukellt analyze --wasm-size <file>` to get per-section breakdown.
Typical hello.wasm sections: type, import, function, export, code, data, custom.

## Size Budget

| Section | Current | Target | Strategy |
|---------|---------|--------|----------|
| type | — | Minimize | Type dedup (#089) |
| import | — | Minimize | Dead import elim (#092) |
| code | — | Minimize | Const-if elim (#093), peephole (#088) |
| data | — | Minimize | String dedup (#091) |
| **Total** | **2639 B** | **< 1024 B** | Combined passes |

## Gap Analysis

Current T3 size (2639 B) exceeds the 1 KB target by ~1615 B.
Further reductions will come from:

1. Aggressive dead-code elimination for unused stdlib functions
2. Import section trimming (remove unused WASI imports)
3. Type section compaction via structural deduplication
4. Data segment merging and alignment optimization

## Verification

```bash
# Measure current size
cargo run -p arukellt -- compile tests/fixtures/hello/hello.ark \
  --target wasm32-wasi-p2 --opt-level 2 -o hello.wasm 2>&1
wc -c hello.wasm

# Section-level analysis
cargo run -p arukellt -- analyze --wasm-size hello.wasm

# Compare opt levels
for opt in 0 1 2; do
  cargo run -p arukellt -- compile tests/fixtures/hello/hello.ark \
    --target wasm32-wasi-p2 --opt-level $opt -o hello_opt${opt}.wasm 2>&1
  echo "opt-level $opt: $(wc -c < hello_opt${opt}.wasm) bytes"
done
```

## References

- roadmap-v4.md §2 (hello.wasm 1 KB target)
- Issue #088 (peephole optimizations)
- Issue #089 (type deduplication)
- Issue #091 (string deduplication)
- Issue #092 (dead import elimination)
- Issue #093 (const-if elimination)
