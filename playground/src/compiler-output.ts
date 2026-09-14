/**
 * Pure transforms from compiler results to playground output sections.
 *
 * This module contains no runtime adapter or program runner. The browser
 * playground displays compiler output and leaves official WASI Component
 * packaging and execution to the CLI toolchain.
 *
 * @module
 */

import type { CompileResult } from "./compiler-types.js";

/** A titled block of compiler output for the output panel. */
export interface ConsoleOutputSection {
  /** Section heading. */
  title: string;
  /** Raw text body. */
  body: string;
}

/** Build display sections from a compile response. */
export function sectionsFromCompileResult(
  result: CompileResult,
): ConsoleOutputSection[] {
  const sections: ConsoleOutputSection[] = [];
  if (result.compilerStdout) {
    sections.push({ title: "Compiler stdout", body: result.compilerStdout });
  }
  if (result.compilerStderr) {
    sections.push({ title: "Compiler stderr", body: result.compilerStderr });
  }
  if (!result.ok && result.error) {
    sections.push({ title: "Build error", body: result.error });
  }
  return sections;
}

/** Merge section lists, omitting empty bodies. */
export function mergeConsoleSections(
  ...groups: ConsoleOutputSection[][]
): ConsoleOutputSection[] {
  const merged: ConsoleOutputSection[] = [];
  for (const group of groups) {
    for (const section of group) {
      if (section.body.length > 0) {
        merged.push(section);
      }
    }
  }
  return merged;
}

/** User-facing status line after a build. */
export function buildStatusMessage(result: CompileResult): string {
  if (!result.ok || !result.wasmBytes) {
    return result.error ?? "Build failed.";
  }
  const timing = `${result.outputSize} bytes, ${Math.round(result.elapsedMs)} ms`;
  return `Build succeeded (${timing}). Download the core Wasm and package it with official WASI tooling to run it.`;
}
