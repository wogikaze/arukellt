# @arukellt/playground

Browser API for the Arukellt language playground — wraps the
`ark-playground-wasm` WebAssembly module with TypeScript types and Web Worker
support.

## Overview

This package provides a typed JavaScript/TypeScript wrapper around the
[`ark-playground-wasm`](../crates/ark-playground-wasm/) Wasm module. It offers
two execution modes:

| Mode | Import | API | Thread |
|------|--------|-----|--------|
| **Direct** | `createPlayground` | Synchronous | Main thread |
| **Worker** | `createWorkerPlayground` | `async` / Promise-based | Web Worker |

Both modes expose the same set of operations:

- **`parse(source)`** — Parse Arukellt source → AST summary + diagnostics
- **`format(source)`** — Format Arukellt source (returns error on syntax errors)
- **`tokenize(source)`** — Tokenize source → token stream + diagnostics
- **`version()`** — Return the Wasm module version string

## Installation

```bash
npm install @arukellt/playground
```

## Quick start

### Main-thread (synchronous)

```ts
import { createPlayground } from "@arukellt/playground";

// Pass the path to the wasm-pack JS glue and the .wasm binary URL.
const pg = await createPlayground(
  "/assets/ark_playground_wasm.js",
  { wasmUrl: "/assets/ark_playground_wasm_bg.wasm" },
);

const result = pg.parse("fn main() {}");
console.log(result.ok);             // true
console.log(result.error_count);    // 0
console.log(result.module?.items);  // [{ kind: "fn", name: "main", ... }]
```

### Worker-based (non-blocking)

For editor-like UIs where responsiveness matters, use the worker-based API.
Wasm execution happens off the main thread.

```ts
import { createWorkerPlayground } from "@arukellt/playground";

const pg = await createWorkerPlayground({
  wasmUrl: "/assets/ark_playground_wasm_bg.wasm",
  workerUrl: "/assets/worker.js",  // built from src/worker.ts
});

const result = await pg.parse("fn main() {}");
console.log(result.ok); // true

// When done, terminate the worker:
pg.destroy();
```

## API reference

### `createPlayground(wasmModulePath, opts): Promise<Playground>`

Create a main-thread playground instance.

| Parameter | Type | Description |
|-----------|------|-------------|
| `wasmModulePath` | `string` | Path to the wasm-pack generated JS module (`ark_playground_wasm.js`) |
| `opts.wasmUrl` | `string \| URL` | URL to the `.wasm` binary |

Returns a `Playground` with synchronous methods.

### `createWorkerPlayground(opts): Promise<WorkerPlayground>`

Create a worker-based playground instance.

| Parameter | Type | Description |
|-----------|------|-------------|
| `opts.wasmUrl` | `string \| URL` | URL to the `.wasm` binary |
| `opts.workerUrl` | `string \| URL` | *(optional)* URL to the worker script |

Returns a `WorkerPlayground` with async (Promise-based) methods.

### Response types

#### `ParseResponse`

```ts
interface ParseResponse {
  ok: boolean;
  module: ModuleSummary | null;
  diagnostics: Diagnostic[];
  error_count: number;
}
```

#### `FormatResponse`

```ts
interface FormatResponse {
  ok: boolean;
  formatted?: string;
  error?: string;
}
```

#### `TokenizeResponse`

```ts
interface TokenizeResponse {
  ok: boolean;
  tokens: Token[];
  diagnostics: Diagnostic[];
}
```

#### `Diagnostic`

```ts
interface Diagnostic {
  code: string;              // e.g. "E0001"
  severity: "error" | "warning" | "help";
  phase: "lex" | "parse";
  message: string;
  labels: DiagnosticLabel[];
  notes: string[];
  suggestion: string | null;
}
```

#### `Token`

```ts
interface Token {
  kind: string;   // e.g. "Fn", "Ident", "LParen"
  text: string;   // source text
  start: number;  // byte offset
  end: number;    // byte offset (exclusive)
}
```

#### `ModuleSummary`

```ts
interface ModuleSummary {
  docs: string[];
  imports: ModuleImport[];
  items: ModuleItem[];
}

interface ModuleImport {
  module_name: string;
  alias: string | null;
}

interface ModuleItem {
  kind: "fn" | "struct" | "enum" | "trait" | "impl";
  name: string;
  is_pub: boolean;
  docs: string[];
}
```

## Building

### Prerequisites

- Node.js ≥ 18
- Rust with `wasm32-unknown-unknown` target
- `wasm-pack` (for building the Wasm module)

### Build the Wasm module

```bash
cd crates/ark-playground-wasm
wasm-pack build --target web --release
```

### Build the TypeScript wrapper

```bash
cd playground
npm install
npm run build
```

### Build everything

```bash
cd playground
npm run build:all
```

## Testing

```bash
# Type-check only (no Wasm required)
npm run test:typecheck

# Run structural tests (requires build first)
npm run build
npm test
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  Browser (main thread)                              │
│                                                     │
│  ┌──────────────┐      ┌───────────────────────┐   │
│  │ Editor UI    │─────▶│ @arukellt/playground   │   │
│  └──────────────┘      │                       │   │
│                        │  createPlayground()    │   │
│                        │  createWorkerPlayground│   │
│                        └──────┬────────────────┘   │
│                               │                     │
│  ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┼ ─ ─ ─ ─ ─ ─ ─ ─   │
│                               │ (postMessage)       │
│  ┌────────────────────────────▼──────────────────┐  │
│  │  Web Worker (optional)                        │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  ark-playground-wasm (WebAssembly)      │  │  │
│  │  │  • parse()  • format()                  │  │  │
│  │  │  • tokenize()  • version()              │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

The package provides two execution paths:

1. **Direct mode** — The Wasm module is loaded on the main thread.
   Simple, low-latency, but blocks the UI during parsing.

2. **Worker mode** — The Wasm module runs in a dedicated Web Worker.
   All calls are non-blocking. Recommended for editor integrations.

## Design references

- [ADR-017: Playground execution model](../docs/adr/ADR-017-playground-execution-model.md)
  — v1 scope: client-side parse/format/check only, no server execution
- [ADR-020: T2 I/O surface](../docs/adr/ADR-020-t2-io-surface.md)
  — Future v2 execution via `arukellt_io` import bridge

## v1 scope

This package implements the **v1 playground surface** as defined in ADR-017:

| Feature | Status |
|---------|--------|
| Parse | ✅ |
| Format | ✅ |
| Tokenize | ✅ |
| Diagnostics | ✅ |
| Full compile/run | ❌ v2 |
| Server-side execution | ❌ v2+ |

## License

MIT
