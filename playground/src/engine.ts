/**
 * Browser-native playground engine.
 *
 * This is a lightweight API-compatible engine for the playground shell. The
 * selfhost compiler remains the source of truth for CLI verification; this
 * module preserves browser parse, tokenize, format, and typecheck response
 * shapes without depending on the retired Rust wasm playground crate.
 */

import type {
  Diagnostic,
  FormatResponse,
  ItemKind,
  ModuleImport,
  ModuleItem,
  ModuleSummary,
  ParseResponse,
  Severity,
  Token,
  TokenizeResponse,
  TypecheckResponse,
} from "./types.js";
import type { CheckResult, CompileOptions } from "./compiler-types.js";
import {
  checkWithCompilerWasm,
  checkWithCompilerWasmSync,
  formatWithCompilerWasmSync,
} from "./compiler-host.js";

const VERSION = "selfhost-playground-ts-v1";

const KEYWORDS = new Map<string, string>([
  ["fn", "Fn"],
  ["struct", "Struct"],
  ["enum", "Enum"],
  ["let", "Let"],
  ["mut", "Mut"],
  ["if", "If"],
  ["else", "Else"],
  ["match", "Match"],
  ["while", "While"],
  ["loop", "Loop"],
  ["for", "For"],
  ["in", "In"],
  ["break", "Break"],
  ["continue", "Continue"],
  ["return", "Return"],
  ["pub", "Pub"],
  ["import", "Import"],
  ["as", "As"],
  ["trait", "Trait"],
  ["impl", "Impl"],
  ["use", "Use"],
]);

const MULTI_CHAR_TOKENS = new Map<string, string>([
  ["==", "EqEq"],
  ["!=", "BangEq"],
  ["<=", "LtEq"],
  [">=", "GtEq"],
  ["&&", "AmpAmp"],
  ["||", "PipePipe"],
  ["<<", "Shl"],
  [">>", "Shr"],
  ["->", "Arrow"],
  ["=>", "FatArrow"],
  ["..", "DotDot"],
  ["::", "ColonColon"],
]);

const SINGLE_CHAR_TOKENS = new Map<string, string>([
  ["+", "Plus"],
  ["-", "Minus"],
  ["*", "Star"],
  ["/", "Slash"],
  ["%", "Percent"],
  ["<", "Lt"],
  [">", "Gt"],
  ["!", "Bang"],
  ["&", "Amp"],
  ["|", "Pipe"],
  ["^", "Caret"],
  ["~", "Tilde"],
  ["=", "Eq"],
  ["(", "LParen"],
  [")", "RParen"],
  ["{", "LBrace"],
  ["}", "RBrace"],
  ["[", "LBracket"],
  ["]", "RBracket"],
  [",", "Comma"],
  [";", "Semi"],
  [".", "Dot"],
  ["?", "Question"],
  [":", "Colon"],
]);

const ITEM_KINDS = new Map<string, ItemKind>([
  ["Fn", "fn"],
  ["Struct", "struct"],
  ["Enum", "enum"],
  ["Trait", "trait"],
  ["Impl", "impl"],
]);

const OPEN_TO_CLOSE = new Map<string, string>([
  ["LParen", "RParen"],
  ["LBrace", "RBrace"],
  ["LBracket", "RBracket"],
]);

const CLOSE_TO_OPEN = new Map<string, string>([
  ["RParen", "LParen"],
  ["RBrace", "LBrace"],
  ["RBracket", "LBracket"],
]);

function diagnostic(
  code: string,
  phase: Diagnostic["phase"],
  message: string,
  start: number,
  end: number,
  label: string,
): Diagnostic {
  return {
    code,
    severity: "error",
    phase,
    message,
    labels: [{ file_id: 0, start, end, message: label }],
    notes: [],
    suggestion: null,
  };
}

function isIdentStart(ch: string): boolean {
  return /[A-Za-z_]/.test(ch);
}

function isIdentContinue(ch: string): boolean {
  return /[A-Za-z0-9_]/.test(ch);
}

function isDigit(ch: string): boolean {
  return /[0-9]/.test(ch);
}

function identKind(text: string): string {
  if (text === "true" || text === "false") {
    return `BoolLit(${text})`;
  }
  return KEYWORDS.get(text) ?? `Ident(${JSON.stringify(text)})`;
}

function cleanDocComment(text: string): string {
  return text.replace(/^\/\/[\/!]\s?/, "").trim();
}

function tokenIdentText(token: Token | undefined): string | null {
  if (!token) return null;
  if (token.kind.startsWith("Ident(")) return token.text;
  return null;
}

function countErrors(diagnostics: Diagnostic[]): number {
  return diagnostics.filter((diag) => diag.severity === "error").length;
}

let configuredCompilerBytes: Uint8Array | null = null;

interface CompilerJsonDiagnostic {
  code?: unknown;
  severity?: unknown;
  span?: {
    start?: unknown;
    end?: unknown;
  };
  message?: unknown;
}

interface CompilerJsonEnvelope {
  phase?: unknown;
  success?: unknown;
  diagnostics?: unknown;
}

export function configureTypecheckCompilerWasm(compilerBytes: Uint8Array | null): void {
  configuredCompilerBytes = compilerBytes;
}

export function tokenizeSource(source: string): TokenizeResponse {
  const tokens: Token[] = [];
  const diagnostics: Diagnostic[] = [];
  let i = 0;

  while (i < source.length) {
    const ch = source[i];

    if (/\s/.test(ch)) {
      i += 1;
      continue;
    }

    if (source.startsWith("///", i) || source.startsWith("//!", i)) {
      const start = i;
      while (i < source.length && source[i] !== "\n") i += 1;
      const text = source.slice(start, i);
      tokens.push({
        kind: text.startsWith("//!") ? "InnerDocComment" : "OuterDocComment",
        text,
        start,
        end: i,
      });
      continue;
    }

    if (source.startsWith("//", i)) {
      const start = i;
      while (i < source.length && source[i] !== "\n") i += 1;
      tokens.push({ kind: "LineComment", text: source.slice(start, i), start, end: i });
      continue;
    }

    if (source.startsWith("/*", i)) {
      const start = i;
      i += 2;
      while (i < source.length && !source.startsWith("*/", i)) i += 1;
      if (i >= source.length) {
        diagnostics.push(
          diagnostic("ELEX001", "lex", "unterminated block comment", start, source.length, "comment starts here"),
        );
        tokens.push({ kind: "BlockComment", text: source.slice(start), start, end: source.length });
        break;
      }
      i += 2;
      tokens.push({ kind: "BlockComment", text: source.slice(start, i), start, end: i });
      continue;
    }

    if (ch === '"' || ch === "'") {
      const quote = ch;
      const start = i;
      i += 1;
      let escaped = false;
      let closed = false;
      while (i < source.length) {
        const current = source[i];
        if (escaped) {
          escaped = false;
        } else if (current === "\\") {
          escaped = true;
        } else if (current === quote) {
          i += 1;
          closed = true;
          break;
        } else if (current === "\n") {
          break;
        }
        i += 1;
      }
      if (!closed) {
        diagnostics.push(
          diagnostic("ELEX002", "lex", "unterminated string literal", start, i, "literal starts here"),
        );
      }
      tokens.push({
        kind: quote === '"' ? "StringLit" : "CharLit",
        text: source.slice(start, i),
        start,
        end: i,
      });
      continue;
    }

    if (isDigit(ch)) {
      const start = i;
      while (i < source.length && isDigit(source[i])) i += 1;
      let kind = "IntLit";
      if (source[i] === "." && source[i + 1] !== ".") {
        kind = "FloatLit";
        i += 1;
        while (i < source.length && isDigit(source[i])) i += 1;
      }
      const text = source.slice(start, i);
      tokens.push({ kind: `${kind}(${text})`, text, start, end: i });
      continue;
    }

    if (isIdentStart(ch)) {
      const start = i;
      i += 1;
      while (i < source.length && isIdentContinue(source[i])) i += 1;
      const text = source.slice(start, i);
      tokens.push({ kind: identKind(text), text, start, end: i });
      continue;
    }

    const two = source.slice(i, i + 2);
    const multiKind = MULTI_CHAR_TOKENS.get(two);
    if (multiKind) {
      tokens.push({ kind: multiKind, text: two, start: i, end: i + 2 });
      i += 2;
      continue;
    }

    const singleKind = SINGLE_CHAR_TOKENS.get(ch);
    if (singleKind) {
      tokens.push({ kind: singleKind, text: ch, start: i, end: i + 1 });
      i += 1;
      continue;
    }

    diagnostics.push(
      diagnostic("ELEX003", "lex", `unexpected character '${ch}'`, i, i + 1, "unexpected character"),
    );
    tokens.push({ kind: "Error", text: ch, start: i, end: i + 1 });
    i += 1;
  }

  tokens.push({ kind: "Eof", text: "", start: source.length, end: source.length });
  return { ok: diagnostics.length === 0, tokens, diagnostics };
}

function parseImports(tokens: Token[]): ModuleImport[] {
  const imports: ModuleImport[] = [];

  for (let i = 0; i < tokens.length; i += 1) {
    if (tokens[i].kind !== "Import" && tokens[i].kind !== "Use") continue;

    const pathParts: string[] = [];
    let alias: string | null = null;
    for (let j = i + 1; j < tokens.length; j += 1) {
      const token = tokens[j];
      if (token.kind === "Semi" || token.kind === "Eof" || ITEM_KINDS.has(token.kind)) break;
      if (token.kind === "As") {
        alias = tokenIdentText(tokens[j + 1]);
        break;
      }
      const ident = tokenIdentText(token);
      if (ident) pathParts.push(ident);
    }
    if (pathParts.length > 0) {
      imports.push({ module_name: pathParts.join("::"), alias });
    }
  }

  return imports;
}

function parseItems(tokens: Token[]): { docs: string[]; items: ModuleItem[]; diagnostics: Diagnostic[] } {
  const moduleDocs: string[] = [];
  const items: ModuleItem[] = [];
  const diagnostics: Diagnostic[] = [];
  let pendingDocs: string[] = [];
  let pendingPub = false;

  for (let i = 0; i < tokens.length; i += 1) {
    const token = tokens[i];
    if (token.kind === "InnerDocComment") {
      moduleDocs.push(cleanDocComment(token.text));
      continue;
    }
    if (token.kind === "OuterDocComment") {
      pendingDocs.push(cleanDocComment(token.text));
      continue;
    }
    if (token.kind === "Pub") {
      pendingPub = true;
      continue;
    }

    const kind = ITEM_KINDS.get(token.kind);
    if (!kind) {
      if (token.kind !== "LineComment" && token.kind !== "BlockComment") {
        pendingDocs = [];
        pendingPub = false;
      }
      continue;
    }

    const nameToken = tokens.slice(i + 1).find((candidate) => tokenIdentText(candidate) !== null);
    const name = tokenIdentText(nameToken);
    if (!name) {
      diagnostics.push(
        diagnostic("EPARSE001", "parse", `missing ${kind} name`, token.start, token.end, "item keyword needs a name"),
      );
      continue;
    }

    items.push({
      kind,
      name,
      is_pub: pendingPub,
      docs: pendingDocs,
    });
    pendingDocs = [];
    pendingPub = false;
  }

  return { docs: moduleDocs, items, diagnostics };
}

function delimiterDiagnostics(tokens: Token[]): Diagnostic[] {
  const diagnostics: Diagnostic[] = [];
  const stack: Token[] = [];

  for (const token of tokens) {
    if (OPEN_TO_CLOSE.has(token.kind)) {
      stack.push(token);
      continue;
    }
    const expectedOpen = CLOSE_TO_OPEN.get(token.kind);
    if (!expectedOpen) continue;
    const actualOpen = stack.pop();
    if (!actualOpen || actualOpen.kind !== expectedOpen) {
      diagnostics.push(
        diagnostic("EPARSE002", "parse", `unmatched '${token.text}'`, token.start, token.end, "closing delimiter has no opener"),
      );
    }
  }

  for (const token of stack.reverse()) {
    diagnostics.push(
      diagnostic("EPARSE003", "parse", `unclosed '${token.text}'`, token.start, token.end, "opening delimiter is not closed"),
    );
  }

  return diagnostics;
}

export function parseSource(source: string): ParseResponse {
  const tokenized = tokenizeSource(source);
  const itemResult = parseItems(tokenized.tokens);
  const diagnostics = [
    ...tokenized.diagnostics,
    ...itemResult.diagnostics,
    ...delimiterDiagnostics(tokenized.tokens),
  ];
  const module: ModuleSummary = {
    docs: itemResult.docs,
    imports: parseImports(tokenized.tokens),
    items: itemResult.items,
  };
  const error_count = countErrors(diagnostics);
  return {
    ok: error_count === 0,
    module,
    diagnostics,
    error_count,
  };
}

export function formatSource(source: string): FormatResponse {
  const parsed = parseSource(source);
  if (!parsed.ok) {
    return { ok: false, error: "source contains syntax errors" };
  }

  if (!configuredCompilerBytes) {
    return { ok: false, error: "selfhost compiler wasm has not been initialised" };
  }

  const result = formatWithCompilerWasmSync(configuredCompilerBytes, source);
  if (!result.ok || result.formatted === null) {
    return {
      ok: false,
      error: result.error ?? (result.compilerStderr.trim() || "formatting failed"),
    };
  }

  return { ok: true, formatted: result.formatted };
}

export function typecheckSource(source: string): TypecheckResponse {
  if (!configuredCompilerBytes) {
    return typecheckUnavailable("selfhost compiler wasm has not been initialised", source);
  }
  return typecheckSourceWithCompilerBytesSync(source, configuredCompilerBytes);
}

export function typecheckSourceWithCompilerBytesSync(
  source: string,
  compilerBytes: Uint8Array,
  options: CompileOptions = {},
): TypecheckResponse {
  const result = checkWithCompilerWasmSync(compilerBytes, source, options);
  return typecheckResponseFromCheckResult(result, source);
}

export async function typecheckSourceWithCompilerBytes(
  source: string,
  compilerBytes: Uint8Array,
  options: CompileOptions = {},
): Promise<TypecheckResponse> {
  const result = await checkWithCompilerWasm(compilerBytes, source, options);
  return typecheckResponseFromCheckResult(result, source);
}

export function engineVersion(): string {
  return VERSION;
}

function typecheckResponseFromCheckResult(
  result: CheckResult,
  source: string,
): TypecheckResponse {
  const parsedEnvelope = parseCompilerJsonEnvelope(result.compilerStdout);
  if (parsedEnvelope) {
    let diagnostics = compilerDiagnosticsToPlayground(
      parsedEnvelope,
      source,
      result.compilerStderr,
    );
    if (!result.ok && diagnostics.length === 0) {
      diagnostics = diagnosticsFromCompilerText(
        result.compilerStderr || result.error || "typecheck failed",
        source,
        phaseFromCompiler(parsedEnvelope.phase),
      );
    }
    const error_count = countErrors(diagnostics);
    const success = parsedEnvelope.success === true || result.ok;
    return {
      ok: success && error_count === 0,
      diagnostics,
      error_count,
    };
  }

  if (result.ok) {
    return { ok: true, diagnostics: [], error_count: 0 };
  }

  const diagnosticText = result.compilerStderr || result.compilerStdout || result.error || "typecheck failed";
  const diagnostics = diagnosticsFromCompilerText(diagnosticText, source);
  return {
    ok: false,
    diagnostics,
    error_count: countErrors(diagnostics),
  };
}

function parseCompilerJsonEnvelope(stdout: string): CompilerJsonEnvelope | null {
  const trimmed = stdout.trim();
  if (!trimmed.startsWith("{")) return null;
  try {
    const parsed = JSON.parse(trimmed) as CompilerJsonEnvelope;
    if (parsed && typeof parsed === "object") return parsed;
  } catch {
    return null;
  }
  return null;
}

function compilerDiagnosticsToPlayground(
  envelope: CompilerJsonEnvelope,
  source: string,
  fallbackText: string,
): Diagnostic[] {
  if (!Array.isArray(envelope.diagnostics)) {
    return [];
  }

  const phase = phaseFromCompiler(envelope.phase);
  return envelope.diagnostics.map((diag, index) =>
    compilerDiagnosticToPlayground(diag as CompilerJsonDiagnostic, phase, source, fallbackText, index),
  );
}

function compilerDiagnosticToPlayground(
  diag: CompilerJsonDiagnostic,
  phase: Diagnostic["phase"],
  source: string,
  fallbackText: string,
  index: number,
): Diagnostic {
  const message = typeof diag.message === "string" && diag.message.length > 0
    ? diag.message
    : fallbackText || "typecheck failed";
  const start = clampOffset(numberOr(diag.span?.start, 0), source.length);
  const rawEnd = numberOr(diag.span?.end, start);
  const end = Math.max(start, clampOffset(rawEnd, source.length));

  return {
    code: typeof diag.code === "string" && diag.code.length > 0
      ? diag.code
      : fallbackCodeForPhase(phase, index),
    severity: severityFromCompiler(diag.severity),
    phase,
    message,
    labels: [{ file_id: 0, start, end, message }],
    notes: [],
    suggestion: null,
  };
}

function diagnosticsFromCompilerText(
  text: string,
  source: string,
  fallbackPhase: Diagnostic["phase"] = "typecheck",
): Diagnostic[] {
  const phase = text.includes("|parse]") ? "parse" : fallbackPhase;
  const codeMatch = text.match(/error\[(E\d{4})\|/u);
  return [
    diagnostic(
      codeMatch?.[1] ?? fallbackCodeForPhase(phase, 0),
      phase,
      text.trim() || "typecheck failed",
      0,
      Math.min(source.length, 1),
      "compiler reported this diagnostic",
    ),
  ];
}

function typecheckUnavailable(message: string, source: string): TypecheckResponse {
  const diagnostics = [
    diagnostic(
      "EPLAYGROUND001",
      "typecheck",
      message,
      0,
      Math.min(source.length, 1),
      "initialise the playground with the selfhost compiler wasm",
    ),
  ];
  return { ok: false, diagnostics, error_count: 1 };
}

function phaseFromCompiler(value: unknown): Diagnostic["phase"] {
  if (value === 1 || value === "1" || value === "lex") return "lex";
  if (value === 2 || value === "2" || value === "parse") return "parse";
  return "typecheck";
}

function severityFromCompiler(value: unknown): Severity {
  if (value === "warning") return "warning";
  if (value === "help") return "help";
  return "error";
}

function fallbackCodeForPhase(phase: Diagnostic["phase"], index: number): string {
  if (phase === "lex") return `E00${String(index).padStart(2, "0")}`;
  if (phase === "parse") return `E01${String(index).padStart(2, "0")}`;
  return `E02${String(index).padStart(2, "0")}`;
}

function numberOr(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function clampOffset(value: number, length: number): number {
  return Math.max(0, Math.min(Math.trunc(value), length));
}
