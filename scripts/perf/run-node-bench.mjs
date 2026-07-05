#!/usr/bin/env node
// scripts/perf/run-node-bench.mjs — Node.js runtime benchmark adapter.
//
// Loads a wasm file, instantiates it with WASI shims, runs warmups + timed
// iterations, and outputs a JSON result on stdout.
//
// Usage:
//   node run-node-bench.mjs <wasm-path> [--expected <text>] \
//       [--iterations N] [--warmups N] [--target p1|p2]
//
// Output JSON:
//   {
//     "runtime": "node", "target": "p1|p2", "status": "ok|error",
//     "correctness": "pass|fail|n/a", "expected": "...", "actual": "...",
//     "timings_ms": [...], "median_ms": ..., "p50_ms": ..., "p95_ms": ...,
//     "min_ms": ..., "max_ms": ..., "iterations": N, "warmups": N,
//     "error": null | "message"
//   }

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const { instantiateWithWasi } = await import(join(__dirname, 'wasi-shim.mjs'));

function parseArgs(argv) {
  const args = { wasmPath: null, expected: null, iterations: 10, warmups: 2, target: null };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--expected') { args.expected = argv[++i]; }
    else if (a === '--iterations') { args.iterations = parseInt(argv[++i], 10); }
    else if (a === '--warmups') { args.warmups = parseInt(argv[++i], 10); }
    else if (a === '--target') { args.target = argv[++i]; }
    else if (!a.startsWith('--')) { args.wasmPath = a; }
  }
  return args;
}

function median(sorted) {
  const n = sorted.length;
  if (n === 0) return null;
  if (n % 2 === 1) return sorted[Math.floor(n / 2)];
  return (sorted[n / 2 - 1] + sorted[n / 2]) / 2;
}

function percentile(sorted, p) {
  if (sorted.length === 0) return null;
  const idx = Math.min(sorted.length - 1, Math.floor((p / 100) * sorted.length));
  return sorted[idx];
}

function result(obj) {
  console.log(JSON.stringify(obj));
}

async function main() {
  const args = parseArgs(process.argv);
  if (!args.wasmPath) {
    result({ runtime: 'node', status: 'error', error: 'no wasm path' });
    process.exit(1);
  }

  let bytes;
  try {
    bytes = readFileSync(args.wasmPath);
  } catch (e) {
    result({ runtime: 'node', target: args.target, status: 'error', error: `read: ${e.message}` });
    process.exit(1);
  }

  let instance, shims, flavor;
  try {
    ({ instance, shims, flavor } = await instantiateWithWasi(bytes, args.target));
  } catch (e) {
    result({ runtime: 'node', target: args.target, status: 'error', error: `instantiate: ${e.message}` });
    process.exit(1);
  }

  const start = instance.exports._start;
  if (typeof start !== 'function') {
    result({ runtime: 'node', target: flavor, status: 'error', error: 'no _start export' });
    process.exit(1);
  }

  // Warmups
  for (let i = 0; i < args.warmups; i++) {
    shims.resetStdout();
    try { start(); } catch (e) { /* ignore warmup errors */ }
  }

  // Timed iterations
  const timings = [];
  let lastError = null;
  let runSucceeded = false;
  for (let i = 0; i < args.iterations; i++) {
    shims.resetStdout();
    const t0 = performance.now();
    try {
      start();
      runSucceeded = true;
    } catch (e) {
      lastError = e.message;
    }
    const t1 = performance.now();
    timings.push(t1 - t0);
  }

  if (!runSucceeded) {
    result({ runtime: 'node', target: flavor, status: 'error', error: lastError || 'all runs failed', iterations: args.iterations, warmups: args.warmups });
    process.exit(1);
  }

  timings.sort((a, b) => a - b);
  const actual = shims.getStdout().trimEnd();
  const expected = args.expected ? args.expected.trimEnd() : null;
  const correctness = expected !== null ? (actual === expected ? 'pass' : 'fail') : 'n/a';

  result({
    runtime: 'node',
    target: flavor,
    status: 'ok',
    correctness,
    expected: expected,
    actual: actual,
    timings_ms: timings,
    median_ms: median(timings),
    p50_ms: percentile(timings, 50),
    p95_ms: percentile(timings, 95),
    min_ms: timings[0],
    max_ms: timings[timings.length - 1],
    iterations: args.iterations,
    warmups: args.warmups,
    error: null,
  });
}

main().catch((e) => {
  result({ runtime: 'node', status: 'error', error: e.message });
  process.exit(1);
});
