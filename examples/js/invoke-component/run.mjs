#!/usr/bin/env node
/**
 * Invoke an Ark-exported component via wasmtime CLI from Node.js.
 *
 * Prerequisite: bash examples/ark/export-library/run.sh (or pass COMPONENT_WASM).
 */
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../..");
const component =
  process.env.COMPONENT_WASM ??
  join(root, ".build/examples/ark-export/calculator.component.wasm");

function wasmtime() {
  return process.env.WASMTIME_BIN ?? "wasmtime";
}

function invoke(expr) {
  const out = execFileSync(
    wasmtime(),
    ["run", "--wasm", "gc", "--wasm", "component-model", "--invoke", expr, component],
    { encoding: "utf8" },
  );
  return out.trim();
}

if (!existsSync(component)) {
  console.error(`missing ${component} — run: bash examples/ark/export-library/run.sh`);
  process.exit(1);
}

const cases = [
  ["add(3, 4)", "7"],
  ["mul(6, 7)", "42"],
];

let failed = 0;
for (const [expr, want] of cases) {
  const got = invoke(expr);
  if (got !== want) {
    console.error(`FAIL ${expr}: expected ${want}, got ${got}`);
    failed += 1;
  } else {
    console.log(`PASS ${expr} -> ${got}`);
  }
}

if (failed > 0) {
  process.exit(1);
}
console.log("PASS js/invoke-component");
