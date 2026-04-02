/**
 * Tests for the syntax highlighting module.
 *
 * These tests verify token classification and HTML generation without
 * requiring the Wasm module — they use hand-crafted token data.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  classifyTokenKind,
  categoryClass,
  highlightTokens,
} from "../highlight.js";
import type { Token } from "../types.js";

// ---------------------------------------------------------------------------
// classifyTokenKind
// ---------------------------------------------------------------------------

describe("classifyTokenKind", () => {
  it("classifies unit-variant keywords", () => {
    const keywords = [
      "Fn", "Struct", "Enum", "Let", "Mut", "If", "Else", "Match",
      "While", "Loop", "For", "In", "Break", "Continue", "Return",
      "Pub", "Import", "As", "Trait", "Impl", "Use",
    ];
    for (const kw of keywords) {
      assert.equal(classifyTokenKind(kw), "keyword", `Expected ${kw} to be keyword`);
    }
  });

  it("classifies reserved keywords (tuple variant)", () => {
    assert.equal(classifyTokenKind('Reserved("async")'), "keyword");
    assert.equal(classifyTokenKind("Reserved"), "keyword");
  });

  it("classifies operators", () => {
    const ops = [
      "Plus", "Minus", "Star", "Slash", "Percent",
      "EqEq", "BangEq", "Lt", "LtEq", "Gt", "GtEq",
      "AmpAmp", "PipePipe", "Bang", "Amp", "Pipe",
      "Caret", "Tilde", "Shl", "Shr", "Eq", "Arrow", "FatArrow",
    ];
    for (const op of ops) {
      assert.equal(classifyTokenKind(op), "operator", `Expected ${op} to be operator`);
    }
  });

  it("classifies punctuation", () => {
    const puncts = [
      "LParen", "RParen", "LBrace", "RBrace", "LBracket", "RBracket",
      "Comma", "Semi", "Dot", "DotDot", "Question", "Colon", "ColonColon",
    ];
    for (const p of puncts) {
      assert.equal(classifyTokenKind(p), "punctuation", `Expected ${p} to be punctuation`);
    }
  });

  it("classifies numeric literals (tuple variants)", () => {
    assert.equal(classifyTokenKind("IntLit(42)"), "number");
    assert.equal(classifyTokenKind("FloatLit(3.14)"), "number");
    assert.equal(classifyTokenKind("TypedIntLit(42, \"u8\")"), "number");
    assert.equal(classifyTokenKind("TypedFloatLit(3.14, \"f32\")"), "number");
    // Also handle bare prefix (in case of simplified output).
    assert.equal(classifyTokenKind("IntLit"), "number");
    assert.equal(classifyTokenKind("FloatLit"), "number");
  });

  it("classifies string literals (tuple variants)", () => {
    assert.equal(classifyTokenKind('StringLit("hello")'), "string");
    assert.equal(classifyTokenKind("CharLit('a')"), "string");
    assert.equal(classifyTokenKind("FStringLit([Lit(\"hi\")])"), "string");
    assert.equal(classifyTokenKind("StringLit"), "string");
  });

  it("classifies boolean literals (tuple variants)", () => {
    assert.equal(classifyTokenKind("BoolLit(true)"), "boolean");
    assert.equal(classifyTokenKind("BoolLit(false)"), "boolean");
    assert.equal(classifyTokenKind("BoolLit"), "boolean");
  });

  it("classifies identifiers (tuple variants)", () => {
    assert.equal(classifyTokenKind('Ident("main")'), "identifier");
    assert.equal(classifyTokenKind("Ident"), "identifier");
  });

  it("classifies doc comments (tuple variants)", () => {
    assert.equal(classifyTokenKind('OuterDocComment("/// docs")'), "comment");
    assert.equal(classifyTokenKind('InnerDocComment("//! module docs")'), "comment");
    assert.equal(classifyTokenKind("OuterDocComment"), "comment");
  });

  it("classifies plain/structural tokens", () => {
    assert.equal(classifyTokenKind("Newline"), "plain");
    assert.equal(classifyTokenKind("Eof"), "plain");
    assert.equal(classifyTokenKind("Error"), "plain");
  });

  it("returns plain for unknown kinds", () => {
    assert.equal(classifyTokenKind("SomethingNew"), "plain");
    assert.equal(classifyTokenKind(""), "plain");
  });
});

// ---------------------------------------------------------------------------
// categoryClass
// ---------------------------------------------------------------------------

describe("categoryClass", () => {
  it("generates correct CSS class names", () => {
    assert.equal(categoryClass("keyword"), "ark-hl-keyword");
    assert.equal(categoryClass("string"), "ark-hl-string");
    assert.equal(categoryClass("number"), "ark-hl-number");
    assert.equal(categoryClass("comment"), "ark-hl-comment");
    assert.equal(categoryClass("operator"), "ark-hl-operator");
    assert.equal(categoryClass("punctuation"), "ark-hl-punctuation");
    assert.equal(categoryClass("identifier"), "ark-hl-identifier");
    assert.equal(categoryClass("boolean"), "ark-hl-boolean");
    assert.equal(categoryClass("plain"), "ark-hl-plain");
  });
});

// ---------------------------------------------------------------------------
// highlightTokens
// ---------------------------------------------------------------------------

describe("highlightTokens", () => {
  it("produces highlighted HTML for a simple function", () => {
    const source = "fn main() {}";
    const tokens: Token[] = [
      { kind: "Fn", text: "fn", start: 0, end: 2 },
      { kind: 'Ident("main")', text: "main", start: 3, end: 7 },
      { kind: "LParen", text: "(", start: 7, end: 8 },
      { kind: "RParen", text: ")", start: 8, end: 9 },
      { kind: "LBrace", text: "{", start: 10, end: 11 },
      { kind: "RBrace", text: "}", start: 11, end: 12 },
      { kind: "Eof", text: "", start: 12, end: 12 },
    ];

    const html = highlightTokens(source, tokens);

    assert.ok(html.includes('<span class="ark-hl-keyword">fn</span>'));
    assert.ok(html.includes('<span class="ark-hl-identifier">main</span>'));
    assert.ok(html.includes('<span class="ark-hl-punctuation">(</span>'));
    assert.ok(html.includes('<span class="ark-hl-punctuation">)</span>'));
    assert.ok(html.includes('<span class="ark-hl-punctuation">{</span>'));
    assert.ok(html.includes('<span class="ark-hl-punctuation">}</span>'));
  });

  it("preserves whitespace gaps between tokens", () => {
    const source = "let x = 42";
    const tokens: Token[] = [
      { kind: "Let", text: "let", start: 0, end: 3 },
      { kind: 'Ident("x")', text: "x", start: 4, end: 5 },
      { kind: "Eq", text: "=", start: 6, end: 7 },
      { kind: "IntLit(42)", text: "42", start: 8, end: 10 },
      { kind: "Eof", text: "", start: 10, end: 10 },
    ];

    const html = highlightTokens(source, tokens);

    // Gaps should be plain text (spaces).
    assert.ok(
      html.includes('</span> <span'),
      "Expected whitespace between tokens",
    );
    assert.ok(html.includes('<span class="ark-hl-number">42</span>'));
  });

  it("escapes HTML special characters in source text", () => {
    const source = 'let x = "<>&"';
    const tokens: Token[] = [
      { kind: "Let", text: "let", start: 0, end: 3 },
      { kind: 'Ident("x")', text: "x", start: 4, end: 5 },
      { kind: "Eq", text: "=", start: 6, end: 7 },
      { kind: 'StringLit("<>&")', text: '"<>&"', start: 8, end: 13 },
      { kind: "Eof", text: "", start: 13, end: 13 },
    ];

    const html = highlightTokens(source, tokens);

    assert.ok(html.includes("&lt;&gt;&amp;"), "HTML chars must be escaped");
    assert.ok(!html.includes("<>&"), "Raw HTML chars must not appear");
  });

  it("appends trailing newline if missing", () => {
    const source = "fn main()";
    const tokens: Token[] = [
      { kind: "Fn", text: "fn", start: 0, end: 2 },
      { kind: 'Ident("main")', text: "main", start: 3, end: 7 },
      { kind: "LParen", text: "(", start: 7, end: 8 },
      { kind: "RParen", text: ")", start: 8, end: 9 },
      { kind: "Eof", text: "", start: 9, end: 9 },
    ];

    const html = highlightTokens(source, tokens);
    assert.ok(html.endsWith("\n"), "Expected trailing newline");
  });

  it("handles empty source", () => {
    const html = highlightTokens("", []);
    assert.equal(html, "\n");
  });

  it("handles tokens with simplified kind names", () => {
    const source = "fn main";
    const tokens: Token[] = [
      { kind: "Fn", text: "fn", start: 0, end: 2 },
      { kind: "Ident", text: "main", start: 3, end: 7 },
      { kind: "Eof", text: "", start: 7, end: 7 },
    ];

    const html = highlightTokens(source, tokens);
    assert.ok(html.includes('<span class="ark-hl-keyword">fn</span>'));
    assert.ok(html.includes('<span class="ark-hl-identifier">main</span>'));
  });
});
