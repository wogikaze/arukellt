import assert from "node:assert/strict";
import { test } from "node:test";

import { stdinBytesToLineChunks } from "../t2-runner.js";

const enc = new TextEncoder();

function decodeChunks(chunks: Uint8Array[]): string[] {
  const dec = new TextDecoder();
  return chunks.map((chunk) => dec.decode(chunk));
}

test("stdinBytesToLineChunks splits newline-delimited REPL input", () => {
  assert.deepEqual(decodeChunks(stdinBytesToLineChunks(enc.encode("1 2 +\n"))), [
    "1 2 +\n",
    "\n",
  ]);
  assert.deepEqual(decodeChunks(stdinBytesToLineChunks(enc.encode("1 2 +\n\n"))), [
    "1 2 +\n",
    "\n",
    "\n",
  ]);
  assert.deepEqual(decodeChunks(stdinBytesToLineChunks(enc.encode("1 2 +"))), ["1 2 +"]);
  assert.deepEqual(stdinBytesToLineChunks(new Uint8Array()), []);
});
