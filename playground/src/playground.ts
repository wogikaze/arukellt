/**
 * Main-thread playground implementation.
 *
 * Exposes typed, synchronous parse/format/tokenize
 * APIs. For non-blocking usage, see {@link ../worker-client}.
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
import type {
  Playground,
  PlaygroundOptions,
  ParseResponse,
  FormatResponse,
  TokenizeResponse,
  TypecheckResponse,
} from "./types.js";

/**
 * Create a playground instance that runs on the main thread.
 *
 * ```ts
 * import { createPlayground } from "@arukellt/playground";
 *
 * const pg = await createPlayground({
 *   wasmUrl: "/assets/playground-engine",
 * });
 *
 * const result = pg.parse("fn main() {}");
 * console.log(result.ok); // true
 * ```
 *
 * @param enginePath - Reserved for backward compatibility with the former
 *   Wasm-backed API. The browser-native engine is bundled with this package.
 * @param opts - Playground configuration. `wasmUrl` is accepted for backward
 *   compatibility and is not fetched.
 * @returns An initialised {@link Playground} instance.
 */
export async function createPlayground(
  enginePath: string,
  _opts: PlaygroundOptions,
): Promise<Playground> {
  void enginePath;

  let destroyed = false;

  function ensureAlive(): void {
    if (destroyed) {
      throw new Error("Playground instance has been destroyed");
    }
  }

  return {
    parse(source: string): ParseResponse {
      ensureAlive();
      typecheckSource(source);
      return parseSource(source);
    },

    format(source: string): FormatResponse {
      ensureAlive();
      return formatSource(source);
    },

    tokenize(source: string): TokenizeResponse {
      ensureAlive();
      return tokenizeSource(source);
    },

    typecheck(source: string): TypecheckResponse {
      ensureAlive();
      return typecheckSource(source);
    },

    version(): string {
      ensureAlive();
      return engineVersion();
    },

    destroy(): void {
      destroyed = true;
    },
  };
}
