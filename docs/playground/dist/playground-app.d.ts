/**
 * Integrated playground application component.
 *
 * Wires together the editor shell, parse API, and diagnostics panel into
 * a single cohesive component. Editing triggers re-parse and diagnostic
 * updates, including inline error markers in the editor.
 *
 * ## Architecture
 *
 * ```
 * ┌─────────────────────────────────────────────┐
 * │ .ark-playground-app                         │
 * │ ┌─────────────────────────────────────────┐ │
 * │ │ Editor (textarea + backdrop + overlay)  │ │
 * │ │   └── .ark-editor-diagnostics (markers) │ │
 * │ └─────────────────────────────────────────┘ │
 * │ ┌─────────────────────────────────────────┐ │
 * │ │ Diagnostics Panel (list of issues)      │ │
 * │ └─────────────────────────────────────────┘ │
 * └─────────────────────────────────────────────┘
 * ```
 *
 * The diagnostic overlay is a `<pre>` element inserted into the editor's
 * DOM between the backdrop and textarea. It shows transparent text with
 * wavy underlines at diagnostic label positions, perfectly aligned with
 * the editor text.
 *
 * @module
 */
import type { ParseResponse, Diagnostic } from "./types.js";
import type { TokenizeFn, ArkEditor } from "./editor.js";
import type { DiagnosticsPanel } from "./diagnostics.js";
/**
 * A parse function compatible with both synchronous and asynchronous
 * playground APIs.
 *
 * - Synchronous: `Playground.parse()` returns `ParseResponse` directly.
 * - Asynchronous: `WorkerPlayground.parse()` returns `Promise<ParseResponse>`.
 */
export type ParseFn = (source: string) => ParseResponse | Promise<ParseResponse>;
/** Options for creating a playground application. */
export interface PlaygroundAppOptions {
    /** Initial source code to display. */
    initialValue?: string;
    /**
     * Parse function for diagnostics.
     *
     * Compatible with both `Playground.parse()` (sync) and
     * `WorkerPlayground.parse()` (async).
     */
    parse: ParseFn;
    /**
     * Tokenize function for syntax highlighting.
     *
     * If omitted, the editor works as a plain text editor without
     * syntax highlighting.
     */
    tokenize?: TokenizeFn;
    /**
     * Debounce interval in milliseconds for re-parsing on edit.
     * Default: `300`.
     */
    parseDebounceMs?: number;
    /**
     * Debounce interval in milliseconds for syntax highlighting.
     * Default: `30`.
     */
    highlightDebounceMs?: number;
    /** Tab size in spaces. Default: `4`. */
    tabSize?: number;
    /** Placeholder text shown when the editor is empty. */
    placeholder?: string;
    /** Whether the editor is read-only. Default: `false`. */
    readOnly?: boolean;
    /**
     * Callback invoked whenever diagnostics are updated.
     */
    onDiagnostics?: (diagnostics: Diagnostic[], response: ParseResponse) => void;
}
/** A playground application instance. */
export interface PlaygroundApp {
    /** The editor instance. */
    readonly editor: ArkEditor;
    /** The diagnostics panel instance. */
    readonly diagnosticsPanel: DiagnosticsPanel;
    /** Force an immediate re-parse of the current editor content. */
    parse(): void;
    /** Destroy the application and clean up all resources. */
    destroy(): void;
}
/**
 * Create an integrated playground application.
 *
 * Mounts an editor and diagnostics panel inside the given container,
 * wired together so that editing triggers re-parsing and updates both
 * the diagnostics panel and inline error markers.
 *
 * @param container - The DOM element to mount the application into.
 * @param options - Application configuration.
 * @returns A {@link PlaygroundApp} instance.
 *
 * @example
 * ```ts
 * import { createPlayground, createPlaygroundApp } from "@arukellt/playground";
 *
 * const pg = await createPlayground(wasmPath, { wasmUrl });
 * const app = createPlaygroundApp(document.getElementById("app")!, {
 *   initialValue: "fn main() {\n    let x = 42\n}\n",
 *   parse: (src) => pg.parse(src),
 *   tokenize: (src) => pg.tokenize(src),
 * });
 * ```
 */
export declare function createPlaygroundApp(container: HTMLElement, options: PlaygroundAppOptions): PlaygroundApp;
//# sourceMappingURL=playground-app.d.ts.map