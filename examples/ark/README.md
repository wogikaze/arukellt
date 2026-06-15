# Ark interop examples

## [`export-library/`](export-library/README.md)

Compile once, call many times: turn an `.ark` library into a **component export**
and invoke `add(3, 4)` from wasmtime (same entry point Rust/JS examples use).

## [`link-compiled/`](link-compiled/README.md)

**Ark consumer + foreign provider**: `client.ark` imports `test:host/math` via WIT,
then `arukellt compose --validate` plugs in a provider component (built from
[`../rust/host-provider/`](../rust/host-provider/README.md)) so the dependency graph
links correctly (`run()` would return `42` at runtime when `wac plug` + wasmtime succeed).

## [`compose-plug/`](compose-plug/README.md)

**Ark library provider** plugged into a Rust runner socket (`wac plug`). Demonstrates
reusing a compiled Ark export as a dependency of another component.
