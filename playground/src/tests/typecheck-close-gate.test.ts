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

import { parseSource, typecheckSource } from "../engine.js";

/** Syntax-valid source with a type error (bool assigned to i32). */
const TYPE_ERROR_SOURCE = `fn main() {
    let x: i32 = true
}
`;

describe("typecheck close-gate (#472 / #500)", () => {
  it("reports type errors that parse cleanly", () => {
    const parsed = parseSource(TYPE_ERROR_SOURCE);
    assert.equal(parsed.ok, true, "fixture must parse successfully");

    const checked = typecheckSource(TYPE_ERROR_SOURCE);
    assert.equal(checked.ok, false, "typecheck must fail on type error");
    assert.ok(checked.error_count > 0, "typecheck must emit diagnostics");

    const hasTypePhase = checked.diagnostics.some(
      (diag) =>
        diag.phase === "typecheck" ||
        diag.phase === "type" ||
        diag.code.startsWith("E02"),
    );
    assert.ok(
      hasTypePhase,
      "diagnostics must include typecheck phase or E02* code, not parse-only output",
    );
  });
});
