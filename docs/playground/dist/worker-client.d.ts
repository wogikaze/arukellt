/**
 * Worker-based playground client.
 *
 * Provides the same API as the main-thread {@link Playground}, but all
 * operations are dispatched to a dedicated Web Worker so the Wasm module
 * never blocks the main thread.
 *
 * @module
 */
import type { WorkerPlayground, WorkerPlaygroundOptions } from "./types.js";
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
export declare function createWorkerPlayground(opts: WorkerPlaygroundOptions): Promise<WorkerPlayground>;
//# sourceMappingURL=worker-client.d.ts.map