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
export {};
//# sourceMappingURL=worker.d.ts.map