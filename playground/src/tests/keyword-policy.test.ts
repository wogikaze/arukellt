import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { tokenizeSource } from "../engine.js";
import { classifyTokenKind } from "../highlight.js";

describe("keyword policy", () => {
  it("keeps boolean literals lexical and distinct from identifiers", () => {
    const result = tokenizeSource("true false");
    assert.deepEqual(
      result.tokens.slice(0, 2).map((token) => token.kind),
      ["BoolLit(true)", "BoolLit(false)"],
    );
    assert.equal(result.ok, true);
    assert.equal(result.diagnostics.length, 0);
  });

  it("rejects future-reserved words before their feature is implemented", () => {
    const result = tokenizeSource("fn await() {}");
    assert.equal(result.tokens[1]?.kind, 'Reserved("await")');
    assert.equal(classifyTokenKind(result.tokens[1]?.kind ?? ""), "keyword");
    assert.equal(result.ok, false);
    assert.equal(result.diagnostics[0]?.code, "ELEX004");
  });

  it("keeps contextual words available to ordinary identifier syntax", () => {
    const result = tokenizeSource("type mod");
    assert.deepEqual(
      result.tokens.slice(0, 2).map((token) => token.kind),
      ['Ident("type")', 'Ident("mod")'],
    );
    assert.equal(result.ok, true);
  });
});
