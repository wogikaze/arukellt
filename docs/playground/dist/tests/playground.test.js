/**
 * Integration tests for the browser-facing playground client.
 *
 * These tests run under Node.js and use a temporary ES module to verify
 * that the main-thread playground parse path invokes both the parser and
 * checker exports from the loaded Wasm module.
 *
 * @module
 */
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { createPlayground } from "../playground.js";
function makeResponse(ok) {
    return JSON.stringify({
        ok,
        module: null,
        diagnostics: [],
        error_count: 0,
    });
}
describe("createPlayground parse path", () => {
    it("invokes the checker surface when parsing source", async () => {
        const tempDir = await mkdtemp(path.join(os.tmpdir(), "arukellt-playground-"));
        const modulePath = path.join(tempDir, "mock-wasm-module.js");
        await writeFile(modulePath, [
            "let parseCalls = 0;",
            "let typecheckCalls = 0;",
            "",
            "export function getCallCounts() {",
            "  return { parseCalls, typecheckCalls };",
            "}",
            "",
            "export default async function init() {",
            "  return undefined;",
            "}",
            "",
            "export function parse(_source) {",
            "  parseCalls += 1;",
            `  return ${JSON.stringify(makeResponse(true))};`,
            "}",
            "",
            "export function format() {",
            "  return JSON.stringify({ ok: true, formatted: \"\" });",
            "}",
            "",
            "export function tokenize() {",
            "  return JSON.stringify({ ok: true, tokens: [], diagnostics: [] });",
            "}",
            "",
            "export function typecheck(_source) {",
            "  typecheckCalls += 1;",
            `  return ${JSON.stringify(makeResponse(true))};`,
            "}",
            "",
            "export function version() {",
            "  return \"mock-version\";",
            "}",
            "",
        ].join("\n"));
        try {
            const wasmModuleUrl = pathToFileURL(modulePath).href;
            const pg = await createPlayground(wasmModuleUrl, {
                wasmUrl: new URL("file:///tmp/mock-playground.wasm"),
            });
            const result = pg.parse("fn main() {}");
            assert.equal(result.ok, true);
            const mod = await import(wasmModuleUrl);
            const counts = mod.getCallCounts();
            assert.equal(counts.parseCalls, 1);
            assert.equal(counts.typecheckCalls, 1);
        }
        finally {
            await rm(tempDir, { recursive: true, force: true });
        }
    });
});
//# sourceMappingURL=playground.test.js.map