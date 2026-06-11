import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

import { runT2Wasm, moduleImportsArukelltIo } from "../t2-runner.js";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "../../..");

test("runT2Wasm executes compiled T2 stdio fixture and captures stdout", async () => {
  const wasmPath = resolve(repoRoot, ".build/t2-test/t2_stdio_s3.wasm");
  const wasmBytes = new Uint8Array(await readFile(wasmPath));

  assert.ok(moduleImportsArukelltIo(wasmBytes));

  const result = await runT2Wasm(wasmBytes);
  assert.equal(result.ok, true);
  assert.equal(result.stdout, "hello");
  assert.equal(result.stderr, "");
  assert.equal(result.exitCode, 0);
  assert.equal(result.trap, null);
});
