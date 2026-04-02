/**
 * Share link encoding and decoding for the Arukellt playground.
 *
 * Implements ADR-021: Playground Share URL Format. Share URLs encode
 * playground state in the URL fragment using DEFLATE compression and
 * base64url encoding:
 *
 *     <base-url>/playground#share/<version>/<payload>
 *
 * Encoding pipeline (ADR-021 §2):
 *
 *     PlaygroundState → JSON.stringify (canonical keys) → UTF-8
 *       → deflate-raw → base64url → #share/1/<payload>
 *
 * Decoding is the exact reverse.
 *
 * @module
 */

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Current share format version (ADR-021 §7.3: always encode with latest). */
const SHARE_FORMAT_VERSION = 1;

/** Fragment prefix for share URLs (ADR-021 §8). */
const SHARE_PREFIX = "share/";

/** Fragment prefix for example URLs (ADR-021 §8). */
const EXAMPLE_PREFIX = "example/";

/** Target URL length budget in characters (ADR-021 §5.1). */
const URL_LENGTH_TARGET = 8_192;

/** Hard URL length limit — URLs exceeding this MUST NOT be generated (ADR-021 §5.4). */
const URL_LENGTH_HARD_LIMIT = 65_536;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/**
 * Playground state encoded in a share URL payload (ADR-021 §3).
 *
 * Unknown fields are preserved for forward compatibility (§3.3).
 */
export interface SharePayload {
  /** Source code text (UTF-8, required). May be empty string. */
  src: string;
  /** Compiler/frontend version (semver, optional). */
  ver?: string;
  /** Example ID slug (optional). */
  ex?: string;
  /** Feature flags — keys are kebab-case, values are booleans or strings (optional). */
  f?: Record<string, boolean | string>;
  /** Index signature for unknown fields (forward compatibility, §3.3). */
  [key: string]: unknown;
}

/** Result of encoding playground state into a share URL fragment. */
export interface ShareEncodeResult {
  /** The URL fragment string (e.g., `#share/1/eNpL...`). */
  fragment: string;
  /** Whether the fragment exceeds the target length budget (8,192 chars). */
  exceedsTarget: boolean;
  /** Whether the fragment exceeds the hard limit (65,536 chars) and must not be used. */
  exceedsHardLimit: boolean;
  /** Total fragment length in characters. */
  fragmentLength: number;
}

/** Result of decoding a share URL fragment. */
export type ShareDecodeResult =
  | { ok: true; payload: SharePayload }
  | { ok: false; error: string };

/**
 * Parsed URL fragment action (ADR-021 §8).
 *
 * The playground router dispatches on the fragment prefix:
 * - `share` — decode a share link
 * - `example` — load a curated example by ID
 * - `none` — empty/absent fragment → default state
 * - `unknown` — unrecognized prefix → no-op (load default state)
 */
export type FragmentAction =
  | { type: "share"; version: number; encodedPayload: string }
  | { type: "example"; id: string }
  | { type: "none" }
  | { type: "unknown"; prefix: string };

// ---------------------------------------------------------------------------
// Base64url encoding (RFC 4648 §5, no padding)
// ---------------------------------------------------------------------------

/** Encode bytes to base64url without padding. */
function base64urlEncode(bytes: Uint8Array): string {
  let binary = "";
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  const base64 = btoa(binary);
  return base64.replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/** Decode base64url (no padding) to bytes. */
function base64urlDecode(str: string): Uint8Array {
  // Restore standard base64 alphabet and add padding
  let base64 = str.replace(/-/g, "+").replace(/_/g, "/");
  const padLen = (4 - (base64.length % 4)) % 4;
  base64 += "=".repeat(padLen);

  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

// ---------------------------------------------------------------------------
// Compression — raw DEFLATE (RFC 1951) via CompressionStream API
// ---------------------------------------------------------------------------

/** Read all chunks from a ReadableStream into a single Uint8Array. */
async function readAllBytes(
  readable: ReadableStream<Uint8Array>,
): Promise<Uint8Array> {
  const reader = readable.getReader();
  const chunks: Uint8Array[] = [];
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value);
  }
  if (chunks.length === 1) return chunks[0];
  const totalLength = chunks.reduce((sum, c) => sum + c.length, 0);
  const result = new Uint8Array(totalLength);
  let offset = 0;
  for (const chunk of chunks) {
    result.set(chunk, offset);
    offset += chunk.length;
  }
  return result;
}

/** Compress data using raw DEFLATE (no zlib/gzip wrapper). */
async function deflateRaw(data: Uint8Array): Promise<Uint8Array> {
  const cs = new CompressionStream("deflate-raw");
  const writer = cs.writable.getWriter();
  // Write and close — capture promises to avoid unhandled rejections.
  const writeDone = writer
    .write(data as Uint8Array<ArrayBuffer>)
    .then(() => writer.close())
    .catch(() => {});
  const result = await readAllBytes(cs.readable);
  await writeDone;
  return result;
}

/** Decompress raw DEFLATE data. */
async function inflateRaw(data: Uint8Array): Promise<Uint8Array> {
  const ds = new DecompressionStream("deflate-raw");
  const writer = ds.writable.getWriter();
  // Write and close — suppress writer-side errors that surface on readable.
  const writeDone = writer
    .write(data as Uint8Array<ArrayBuffer>)
    .then(() => writer.close())
    .catch(() => {});
  try {
    const result = await readAllBytes(ds.readable);
    await writeDone;
    return result;
  } catch (err) {
    await writeDone;
    throw err;
  }
}

// ---------------------------------------------------------------------------
// JSON serialization — canonical key order (ADR-021 §2.1)
// ---------------------------------------------------------------------------

/**
 * Serialize a SharePayload to JSON with alphabetically sorted keys.
 *
 * Omits keys with `undefined` values. Produces deterministic output
 * so the same state always encodes to the same URL (ADR-021 §6.2).
 */
function canonicalStringify(payload: SharePayload): string {
  const sorted: Record<string, unknown> = {};
  const keys = Object.keys(payload).sort();
  for (const key of keys) {
    const value = payload[key];
    if (value !== undefined) {
      sorted[key] = value;
    }
  }
  return JSON.stringify(sorted);
}

// ---------------------------------------------------------------------------
// Encode
// ---------------------------------------------------------------------------

/**
 * Encode playground state into a share URL fragment.
 *
 * Follows the ADR-021 §2 pipeline:
 *
 *     state → canonical JSON → UTF-8 → deflate-raw → base64url → fragment
 *
 * @param payload - The playground state to encode. Must include `src`.
 * @returns Encoded share fragment with length metadata.
 *
 * @example
 * ```ts
 * const result = await encodeSharePayload({ src: 'fn main() {}' });
 * // result.fragment === "#share/1/eNpL..."
 * window.location.hash = result.fragment.slice(1);
 * ```
 */
export async function encodeSharePayload(
  payload: SharePayload,
): Promise<ShareEncodeResult> {
  const json = canonicalStringify(payload);
  const jsonBytes = new TextEncoder().encode(json);
  const compressed = await deflateRaw(jsonBytes);
  const encoded = base64urlEncode(compressed);
  const fragment = `#${SHARE_PREFIX}${SHARE_FORMAT_VERSION}/${encoded}`;

  return {
    fragment,
    exceedsTarget: fragment.length > URL_LENGTH_TARGET,
    exceedsHardLimit: fragment.length > URL_LENGTH_HARD_LIMIT,
    fragmentLength: fragment.length,
  };
}

// ---------------------------------------------------------------------------
// Decode
// ---------------------------------------------------------------------------

/**
 * Decode a share URL fragment back into playground state.
 *
 * Follows the ADR-021 §2 reverse pipeline:
 *
 *     fragment → base64url → inflate-raw → UTF-8 → JSON.parse → validate
 *
 * On failure, returns an error result instead of throwing (ADR-021 §6.3).
 *
 * @param fragment - The URL fragment string (with or without leading `#`).
 * @returns Decoded payload on success, or an error description.
 *
 * @example
 * ```ts
 * const result = await decodeSharePayload(window.location.hash);
 * if (result.ok) {
 *   editor.setValue(result.payload.src);
 * } else {
 *   showError(result.error);
 * }
 * ```
 */
export async function decodeSharePayload(
  fragment: string,
): Promise<ShareDecodeResult> {
  try {
    const frag = fragment.startsWith("#") ? fragment.slice(1) : fragment;

    if (!frag.startsWith(SHARE_PREFIX)) {
      return { ok: false, error: "Not a share URL fragment" };
    }

    const rest = frag.slice(SHARE_PREFIX.length);
    const slashIdx = rest.indexOf("/");
    if (slashIdx === -1) {
      return { ok: false, error: "Missing format version separator" };
    }

    const versionStr = rest.slice(0, slashIdx);
    const version = parseInt(versionStr, 10);
    if (isNaN(version) || version < 1) {
      return { ok: false, error: `Invalid format version: ${versionStr}` };
    }

    const encoded = rest.slice(slashIdx + 1);
    if (encoded.length === 0) {
      return { ok: false, error: "Empty payload" };
    }

    // Dispatch by version (ADR-021 §7.2: support all prior versions)
    switch (version) {
      case 1:
        return await decodeV1(encoded);
      default:
        return {
          ok: false,
          error: `Unsupported share format version: ${version}`,
        };
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return { ok: false, error: `Decode failed: ${message}` };
  }
}

/** Decode a version 1 payload string. */
async function decodeV1(encoded: string): Promise<ShareDecodeResult> {
  // 1. Base64url decode
  let compressed: Uint8Array;
  try {
    compressed = base64urlDecode(encoded);
  } catch {
    return { ok: false, error: "Invalid base64url encoding" };
  }

  // 2. Inflate (raw DEFLATE)
  let jsonBytes: Uint8Array;
  try {
    jsonBytes = await inflateRaw(compressed);
  } catch {
    return { ok: false, error: "Decompression failed" };
  }

  // 3. UTF-8 decode
  const json = new TextDecoder().decode(jsonBytes);

  // 4. Parse JSON
  let parsed: unknown;
  try {
    parsed = JSON.parse(json);
  } catch {
    return { ok: false, error: "Invalid JSON payload" };
  }

  // 5. Validate required structure
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    return { ok: false, error: "Payload must be a JSON object" };
  }

  const obj = parsed as Record<string, unknown>;

  if (typeof obj.src !== "string") {
    return { ok: false, error: 'Missing or invalid required field: "src"' };
  }

  // Validate known optional field types (if present)
  if (obj.ver !== undefined && typeof obj.ver !== "string") {
    return { ok: false, error: 'Invalid field type: "ver" must be a string' };
  }
  if (obj.ex !== undefined && typeof obj.ex !== "string") {
    return { ok: false, error: 'Invalid field type: "ex" must be a string' };
  }
  if (obj.f !== undefined) {
    if (typeof obj.f !== "object" || obj.f === null || Array.isArray(obj.f)) {
      return {
        ok: false,
        error: 'Invalid field type: "f" must be an object',
      };
    }
  }

  // Preserve all fields including unknown ones (ADR-021 §3.3)
  return { ok: true, payload: obj as SharePayload };
}

// ---------------------------------------------------------------------------
// Fragment routing (ADR-021 §8)
// ---------------------------------------------------------------------------

/**
 * Parse a URL fragment to determine the playground action.
 *
 * Inspects the fragment prefix to dispatch:
 * - `#share/<v>/<payload>` → share link (decode with {@link decodeSharePayload})
 * - `#example/<id>` → curated example (load with examples module)
 * - empty/absent → default state
 * - unrecognized → unknown (treated as no-op per ADR-021 §8)
 *
 * @param fragment - The URL fragment (with or without leading `#`).
 */
export function parseFragment(fragment: string): FragmentAction {
  const frag = fragment.startsWith("#") ? fragment.slice(1) : fragment;

  if (frag.length === 0) {
    return { type: "none" };
  }

  if (frag.startsWith(SHARE_PREFIX)) {
    const rest = frag.slice(SHARE_PREFIX.length);
    const slashIdx = rest.indexOf("/");
    if (slashIdx === -1) {
      return { type: "unknown", prefix: SHARE_PREFIX };
    }
    const version = parseInt(rest.slice(0, slashIdx), 10);
    const encodedPayload = rest.slice(slashIdx + 1);
    if (isNaN(version) || encodedPayload.length === 0) {
      return { type: "unknown", prefix: SHARE_PREFIX };
    }
    return { type: "share", version, encodedPayload };
  }

  if (frag.startsWith(EXAMPLE_PREFIX)) {
    const id = frag.slice(EXAMPLE_PREFIX.length);
    if (id.length === 0) {
      return { type: "unknown", prefix: EXAMPLE_PREFIX };
    }
    return { type: "example", id };
  }

  // Unrecognized prefix — no-op (ADR-021 §8)
  const slashIdx = frag.indexOf("/");
  const prefix = slashIdx !== -1 ? frag.slice(0, slashIdx + 1) : frag;
  return { type: "unknown", prefix };
}

// ---------------------------------------------------------------------------
// Re-exported constants
// ---------------------------------------------------------------------------

/** Current share format version used when encoding. */
export const CURRENT_SHARE_VERSION: number = SHARE_FORMAT_VERSION;

/** Target URL length budget in characters (ADR-021 §5.1). */
export const SHARE_URL_TARGET_LENGTH: number = URL_LENGTH_TARGET;

/** Hard URL length limit in characters (ADR-021 §5.4). */
export const SHARE_URL_HARD_LIMIT: number = URL_LENGTH_HARD_LIMIT;
