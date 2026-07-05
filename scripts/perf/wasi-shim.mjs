// scripts/perf/wasi-shim.mjs — Shared WASI P1/P2 import shims for Node.js and browser.
//
// Used by run-node-bench.mjs and run-browser-bench.mjs to provide the host
// imports that Arukellt wasm modules expect. Two flavors are exported:
//
//   * buildWasiP1Shims(memory)  — for wasm32-wasi-p1 (linear memory) modules
//   * buildWasiP2Shims(memory)  — for wasm32-wasi-p2 (GC) modules
//
// Both shims collect stdout into a buffer and expose it via `getStdout()`.

function utf8Decode(mem, ptr, len) {
  return new TextDecoder().decode(new Uint8Array(mem.buffer, ptr, len));
}

function utf8Encode(mem, ptr, str) {
  const bytes = new TextEncoder().encode(str);
  const view = new Uint8Array(mem.buffer, ptr, bytes.length);
  view.set(bytes);
  return bytes.length;
}

/**
 * Build WASI Preview 1 import shims (wasi_snapshot_preview1.*).
 * Returns { imports, getStdout, getStderr }.
 */
export function buildWasiP1Shims(memory) {
  let stdout = '';
  let stderr = '';
  const u32 = (ptr) => new Uint32Array(memory.buffer, ptr, 1);
  const u64 = (ptr) => new BigUint64Array(memory.buffer, ptr, 1);

  const imports = {
    wasi_snapshot_preview1: {
      fd_write: (fd, iovs, iovsLen, nwrittenPtr) => {
        for (let i = 0; i < iovsLen; i++) {
          const iov = new Uint32Array(memory.buffer, iovs + i * 8, 2);
          const ptr = iov[0], len = iov[1];
          const text = utf8Decode(memory, ptr, len);
          if (fd === 1) stdout += text;
          else if (fd === 2) stderr += text;
        }
        u32(nwrittenPtr)[0] = 0;
        return 0;
      },
      fd_read: (fd, iovs, iovsLen, nreadPtr) => {
        u32(nreadPtr)[0] = 0;
        return 0;
      },
      fd_close: () => 0,
      args_sizes_get: (argcPtr, argvBufSizePtr) => {
        u32(argcPtr)[0] = 0;
        u32(argvBufSizePtr)[0] = 0;
        return 0;
      },
      args_get: () => 0,
      path_open: () => -1,
      proc_exit: (code) => { throw new Error(`proc_exit(${code})`); },
      clock_time_get: (id, precision, outPtr) => {
        u64(outPtr)[0] = BigInt(Date.now()) * 1000000n;
        return 0;
      },
      random_get: (bufPtr, len) => {
        const view = new Uint8Array(memory.buffer, bufPtr, len);
        for (let i = 0; i < len; i++) view[i] = Math.floor(Math.random() * 256);
        return 0;
      },
    },
  };

  return {
    imports,
    getStdout: () => stdout,
    getStderr: () => stderr,
    resetStdout: () => { stdout = ''; stderr = ''; },
  };
}

/**
 * Build WASI Preview 2 import shims (wasi:cli/*, wasi:filesystem/*, etc.).
 * Returns { imports, getStdout, getStderr, resetStdout }.
 */
export function buildWasiP2Shims(memory) {
  let stdout = '';
  let stderr = '';

  const imports = {
    'wasi:cli/stdout@0.2.0': {
      write: (fd, ptr, len) => {
        const text = utf8Decode(memory, ptr, len);
        stdout += text;
        return len;
      },
    },
    'wasi:cli/stderr@0.2.0': {
      write: (fd, ptr, len) => {
        const text = utf8Decode(memory, ptr, len);
        stderr += text;
        return len;
      },
    },
    'wasi:cli/stdin@0.2.0': {
      read: (fd, ptr, len) => 0,
    },
    'wasi:cli/exit@0.2.0': {
      exit: (code) => { throw new Error(`exit(${code})`); },
    },
    'wasi:cli/environment@0.2.0': {
      'args-sizes': () => [0, 0],
      arguments: (argvPtr, argvBufPtr) => 0,
    },
    'wasi:filesystem/types@0.2.0': {
      'open-at': () => -1,
      close: () => 0,
    },
    'wasi:clocks/monotonic-clock@0.2.0': {
      now: () => BigInt(Date.now()) * 1000000n,
    },
    'wasi:clocks/wall-clock@0.2.0': {
      now: () => [BigInt(Date.now()) * 1000000n, 0n],
    },
    'wasi:random/random@0.2.0': {
      'get-random-u64': () => BigInt(Math.floor(Math.random() * 1e15)),
    },
  };

  return {
    imports,
    getStdout: () => stdout,
    getStderr: () => stderr,
    resetStdout: () => { stdout = ''; stderr = ''; },
  };
}

/**
 * Detect whether a wasm module uses P1 or P2 imports by inspecting import
 * module names. Returns 'p1', 'p2', or 'unknown'.
 */
export function detectWasiFlavor(imports) {
  const modules = new Set(imports.map((imp) => imp.module));
  if (modules.has('wasi_snapshot_preview1')) return 'p1';
  for (const m of modules) {
    if (m.startsWith('wasi:')) return 'p2';
  }
  return 'unknown';
}

/**
 * Instantiate a wasm module with the correct WASI shim flavor.
 * Returns { instance, shims }.
 */
export async function instantiateWithWasi(wasmBytes, targetFlavor) {
  const mod = new WebAssembly.Module(wasmBytes);
  const mem = null; // will be set after instantiation

  // Peek at imports to determine flavor if not specified
  const importDescs = WebAssembly.Module.imports(mod);
  const flavor = targetFlavor || detectWasiFlavor(importDescs);

  // We need a memory reference for the shims, but memory comes from the
  // instance. Use a placeholder that we patch after instantiation.
  let memoryRef = { buffer: new ArrayBuffer(0) };

  let shims;
  if (flavor === 'p1') {
    shims = buildWasiP1Shims(memoryRef);
  } else {
    shims = buildWasiP2Shims(memoryRef);
  }

  const instance = new WebAssembly.Instance(mod, shims.imports);

  // Patch the memory reference to point to the actual instance memory
  if (instance.exports.memory) {
    Object.defineProperty(memoryRef, 'buffer', {
      get: () => instance.exports.memory.buffer,
      configurable: true,
    });
  }

  return { instance, shims, flavor };
}
