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
import { EXAMPLES, getExample } from "../examples.js";
import { compileWithCompilerWasm } from "../compiler-host.js";
import { runT2Wasm } from "../t2-runner.js";
import { isRunnableT2Output } from "../compiler-client.js";
const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "../../..");
/** Expected stdout for each playground example source (T2 build/run). */
const EXPECTED_STDOUT = {
    "hello-world": "Hello, world!\n",
    variables: "52\n",
    functions: "7\n",
    structs: "1.0\n2.0\n",
    enums: "green\n",
    fibonacci: "55\n",
    traits: "Hello, Alice\n",
    "rpn-repl": "RPN REPL\nenter space-separated tokens (e.g. 3 4 +). Ctrl+Z to exit.\n\n> 3\nok: result = 3\n> bye\n",
};
async function loadCompilerBytes() {
    const candidates = [
        resolve(repoRoot, "docs/playground/assets/arukellt-selfhost.wasm"),
        resolve(repoRoot, ".bootstrap-build/arukellt-s3.wasm"),
        resolve(repoRoot, ".build/selfhost/arukellt-s3.wasm"),
        resolve(repoRoot, ".build/selfhost/arukellt-s2.wasm"),
        resolve(repoRoot, "bootstrap/arukellt-selfhost.wasm"),
    ];
    for (const path of candidates) {
        try {
            return new Uint8Array(await readFile(path));
        }
        catch {
            // try next candidate
        }
    }
    throw new Error("no compiler wasm candidate found");
}
function runOptionsFor(example) {
    if (example.stdin === undefined) {
        return {};
    }
    return {
        stdin: new TextEncoder().encode(example.stdin),
        stdinMode: example.stdinMode ?? "line",
    };
}
describe("Playground examples build and run", () => {
    it("rpn-repl reads line-delimited virtual stdin", async () => {
        const compiler = await loadCompilerBytes();
        const example = getExample("rpn-repl");
        assert.ok(example);
        const compile = await compileWithCompilerWasm(compiler, example.source, {
            timeoutMs: 120_000,
        });
        assert.equal(compile.ok, true, compile.error ?? compile.compilerStderr ?? undefined);
        assert.ok(compile.wasmBytes);
        const run = await runT2Wasm(compile.wasmBytes, runOptionsFor(example));
        assert.ok(run.ok, run.trap ?? "run failed");
        assert.equal(run.stdout, EXPECTED_STDOUT["rpn-repl"]);
    });
    it("each catalog example compiles to runnable T2 wasm with expected stdout", async () => {
        const compiler = await loadCompilerBytes();
        for (const example of EXAMPLES) {
            const expected = EXPECTED_STDOUT[example.id];
            assert.ok(expected !== undefined, `missing EXPECTED_STDOUT entry for "${example.id}"`);
            const compile = await compileWithCompilerWasm(compiler, example.source, {
                timeoutMs: 120_000,
            });
            assert.equal(compile.ok, true, `${example.id}: compile failed: ${compile.error ?? compile.compilerStderr}`);
            assert.ok(compile.wasmBytes, `${example.id}: missing wasm output`);
            assert.equal(isRunnableT2Output(compile.wasmBytes), true, `${example.id}: wasm does not import arukellt_io`);
            const run = await runT2Wasm(compile.wasmBytes, runOptionsFor(example));
            assert.equal(run.ok, true, `${example.id}: run failed: ${run.trap}`);
            assert.equal(run.stdout, expected, `${example.id}: stdout mismatch`);
        }
    });
});
//# sourceMappingURL=examples-run.test.js.map