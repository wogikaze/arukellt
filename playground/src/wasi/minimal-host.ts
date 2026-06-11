/**
 * Minimal WASI Preview 1 host for running the selfhost compiler Wasm.
 *
 * @module
 */

const WASI_ERRNO_SUCCESS = 0;
const WASI_ERRNO_BADF = 8;
const WASI_ERRNO_NOENT = 44;

const O_CREAT = 0x0001;
const O_TRUNC = 0x0008;

const textDecoder = new TextDecoder();

export interface WasiHostOptions {
  argv: string[];
  files: Map<string, Uint8Array>;
  stdoutLimit?: number;
  stderrLimit?: number;
}

export interface WasiHostHandle {
  imports: WebAssembly.Imports;
  result: WasiHostResult;
  bindMemory(memory: WebAssembly.Memory): void;
}

export interface WasiHostResult {
  exitCode: number;
  stdout: string;
  stderr: string;
}

/**
 * Create a WASI import object backed by an in-memory virtual filesystem.
 */
export function createWasiHost(opts: WasiHostOptions): WasiHostHandle {
  const files = opts.files;
  const stdoutChunks: string[] = [];
  const stderrChunks: string[] = [];
  const stdoutLimit = opts.stdoutLimit ?? 1_048_576;
  const stderrLimit = opts.stderrLimit ?? 1_048_576;
  let stdoutBytes = 0;
  let stderrBytes = 0;
  let memory: WebAssembly.Memory | null = null;

  const result: WasiHostResult = {
    exitCode: 0,
    stdout: "",
    stderr: "",
  };

  const fdTable = new Map<number, { path: string; readable: boolean; writable: boolean; position: number }>();
  fdTable.set(0, { path: "<stdin>", readable: true, writable: false, position: 0 });
  fdTable.set(1, { path: "<stdout>", readable: false, writable: true, position: 0 });
  fdTable.set(2, { path: "<stderr>", readable: false, writable: true, position: 0 });
  fdTable.set(3, { path: ".", readable: true, writable: true, position: 0 });

  let nextFd = 4;

  function requireMemory(): WebAssembly.Memory {
    if (!memory) throw new Error("WASI host memory is not bound yet");
    return memory;
  }

  function view(): DataView {
    return new DataView(requireMemory().buffer);
  }

  function readBytes(ptr: number, len: number): Uint8Array {
    return new Uint8Array(requireMemory().buffer, ptr, len);
  }

  function readString(ptr: number, len: number): string {
    return textDecoder.decode(readBytes(ptr, len));
  }

  function appendStdout(text: string): number {
    if (stdoutBytes + text.length > stdoutLimit) {
      throw new Error("compiler stdout exceeded size limit");
    }
    stdoutBytes += text.length;
    stdoutChunks.push(text);
    result.stdout = stdoutChunks.join("");
    return text.length;
  }

  function appendStderr(text: string): number {
    if (stderrBytes + text.length > stderrLimit) {
      throw new Error("compiler stderr exceeded size limit");
    }
    stderrBytes += text.length;
    stderrChunks.push(text);
    result.stderr = stderrChunks.join("");
    return text.length;
  }

  function normalizePath(path: string): string {
    if (path === "." || path === "") return "/";
    if (!path.startsWith("/")) return `/${path}`;
    return path;
  }

  const imports: WebAssembly.Imports = {
    wasi_snapshot_preview1: {
      fd_write(fd: number, iovPtr: number, iovCount: number, nwrittenPtr: number): number {
        const file = fdTable.get(fd);
        if (!file?.writable) return WASI_ERRNO_BADF;
        const mem = view();
        let total = 0;
        for (let i = 0; i < iovCount; i++) {
          const base = iovPtr + i * 8;
          const bufPtr = mem.getUint32(base, true);
          const bufLen = mem.getUint32(base + 4, true);
          const chunk = readBytes(bufPtr, bufLen);
          const text = textDecoder.decode(chunk);
          if (fd === 1) total += appendStdout(text);
          else if (fd === 2) total += appendStderr(text);
          else if (file.path !== "<stdout>" && file.path !== "<stderr>") {
            const key = normalizePath(file.path);
            const existing = files.get(key) ?? new Uint8Array();
            const merged = new Uint8Array(existing.length + chunk.length);
            merged.set(existing, 0);
            merged.set(chunk, existing.length);
            files.set(key, merged);
            file.position = merged.length;
            total += bufLen;
          }
        }
        mem.setUint32(nwrittenPtr, total, true);
        return WASI_ERRNO_SUCCESS;
      },

      args_sizes_get(argcPtr: number, argvBufSizePtr: number): number {
        const mem = view();
        mem.setUint32(argcPtr, opts.argv.length, true);
        let size = 0;
        for (const arg of opts.argv) size += arg.length + 1;
        mem.setUint32(argvBufSizePtr, size, true);
        return WASI_ERRNO_SUCCESS;
      },

      args_get(argvPtr: number, argvBufPtr: number): number {
        const mem = view();
        let bufOffset = argvBufPtr;
        for (let i = 0; i < opts.argv.length; i++) {
          const arg = opts.argv[i]!;
          mem.setUint32(argvPtr + i * 4, bufOffset, true);
          for (let j = 0; j < arg.length; j++) {
            mem.setUint8(bufOffset + j, arg.charCodeAt(j));
          }
          mem.setUint8(bufOffset + arg.length, 0);
          bufOffset += arg.length + 1;
        }
        return WASI_ERRNO_SUCCESS;
      },

      path_open(
        dirfd: number,
        _dirFlags: number,
        pathPtr: number,
        pathLen: number,
        oflags: number,
        _rightsBase: bigint,
        _rightsInheriting: bigint,
        _fdFlags: number,
        openedFdPtr: number,
      ): number {
        const dir = fdTable.get(dirfd);
        if (!dir?.readable) return WASI_ERRNO_BADF;
        const full = normalizePath(readString(pathPtr, pathLen));
        const exists = files.has(full);
        const create = (oflags & O_CREAT) !== 0;
        const truncate = (oflags & O_TRUNC) !== 0;
        if (!exists && !create) return WASI_ERRNO_NOENT;
        if (!exists || truncate) files.set(full, new Uint8Array());
        const fd = nextFd++;
        fdTable.set(fd, { path: full, readable: true, writable: true, position: 0 });
        view().setUint32(openedFdPtr, fd, true);
        return WASI_ERRNO_SUCCESS;
      },

      fd_read(fd: number, iovPtr: number, iovCount: number, nreadPtr: number): number {
        const file = fdTable.get(fd);
        if (!file?.readable || file.path === "<stdout>" || file.path === "<stderr>") {
          return WASI_ERRNO_BADF;
        }
        const content = files.get(normalizePath(file.path));
        if (!content) return WASI_ERRNO_NOENT;
        const mem = view();
        let total = 0;
        for (let i = 0; i < iovCount; i++) {
          const base = iovPtr + i * 8;
          const bufPtr = mem.getUint32(base, true);
          const bufLen = mem.getUint32(base + 4, true);
          const available = content.length - file.position;
          if (available <= 0) break;
          const toCopy = Math.min(bufLen, available);
          new Uint8Array(requireMemory().buffer, bufPtr, toCopy).set(
            content.subarray(file.position, file.position + toCopy),
          );
          file.position += toCopy;
          total += toCopy;
          if (toCopy < bufLen) break;
        }
        mem.setUint32(nreadPtr, total, true);
        return WASI_ERRNO_SUCCESS;
      },

      fd_close(fd: number): number {
        if (fd >= 3) fdTable.delete(fd);
        return WASI_ERRNO_SUCCESS;
      },

      proc_exit(code: number): void {
        result.exitCode = code >>> 0;
        throw new WasiExit(result.exitCode);
      },
    },
  };

  return {
    imports,
    result,
    bindMemory(mem: WebAssembly.Memory): void {
      memory = mem;
    },
  };
}

/** Thrown by `proc_exit` to unwind synchronous Wasm execution. */
export class WasiExit extends Error {
  readonly code: number;

  constructor(code: number) {
    super(`WASI proc_exit(${code})`);
    this.name = "WasiExit";
    this.code = code;
  }
}

/** Read a virtual file after a WASI-hosted compile run. */
export function readVirtualFile(
  files: Map<string, Uint8Array>,
  path: string,
): Uint8Array | null {
  const key = path.startsWith("/") ? path : `/${path}`;
  return files.get(key) ?? null;
}
