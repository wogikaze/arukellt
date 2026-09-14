# Ark interop examples

## [`export-library/`](export-library/README.md)

Compile once, call many times: turn an `.ark` library into a **component export**
and invoke `add(3, 4)` from Wasmtime or JavaScript.

## [`link-compiled/`](link-compiled/README.md)

**Ark consumer + official WIT provider**: `client.ark` imports `test:host/math` via WIT,
then `arukellt compose --validate` and `wac plug` connect a provider component
packaged directly from the WIT contract.

## [`compose-plug/`](compose-plug/README.md)

**Official WIT provider** plugged into a socket component (`wac plug`). Demonstrates
reusing a component contract as a dependency without a language-specific host.
