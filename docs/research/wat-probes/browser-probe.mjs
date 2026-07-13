#!/usr/bin/env node
/**
 * Run a core Wasm probe inside headless Chrome (puppeteer).
 *
 * Usage:
 *   node browser-probe.mjs <wasm-path> <expected> [--chrome <path>]
 *
 * expected: same tokens as run-probes.py NODE_HARNESS
 * Exit 0 on success.
 */
import { readFileSync, existsSync } from "node:fs";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

def findPuppeteer() {
  const candidates = [
    join(__dirname, "../../../scripts/dev/wat-probe-browser/node_modules/puppeteer"),
    join(__dirname, "../../../scripts/perf/node_modules/puppeteer-core"),
  ];
  for (const c of candidates) {
    try {
      return require(c);
    } catch {
      /* try next */
    }
  }
  throw new Error(
    "puppeteer not found; run: (cd scripts/dev/wat-probe-browser && npm i)",
  );
}

function parseArgs(argv) {
  const args = { wasm: null, expected: "42", chrome: process.env.CHROME_PATH || null };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--chrome") args.chrome = argv[++i];
    else if (!args.wasm) args.wasm = a;
    else args.expected = a;
  }
  return args;
}

async function main() {
  const args = parseArgs(process.argv);
  if (!args.wasm || !existsSync(args.wasm)) {
    console.error("usage: browser-probe.mjs <wasm> <expected>");
    process.exit(2);
  }
  const puppeteer = findPuppeteer();
  const chromePath =
    args.chrome ||
    (typeof puppeteer.executablePath === "function" ? puppeteer.executablePath() : null);
  if (!chromePath || !existsSync(chromePath)) {
    console.error(JSON.stringify({ ok: false, error: "chrome missing", chromePath }));
    process.exit(1);
  }

  const bytes = [...readFileSync(args.wasm)];
  const expected = args.expected;
  const browser = await puppeteer.launch({
    executablePath: chromePath,
    headless: true,
    args: ["--no-sandbox", "--disable-gpu", "--disable-dev-shm-usage"],
  });
  try {
    const page = await browser.newPage();
    const result = await page.evaluate(
      async (arr, expected) => {
        const bytes = new Uint8Array(arr);
        const out = { validate: false };
        try {
          out.validate = WebAssembly.validate(bytes);
        } catch (e) {
          return { ok: false, error: "validate-throw: " + e };
        }
        if (expected === "js-string") {
          try {
            const ok = WebAssembly.validate(bytes, { builtins: ["js-string"] });
            return { ok, validate: out.validate, builtins: ok };
          } catch (e) {
            return { ok: false, validate: out.validate, builtins: false, error: String(e) };
          }
        }
        if (expected === "js-bigint") {
          try {
            const { instance } = await WebAssembly.instantiate(bytes);
            const r = instance.exports.test(1n);
            return { ok: r === 1n, result: String(r) };
          } catch (e) {
            return { ok: false, error: String(e) };
          }
        }
        try {
          const { instance } = await WebAssembly.instantiate(bytes);
          if (expected === "trap") {
            try {
              instance.exports.test();
              return { ok: false, error: "expected trap" };
            } catch (e) {
              return { ok: true, trap: true, error: String(e) };
            }
          }
          let result;
          if (expected === "param") result = instance.exports.test(1000);
          else result = instance.exports.test();
          out.result = String(result);
          if (expected === "validate" || expected === "tooling") return { ok: true, ...out };
          if (expected === "param") return { ok: Number(result) === 0, ...out };
          if (String(result) === expected || (expected === "-1" && Number(result) === -1)) {
            return { ok: true, ...out };
          }
          return { ok: false, ...out, expected };
        } catch (e) {
          return { ok: expected === "trap", validate: out.validate, error: String(e) };
        }
      },
      bytes,
      expected,
    );
    console.log(JSON.stringify({ chrome: chromePath, ...result }));
    process.exit(result.ok ? 0 : 1);
  } finally {
    await browser.close();
  }
}

main().catch((e) => {
  console.error(JSON.stringify({ ok: false, error: String(e) }));
  process.exit(1);
});
