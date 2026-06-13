import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

import {
  compileSource,
  configureTypecheckCompilerWasm,
  runSource,
  runWasm,
} from "../engine.js";
import { isRunnableT2Output } from "../compiler-client.js";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "../../..");

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

describe("engine compile/run API", () => {
  it("compileSource and runSource execute T2 stdio fixture", async () => {
    configureTypecheckCompilerWasm(await loadCompilerBytes());
    const source = [
      "fn main() {",
      '    print("engine-run")',
      "}",
      "",
    ].join("\n");

    const chained = await runSource(source, { timeoutMs: 120_000 });
    assert.equal(chained.compile.ok, true, chained.compile.error ?? chained.compile.compilerStderr);
    assert.ok(chained.compile.wasmBytes);
    assert.equal(isRunnableT2Output(chained.compile.wasmBytes), true);
    assert.ok(chained.run);
    assert.equal(chained.run!.ok, true, chained.run!.trap ?? "run failed");
    assert.equal(chained.run!.stdout, "engine-run");
  });

  it("runWasm executes precompiled fixture bytes", async () => {
    const wasmPath = resolve(repoRoot, ".build/t2-test/t2_stdio_s3.wasm");
    const wasmBytes = new Uint8Array(await readFile(wasmPath));
    const result = await runWasm(wasmBytes);
    assert.equal(result.ok, true);
    assert.equal(result.stdout, "hello");
  });

  it("compileSource fails gracefully without configured compiler bytes", async () => {
    configureTypecheckCompilerWasm(null);
    const result = await compileSource("fn main() {}");
    assert.equal(result.ok, false);
    assert.match(result.error ?? "", /not been initialised/);
  });
});
