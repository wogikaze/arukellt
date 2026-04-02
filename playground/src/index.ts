/**
 * @arukellt/playground — Browser API for the Arukellt language playground.
 *
 * This package wraps the `ark-playground-wasm` WebAssembly module and
 * provides a typed, ergonomic API for parsing, formatting, and tokenizing
 * Arukellt source code in the browser.
 *
 * ## Quick start
 *
 * ### Main-thread (synchronous)
 *
 * ```ts
 * import { createPlayground } from "@arukellt/playground";
 *
 * const pg = await createPlayground(
 *   "/assets/ark_playground_wasm.js",
 *   { wasmUrl: "/assets/ark_playground_wasm_bg.wasm" },
 * );
 *
 * const result = pg.parse("fn main() {}");
 * console.log(result.ok);             // true
 * console.log(result.module?.items);   // [{ kind: "fn", name: "main", ... }]
 * ```
 *
 * ### Worker-based (asynchronous, non-blocking)
 *
 * ```ts
 * import { createWorkerPlayground } from "@arukellt/playground";
 *
 * const pg = await createWorkerPlayground({
 *   wasmUrl: "/assets/ark_playground_wasm_bg.wasm",
 *   workerUrl: "/assets/worker.js",
 * });
 *
 * const result = await pg.parse("fn main() {}");
 * pg.destroy();
 * ```
 *
 * @module
 */

// Re-export factory functions.
export { createPlayground } from "./playground.js";
export { createWorkerPlayground } from "./worker-client.js";

// Re-export editor components.
export { createEditor } from "./editor.js";

// Re-export highlighting utilities.
export {
  classifyTokenKind,
  categoryClass,
  highlightTokens,
} from "./highlight.js";

// Re-export theme utilities.
export { DEFAULT_THEME_CSS, injectTheme } from "./theme.js";

// Re-export diagnostics components.
export {
  createDiagnosticsPanel,
  offsetToLineCol,
  buildDiagnosticOverlay,
  DIAGNOSTICS_CSS,
  injectDiagnosticStyles,
} from "./diagnostics.js";

// Re-export playground app integration.
export { createPlaygroundApp } from "./playground-app.js";

// Re-export share link encoder/decoder (ADR-021).
export {
  encodeSharePayload,
  decodeSharePayload,
  parseFragment,
  CURRENT_SHARE_VERSION,
  SHARE_URL_TARGET_LENGTH,
  SHARE_URL_HARD_LIMIT,
} from "./share.js";

// Re-export examples catalog.
export {
  EXAMPLES,
  getExample,
  getExampleList,
  getExamplesByTag,
} from "./examples.js";

// Re-export all types for consumer use.
export type {
  // Configuration
  PlaygroundOptions,
  WorkerPlaygroundOptions,

  // Playground interfaces
  Playground,
  WorkerPlayground,

  // Response types
  ParseResponse,
  FormatResponse,
  TokenizeResponse,

  // Diagnostic types
  Diagnostic,
  DiagnosticLabel,
  Severity,
  Phase,

  // AST summary types
  ModuleSummary,
  ModuleItem,
  ModuleImport,
  ItemKind,

  // Token types
  Token,

  // Worker protocol (advanced usage)
  WorkerRequest,
  WorkerResponse,
} from "./types.js";

// Re-export editor types.
export type {
  TokenizeFn,
  EditorOptions,
  ArkEditor,
} from "./editor.js";

// Re-export highlight types.
export type { HighlightCategory } from "./highlight.js";

// Re-export diagnostics types.
export type {
  DiagnosticsPanel,
  DiagnosticsPanelOptions,
} from "./diagnostics.js";

// Re-export playground app types.
export type {
  ParseFn,
  PlaygroundAppOptions,
  PlaygroundApp,
} from "./playground-app.js";

// Re-export share types.
export type {
  SharePayload,
  ShareEncodeResult,
  ShareDecodeResult,
  FragmentAction,
} from "./share.js";

// Re-export example types.
export type { ExampleEntry } from "./examples.js";
