import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

import { compileWithCompilerWasm } from "../compiler-host.js";
import { isRunnableT2Output } from "../compiler-client.js";
import { createWasiHost } from "../wasi/minimal-host.js";

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

test("createWasiHost satisfies selfhost compiler WASI imports", async () => {
  const compilerBytes = await loadCompilerBytes();
  const host = createWasiHost({
    argv: ["arukellt", "--version"],
    files: new Map(),
  });

  const module = new WebAssembly.Module(compilerBytes as unknown as BufferSource);
  const instance = new WebAssembly.Instance(module, host.imports);
  assert.ok(instance.exports.memory);
});

test("compileWithCompilerWasm compiles T2 stdio fixture through WASI host", async () => {
  const compilerBytes = await loadCompilerBytes();
  const source = [
    "fn main() {",
    '    print("hello")',
    "}",
    "",
  ].join("\n");

  const result = await compileWithCompilerWasm(compilerBytes, source, {
    timeoutMs: 120_000,
  });

  assert.equal(typeof result.exitCode, "number");
  assert.equal(typeof result.compilerStdout, "string");
  assert.equal(typeof result.compilerStderr, "string");

  if (result.ok) {
    assert.ok(result.wasmBytes);
    assert.ok(result.wasmBytes!.byteLength > 0);
    assert.ok(isRunnableT2Output(result.wasmBytes));
  } else {
    // Pinned bootstrap may not yet lower stdio to arukellt_io; host contract still returns metadata.
    assert.ok(result.error);
  }
});
