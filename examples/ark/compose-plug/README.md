# Plug an Ark library into another component

Runnable demo of **Ark-exported library → component composition**:

1. `math_lib.ark` compiles to a provider component exporting `add`.
2. `runner-cargo/` (Rust) is the consumer socket importing `add`, exporting `run()`.
3. `wac plug` links them; `wasmtime --invoke 'run()'` → `42`.

This is the same architecture as `tests/component-interop/compose/`, packaged for the
examples tree. For Ark **importing** a pre-built component via WIT, see
[`../link-compiled/`](../link-compiled/README.md).

## Run

```bash
bash examples/ark/compose-plug/run.sh
```
