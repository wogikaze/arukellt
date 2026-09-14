/**
 * Main-thread playground implementation.
 *
 * Exposes typed, synchronous parse/format/tokenize
 * APIs. For non-blocking usage, see {@link ../worker-client}.
 *
 * @module
 */

import type { CompileOptions } from "./compiler-types.js";
import {
  engineVersion,
  formatSource,
  parseSource,
  tokenizeSource,
  configureTypecheckCompilerWasm,
  typecheckSource,
  typecheckSourceWithCompilerBytesSync,
  compileSource,
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
 * @param opts - Playground configuration. `wasmUrl` points to the compiler
 *   Wasm module used for compiler-backed type checking.
 * @returns An initialised {@link Playground} instance.
 */
export async function createPlayground(
  opts: PlaygroundOptions,
): Promise<Playground> {
  const compilerBytes = await loadCompilerBytes(opts.wasmUrl);
  if (compilerBytes) {
    configureTypecheckCompilerWasm(compilerBytes);
  }

  let destroyed = false;

  function ensureAlive(): void {
    if (destroyed) {
      throw new Error("Playground instance has been destroyed");
    }
  }

  return {
    parse(source: string): ParseResponse {
      ensureAlive();
      const parsed = parseSource(source);
      if (!compilerBytes || !parsed.ok) return parsed;
      const checked = typecheckSourceWithCompilerBytesSync(source, compilerBytes);
      return {
        ...parsed,
        ok: checked.ok,
        diagnostics: checked.diagnostics,
        error_count: checked.error_count,
      };
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
      if (compilerBytes) {
        return typecheckSourceWithCompilerBytesSync(source, compilerBytes);
      }
      return typecheckSource(source);
    },

    compile(source: string, options?: CompileOptions) {
      ensureAlive();
      return compileSource(source, options);
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

async function loadCompilerBytes(wasmUrl: string | URL): Promise<Uint8Array | null> {
  try {
    const response = await fetch(wasmUrl);
    if (!response.ok) return null;
    const bytes = new Uint8Array(await response.arrayBuffer());
    return WebAssembly.validate(bytes) ? bytes : null;
  } catch {
    return null;
  }
}
