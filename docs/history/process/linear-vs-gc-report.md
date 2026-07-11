# Linear Memory vs Wasm GC — Performance Comparison

Generated: 2026-07-05T12:21:05+0900
Iterations: 5, Warmups: 2

Fixtures from ADR-002 (GC vs non-GC decision benchmark).

## wasmtime

| Fixture | Linear (ms) | GC (ms) | Ratio GC/Linear | Linear status | GC status |
|---------|------------|---------|-----------------|---------------|-----------|
| hello | 0.124 | 0.069 | 0.56x | ok | ok |
| string_concat | 0.129 | 0.146 | 1.13x | ok | ok |
| vec_pushpop | 2.472 | 51.579 | 20.86x ⚠️ | ok | ok |
| binary_tree | 5.865 | 6.517 | 1.11x | ok | ok |
| result_heavy | 0.889 | 15.095 | 16.99x ⚠️ | ok | ok |
| file_read | 4.251 (fail) | error: failed to compile: wasm[0]::function[28] | — | ok | error |

## node

| Fixture | Linear (ms) | GC (ms) | Ratio GC/Linear | Linear status | GC status |
|---------|------------|---------|-----------------|---------------|-----------|
| hello | 0.050 | 0.030 | 0.60x | ok | ok |
| string_concat | 0.064 | 0.042 | 0.65x | ok | ok |
| vec_pushpop | 2.909 | 2.778 | 0.95x | ok | ok |
| binary_tree | 5.667 | 5.171 | 0.91x | ok | ok |
| result_heavy | 0.518 | 0.465 | 0.90x | ok | ok |
| file_read | 0.837 (fail) | error: instantiate: WebAssembly.Module(): Compi | — | ok | error |

## browser

| Fixture | Linear (ms) | GC (ms) | Ratio GC/Linear | Linear status | GC status |
|---------|------------|---------|-----------------|---------------|-----------|
| hello | 0.000 | 0.000 | — | ok | ok |
| string_concat | 0.000 | 0.000 | — | ok | ok |
| vec_pushpop | 2.100 | 0.900 | 0.43x | ok | ok |
| binary_tree | 5.800 | 5.700 | 0.98x | ok | ok |
| result_heavy | 0.800 | 0.700 | 0.88x | ok | ok |
| file_read | 0.800 (fail) | error: WebAssembly.Module(): Compiling function | — | ok | error |

## Cross-Runtime Summary (median ms)

| Fixture | Target | wasmtime | node | browser |
|---------|--------|----------|------|---------|
| hello | linear | 0.124 | 0.050 | 0.000 |
| hello | gc | 0.069 | 0.030 | 0.000 |
| string_concat | linear | 0.129 | 0.064 | 0.000 |
| string_concat | gc | 0.146 | 0.042 | 0.000 |
| vec_pushpop | linear | 2.472 | 2.909 | 2.100 |
| vec_pushpop | gc | 51.579 | 2.778 | 0.900 |
| binary_tree | linear | 5.865 | 5.667 | 5.800 |
| binary_tree | gc | 6.517 | 5.171 | 5.700 |
| result_heavy | linear | 0.889 | 0.518 | 0.800 |
| result_heavy | gc | 15.095 | 0.465 | 0.700 |
| file_read | linear | 4.251 | 0.837 | 0.800 |
| file_read | gc | — | — | — |

## Notes

- **Linear** = `wasm32-wasi-p1` (linear memory + bump allocator)
- **GC** = `wasm32-wasi-p2` (Wasm GC types, ADR-035 Phase 1 partial)
- **wasmtime** = wasmtime-py (Cranelift), instantiate once + repeated _start calls
- **node** = Node.js v23 (V8 12.9) native WebAssembly API
- **browser** = headless Chrome 147 (V8) via puppeteer-core
- GC target is ADR-035 Phase 1 partial: some fixtures may fail GC compilation/execution
- Ratio >= 1.5x means GC is slower than linear (ADR-002 threshold)
