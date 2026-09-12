/**
 * Types for the playground compiler worker / process host (ADR-017).
 *
 * @module
 */

/** Options for a compile request. */
export interface CompileOptions {
  /** Compilation target. Defaults to `wasm32-gc`. */
  target?: string;
  /** Virtual input path inside the worker filesystem. */
  inputPath?: string;
  /** Virtual output path inside the worker filesystem. */
  outputPath?: string;
  /** Wall-clock timeout in milliseconds. */
  timeoutMs?: number;
  /** Maximum source byte length. */
  maxSourceBytes?: number;
  /** Maximum output Wasm byte length. */
  maxOutputBytes?: number;
}

/** Result of a compile request. */
export interface CompileResult {
  /** Whether compilation succeeded (exit code 0 and output present). */
  ok: boolean;
  /** Compiler process exit code. */
  exitCode: number;
  /** Captured compiler stdout. */
  compilerStdout: string;
  /** Captured compiler stderr. */
  compilerStderr: string;
  /** Compiled Wasm bytes when compilation succeeded. */
  wasmBytes: Uint8Array | null;
  /** Output Wasm size in bytes. */
  outputSize: number;
  /** Elapsed compile time in milliseconds. */
  elapsedMs: number;
  /** Error message when the host failed before/during compilation. */
  error: string | null;
}

/** Result of a type-check request. */
export interface CheckResult {
  /** Whether checking succeeded (exit code 0 and no host trap). */
  ok: boolean;
  /** Compiler process exit code. */
  exitCode: number;
  /** Captured compiler stdout, including JSON diagnostics when requested. */
  compilerStdout: string;
  /** Captured compiler stderr. */
  compilerStderr: string;
  /** Elapsed check time in milliseconds. */
  elapsedMs: number;
  /** Error message when the host failed before/during checking. */
  error: string | null;
}

/** Result of a format request. */
export interface FormatResult {
  /** Whether formatting succeeded (exit code 0 and no host trap). */
  ok: boolean;
  /** Compiler process exit code. */
  exitCode: number;
  /** Captured compiler stdout. */
  compilerStdout: string;
  /** Captured compiler stderr. */
  compilerStderr: string;
  /** Formatted source when formatting succeeded. */
  formatted: string | null;
  /** Elapsed format time in milliseconds. */
  elapsedMs: number;
  /** Error message when the host failed before/during formatting. */
  error: string | null;
}

/** Availability state for the compiler-backed build control. */
export interface CompilerRuntimeAvailability {
  /** Whether the compiler Wasm asset is present. */
  compilerAssetPresent: boolean;
  /** Whether WebAssembly is available in this environment. */
  wasmSupported: boolean;
  /** User-facing reason when compiler-backed build is unavailable. */
  reason: string | null;
}
