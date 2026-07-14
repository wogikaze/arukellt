/**
 * Browser/Node runner for compiled `wasm32` (non-GC) Wasm modules.
 *
 * @module
 */
const textDecoder = new TextDecoder();
const textEncoder = new TextEncoder();
function getWasmInstance(instantiated) {
    if ("instance" in instantiated) {
        return instantiated.instance;
    }
    return instantiated;
}
/**
 * Split stdin bytes into newline-delimited chunks for line-oriented reads.
 *
 * Each chunk ends with `\n` except a final line with no trailing newline in the
 * source text.
 */
export function stdinBytesToLineChunks(stdin) {
    if (stdin.length === 0) {
        return [];
    }
    const text = textDecoder.decode(stdin);
    const parts = text.split("\n");
    const chunks = [];
    for (let i = 0; i < parts.length; i++) {
        const isLast = i === parts.length - 1;
        const suffix = isLast && !text.endsWith("\n") ? "" : "\n";
        chunks.push(textEncoder.encode(parts[i] + suffix));
    }
    return chunks;
}
function createStdinReader(stdin, mode) {
    if (mode === "stream") {
        let offset = 0;
        return {
            read(ptr, len, memory) {
                const available = stdin.length - offset;
                if (available <= 0)
                    return 0;
                const toCopy = Math.min(len, available);
                new Uint8Array(memory.buffer, ptr, toCopy).set(stdin.subarray(offset, offset + toCopy));
                offset += toCopy;
                return toCopy;
            },
        };
    }
    const lines = stdinBytesToLineChunks(stdin);
    let lineIndex = 0;
    let pending = null;
    let endCurrentSession = false;
    return {
        read(ptr, len, memory) {
            if (endCurrentSession) {
                endCurrentSession = false;
                return 0;
            }
            if (!pending) {
                if (lineIndex >= lines.length) {
                    return 0;
                }
                pending = lines[lineIndex];
                lineIndex += 1;
            }
            const toCopy = Math.min(len, pending.length);
            new Uint8Array(memory.buffer, ptr, toCopy).set(pending.subarray(0, toCopy));
            pending = toCopy < pending.length ? pending.subarray(toCopy) : null;
            if (!pending) {
                endCurrentSession = true;
            }
            return toCopy;
        },
    };
}
/**
 * Instantiate and execute wasm32 Wasm bytes with an `arukellt_io` stdio host.
 */
export async function runT2Wasm(wasmBytes, options = {}) {
    const started = performance.now();
    const stdin = options.stdin ?? new Uint8Array();
    const stdinMode = options.stdinMode ?? "stream";
    const stdinReader = createStdinReader(stdin, stdinMode);
    const stdoutChunks = [];
    const stderrChunks = [];
    const imports = {
        arukellt_io: {
            write(ptr, len) {
                const memory = activeMemory;
                if (!memory)
                    return;
                stdoutChunks.push(textDecoder.decode(new Uint8Array(memory.buffer, ptr, len)));
            },
            write_err(ptr, len) {
                const memory = activeMemory;
                if (!memory)
                    return;
                stderrChunks.push(textDecoder.decode(new Uint8Array(memory.buffer, ptr, len)));
            },
            flush() {
                // Lines are assembled when the host reads stdout.
            },
            flush_err() {
                // Lines are assembled when the host reads stderr.
            },
            read(ptr, len) {
                const memory = activeMemory;
                if (!memory)
                    return 0;
                return stdinReader.read(ptr, len, memory);
            },
        },
    };
    let activeMemory = null;
    let exitCode = 0;
    let trap = null;
    try {
        const instantiated = await WebAssembly.instantiate(wasmBytes, imports);
        const instance = getWasmInstance(instantiated);
        const memory = instance.exports.memory;
        if (!memory) {
            throw new Error("wasm32 module does not export memory");
        }
        activeMemory = memory;
        const start = instance.exports._start;
        const main = instance.exports.main;
        const entry = start ?? main;
        if (typeof entry !== "function") {
            throw new Error("wasm32 module does not export _start or main");
        }
        entry();
    }
    catch (err) {
        trap = err instanceof Error ? err.message : String(err);
        exitCode = 1;
    }
    return {
        ok: trap === null,
        stdout: stdoutChunks.join(""),
        stderr: stderrChunks.join(""),
        exitCode,
        trap,
        elapsedMs: performance.now() - started,
    };
}
/** Return true when a Wasm module imports the wasm32 stdio surface. */
export function moduleImportsArukelltIo(bytes) {
    const needle = new TextEncoder().encode("arukellt_io");
    for (let i = 0; i <= bytes.length - needle.length; i++) {
        let matched = true;
        for (let j = 0; j < needle.length; j++) {
            if (bytes[i + j] !== needle[j]) {
                matched = false;
                break;
            }
        }
        if (matched)
            return true;
    }
    return false;
}
//# sourceMappingURL=t2-runner.js.map