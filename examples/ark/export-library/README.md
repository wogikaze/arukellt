# Export an Ark library as a component

Shows the **compile-once, call-from-anywhere** path:

1. `calculator.ark` defines `pub fn` exports with component-compatible scalar types.
2. `calculator.wit` describes the export surface for `wasm-tools component embed`.
3. `wasm-tools component new` produces `calculator.component.wasm` (with a WASI adapter).
4. Any host with component-model support invokes exports (wasmtime, Rust, JS).

The same artifact is reused by:

- [`../../rust/invoke-component/`](../rust/invoke-component/README.md)
- [`../../js/invoke-component/`](../js/invoke-component/README.md)

## Run

```bash
bash examples/ark/export-library/run.sh
```

## Manual steps

```bash
scripts/run/arukellt-selfhost.sh compile \
  examples/ark/export-library/calculator.ark \
  --target wasm32-wasi-p1 --emit wasm \
  -o .build/examples/ark-export/calculator.core.wasm

wasm-tools component embed examples/ark/export-library/calculator.wit \
  .build/examples/ark-export/calculator.core.wasm \
  -o .build/examples/ark-export/calculator.embed.wasm

wasm-tools component new .build/examples/ark-export/calculator.embed.wasm \
  --adapt wasi_snapshot_preview1=.build/examples/ark-export/wasi_snapshot_preview1.reactor.wasm \
  -o .build/examples/ark-export/calculator.component.wasm

wasmtime run --wasm gc --wasm component-model \
  --invoke 'add(3, 4)' .build/examples/ark-export/calculator.component.wasm
# => 7
```

## Notes

- String / list / record exports need canonical ABI adapters; stick to scalars for portable interop today.
- Core Wasm (`--emit wasm`) is a separate artifact; components add the canonical ABI boundary.
