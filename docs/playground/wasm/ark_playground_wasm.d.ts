/* tslint:disable */
/* eslint-disable */

/**
 * Format Arukellt source code.
 *
 * Returns `None`-equivalent (`ok: false`) if the source has syntax errors.
 *
 * # Returns
 *
 * JSON string with shape:
 * ```json
 * { "ok": true, "formatted": "fn main() {\n}\n" }
 * ```
 * or on error:
 * ```json
 * { "ok": false, "error": "source contains syntax errors" }
 * ```
 */
export function format(source: string): string;

/**
 * Parse Arukellt source code, returning a JSON object with the AST summary
 * and any diagnostics.
 *
 * # Returns
 *
 * JSON string with shape:
 * ```json
 * {
 *   "ok": true,
 *   "module": { "docs": [], "imports": [], "items": [] },
 *   "diagnostics": [],
 *   "error_count": 0
 * }
 * ```
 */
export function parse(source: string): string;

/**
 * Tokenize Arukellt source code, returning a JSON array of tokens and
 * any lexer diagnostics.
 *
 * # Returns
 *
 * JSON string with shape:
 * ```json
 * {
 *   "ok": true,
 *   "tokens": [{ "kind": "Fn", "text": "fn", "start": 0, "end": 2 }, ...],
 *   "diagnostics": []
 * }
 * ```
 */
export function tokenize(source: string): string;

/**
 * Type-check Arukellt source code, returning a JSON object with diagnostics.
 *
 * Runs the full parse → resolve → type-check pipeline and returns all
 * diagnostics from all phases.
 *
 * # Returns
 *
 * JSON string with shape:
 * ```json
 * {
 *   "ok": true,
 *   "diagnostics": [],
 *   "error_count": 0
 * }
 * ```
 * or on errors:
 * ```json
 * {
 *   "ok": false,
 *   "diagnostics": [{"code": "E0100", "severity": "error", "phase": "typecheck", "message": "...", "labels": [], "notes": [], "suggestion": null}],
 *   "error_count": 1
 * }
 * ```
 */
export function typecheck(source: string): string;

/**
 * Return the crate version.
 */
export function version(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly parse: (a: number, b: number) => [number, number];
    readonly format: (a: number, b: number) => [number, number];
    readonly tokenize: (a: number, b: number) => [number, number];
    readonly typecheck: (a: number, b: number) => [number, number];
    readonly version: () => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
