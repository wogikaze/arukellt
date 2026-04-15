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
export { classifyTokenKind, categoryClass, highlightTokens, } from "./highlight.js";
// Re-export theme utilities.
export { DEFAULT_THEME_CSS, injectTheme } from "./theme.js";
// Re-export diagnostics components.
export { createDiagnosticsPanel, offsetToLineCol, buildDiagnosticOverlay, DIAGNOSTICS_CSS, injectDiagnosticStyles, } from "./diagnostics.js";
// Re-export playground app integration.
export { createPlaygroundApp } from "./playground-app.js";
// Re-export capability detection.
export { checkCapabilities, capabilityWarningsToDiagnostics, UNSUPPORTED_CAPABILITIES, getCapabilityInfo, } from "./capability-check.js";
// Re-export telemetry guardrails (v1: disabled; see docs/playground/privacy-telemetry-policy.md).
export { TELEMETRY_DISABLED, reportError, reportWasmLoadError, reportCompilerPanic, } from "./telemetry.js";
// Re-export share link encoder/decoder (ADR-021).
export { encodeSharePayload, decodeSharePayload, parseFragment, checkVersionMismatch, encodeSharePayloadWithVersion, CURRENT_SHARE_VERSION, SHARE_URL_TARGET_LENGTH, SHARE_URL_HARD_LIMIT, REPRODUCIBILITY_CONTRACT, } from "./share.js";
// Re-export examples catalog.
export { EXAMPLES, FIXTURE_BASE_PATH, getExample, getExampleList, getExamplesByTag, getFixtureMap, } from "./examples.js";
//# sourceMappingURL=index.js.map