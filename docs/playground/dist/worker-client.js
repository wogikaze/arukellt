/**
 * Worker-based playground client.
 *
 * Provides the same API as the main-thread {@link Playground}, but all
 * operations are dispatched to a dedicated Web Worker so the Wasm module
 * never blocks the main thread.
 *
 * @module
 */
/**
 * Create a playground that runs in a Web Worker.
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
 * console.log(result.ok); // true
 *
 * pg.destroy(); // terminate the worker
 * ```
 *
 * @param opts - Configuration including wasm binary URL and optional worker script URL.
 * @returns An initialised {@link WorkerPlayground} instance.
 */
export async function createWorkerPlayground(opts) {
    // Determine the worker script URL.
    let workerUrl;
    if (opts.workerUrl) {
        workerUrl = opts.workerUrl;
    }
    else {
        // Default: co-located worker.js alongside this module.
        workerUrl = new URL("./worker.js", import.meta.url);
    }
    const worker = new Worker(workerUrl, { type: "module" });
    // Message ID counter for request/response correlation.
    let nextId = 1;
    // Pending request callbacks, keyed by message ID.
    const pending = new Map();
    // Route worker responses to pending promises.
    worker.addEventListener("message", (event) => {
        const { id, ok } = event.data;
        const entry = pending.get(id);
        if (!entry)
            return;
        pending.delete(id);
        if (ok) {
            entry.resolve(event.data.result);
        }
        else {
            entry.reject(new Error(event.data.error));
        }
    });
    // Handle unrecoverable worker errors.
    worker.addEventListener("error", (event) => {
        const err = new Error(`Worker error: ${event.message}`);
        for (const [, entry] of pending) {
            entry.reject(err);
        }
        pending.clear();
    });
    /**
     * Send a request to the worker and wait for the response.
     * @internal
     */
    function send(msg) {
        return new Promise((resolve, reject) => {
            const id = nextId++;
            pending.set(id, {
                resolve: resolve,
                reject,
            });
            worker.postMessage({ ...msg, id });
        });
    }
    // Initialise the Wasm module inside the worker.
    await send({ cmd: "init", wasmUrl: opts.wasmUrl.toString() });
    let destroyed = false;
    return {
        async parse(source) {
            if (destroyed)
                throw new Error("Worker playground has been destroyed");
            return send({ cmd: "parse", source });
        },
        async format(source) {
            if (destroyed)
                throw new Error("Worker playground has been destroyed");
            return send({ cmd: "format", source });
        },
        async tokenize(source) {
            if (destroyed)
                throw new Error("Worker playground has been destroyed");
            return send({ cmd: "tokenize", source });
        },
        async typecheck(source) {
            if (destroyed)
                throw new Error("Worker playground has been destroyed");
            return send({ cmd: "typecheck", source });
        },
        async version() {
            if (destroyed)
                throw new Error("Worker playground has been destroyed");
            return send({ cmd: "version" });
        },
        destroy() {
            destroyed = true;
            worker.terminate();
            // Reject any remaining pending requests.
            const err = new Error("Worker terminated");
            for (const [, entry] of pending) {
                entry.reject(err);
            }
            pending.clear();
        },
    };
}
//# sourceMappingURL=worker-client.js.map