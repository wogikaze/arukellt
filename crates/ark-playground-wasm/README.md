# ark-playground-wasm

Browser-side Arukellt playground module compiled to WebAssembly via
[`wasm-bindgen`](https://rustwasm.github.io/docs/wasm-bindgen/).

## Exported JS API

All functions accept and return **strings**. Responses are JSON-encoded.

| Function            | Input          | Output (JSON)                                   |
|---------------------|----------------|-------------------------------------------------|
| `parse(source)`     | Arukellt source| `{ ok, module, diagnostics, error_count }`      |
| `format(source)`    | Arukellt source| `{ ok, formatted?, error? }`                    |
| `tokenize(source)`  | Arukellt source| `{ ok, tokens, diagnostics }`                   |
| `version()`         | —              | Version string (e.g. `"0.1.0"`)                 |

### `parse(source)` response shape

```json
{
  "ok": true,
  "module": {
    "docs": ["module doc comment"],
    "imports": [{ "module_name": "io", "alias": null }],
    "items": [
      { "kind": "fn", "name": "main", "is_pub": false, "docs": [] }
    ]
  },
  "diagnostics": [
    {
      "code": "E0001",
      "severity": "error",
      "phase": "parse",
      "message": "unexpected token",
      "labels": [{ "file_id": 0, "start": 3, "end": 5, "message": "here" }],
      "notes": [],
      "suggestion": null
    }
  ],
  "error_count": 0
}
```

### `format(source)` response shape

```json
{ "ok": true, "formatted": "fn main() {\n}\n" }
```

On syntax error:

```json
{ "ok": false, "error": "source contains syntax errors" }
```

### `tokenize(source)` response shape

```json
{
  "ok": true,
  "tokens": [
    { "kind": "Fn", "text": "fn", "start": 0, "end": 2 },
    { "kind": "Ident", "text": "main", "start": 3, "end": 7 }
  ],
  "diagnostics": []
}
```

## Building

### Prerequisites

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack   # optional, for JS package generation
```

### cargo check (wasm target)

```sh
cargo check --target wasm32-unknown-unknown -p ark-playground-wasm
```

### wasm-pack build (generates JS/TS bindings in `pkg/`)

```sh
cd crates/ark-playground-wasm
wasm-pack build --target web --release
```

### Native tests

```sh
cargo test -p ark-playground-wasm
```

## Wasm module size

| Build          | Size   |
|----------------|--------|
| Release (raw)  | ~330 KB |
| wasm-opt (via wasm-pack) | ~247 KB |

Target: < 5 MB ✅ (actual: **247 KB** after wasm-opt)

## Architecture

This crate is a thin `wasm-bindgen` wrapper around:

- **`ark-lexer`** — tokenization
- **`ark-parser`** — parsing + formatting
- **`ark-diagnostics`** — diagnostic types

It re-exports their functionality as JSON-returning functions callable from
JavaScript. No file I/O, no environment variables, no C FFI — pure Rust
compiled to `wasm32-unknown-unknown`.

The crate is excluded from `default-members` in the workspace `Cargo.toml`
because it requires the wasm target to build as `cdylib`.

## Design reference

See [ADR-017](../../docs/adr/ADR-017-playground-execution-model.md) for the
playground execution model decision (client-side parse/format/check).
