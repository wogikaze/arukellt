/**
 * Main-thread playground implementation.
 *
 * Loads the Wasm module and exposes typed, synchronous parse/format/tokenize
 * APIs. For non-blocking usage, see {@link ../worker-client}.
 *
 * @module
 */
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
export async function createPlayground(wasmModulePath, opts) {
    // Dynamically import the wasm-pack generated ES module.
    const wasm = await import(/* webpackIgnore: true */ wasmModulePath);
    // Initialise the Wasm module by passing the binary URL.
    await wasm.default(opts.wasmUrl);
    let destroyed = false;
    function ensureAlive() {
        if (destroyed) {
            throw new Error("Playground instance has been destroyed");
        }
    }
    return {
        parse(source) {
            ensureAlive();
            const json = wasm.parse(source);
            return JSON.parse(json);
        },
        format(source) {
            ensureAlive();
            const json = wasm.format(source);
            return JSON.parse(json);
        },
        tokenize(source) {
            ensureAlive();
            const json = wasm.tokenize(source);
            return JSON.parse(json);
        },
        version() {
            ensureAlive();
            return wasm.version();
        },
        destroy() {
            destroyed = true;
        },
    };
}
//# sourceMappingURL=playground.js.map