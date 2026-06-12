/**
 * Close-gate contract for issues #472 / #500.
 *
 * Playground typecheck must report type-phase diagnostics for sources that parse
 * cleanly but fail the type checker. Parse-only delegation is a false-done pattern.
 *
 * @module
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { parseSource, typecheckSourceWithCompilerBytesSync } from "../engine.js";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "../../..");

/** Syntax-valid source with a type error (bool assigned to i32). */
const TYPE_ERROR_SOURCE = `fn main() {
    let x: i32 = true
}
`;

describe("typecheck close-gate (#472 / #500)", () => {
  it("reports type errors that parse cleanly", async () => {
    const compilerBytes = await loadCompilerBytes();
    const parsed = parseSource(TYPE_ERROR_SOURCE);
    assert.equal(parsed.ok, true, "fixture must parse successfully");

    const checked = typecheckSourceWithCompilerBytesSync(TYPE_ERROR_SOURCE, compilerBytes, {
      timeoutMs: 120_000,
    });
    assert.equal(checked.ok, false, "typecheck must fail on type error");
    assert.ok(checked.error_count > 0, "typecheck must emit diagnostics");

    const hasTypePhase = checked.diagnostics.some(
      (diag) =>
        diag.phase === "typecheck" ||
        diag.code.startsWith("E02"),
    );
    assert.ok(
      hasTypePhase,
      "diagnostics must include typecheck phase or E02* code, not parse-only output",
    );
  });
});

async function loadCompilerBytes(): Promise<Uint8Array> {
  const candidates = [
    resolve(repoRoot, ".build/selfhost/arukellt-s3.wasm"),
    resolve(repoRoot, ".build/selfhost/arukellt-s2.wasm"),
    resolve(repoRoot, "bootstrap/arukellt-selfhost.wasm"),
  ];
  for (const path of candidates) {
    try {
      return new Uint8Array(await readFile(path));
    } catch {
      // try next candidate
    }
  }
  throw new Error("no compiler wasm candidate found");
}
