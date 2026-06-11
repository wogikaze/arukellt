/**
 * Dedicated Web Worker for playground compile/run orchestration (ADR-032).
 *
 * @module
 */

import { compileWithCompilerWasm } from "./compiler-host.js";
import { runT2Wasm } from "./t2-runner.js";
import type { CompileOptions, CompileResult, RunOptions, RunResult } from "./compiler-types.js";

export type CompilerWorkerRequest =
  | { id: number; cmd: "init"; compilerUrl: string }
  | { id: number; cmd: "compile"; source: string; options?: CompileOptions }
  | { id: number; cmd: "run"; wasmBytes: Uint8Array; options?: RunOptions };

export type CompilerWorkerResponse =
  | { id: number; ok: true; result: unknown }
  | { id: number; ok: false; error: string };

let compilerBytes: Uint8Array | null = null;

async function handleMessage(msg: CompilerWorkerRequest): Promise<CompilerWorkerResponse> {
  const { id, cmd } = msg;
  try {
    if (cmd === "init") {
      const response = await fetch(msg.compilerUrl);
      if (!response.ok) {
        return { id, ok: false, error: `failed to load compiler asset (${response.status})` };
      }
      const buffer = await response.arrayBuffer();
      compilerBytes = new Uint8Array(buffer);
      return { id, ok: true, result: { size: compilerBytes.byteLength } };
    }

    if (!compilerBytes) {
      return { id, ok: false, error: "compiler worker not initialised" };
    }

    if (cmd === "compile") {
      const result = await compileWithCompilerWasm(compilerBytes, msg.source, msg.options);
      return { id, ok: true, result: serialiseCompileResult(result) };
    }

    if (cmd === "run") {
      const result = await runT2Wasm(msg.wasmBytes, msg.options);
      return { id, ok: true, result: result satisfies RunResult };
    }

    return { id, ok: false, error: `unknown command: ${(msg as CompilerWorkerRequest).cmd}` };
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return { id, ok: false, error: message };
  }
}

function serialiseCompileResult(result: CompileResult): CompileResult {
  return {
    ...result,
    wasmBytes: result.wasmBytes ? result.wasmBytes : null,
  };
}

declare const self: DedicatedWorkerGlobalScope;

self.addEventListener("message", async (event: MessageEvent<CompilerWorkerRequest>) => {
  const response = await handleMessage(event.data);
  self.postMessage(response);
});
