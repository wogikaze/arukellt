/**
 * ADR-020 console bridge — pure transforms from compile/run results to UI sections.
 *
 * Maps `arukellt_io` host capture (`RunResult.stdout` / `stderr`) to display sections.
 * This module has no DOM dependency; {@link createRunOutputPanel} renders sections.
 *
 * @module
 */

import type { CompileResult, RunResult } from "./compiler-types.js";

/** A titled block of program or compiler output for the stdout panel. */
export interface ConsoleOutputSection {
  /** Section heading (e.g. "Program stdout"). */
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

/** Build display sections from a T2 run response (ADR-020 `arukellt_io` capture). */
export function sectionsFromRunResult(result: RunResult): ConsoleOutputSection[] {
  const sections: ConsoleOutputSection[] = [];
  if (result.stdout) {
    sections.push({ title: "Program stdout", body: result.stdout });
  }
  if (result.stderr) {
    sections.push({ title: "Program stderr", body: result.stderr });
  }
  if (result.trap) {
    sections.push({ title: "Trap / runtime error", body: result.trap });
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

/** User-facing status line after a successful build. */
export function buildStatusMessage(result: CompileResult, runnable: boolean): string {
  if (!result.ok || !result.wasmBytes) {
    return result.error ?? "Build failed.";
  }
  const timing = `${result.outputSize} bytes, ${Math.round(result.elapsedMs)} ms`;
  if (runnable) {
    return `Build succeeded (${timing}).`;
  }
  return `Build succeeded (${timing}), but Run is unavailable until the compiler emits arukellt_io imports.`;
}

/** User-facing status line after a run. */
export function runStatusMessage(result: RunResult): string {
  if (result.ok) {
    return `Run finished (exit ${result.exitCode}, ${Math.round(result.elapsedMs)} ms).`;
  }
  return "Run failed.";
}
