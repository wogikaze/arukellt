/**
 * Types for the playground compiler worker / process host (ADR-032).
 *
 * @module
 */
/** Options for a compile request. */
export interface CompileOptions {
    /** Compilation target. Defaults to `wasm32-freestanding`. */
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
/** How virtual stdin is delivered through `arukellt_io.read`. */
export type StdinMode = "stream" | "line";
/** Options for running compiled T2 Wasm. */
export interface RunOptions {
    /** Stdin bytes supplied to `arukellt_io.read`. */
    stdin?: Uint8Array;
    /**
     * Stdin delivery mode.
     *
     * - `stream` (default): byte stream until EOF.
     * - `line`: one line per `read_to_string` call (REPL-friendly).
     */
    stdinMode?: StdinMode;
    /** Wall-clock timeout in milliseconds. */
    timeoutMs?: number;
}
/** Result of running compiled T2 Wasm. */
export interface RunResult {
    /** Whether instantiation and execution completed without trap. */
    ok: boolean;
    /** Program stdout. */
    stdout: string;
    /** Program stderr. */
    stderr: string;
    /** Exit code when available (0 when `_start` returns normally). */
    exitCode: number;
    /** Trap or runtime error text. */
    trap: string | null;
    /** Elapsed run time in milliseconds. */
    elapsedMs: number;
}
/** Availability state for Build/Run controls. */
export interface CompilerRuntimeAvailability {
    /** Whether the compiler Wasm asset is present. */
    compilerAssetPresent: boolean;
    /** Whether WebAssembly is available in this environment. */
    wasmSupported: boolean;
    /** Whether the T2 runner can execute (compiler + wasm). */
    runSupported: boolean;
    /** User-facing reason when build/run is unavailable. */
    reason: string | null;
}
//# sourceMappingURL=compiler-types.d.ts.map