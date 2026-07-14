# Export an Ark library as a component

Shows the **compile-once, call-from-anywhere** path:

1. `calculator.ark` defines `pub fn` exports with component-compatible scalar types.
2. `arukellt compile --target wasm32-gc --emit component` produces
   `calculator.component.wasm` with inline WIT metadata and canonical-ABI exports.
3. Any host with component-model support invokes exports (wasmtime, Rust, JS).

The same artifact is reused by:

- [`../../rust/invoke-component/`](../rust/invoke-component/README.md)
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

## Appendix: external WIT + wasm-tools embed

When you need a hand-authored WIT package name or custom world, you can still embed WIT
externally:

```bash
scripts/run/arukellt-selfhost.sh compile \
  examples/ark/export-library/calculator.ark \
  --target wasm32 --emit wasm \
  -o .build/examples/ark-export/calculator.core.wasm

wasm-tools component embed examples/ark/export-library/calculator.wit \
  .build/examples/ark-export/calculator.core.wasm \
  -o .build/examples/ark-export/calculator.embed.wasm

wasm-tools component new .build/examples/ark-export/calculator.embed.wasm \
  --adapt wasi_snapshot_preview1=.build/examples/ark-export/wasi_snapshot_preview1.reactor.wasm \
  -o .build/examples/ark-export/calculator.component.wasm
```

## Notes

- String / list / record exports need canonical ABI adapters; stick to scalars for portable interop today.
- Core Wasm (`--emit wasm`) is a separate artifact; components add the canonical ABI boundary.
- Library modules with both `pub fn` exports and a `main` entry compile as library components when exports are present; command-only programs use the P2 command wrapper.
