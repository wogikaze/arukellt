# Invoke an Ark component from Rust

Uses **wasmtime** as the host runtime. The Ark artifact is built from
[`../ark/export-library/calculator.ark`](../ark/export-library/calculator.ark).

For a custom Rust embedder, point wasmtime's component API at the same
`.component.wasm` file; this example keeps the host side minimal so the Ark
export surface stays in focus.

## Run

```bash
bash examples/rust/invoke-component/run.sh
```

## Manual

```bash
bash examples/ark/export-library/run.sh   # builds .build/examples/ark-export/calculator.component.wasm
wasmtime run --wasm gc --wasm component-model \
  --invoke 'add(10, 32)' .build/examples/ark-export/calculator.component.wasm
```
