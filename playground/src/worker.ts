/**
 * Web Worker script for the Arukellt playground.
 *
 * This script runs inside a dedicated Web Worker. It receives messages
 * from the main thread, delegates to the browser-native engine, and posts
 * results back. The engine is ready after the first `init` message.
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

import {
  engineVersion,
  formatSource,
  parseSource,
  tokenizeSource,
  typecheckSource,
} from "./engine.js";
import type { WorkerRequest, WorkerResponse } from "./types.js";

// ---------------------------------------------------------------------------
// Engine state
// ---------------------------------------------------------------------------

let initialised = false;

// ---------------------------------------------------------------------------
// Message handler
// ---------------------------------------------------------------------------

/**
 * Handle an incoming message from the main thread.
 */
async function handleMessage(msg: WorkerRequest): Promise<WorkerResponse> {
  const { id, cmd } = msg;

  try {
    if (cmd === "init") {
      void msg.wasmUrl;
      initialised = true;
      return { id, ok: true, result: null };
    }

    if (!initialised) {
      return { id, ok: false, error: "Worker not initialised — send 'init' first" };
    }

    switch (cmd) {
      case "parse": {
        return { id, ok: true, result: parseSource(msg.source) };
      }
      case "format": {
        return { id, ok: true, result: formatSource(msg.source) };
      }
      case "tokenize": {
        return { id, ok: true, result: tokenizeSource(msg.source) };
      }
      case "typecheck": {
        return { id, ok: true, result: typecheckSource(msg.source) };
      }
      case "version": {
        return { id, ok: true, result: engineVersion() };
      }
      default:
        return { id, ok: false, error: `Unknown command: ${(msg as WorkerRequest).cmd}` };
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return { id, ok: false, error: message };
  }
}

// ---------------------------------------------------------------------------
// Worker entry point
// ---------------------------------------------------------------------------

// Declare `self` for the Web Worker global scope.
declare const self: DedicatedWorkerGlobalScope;

self.addEventListener("message", async (event: MessageEvent<WorkerRequest>) => {
  const response = await handleMessage(event.data);
  self.postMessage(response);
});
