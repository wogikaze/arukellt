/**
 * Integration tests for the browser-facing playground client.
 *
 * These tests run under Node.js and verify that the main-thread playground
 * exposes the browser-native engine response shapes.
 *
 * @module
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";

import { createPlayground } from "../playground.js";

describe("createPlayground", () => {
  it("parses, tokenizes, formats, and type-checks source", async () => {
    const pg = await createPlayground("/unused/playground-engine.js", {
      wasmUrl: new URL("file:///tmp/unused-playground-engine"),
    });

    const source = "pub fn main() {\n    let msg = \"hello\";\n}\n";
    const parsed = pg.parse(source);
    assert.equal(parsed.ok, true);
    assert.equal(parsed.error_count, 0);
    assert.equal(parsed.module?.items[0]?.kind, "fn");
    assert.equal(parsed.module?.items[0]?.name, "main");
    assert.equal(parsed.module?.items[0]?.is_pub, true);

    const tokenized = pg.tokenize(source);
    assert.equal(tokenized.ok, true);
    assert.ok(tokenized.tokens.some((token) => token.kind === "Fn"));
    assert.ok(tokenized.tokens.some((token) => token.kind.startsWith("StringLit")));

    const formatted = pg.format(source);
    assert.equal(formatted.ok, true);
    assert.ok(formatted.formatted?.endsWith("\n"));

    const checked = pg.typecheck(source);
    assert.equal(checked.ok, true);
    assert.equal(checked.error_count, 0);
    assert.match(pg.version(), /^selfhost-playground-ts-/);
  });
});
