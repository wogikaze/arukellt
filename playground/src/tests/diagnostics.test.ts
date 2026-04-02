/**
 * Tests for the diagnostics module.
 *
 * Tests the pure utility functions `offsetToLineCol` and
 * `buildDiagnosticOverlay` which do not require a browser environment.
 *
 * @module
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";

import { offsetToLineCol, buildDiagnosticOverlay } from "../diagnostics.js";
import type { Diagnostic } from "../types.js";

// ---------------------------------------------------------------------------
// Helper: create a minimal Diagnostic for testing
// ---------------------------------------------------------------------------

function makeDiag(
  overrides: Partial<Diagnostic> & { labels?: Diagnostic["labels"] } = {},
): Diagnostic {
  return {
    code: "E0001",
    severity: "error",
    phase: "parse",
    message: "unexpected token",
    labels: [],
    notes: [],
    suggestion: null,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// offsetToLineCol
// ---------------------------------------------------------------------------

describe("offsetToLineCol", () => {
  it("returns 1:1 for offset 0", () => {
    const result = offsetToLineCol("hello", 0);
    assert.deepStrictEqual(result, { line: 1, col: 1 });
  });

  it("returns correct position within a single line", () => {
    const result = offsetToLineCol("fn main()", 3);
    assert.deepStrictEqual(result, { line: 1, col: 4 });
  });

  it("handles offset at end of single line", () => {
    const result = offsetToLineCol("hello", 5);
    assert.deepStrictEqual(result, { line: 1, col: 6 });
  });

  it("handles newlines correctly", () => {
    const source = "line1\nline2\nline3";
    // Offset 6 is the 'l' in "line2"
    assert.deepStrictEqual(offsetToLineCol(source, 6), { line: 2, col: 1 });
    // Offset 8 is the 'n' in "line2"
    assert.deepStrictEqual(offsetToLineCol(source, 8), { line: 2, col: 3 });
    // Offset 12 is the 'l' in "line3"
    assert.deepStrictEqual(offsetToLineCol(source, 12), { line: 3, col: 1 });
  });

  it("handles offset at a newline character", () => {
    const source = "ab\ncd";
    // Offset 2 is the '\n' itself
    assert.deepStrictEqual(offsetToLineCol(source, 2), { line: 1, col: 3 });
    // Offset 3 is 'c' on line 2
    assert.deepStrictEqual(offsetToLineCol(source, 3), { line: 2, col: 1 });
  });

  it("clamps offset beyond source length", () => {
    const result = offsetToLineCol("hi", 100);
    assert.deepStrictEqual(result, { line: 1, col: 3 });
  });

  it("clamps negative offset to 0", () => {
    const result = offsetToLineCol("hi", -5);
    assert.deepStrictEqual(result, { line: 1, col: 1 });
  });

  it("handles empty source", () => {
    const result = offsetToLineCol("", 0);
    assert.deepStrictEqual(result, { line: 1, col: 1 });
  });

  it("handles consecutive newlines", () => {
    const source = "a\n\nb";
    assert.deepStrictEqual(offsetToLineCol(source, 2), { line: 2, col: 1 });
    assert.deepStrictEqual(offsetToLineCol(source, 3), { line: 3, col: 1 });
  });

  it("handles multi-line real code", () => {
    const source = "fn main() {\n    let x = 42\n}";
    // 'l' in 'let' is at offset 16
    assert.deepStrictEqual(offsetToLineCol(source, 16), { line: 2, col: 5 });
    // '}' is at offset 27
    assert.deepStrictEqual(offsetToLineCol(source, 27), { line: 3, col: 1 });
  });
});

// ---------------------------------------------------------------------------
// buildDiagnosticOverlay
// ---------------------------------------------------------------------------

describe("buildDiagnosticOverlay", () => {
  it("returns empty string for no diagnostics", () => {
    const result = buildDiagnosticOverlay("fn main() {}", []);
    assert.equal(result, "");
  });

  it("returns empty string for empty source", () => {
    const diag = makeDiag({
      labels: [{ file_id: 0, start: 0, end: 1, message: "here" }],
    });
    const result = buildDiagnosticOverlay("", [diag]);
    assert.equal(result, "");
  });

  it("returns empty string for diagnostics without labels", () => {
    const diag = makeDiag({ labels: [] });
    const result = buildDiagnosticOverlay("fn main()", [diag]);
    assert.equal(result, "");
  });

  it("wraps error range in a marker span", () => {
    const source = "fn 123";
    const diag = makeDiag({
      severity: "error",
      labels: [{ file_id: 0, start: 3, end: 6, message: "unexpected" }],
    });
    const result = buildDiagnosticOverlay(source, [diag]);
    assert.ok(
      result.includes('<span class="ark-diag-marker-error">123</span>'),
      `Expected error marker span, got: ${result}`,
    );
    // Text before the marker should be plain
    assert.ok(
      result.startsWith("fn "),
      `Expected plain text before marker, got: ${result}`,
    );
  });

  it("wraps warning range in a warning marker span", () => {
    const source = "let _x = 42";
    const diag = makeDiag({
      severity: "warning",
      labels: [{ file_id: 0, start: 4, end: 6, message: "unused" }],
    });
    const result = buildDiagnosticOverlay(source, [diag]);
    assert.ok(
      result.includes('<span class="ark-diag-marker-warning">_x</span>'),
      `Expected warning marker span, got: ${result}`,
    );
  });

  it("wraps help range in a help marker span", () => {
    const source = "let x = 42";
    const diag = makeDiag({
      severity: "help",
      labels: [{ file_id: 0, start: 4, end: 5, message: "rename" }],
    });
    const result = buildDiagnosticOverlay(source, [diag]);
    assert.ok(
      result.includes('<span class="ark-diag-marker-help">x</span>'),
      `Expected help marker span, got: ${result}`,
    );
  });

  it("handles multiple non-overlapping diagnostics", () => {
    const source = "fn bad() bad2";
    const diag1 = makeDiag({
      severity: "error",
      labels: [{ file_id: 0, start: 3, end: 6, message: "first" }],
    });
    const diag2 = makeDiag({
      severity: "warning",
      labels: [{ file_id: 0, start: 9, end: 13, message: "second" }],
    });
    const result = buildDiagnosticOverlay(source, [diag1, diag2]);
    assert.ok(
      result.includes('<span class="ark-diag-marker-error">bad</span>'),
    );
    assert.ok(
      result.includes('<span class="ark-diag-marker-warning">bad2</span>'),
    );
  });

  it("highest severity wins for overlapping ranges", () => {
    const source = "abcdef";
    // Warning covers bytes 1-5, error covers bytes 2-4
    const warnDiag = makeDiag({
      severity: "warning",
      labels: [{ file_id: 0, start: 1, end: 5, message: "warn" }],
    });
    const errorDiag = makeDiag({
      severity: "error",
      labels: [{ file_id: 0, start: 2, end: 4, message: "err" }],
    });
    const result = buildDiagnosticOverlay(source, [warnDiag, errorDiag]);
    // Byte 1: warning ('b'), Bytes 2-3: error ('cd'), Byte 4: warning ('e')
    assert.ok(
      result.includes('<span class="ark-diag-marker-warning">b</span>'),
      `Expected warning marker for 'b', got: ${result}`,
    );
    assert.ok(
      result.includes('<span class="ark-diag-marker-error">cd</span>'),
      `Expected error marker for 'cd', got: ${result}`,
    );
    assert.ok(
      result.includes('<span class="ark-diag-marker-warning">e</span>'),
      `Expected warning marker for 'e', got: ${result}`,
    );
  });

  it("escapes HTML in source text", () => {
    const source = 'x = "<>"';
    const diag = makeDiag({
      severity: "error",
      labels: [{ file_id: 0, start: 4, end: 8, message: "bad" }],
    });
    const result = buildDiagnosticOverlay(source, [diag]);
    assert.ok(
      result.includes("&lt;&gt;"),
      "Expected HTML-escaped angle brackets",
    );
    assert.ok(!result.includes("<>"), "Raw angle brackets must not appear");
  });

  it("clamps label ranges to source bounds", () => {
    const source = "abc";
    const diag = makeDiag({
      severity: "error",
      labels: [{ file_id: 0, start: 1, end: 100, message: "out of range" }],
    });
    const result = buildDiagnosticOverlay(source, [diag]);
    assert.ok(
      result.includes('<span class="ark-diag-marker-error">bc</span>'),
      `Expected clamped range marker, got: ${result}`,
    );
  });

  it("appends trailing newline", () => {
    const source = "fn main()";
    const diag = makeDiag({
      severity: "error",
      labels: [{ file_id: 0, start: 3, end: 7, message: "here" }],
    });
    const result = buildDiagnosticOverlay(source, [diag]);
    assert.ok(result.endsWith("\n"), "Expected trailing newline");
  });

  it("handles diagnostic covering entire source", () => {
    const source = "bad";
    const diag = makeDiag({
      severity: "error",
      labels: [{ file_id: 0, start: 0, end: 3, message: "all bad" }],
    });
    const result = buildDiagnosticOverlay(source, [diag]);
    assert.ok(
      result.includes('<span class="ark-diag-marker-error">bad</span>'),
    );
  });

  it("skips labels with zero-length ranges", () => {
    const source = "abc";
    const diag = makeDiag({
      severity: "error",
      labels: [{ file_id: 0, start: 1, end: 1, message: "zero-width" }],
    });
    const result = buildDiagnosticOverlay(source, [diag]);
    assert.equal(result, "", "Zero-width label should produce no overlay");
  });

  it("handles multiple labels in a single diagnostic", () => {
    const source = "fn a() b()";
    const diag = makeDiag({
      severity: "error",
      labels: [
        { file_id: 0, start: 3, end: 4, message: "first" },
        { file_id: 0, start: 7, end: 8, message: "second" },
      ],
    });
    const result = buildDiagnosticOverlay(source, [diag]);
    assert.ok(
      result.includes('<span class="ark-diag-marker-error">a</span>'),
    );
    assert.ok(
      result.includes('<span class="ark-diag-marker-error">b</span>'),
    );
  });
});
