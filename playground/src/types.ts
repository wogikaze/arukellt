/**
 * TypeScript type definitions for the Arukellt playground API.
 *
 * These types mirror the JSON response shapes returned by the
 * `ark-playground-wasm` module. See `crates/ark-playground-wasm/README.md`
 * for the canonical Rust definitions.
 *
 * @module
 */

// ---------------------------------------------------------------------------
// Diagnostic types
// ---------------------------------------------------------------------------

/** Severity level of a diagnostic. */
export type Severity = "error" | "warning" | "help";

/** Phase in the compiler pipeline that produced the diagnostic. */
export type Phase = "lex" | "parse";

/** A source span label attached to a diagnostic. */
export interface DiagnosticLabel {
  /** File identifier (always `0` in the playground — single-file mode). */
  file_id: number;
  /** Start byte offset in the source. */
  start: number;
  /** End byte offset in the source (exclusive). */
  end: number;
  /** Human-readable message for this span. */
  message: string;
}

/** A compiler diagnostic (error, warning, or help). */
export interface Diagnostic {
  /** Machine-readable diagnostic code (e.g. `"E0001"`). */
  code: string;
  /** Severity: `"error"`, `"warning"`, or `"help"`. */
  severity: Severity;
  /** Compiler phase that emitted this diagnostic. */
  phase: Phase;
  /** Human-readable diagnostic message. */
  message: string;
  /** Source span labels with contextual messages. */
  labels: DiagnosticLabel[];
  /** Additional notes attached to the diagnostic. */
  notes: string[];
  /** Optional fix suggestion. */
  suggestion: string | null;
}

// ---------------------------------------------------------------------------
// AST summary types
// ---------------------------------------------------------------------------

/** Item kind in the parsed module. */
export type ItemKind = "fn" | "struct" | "enum" | "trait" | "impl";

/** A single top-level item in the parsed module. */
export interface ModuleItem {
  /** Item kind: `"fn"`, `"struct"`, `"enum"`, `"trait"`, or `"impl"`. */
  kind: ItemKind;
  /** Item name (for `impl` blocks, this is the target type name). */
  name: string;
  /** Whether the item is marked `pub`. */
  is_pub: boolean;
  /** Doc comments attached to the item. */
  docs: string[];
}

/** An import declaration in the parsed module. */
export interface ModuleImport {
  /** The imported module name. */
  module_name: string;
  /** Optional alias (`import math as m` → alias is `"m"`). */
  alias: string | null;
}

/** Lightweight summary of a parsed Arukellt module. */
export interface ModuleSummary {
  /** Module-level doc comments. */
  docs: string[];
  /** Import declarations. */
  imports: ModuleImport[];
  /** Top-level items (functions, structs, enums, traits, impl blocks). */
  items: ModuleItem[];
}

// ---------------------------------------------------------------------------
// Token types
// ---------------------------------------------------------------------------

/** A single lexer token. */
export interface Token {
  /** Token kind (e.g. `"Fn"`, `"Ident"`, `"LParen"`). */
  kind: string;
  /** The source text of this token. */
  text: string;
  /** Start byte offset in the source. */
  start: number;
  /** End byte offset in the source (exclusive). */
  end: number;
}

// ---------------------------------------------------------------------------
// API response envelopes
// ---------------------------------------------------------------------------

/** Response from `parse()`. */
export interface ParseResponse {
  /** `true` if parsing succeeded without errors. */
  ok: boolean;
  /** Module summary (present even on error, may be partial). */
  module: ModuleSummary | null;
  /** All diagnostics from lexing and parsing. */
  diagnostics: Diagnostic[];
  /** Number of error-severity diagnostics. */
  error_count: number;
}

/** Response from `format()`. */
export interface FormatResponse {
  /** `true` if formatting succeeded. */
  ok: boolean;
  /** Formatted source code (present when `ok` is `true`). */
  formatted?: string;
  /** Error message (present when `ok` is `false`). */
  error?: string;
}

/** Response from `tokenize()`. */
export interface TokenizeResponse {
  /** `true` if tokenization succeeded without errors. */
  ok: boolean;
  /** Token stream. */
  tokens: Token[];
  /** Lexer diagnostics. */
  diagnostics: Diagnostic[];
}

// ---------------------------------------------------------------------------
// Configuration and playground interface
// ---------------------------------------------------------------------------

/** Options for initialising the playground. */
export interface PlaygroundOptions {
  /**
   * URL or path to the `.wasm` binary.
   *
   * When using the `web` target from `wasm-pack`, this is the
   * `ark_playground_wasm_bg.wasm` file.
   *
   * @example "/assets/ark_playground_wasm_bg.wasm"
   */
  wasmUrl: string | URL;
}

/** Options for creating a worker-based playground. */
export interface WorkerPlaygroundOptions extends PlaygroundOptions {
  /**
   * URL or path to the worker script.
   *
   * If omitted, the library constructs a blob-based worker from
   * the bundled worker source (requires same-origin wasm URL).
   */
  workerUrl?: string | URL;
}

/**
 * Synchronous playground API (main-thread execution).
 *
 * All methods execute on the calling thread. For non-blocking
 * execution, use {@link WorkerPlayground} instead.
 */
export interface Playground {
  /** Parse Arukellt source and return AST summary + diagnostics. */
  parse(source: string): ParseResponse;
  /** Format Arukellt source. Returns error if source has syntax errors. */
  format(source: string): FormatResponse;
  /** Tokenize Arukellt source. */
  tokenize(source: string): TokenizeResponse;
  /** Return the Wasm module version. */
  version(): string;
  /** Release the Wasm module resources. */
  destroy(): void;
}

/**
 * Asynchronous playground API (Web Worker execution).
 *
 * All methods return promises because they dispatch to a background
 * Web Worker. The Wasm module never blocks the main thread.
 */
export interface WorkerPlayground {
  /** Parse Arukellt source and return AST summary + diagnostics. */
  parse(source: string): Promise<ParseResponse>;
  /** Format Arukellt source. Returns error if source has syntax errors. */
  format(source: string): Promise<FormatResponse>;
  /** Tokenize Arukellt source. */
  tokenize(source: string): Promise<TokenizeResponse>;
  /** Return the Wasm module version. */
  version(): Promise<string>;
  /** Terminate the worker and release resources. */
  destroy(): void;
}

// ---------------------------------------------------------------------------
// Worker message protocol (internal, exported for worker script)
// ---------------------------------------------------------------------------

/** Commands sent from main thread → worker. */
export type WorkerRequest =
  | { id: number; cmd: "init"; wasmUrl: string }
  | { id: number; cmd: "parse"; source: string }
  | { id: number; cmd: "format"; source: string }
  | { id: number; cmd: "tokenize"; source: string }
  | { id: number; cmd: "version" };

/** Responses sent from worker → main thread. */
export type WorkerResponse =
  | { id: number; ok: true; result: unknown }
  | { id: number; ok: false; error: string };
