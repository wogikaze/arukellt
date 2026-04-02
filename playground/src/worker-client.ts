/**
 * Worker-based playground client.
 *
 * Provides the same API as the main-thread {@link Playground}, but all
 * operations are dispatched to a dedicated Web Worker so the Wasm module
 * never blocks the main thread.
 *
 * @module
 */

import type {
  WorkerPlayground,
  WorkerPlaygroundOptions,
  ParseResponse,
  FormatResponse,
  TokenizeResponse,
  WorkerResponse,
} from "./types.js";

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
export async function createWorkerPlayground(
  opts: WorkerPlaygroundOptions,
): Promise<WorkerPlayground> {
  // Determine the worker script URL.
  let workerUrl: string | URL;
  if (opts.workerUrl) {
    workerUrl = opts.workerUrl;
  } else {
    // Default: co-located worker.js alongside this module.
    workerUrl = new URL("./worker.js", import.meta.url);
  }

  const worker = new Worker(workerUrl, { type: "module" });

  // Message ID counter for request/response correlation.
  let nextId = 1;

  // Pending request callbacks, keyed by message ID.
  const pending = new Map<
    number,
    { resolve: (value: unknown) => void; reject: (reason: Error) => void }
  >();

  // Route worker responses to pending promises.
  worker.addEventListener("message", (event: MessageEvent<WorkerResponse>) => {
    const { id, ok } = event.data;
    const entry = pending.get(id);
    if (!entry) return;
    pending.delete(id);

    if (ok) {
      entry.resolve(event.data.result);
    } else {
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
  function send<T>(msg: Record<string, unknown>): Promise<T> {
    return new Promise((resolve, reject) => {
      const id = nextId++;
      pending.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
      });
      worker.postMessage({ ...msg, id });
    });
  }

  // Initialise the Wasm module inside the worker.
  await send({ cmd: "init", wasmUrl: opts.wasmUrl.toString() });

  let destroyed = false;

  return {
    async parse(source: string): Promise<ParseResponse> {
      if (destroyed) throw new Error("Worker playground has been destroyed");
      return send<ParseResponse>({ cmd: "parse", source });
    },

    async format(source: string): Promise<FormatResponse> {
      if (destroyed) throw new Error("Worker playground has been destroyed");
      return send<FormatResponse>({ cmd: "format", source });
    },

    async tokenize(source: string): Promise<TokenizeResponse> {
      if (destroyed) throw new Error("Worker playground has been destroyed");
      return send<TokenizeResponse>({ cmd: "tokenize", source });
    },

    async version(): Promise<string> {
      if (destroyed) throw new Error("Worker playground has been destroyed");
      return send<string>({ cmd: "version" });
    },

    destroy(): void {
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
