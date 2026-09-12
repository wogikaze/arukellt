import assert from "node:assert/strict";
import { test } from "node:test";

import { compileSource, configureTypecheckCompilerWasm } from "../engine.js";

test("compileSource fails clearly without a configured compiler", async () => {
  configureTypecheckCompilerWasm(null);
  const result = await compileSource("fn main() {}");
  assert.equal(result.ok, false);
  assert.match(result.error ?? "", /not been initialised/);
});
