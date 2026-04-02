/**
 * Main-thread playground implementation.
 *
 * Loads the Wasm module and exposes typed, synchronous parse/format/tokenize
 * APIs. For non-blocking usage, see {@link ../worker-client}.
 *
 * @module
 */

import type {
  Playground,
  PlaygroundOptions,
  ParseResponse,
  FormatResponse,
  TokenizeResponse,
} from "./types.js";

/**
 * Wasm module interface — the shape of the ES module produced by
 * `wasm-pack build --target web`.
 *
 * @internal
 */
interface WasmModule {
  default: (input?: RequestInfo | URL | BufferSource) => Promise<void>;
  parse: (source: string) => string;
  format: (source: string) => string;
  tokenize: (source: string) => string;
  version: () => string;
}

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
export async function createPlayground(
  wasmModulePath: string,
  opts: PlaygroundOptions,
): Promise<Playground> {
  // Dynamically import the wasm-pack generated ES module.
  const wasm: WasmModule = await import(/* webpackIgnore: true */ wasmModulePath);

  // Initialise the Wasm module by passing the binary URL.
  await wasm.default(opts.wasmUrl);

  let destroyed = false;

  function ensureAlive(): void {
    if (destroyed) {
      throw new Error("Playground instance has been destroyed");
    }
  }

  return {
    parse(source: string): ParseResponse {
      ensureAlive();
      const json = wasm.parse(source);
      return JSON.parse(json) as ParseResponse;
    },

    format(source: string): FormatResponse {
      ensureAlive();
      const json = wasm.format(source);
      return JSON.parse(json) as FormatResponse;
    },

    tokenize(source: string): TokenizeResponse {
      ensureAlive();
      const json = wasm.tokenize(source);
      return JSON.parse(json) as TokenizeResponse;
    },

    version(): string {
      ensureAlive();
      return wasm.version();
    },

    destroy(): void {
      destroyed = true;
    },
  };
}
