#!/usr/bin/env node
// scripts/perf/run-browser-bench.mjs — Browser runtime benchmark adapter.
//
// Launches headless Chrome via puppeteer-core, loads a wasm module in the
// browser's WebAssembly engine (V8), and measures execution time.
//
// Usage:
//   node run-browser-bench.mjs <wasm-path> [--expected <text>] \
//       [--iterations N] [--warmups N] [--target p1|p2] [--chrome <path>]
//
// Output: same JSON schema as run-node-bench.mjs (with "runtime": "browser").

import { readFileSync } from 'node:fs';
import { existsSync } from 'node:fs';

function parseArgs(argv) {
  const args = { wasmPath: null, expected: null, iterations: 10, warmups: 2, target: null, chrome: null };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--expected') { args.expected = argv[++i]; }
    else if (a === '--iterations') { args.iterations = parseInt(argv[++i], 10); }
    else if (a === '--warmups') { args.warmups = parseInt(argv[++i], 10); }
    else if (a === '--target') { args.target = argv[++i]; }
    else if (a === '--chrome') { args.chrome = argv[++i]; }
    else if (!a.startsWith('--')) { args.wasmPath = a; }
  }
  return args;
}

function result(obj) {
  console.log(JSON.stringify(obj));
}

function findChrome() {
  const candidates = [
    process.env.CHROME_PATH,
    '/usr/bin/google-chrome',
    '/usr/bin/google-chrome-stable',
    '/usr/bin/chromium',
    '/usr/bin/chromium-browser',
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
  ];
  for (const c of candidates) {
    if (c && existsSync(c)) return c;
  }
  return null;
}

// The browser-side benchmark function. Runs in the page context.
// Receives base64-encoded wasm bytes and parameters.
async function browserBench(wasmBase64, params) {
  const bytes = Uint8Array.from(atob(wasmBase64), (c) => c.charCodeAt(0));
  const mod = new WebAssembly.Module(bytes);
  const importDescs = WebAssembly.Module.imports(mod);

  // Detect WASI flavor
  const modules = new Set(importDescs.map((imp) => imp.module));
  let flavor = params.targetFlavor;
  if (!flavor) {
    if (modules.has('wasi_snapshot_preview1')) flavor = 'p1';
    else if ([...modules].some((m) => m.startsWith('wasi:'))) flavor = 'p2';
    else flavor = 'unknown';
  }

  // Build WASI shims — memory is patched after instantiation
  let memRef = { buffer: new ArrayBuffer(0) };
  let stdout = '';
  let stderr = '';

  function decodeUtf8(ptr, len) {
    return new TextDecoder().decode(new Uint8Array(memRef.buffer, ptr, len));
  }

  let imports;
  if (flavor === 'p1') {
    imports = {
      wasi_snapshot_preview1: {
        fd_write: (fd, iovs, iovsLen, nwrittenPtr) => {
          for (let i = 0; i < iovsLen; i++) {
            const iov = new Uint32Array(memRef.buffer, iovs + i * 8, 2);
            const text = decodeUtf8(iov[0], iov[1]);
            if (fd === 1) stdout += text; else if (fd === 2) stderr += text;
          }
          new Uint32Array(memRef.buffer, nwrittenPtr, 1)[0] = 0;
          return 0;
        },
        fd_read: (fd, iovs, iovsLen, nreadPtr) => { new Uint32Array(memRef.buffer, nreadPtr, 1)[0] = 0; return 0; },
        fd_close: () => 0,
        args_sizes_get: (a, b) => { new Uint32Array(memRef.buffer, a, 1)[0] = 0; new Uint32Array(memRef.buffer, b, 1)[0] = 0; return 0; },
        args_get: () => 0,
        path_open: () => -1,
        proc_exit: (code) => { throw new Error(`proc_exit(${code})`); },
        clock_time_get: (id, prec, out) => { new BigUint64Array(memRef.buffer, out, 1)[0] = BigInt(Date.now()) * 1000000n; return 0; },
        random_get: (buf, len) => { const v = new Uint8Array(memRef.buffer, buf, len); for (let i = 0; i < len; i++) v[i] = Math.floor(Math.random() * 256); return 0; },
      },
    };
  } else {
    imports = {};
    for (const imp of importDescs) {
      if (!imports[imp.module]) imports[imp.module] = {};
      // Provide no-op stubs for all P2 imports; stdout is captured below
      imports[imp.module][imp.name] = (...args) => {
        // stdout write: fd, ptr, len → return len
        if (imp.module.startsWith('wasi:cli/std') && imp.name === 'write') {
          const text = decodeUtf8(args[1], args[2]);
          if (imp.module.includes('stdout')) stdout += text;
          else stderr += text;
          return args[2];
        }
        if (imp.module.includes('exit')) throw new Error(`exit(${args[0]})`);
        if (imp.module.includes('monotonic') && imp.name === 'now') return BigInt(Date.now()) * 1000000n;
        if (imp.module.includes('wall-clock') && imp.name === 'now') return [BigInt(Date.now()) * 1000000n, 0n];
        if (imp.module.includes('random')) return BigInt(Math.floor(Math.random() * 1e15));
        if (imp.module.includes('args-sizes')) return [0, 0];
        return 0;
      };
    }
  }

  const instance = new WebAssembly.Instance(mod, imports);
  if (instance.exports.memory) {
    Object.defineProperty(memRef, 'buffer', {
      get: () => instance.exports.memory.buffer,
      configurable: true,
    });
  }

  const start = instance.exports._start;
  if (typeof start !== 'function') {
    return { runtime: 'browser', target: flavor, status: 'error', error: 'no _start export' };
  }

  // Warmups
  for (let i = 0; i < params.warmups; i++) {
    stdout = ''; stderr = '';
    try { start(); } catch (e) { /* ignore */ }
  }

  // Timed iterations
  const timings = [];
  let lastError = null;
  let runSucceeded = false;
  for (let i = 0; i < params.iterations; i++) {
    stdout = ''; stderr = '';
    const t0 = performance.now();
    try { start(); runSucceeded = true; } catch (e) { lastError = e.message; }
    const t1 = performance.now();
    timings.push(t1 - t0);
  }

  if (!runSucceeded) {
    return { runtime: 'browser', target: flavor, status: 'error', error: lastError || 'all runs failed', iterations: params.iterations, warmups: params.warmups };
  }

  timings.sort((a, b) => a - b);
  const median = timings[Math.floor(timings.length / 2)];
  const p50 = timings[Math.min(timings.length - 1, Math.floor(0.5 * timings.length))];
  const p95 = timings[Math.min(timings.length - 1, Math.floor(0.95 * timings.length))];
  const actual = stdout.trimEnd();
  const expected = params.expected ? params.expected.trimEnd() : null;
  const correctness = expected !== null ? (actual === expected ? 'pass' : 'fail') : 'n/a';

  return {
    runtime: 'browser', target: flavor, status: 'ok', correctness,
    expected, actual, timings_ms: timings,
    median_ms: median, p50_ms: p50, p95_ms: p95,
    min_ms: timings[0], max_ms: timings[timings.length - 1],
    iterations: params.iterations, warmups: params.warmups, error: null,
  };
}

async function main() {
  const args = parseArgs(process.argv);
  if (!args.wasmPath) {
    result({ runtime: 'browser', status: 'error', error: 'no wasm path' });
    process.exit(1);
  }

  const chromePath = args.chrome || findChrome();
  if (!chromePath) {
    result({ runtime: 'browser', status: 'error', error: 'Chrome not found. Use --chrome <path> or set CHROME_PATH.' });
    process.exit(1);
  }

  let bytes;
  try {
    bytes = readFileSync(args.wasmPath);
  } catch (e) {
    result({ runtime: 'browser', target: args.target, status: 'error', error: `read: ${e.message}` });
    process.exit(1);
  }
  const wasmBase64 = Buffer.from(bytes).toString('base64');

  const puppeteer = await import('puppeteer-core');
  let browser;
  try {
    browser = await puppeteer.default.launch({
      executablePath: chromePath,
      headless: true,
      args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage', '--enable-features=WebAssembly'],
    });
  } catch (e) {
    result({ runtime: 'browser', target: args.target, status: 'error', error: `launch: ${e.message}` });
    process.exit(1);
  }

  try {
    const page = await browser.newPage();
    await page.goto('about:blank');
    const benchResult = await page.evaluate(browserBench, wasmBase64, {
      expected: args.expected,
      iterations: args.iterations,
      warmups: args.warmups,
      targetFlavor: args.target,
    });
    result(benchResult);
  } catch (e) {
    result({ runtime: 'browser', target: args.target, status: 'error', error: e.message });
  } finally {
    await browser.close();
  }
}

main().catch((e) => {
  result({ runtime: 'browser', status: 'error', error: e.message });
  process.exit(1);
});
