/**
 * Browser client for the playground compiler worker.
 *
 * @module
 */

import type {
  CompileOptions,
  CompileResult,
  CompilerRuntimeAvailability,
  RunOptions,
  RunResult,
} from "./compiler-types.js";
import type {
  CompilerWorkerRequest,
  CompilerWorkerResponse,
} from "./compiler-worker.js";
import { moduleImportsArukelltIo } from "./t2-runner.js";

export interface CompilerClientOptions {
  compilerUrl: string | URL;
  workerUrl?: string | URL;
}

export interface CompilerClient {
  compile(source: string, options?: CompileOptions): Promise<CompileResult>;
  run(wasmBytes: Uint8Array, options?: RunOptions): Promise<RunResult>;
  checkAvailability(): Promise<CompilerRuntimeAvailability>;
  destroy(): void;
}

function resolveCompilerAssetUrl(compilerUrl: string | URL): string {
  if (typeof globalThis.location === "undefined") {
    return compilerUrl.toString();
  }
  return new URL(compilerUrl, globalThis.location.href).href;
}

/**
 * Create a compiler worker client.
 */
export async function createCompilerClient(
  opts: CompilerClientOptions,
): Promise<CompilerClient> {
  const resolvedCompilerUrl = resolveCompilerAssetUrl(opts.compilerUrl);
  const workerUrl = opts.workerUrl ?? new URL("./compiler-worker.js", import.meta.url);
  const worker = new Worker(workerUrl, { type: "module" });

  let nextId = 1;
  const pending = new Map<
    number,
    { resolve: (value: unknown) => void; reject: (reason: Error) => void }
  >();

  worker.addEventListener("message", (event: MessageEvent<CompilerWorkerResponse>) => {
    const entry = pending.get(event.data.id);
    if (!entry) return;
    pending.delete(event.data.id);
    if (event.data.ok) entry.resolve(event.data.result);
    else entry.reject(new Error(event.data.error));
  });

  worker.addEventListener("error", (event) => {
    const err = new Error(`Compiler worker error: ${event.message}`);
    for (const [, entry] of pending) entry.reject(err);
    pending.clear();
  });

  function send<T>(msg: CompilerWorkerRequest): Promise<T> {
    return new Promise((resolve, reject) => {
      const id = nextId++;
      pending.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
      });
      worker.postMessage({ ...msg, id });
    });
  }

  await send({ id: 0, cmd: "init", compilerUrl: resolvedCompilerUrl });

  let destroyed = false;

  return {
    async compile(source: string, options?: CompileOptions): Promise<CompileResult> {
      if (destroyed) throw new Error("Compiler client destroyed");
      return send<CompileResult>({ id: 0, cmd: "compile", source, options });
    },

    async run(wasmBytes: Uint8Array, options?: RunOptions): Promise<RunResult> {
      if (destroyed) throw new Error("Compiler client destroyed");
      return send<RunResult>({ id: 0, cmd: "run", wasmBytes, options });
    },

    async checkAvailability(): Promise<CompilerRuntimeAvailability> {
      const wasmSupported = typeof WebAssembly !== "undefined";
      if (!wasmSupported) {
        return {
          compilerAssetPresent: false,
          wasmSupported: false,
          runSupported: false,
          reason: "WebAssembly is not available in this browser.",
        };
      }

      try {
        const response = await fetch(resolvedCompilerUrl);
        if (!response.ok) {
          return {
            compilerAssetPresent: false,
            wasmSupported: true,
            runSupported: false,
            reason: "Compiler Wasm asset is missing. Run `npm run build:app` in playground/.",
          };
        }
        const bytes = new Uint8Array(await response.arrayBuffer());
        const hasT2Compiler = bytes.byteLength > 0;
        return {
          compilerAssetPresent: hasT2Compiler,
          wasmSupported: true,
          runSupported: hasT2Compiler,
          reason: null,
        };
      } catch {
        return {
          compilerAssetPresent: false,
          wasmSupported: true,
          runSupported: false,
          reason: "Unable to fetch the compiler Wasm asset.",
        };
      }
    },

    destroy(): void {
      destroyed = true;
      worker.terminate();
      const err = new Error("Compiler worker terminated");
      for (const [, entry] of pending) entry.reject(err);
      pending.clear();
    },
  };
}

/** Whether compiled output looks runnable by the T2 runner. */
export function isRunnableT2Output(wasmBytes: Uint8Array | null): boolean {
  return wasmBytes !== null && moduleImportsArukelltIo(wasmBytes);
}
