/**
 * Formatter tests for the playground engine (selfhost wasm).
 *
 * @module
 */

import { describe, it, before } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

import { configureTypecheckCompilerWasm, formatSource } from "../engine.js";

const FIB_SOURCE = `fn fib(n: i32) -> i32 {
if n <= 1 {
n
} else {
fib(n - 1) + fib(n - 2)
}
}

fn main() {
    let n = 10
    println(fib(n))
}
`;

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "../../..");

async function loadCompilerBytes(): Promise<Uint8Array> {
  const candidates = [
    resolve(repoRoot, ".build/selfhost/arukellt-s2-runtime.wasm"),
    resolve(repoRoot, ".build/selfhost/arukellt-s2.wasm"),
    resolve(repoRoot, ".build/selfhost/arukellt-s3.wasm"),
  ];
  for (const path of candidates) {
    try {
      return new Uint8Array(await readFile(path));
    } catch {
      continue;
    }
  }
  throw new Error("selfhost compiler wasm not found for format tests");
}

describe("formatSource", () => {
  before(async () => {
    configureTypecheckCompilerWasm(await loadCompilerBytes());
  });

  it("keeps else-branch bodies indented", () => {
    const result = formatSource(FIB_SOURCE);
    assert.equal(result.ok, true);
    assert.ok(result.formatted?.includes("    } else {\n        fib(n - 1) + fib(n - 2)"));
  });

  it("is idempotent for if/else blocks", () => {
    const first = formatSource(FIB_SOURCE);
    assert.equal(first.ok, true);
    const second = formatSource(first.formatted ?? "");
    assert.equal(second.ok, true);
    assert.equal(second.formatted, first.formatted);
  });
});
