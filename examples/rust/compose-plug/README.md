# Compose plug: Ark library + Rust runner

Wrapper around the CI-verified smoke test at `tests/component-interop/compose/run.sh`:

1. **Ark provider** — `tests/component-interop/compose/math_lib.ark` exports `add`.
2. **Rust socket** — `tests/component-interop/compose/runner-cargo` imports `add`, exports `run()`.
3. **`wac plug`** links them; `wasmtime --invoke 'run()'` → `42`.

## Run

```bash
bash examples/rust/compose-plug/run.sh
```

Requires `cargo`, `wasm-tools`, `wac`, `wasmtime`.
