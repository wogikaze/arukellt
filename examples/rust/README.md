# Rust interop examples

## [`host-provider/`](host-provider/README.md)

Implement a WIT interface in Rust (`wit-bindgen` + `cargo component build`).
Used as the **provider** side of [`../ark/link-compiled/`](../ark/link-compiled/README.md).

## [`invoke-component/`](invoke-component/README.md)

Call an Ark-exported component from Rust via **wasmtime CLI** (no custom host code required).
Uses the shared [`../ark/export-library/calculator.ark`](../ark/export-library/calculator.ark).

## [`compose-plug/`](compose-plug/README.md)

Plug an Ark-exported math library into a Rust runner guest (`wac plug`), mirroring
`tests/component-interop/compose/`.
