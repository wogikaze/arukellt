/**
 * Browser/Node runner for compiled `wasm32` (non-GC) Wasm modules.
 *
 * @module
 */

import type { RunOptions, RunResult, StdinMode } from "./compiler-types.js";

const textDecoder = new TextDecoder();
const textEncoder = new TextEncoder();

function getWasmInstance(
  instantiated: WebAssembly.Instance | WebAssembly.WebAssemblyInstantiatedSource,
): WebAssembly.Instance {
  if ("instance" in instantiated) {
    return instantiated.instance;
  }
  return instantiated;
}

/**
 * Split stdin bytes into newline-delimited chunks for line-oriented reads.
 *
 * Each chunk ends with `\n` except a final line with no trailing newline in the
 * source text.
 */
export function stdinBytesToLineChunks(stdin: Uint8Array): Uint8Array[] {
  if (stdin.length === 0) {
    return [];
  }

  const text = textDecoder.decode(stdin);
  const parts = text.split("\n");
  const chunks: Uint8Array[] = [];

  for (let i = 0; i < parts.length; i++) {
    const isLast = i === parts.length - 1;
    const suffix = isLast && !text.endsWith("\n") ? "" : "\n";
    chunks.push(textEncoder.encode(parts[i] + suffix));
  }

  return chunks;
}

function createStdinReader(stdin: Uint8Array, mode: StdinMode) {
  if (mode === "stream") {
    let offset = 0;
    return {
      read(ptr: number, len: number, memory: WebAssembly.Memory): number {
        const available = stdin.length - offset;
        if (available <= 0) return 0;
        const toCopy = Math.min(len, available);
        new Uint8Array(memory.buffer, ptr, toCopy).set(
          stdin.subarray(offset, offset + toCopy),
        );
        offset += toCopy;
        return toCopy;
      },
    };
  }

  const lines = stdinBytesToLineChunks(stdin);
  let lineIndex = 0;
  let pending: Uint8Array | null = null;
  let endCurrentSession = false;

  return {
    read(ptr: number, len: number, memory: WebAssembly.Memory): number {
      if (endCurrentSession) {
        endCurrentSession = false;
        return 0;
      }

      if (!pending) {
        if (lineIndex >= lines.length) {
          return 0;
        }
        pending = lines[lineIndex];
        lineIndex += 1;
      }

      const toCopy = Math.min(len, pending.length);
      new Uint8Array(memory.buffer, ptr, toCopy).set(pending.subarray(0, toCopy));
      pending = toCopy < pending.length ? pending.subarray(toCopy) : null;
      if (!pending) {
        endCurrentSession = true;
      }
      return toCopy;
    },
  };
}

/**
 * Instantiate and execute wasm32 Wasm bytes with an `arukellt_io` stdio host.
 */
export async function runT2Wasm(
  wasmBytes: Uint8Array,
  options: RunOptions = {},
): Promise<RunResult> {
  const started = performance.now();
  const stdin = options.stdin ?? new Uint8Array();
  const stdinMode = options.stdinMode ?? "stream";
  const stdinReader = createStdinReader(stdin, stdinMode);
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
        return stdinReader.read(ptr, len, memory);
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
      throw new Error("wasm32 module does not export memory");
    }
    activeMemory = memory;

    const start = instance.exports._start as (() => void) | undefined;
    const main = instance.exports.main as (() => void) | undefined;
    const entry = start ?? main;
    if (typeof entry !== "function") {
      throw new Error("wasm32 module does not export _start or main");
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

/** Return true when a Wasm module imports the wasm32 stdio surface. */
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
