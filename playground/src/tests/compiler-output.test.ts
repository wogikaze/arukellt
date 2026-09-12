import assert from "node:assert/strict";
import { test } from "node:test";

import {
  buildStatusMessage,
  mergeConsoleSections,
  sectionsFromCompileResult,
} from "../compiler-output.js";

test("sectionsFromCompileResult maps compiler output", () => {
  const sections = sectionsFromCompileResult({
    ok: true,
    exitCode: 0,
    compilerStdout: "ok",
    compilerStderr: "",
    wasmBytes: new Uint8Array([0, 97, 115, 109]),
    outputSize: 4,
    elapsedMs: 40,
    error: null,
  });
  assert.deepEqual(sections, [{ title: "Compiler stdout", body: "ok" }]);
  assert.equal(
    buildStatusMessage({
      ok: true,
      exitCode: 0,
      compilerStdout: "",
      compilerStderr: "",
      wasmBytes: new Uint8Array([0]),
      outputSize: 1,
      elapsedMs: 40,
      error: null,
    }),
    "Build succeeded (1 bytes, 40 ms). Download the core Wasm and package it with official WASI tooling to run it.",
  );
});

test("mergeConsoleSections omits empty bodies", () => {
  const merged = mergeConsoleSections(
    [{ title: "A", body: "" }],
    [{ title: "B", body: "x" }],
  );
  assert.deepEqual(merged, [{ title: "B", body: "x" }]);
});
