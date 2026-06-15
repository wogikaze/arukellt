# Rust WIT host provider

Rust component that **exports** `test:host/math` — the provider plugged into
[`../../ark/link-compiled/`](../../ark/link-compiled/README.md).

## Build

```bash
cd examples/rust/host-provider
cargo component build --release
# -> target/wasm32-wasip1/release/wit_import_host_provider.wasm
```

## Run (via Ark compose example)

```bash
bash examples/ark/link-compiled/run.sh
```

This is the same pattern as `tests/fixtures/wit_import/compose_roundtrip/provider-cargo/`,
trimmed for the examples tree.
