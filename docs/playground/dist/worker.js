/**
 * Web Worker script for the Arukellt playground.
 *
 * This script runs inside a dedicated Web Worker. It receives messages
 * from the main thread, delegates to the Wasm module, and posts results
 * back. The Wasm module is loaded once on the first `init` message.
 *
 * ## Message protocol
 *
 * See {@link WorkerRequest} and {@link WorkerResponse} in `types.ts`.
 *
 * ## Usage
 *
 * This file is intended to be loaded as a Web Worker:
 *
 * ```ts
 * const worker = new Worker(new URL("./worker.js", import.meta.url), {
 *   type: "module",
 * });
 * ```
 *
 * @module
 */
let wasmExports = null;
// ---------------------------------------------------------------------------
// Message handler
// ---------------------------------------------------------------------------
/**
 * Handle an incoming message from the main thread.
 */
async function handleMessage(msg) {
    const { id, cmd } = msg;
    try {
        if (cmd === "init") {
            // Dynamically import the wasm-pack generated ES module.
            // The wasmUrl points to the .wasm binary; the JS glue is co-located.
            //
            // We derive the JS module URL from the .wasm URL by replacing the
            // extension — this matches wasm-pack's output layout:
            //   ark_playground_wasm_bg.wasm  → ark_playground_wasm.js
            const wasmUrl = msg.wasmUrl;
            const jsUrl = wasmUrl.replace(/_bg\.wasm$/, ".js");
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            const mod = await import(/* webpackIgnore: true */ jsUrl);
            await mod.default(wasmUrl);
            wasmExports = {
                parse: mod.parse,
                format: mod.format,
                tokenize: mod.tokenize,
                typecheck: mod.typecheck,
                version: mod.version,
            };
            return { id, ok: true, result: null };
        }
        if (!wasmExports) {
            return { id, ok: false, error: "Worker not initialised — send 'init' first" };
        }
        switch (cmd) {
            case "parse": {
                const json = wasmExports.parse(msg.source);
                return { id, ok: true, result: JSON.parse(json) };
            }
            case "format": {
                const json = wasmExports.format(msg.source);
                return { id, ok: true, result: JSON.parse(json) };
            }
            case "tokenize": {
                const json = wasmExports.tokenize(msg.source);
                return { id, ok: true, result: JSON.parse(json) };
            }
            case "typecheck": {
                const json = wasmExports.typecheck(msg.source);
                return { id, ok: true, result: JSON.parse(json) };
            }
            case "version": {
                return { id, ok: true, result: wasmExports.version() };
            }
            default:
                return { id, ok: false, error: `Unknown command: ${msg.cmd}` };
        }
    }
    catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        return { id, ok: false, error: message };
    }
}
self.addEventListener("message", async (event) => {
    const response = await handleMessage(event.data);
    self.postMessage(response);
});
export {};
//# sourceMappingURL=worker.js.map