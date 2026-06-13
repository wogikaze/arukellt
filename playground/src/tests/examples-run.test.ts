/**
 * Integration tests: playground examples compile, run, and match expected stdout.
 *
 * @module
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { EXAMPLES } from "../examples.js";
import { compileWithCompilerWasm } from "../compiler-host.js";
import { runT2Wasm } from "../t2-runner.js";
import { isRunnableT2Output } from "../compiler-client.js";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "../../..");

/** Expected stdout for each playground example source (T2 build/run). */
const EXPECTED_STDOUT: Readonly<Record<string, string>> = {
  "hello-world": "Hello, world!\n",
  variables: "52\n",
  functions: "7\n",
  structs: "1.0\n2.0\n",
  enums: "green\n",
  fibonacci: "55\n",
  traits: "Hello, Alice\n",
};

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

describe("Playground examples build and run", () => {
  it("each catalog example compiles to runnable T2 wasm with expected stdout", async () => {
    const compiler = await loadCompilerBytes();

    for (const example of EXAMPLES) {
      const expected = EXPECTED_STDOUT[example.id];
      assert.ok(
        expected !== undefined,
        `missing EXPECTED_STDOUT entry for "${example.id}"`,
      );

      const compile = await compileWithCompilerWasm(compiler, example.source, {
        timeoutMs: 120_000,
      });

      assert.equal(
        compile.ok,
        true,
        `${example.id}: compile failed: ${compile.error ?? compile.compilerStderr}`,
      );
      assert.ok(compile.wasmBytes, `${example.id}: missing wasm output`);
      assert.equal(
        isRunnableT2Output(compile.wasmBytes),
        true,
        `${example.id}: wasm does not import arukellt_io`,
      );

      const run = await runT2Wasm(compile.wasmBytes);
      assert.equal(run.ok, true, `${example.id}: run failed: ${run.trap}`);
      assert.equal(
        run.stdout,
        expected,
        `${example.id}: stdout mismatch`,
      );
    }
  });
});
