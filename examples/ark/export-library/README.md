# Export an Ark library as a component

Shows the **compile-once, call-from-anywhere** path:

1. `calculator.ark` defines `pub fn` exports with component-compatible scalar types.
2. `arukellt compile --target wasm32-gc --emit component` emits a standard WASI P2
   core module, then packages it as `calculator.component.wasm` with the official
   `wasm-tools component embed/new` pipeline and canonical-ABI exports.
3. Any host with Component Model support invokes exports (Wasmtime or JS).

The same artifact is reused by:

- [`../../js/invoke-component/`](../js/invoke-component/README.md)

## Run

```bash
bash examples/ark/export-library/run.sh
```

Use s2 selfhost for library component output (`ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2.wasm`
or `examples_compile … modern` as in `run.sh`).

## Manual steps

```bash
scripts/run/arukellt-selfhost.sh compile \
  examples/ark/export-library/calculator.ark \
  --target wasm32-gc --emit component \
  -o .build/examples/ark-export/calculator.component.wasm

wasm-tools component wit .build/examples/ark-export/calculator.component.wasm

wasmtime run --wasm gc --wasm component-model \
  --invoke 'add(3, 4)' .build/examples/ark-export/calculator.component.wasm
# => 7
```

## Notes

- Scalar exports are the portable library boundary currently supported by the standard
  P2 core emitter. Unsupported aggregate signatures are rejected before packaging.
- Core Wasm (`--emit wasm`) is a separate artifact; components add the canonical ABI boundary.
- Library modules with both `pub fn` exports and a `main` entry compile as library components when exports are present; command-only programs use the P2 command wrapper.
