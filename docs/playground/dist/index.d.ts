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
export { createPlayground } from "./playground.js";
export { createWorkerPlayground } from "./worker-client.js";
export { createEditor } from "./editor.js";
export { classifyTokenKind, categoryClass, highlightTokens, } from "./highlight.js";
export { DEFAULT_THEME_CSS, injectTheme } from "./theme.js";
export { createDiagnosticsPanel, offsetToLineCol, buildDiagnosticOverlay, DIAGNOSTICS_CSS, injectDiagnosticStyles, } from "./diagnostics.js";
export { createPlaygroundApp } from "./playground-app.js";
export { checkCapabilities, capabilityWarningsToDiagnostics, UNSUPPORTED_CAPABILITIES, getCapabilityInfo, } from "./capability-check.js";
export { encodeSharePayload, decodeSharePayload, parseFragment, checkVersionMismatch, encodeSharePayloadWithVersion, CURRENT_SHARE_VERSION, SHARE_URL_TARGET_LENGTH, SHARE_URL_HARD_LIMIT, REPRODUCIBILITY_CONTRACT, } from "./share.js";
export { EXAMPLES, FIXTURE_BASE_PATH, getExample, getExampleList, getExamplesByTag, getFixtureMap, } from "./examples.js";
export type { PlaygroundOptions, WorkerPlaygroundOptions, Playground, WorkerPlayground, ParseResponse, FormatResponse, TokenizeResponse, Diagnostic, DiagnosticLabel, Severity, Phase, ModuleSummary, ModuleItem, ModuleImport, ItemKind, Token, WorkerRequest, WorkerResponse, } from "./types.js";
export type { TokenizeFn, EditorOptions, ArkEditor, } from "./editor.js";
export type { HighlightCategory } from "./highlight.js";
export type { DiagnosticsPanel, DiagnosticsPanelOptions, } from "./diagnostics.js";
export type { ParseFn, PlaygroundAppOptions, PlaygroundApp, } from "./playground-app.js";
export type { SharePayload, ShareEncodeResult, ShareDecodeResult, FragmentAction, VersionMismatchLevel, VersionMismatchInfo, } from "./share.js";
export type { ExampleEntry } from "./examples.js";
export type { CapabilityId, CapabilityInfo, CapabilityWarning, } from "./capability-check.js";
//# sourceMappingURL=index.d.ts.map