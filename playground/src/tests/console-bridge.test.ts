import assert from "node:assert/strict";
import { test } from "node:test";

import {
  buildStatusMessage,
  mergeConsoleSections,
  runStatusMessage,
  sectionsFromCompileResult,
  sectionsFromRunResult,
} from "../console-bridge.js";

test("sectionsFromRunResult maps ADR-017 stdout capture", () => {
  const sections = sectionsFromRunResult({
    ok: true,
    stdout: "hello\n",
    stderr: "",
    exitCode: 0,
    trap: null,
    elapsedMs: 12,
  });
  assert.deepEqual(sections, [{ title: "Program stdout", body: "hello\n" }]);
  assert.equal(runStatusMessage({
    ok: true,
    stdout: "",
    stderr: "",
    exitCode: 0,
    trap: null,
    elapsedMs: 12,
  }), "Run finished (exit 0, 12 ms).");
});

test("sectionsFromCompileResult and buildStatusMessage cover compiler metadata", () => {
  const compile = sectionsFromCompileResult({
    ok: true,
    exitCode: 0,
    compilerStdout: "ok",
    compilerStderr: "",
    wasmBytes: new Uint8Array([0, 97, 115, 109]),
    outputSize: 3,
    elapsedMs: 40,
    error: null,
  });
  assert.equal(compile.length, 1);
  assert.equal(buildStatusMessage({
    ok: true,
    exitCode: 0,
    compilerStdout: "",
    compilerStderr: "",
    wasmBytes: new Uint8Array([0]),
    outputSize: 1,
    elapsedMs: 40,
    error: null,
  }, true), "Build succeeded (1 bytes, 40 ms).");
});

test("mergeConsoleSections omits empty bodies", () => {
  const merged = mergeConsoleSections(
    [{ title: "A", body: "" }],
    [{ title: "B", body: "x" }],
  );
  assert.deepEqual(merged, [{ title: "B", body: "x" }]);
});
