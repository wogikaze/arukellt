/**
 * Main-thread playground implementation.
 *
 * Loads the Wasm module and exposes typed, synchronous parse/format/tokenize
 * APIs. For non-blocking usage, see {@link ../worker-client}.
 *
 * @module
 */
import type { Playground, PlaygroundOptions } from "./types.js";
/**
 * Create a playground instance that runs on the main thread.
 *
 * ```ts
 * import { createPlayground } from "@arukellt/playground";
 *
 * const pg = await createPlayground({
 *   wasmUrl: "/assets/ark_playground_wasm_bg.wasm",
 * });
 *
 * const result = pg.parse("fn main() {}");
 * console.log(result.ok); // true
 * ```
 *
 * @param wasmModulePath - Path to the `wasm-pack` generated ES module
 *   (`ark_playground_wasm.js`). The module is loaded via dynamic `import()`.
 * @param opts - Playground configuration (wasm binary URL).
 * @returns An initialised {@link Playground} instance.
 */
export declare function createPlayground(wasmModulePath: string, opts: PlaygroundOptions): Promise<Playground>;
//# sourceMappingURL=playground.d.ts.map