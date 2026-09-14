# Plug an Ark library into another component

Runnable demo of **Ark-exported library → component composition**:

1. `provider-match.wit` describes a provider component exporting `add`.
2. `socket-match.wit` describes a consumer socket importing `add` and exporting `run()`.
3. Official `wasm-tools` packages both contracts and `wac plug` links them.

This is the same architecture as `tests/component-interop/compose/`, packaged for the
examples tree. For Ark **importing** a pre-built component via WIT, see
[`../link-compiled/`](../link-compiled/README.md).

## Run

```bash
bash examples/ark/compose-plug/run.sh
```
