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
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { createPlayground } from "../playground.js";
import { configureTypecheckCompilerWasm } from "../engine.js";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "../../..");

describe("createPlayground", () => {
  it("parses, tokenizes, formats, and type-checks source", async () => {
    configureTypecheckCompilerWasm(await loadCompilerBytes());
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
