/**
 * Browser/Node runner for compiled T2 (`wasm32-freestanding`) Wasm modules.
 *
 * @module
 */

import type { RunOptions, RunResult } from "./compiler-types.js";

const textDecoder = new TextDecoder();

function getWasmInstance(
  instantiated: WebAssembly.Instance | WebAssembly.WebAssemblyInstantiatedSource,
): WebAssembly.Instance {
  if ("instance" in instantiated) {
    return instantiated.instance;
  }
  return instantiated;
}

/**
 * Instantiate and execute T2 Wasm bytes with an `arukellt_io` stdio host.
 */
export async function runT2Wasm(
  wasmBytes: Uint8Array,
  options: RunOptions = {},
): Promise<RunResult> {
  const started = performance.now();
  const stdin = options.stdin ?? new Uint8Array();
  let stdinOffset = 0;
  const stdoutChunks: string[] = [];
  const stderrChunks: string[] = [];

  const imports: WebAssembly.Imports = {
    arukellt_io: {
      write(ptr: number, len: number): void {
        const memory = activeMemory;
        if (!memory) return;
        stdoutChunks.push(textDecoder.decode(new Uint8Array(memory.buffer, ptr, len)));
      },
      write_err(ptr: number, len: number): void {
        const memory = activeMemory;
        if (!memory) return;
        stderrChunks.push(textDecoder.decode(new Uint8Array(memory.buffer, ptr, len)));
      },
      flush(): void {
        // Lines are assembled when the host reads stdout.
      },
      flush_err(): void {
        // Lines are assembled when the host reads stderr.
      },
      read(ptr: number, len: number): number {
        const memory = activeMemory;
        if (!memory) return 0;
        const available = stdin.length - stdinOffset;
        if (available <= 0) return 0;
        const toCopy = Math.min(len, available);
        new Uint8Array(memory.buffer, ptr, toCopy).set(
          stdin.subarray(stdinOffset, stdinOffset + toCopy),
        );
        stdinOffset += toCopy;
        return toCopy;
      },
    },
  };

  let activeMemory: WebAssembly.Memory | null = null;
  let exitCode = 0;
  let trap: string | null = null;

  try {
    const instantiated = await WebAssembly.instantiate(wasmBytes, imports);
    const instance = getWasmInstance(instantiated);
    const memory = instance.exports.memory as WebAssembly.Memory | undefined;
    if (!memory) {
      throw new Error("T2 module does not export memory");
    }
    activeMemory = memory;

    const start = instance.exports._start as (() => void) | undefined;
    const main = instance.exports.main as (() => void) | undefined;
    const entry = start ?? main;
    if (typeof entry !== "function") {
      throw new Error("T2 module does not export _start or main");
    }
    entry();
  } catch (err) {
    trap = err instanceof Error ? err.message : String(err);
    exitCode = 1;
  }

  return {
    ok: trap === null,
    stdout: stdoutChunks.join(""),
    stderr: stderrChunks.join(""),
    exitCode,
    trap,
    elapsedMs: performance.now() - started,
  };
}

/** Return true when a Wasm module imports the T2 stdio surface. */
export function moduleImportsArukelltIo(bytes: Uint8Array): boolean {
  const needle = new TextEncoder().encode("arukellt_io");
  for (let i = 0; i <= bytes.length - needle.length; i++) {
    let matched = true;
    for (let j = 0; j < needle.length; j++) {
      if (bytes[i + j] !== needle[j]) {
        matched = false;
        break;
      }
    }
    if (matched) return true;
  }
  return false;
}
