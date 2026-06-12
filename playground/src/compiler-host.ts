/**
 * Node-like compiler process host for the selfhost compiler Wasm.
 *
 * @module
 */

import type { CheckResult, CompileOptions, CompileResult } from "./compiler-types.js";
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

/**
 * Type-check Arukellt source by running `arukellt check --json` in the
 * selfhost compiler Wasm.
 */
export async function checkWithCompilerWasm(
  compilerBytes: Uint8Array,
  source: string,
  options: CompileOptions = {},
): Promise<CheckResult> {
  const runner = createCheckRunner(compilerBytes, source, options);
  if ("result" in runner) return runner.result;

  try {
    const instantiated = await withTimeout(
      WebAssembly.instantiate(compilerBytes, runner.host.imports),
      runner.timeoutMs,
      "compiler instantiation timed out",
    );
    runCompilerInstance(getWasmInstance(instantiated), runner.host);
  } catch (err) {
    runner.trap = err instanceof Error ? err.message : String(err);
  }

  return finishCheck(runner);
}

/**
 * Synchronous type-check entry point for the main-thread playground API.
 *
 * The compiler bytes are fetched during `createPlayground()`, so this method
 * only performs the CPU-bound Wasm instantiation/execution step.
 */
export function checkWithCompilerWasmSync(
  compilerBytes: Uint8Array,
  source: string,
  options: CompileOptions = {},
): CheckResult {
  const runner = createCheckRunner(compilerBytes, source, options);
  if ("result" in runner) return runner.result;

  try {
    const module = new WebAssembly.Module(compilerBytes as unknown as BufferSource);
    const instance = new WebAssembly.Instance(module, runner.host.imports);
    runCompilerInstance(instance, runner.host);
  } catch (err) {
    runner.trap = err instanceof Error ? err.message : String(err);
  }

  return finishCheck(runner);
}

type CheckRunner =
  | { result: CheckResult }
  | {
      host: ReturnType<typeof createWasiHost>;
      timeoutMs: number;
      trap: string | null;
      started: number;
    };

function createCheckRunner(
  compilerBytes: Uint8Array,
  source: string,
  options: CompileOptions,
): CheckRunner {
  const started = performance.now();
  const target = options.target ?? DEFAULT_TARGET;
  const inputPath = options.inputPath ?? DEFAULT_INPUT;
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const maxSourceBytes = options.maxSourceBytes ?? DEFAULT_MAX_SOURCE_BYTES;

  const encoder = new TextEncoder();
  const sourceBytes = encoder.encode(source);
  if (sourceBytes.length > maxSourceBytes) {
    return {
      result: {
        ok: false,
        exitCode: 1,
        compilerStdout: "",
        compilerStderr: "",
        elapsedMs: performance.now() - started,
        error: `source exceeds ${maxSourceBytes} byte limit`,
      },
    };
  }

  const files = new Map<string, Uint8Array>();
  files.set(inputPath, sourceBytes);

  const argv = [
    "arukellt",
    "check",
    inputPath,
    "--target",
    target,
    "--json",
  ];

  return {
    host: createWasiHost({ argv, files }),
    timeoutMs,
    trap: null,
    started,
  };
}

function runCompilerInstance(
  instance: WebAssembly.Instance,
  host: ReturnType<typeof createWasiHost>,
): void {
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
  } catch (err) {
    if (err instanceof WasiExit) {
      host.result.exitCode = err.code;
    } else {
      throw err;
    }
  }
}

function finishCheck(runner: Exclude<CheckRunner, { result: CheckResult }>): CheckResult {
  const ok = runner.host.result.exitCode === 0 && !runner.trap;
  return {
    ok,
    exitCode: runner.host.result.exitCode,
    compilerStdout: runner.host.result.stdout,
    compilerStderr: runner.host.result.stderr,
    elapsedMs: performance.now() - runner.started,
    error: runner.trap ?? (runner.host.result.exitCode !== 0 ? `compiler exited with code ${runner.host.result.exitCode}` : null),
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
