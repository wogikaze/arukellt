/**
 * Node-like compiler process host for the selfhost compiler Wasm.
 *
 * @module
 */

import type { CompileOptions, CompileResult } from "./compiler-types.js";
import {
  createWasiHost,
  readVirtualFile,
  WasiExit,
} from "./wasi/minimal-host.js";

const DEFAULT_TARGET = "wasm32-freestanding";
const DEFAULT_INPUT = "/work/main.ark";
const DEFAULT_OUTPUT = "/work/out.wasm";
const DEFAULT_TIMEOUT_MS = 60_000;
const DEFAULT_MAX_SOURCE_BYTES = 256_000;
const DEFAULT_MAX_OUTPUT_BYTES = 4_000_000;

/**
 * Compile Arukellt source by running the selfhost compiler Wasm with a WASI host.
 */
export async function compileWithCompilerWasm(
  compilerBytes: Uint8Array,
  source: string,
  options: CompileOptions = {},
): Promise<CompileResult> {
  const started = performance.now();
  const target = options.target ?? DEFAULT_TARGET;
  const inputPath = options.inputPath ?? DEFAULT_INPUT;
  const outputPath = options.outputPath ?? DEFAULT_OUTPUT;
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const maxSourceBytes = options.maxSourceBytes ?? DEFAULT_MAX_SOURCE_BYTES;
  const maxOutputBytes = options.maxOutputBytes ?? DEFAULT_MAX_OUTPUT_BYTES;

  const encoder = new TextEncoder();
  const sourceBytes = encoder.encode(source);
  if (sourceBytes.length > maxSourceBytes) {
    return {
      ok: false,
      exitCode: 1,
      compilerStdout: "",
      compilerStderr: "",
      wasmBytes: null,
      outputSize: 0,
      elapsedMs: performance.now() - started,
      error: `source exceeds ${maxSourceBytes} byte limit`,
    };
  }

  const files = new Map<string, Uint8Array>();
  files.set(inputPath, sourceBytes);

  const argv = [
    "arukellt",
    "compile",
    inputPath,
    "--target",
    target,
    "-o",
    outputPath,
  ];

  const host = createWasiHost({ argv, files });

  let exitCode = 1;
  let trap: string | null = null;

  try {
    const instantiated = await withTimeout(
      WebAssembly.instantiate(compilerBytes, host.imports),
      timeoutMs,
      "compiler instantiation timed out",
    );
    const instance = getWasmInstance(instantiated);
    const memory = instance.exports.memory as WebAssembly.Memory | undefined;
    if (!memory) {
      throw new Error("compiler module does not export memory");
    }
    host.bindMemory(memory);

    const start = instance.exports._start as (() => void) | undefined;
    if (typeof start !== "function") {
      throw new Error("compiler module does not export _start");
    }

    try {
      start();
      exitCode = host.result.exitCode;
    } catch (err) {
      if (err instanceof WasiExit) {
        exitCode = err.code;
      } else {
        throw err;
      }
    }
  } catch (err) {
    trap = err instanceof Error ? err.message : String(err);
  }

  const output = readVirtualFile(files, outputPath);
  const outputSize = output?.byteLength ?? 0;
  if (output && outputSize > maxOutputBytes) {
    return {
      ok: false,
      exitCode,
      compilerStdout: host.result.stdout,
      compilerStderr: host.result.stderr,
      wasmBytes: null,
      outputSize,
      elapsedMs: performance.now() - started,
      error: `output exceeds ${maxOutputBytes} byte limit`,
    };
  }

  const ok = exitCode === 0 && output !== null && !trap;
  return {
    ok,
    exitCode,
    compilerStdout: host.result.stdout,
    compilerStderr: host.result.stderr,
    wasmBytes: ok ? output : null,
    outputSize,
    elapsedMs: performance.now() - started,
    error: trap ?? (exitCode !== 0 ? `compiler exited with code ${exitCode}` : null),
  };
}

function getWasmInstance(
  instantiated: WebAssembly.Instance | WebAssembly.WebAssemblyInstantiatedSource,
): WebAssembly.Instance {
  if ("instance" in instantiated) {
    return instantiated.instance;
  }
  return instantiated;
}

async function withTimeout<T>(
  promise: Promise<T>,
  timeoutMs: number,
  message: string,
): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      promise,
      new Promise<T>((_, reject) => {
        timer = setTimeout(() => reject(new Error(message)), timeoutMs);
      }),
    ]);
  } finally {
    if (timer !== undefined) clearTimeout(timer);
  }
}
