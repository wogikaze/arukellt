/**
 * Round-trip tests for the share link encoder/decoder.
 *
 * Verifies the ADR-021 §6 round-trip contract:
 *
 *     ∀ state ∈ ValidPlaygroundState:
 *         decode(encode(state)) = state
 *
 * Test cases from ADR-021 §6.4.
 *
 * @module
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  encodeSharePayload,
  decodeSharePayload,
  parseFragment,
  CURRENT_SHARE_VERSION,
  SHARE_URL_TARGET_LENGTH,
  SHARE_URL_HARD_LIMIT,
} from "../share.js";
import type {
  SharePayload,
  ShareEncodeResult,
  FragmentAction,
} from "../share.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Assert round-trip: encode(payload) → fragment → decode(fragment) = payload.
 *
 * Compares fields semantically per ADR-021 §6.1.
 */
async function assertRoundTrip(
  payload: SharePayload,
  label: string,
): Promise<void> {
  const encoded = await encodeSharePayload(payload);
  assert.ok(
    encoded.fragment.startsWith("#share/"),
    `${label}: fragment should start with #share/`,
  );

  const decoded = await decodeSharePayload(encoded.fragment);
  assert.ok(decoded.ok, `${label}: decode should succeed — ${decoded.ok ? "" : decoded.error}`);
  if (!decoded.ok) return; // unreachable after assert, satisfies TS

  // Semantic identity (ADR-021 §6.1)
  assert.equal(decoded.payload.src, payload.src, `${label}: src mismatch`);

  // Optional fields: compare if present, both absent is OK
  if (payload.ver !== undefined) {
    assert.equal(decoded.payload.ver, payload.ver, `${label}: ver mismatch`);
  } else {
    assert.equal(decoded.payload.ver, undefined, `${label}: ver should be absent`);
  }

  if (payload.ex !== undefined) {
    assert.equal(decoded.payload.ex, payload.ex, `${label}: ex mismatch`);
  } else {
    assert.equal(decoded.payload.ex, undefined, `${label}: ex should be absent`);
  }

  if (payload.f !== undefined) {
    assert.deepEqual(decoded.payload.f, payload.f, `${label}: f mismatch`);
  } else {
    assert.equal(decoded.payload.f, undefined, `${label}: f should be absent`);
  }
}

// ---------------------------------------------------------------------------
// Round-trip tests (ADR-021 §6.4)
// ---------------------------------------------------------------------------

describe("Share link round-trip (ADR-021 §6.4)", () => {
  it("empty string — minimal payload", async () => {
    await assertRoundTrip({ src: "" }, "empty string");
  });

  it("ASCII-only — basic round-trip", async () => {
    await assertRoundTrip({ src: "fn main() {}" }, "ASCII-only");
  });

  it("Unicode — UTF-8 preservation", async () => {
    await assertRoundTrip(
      { src: "// こんにちは\nfn main() {}" },
      "Unicode",
    );
  });

  it("large program — compression under URL limit", async () => {
    // Generate 10,000 characters of Arukellt-like source
    const lines: string[] = [];
    for (let i = 0; i < 200; i++) {
      lines.push(`fn func_${i}(x: i32, y: i32) -> i32 {`);
      lines.push(`    let result = x + y + ${i}`);
      lines.push("    result");
      lines.push("}");
      lines.push("");
    }
    const src = lines.join("\n");
    assert.ok(src.length >= 10_000, `Source should be >= 10000 chars, got ${src.length}`);

    const result = await encodeSharePayload({ src });
    assert.ok(
      !result.exceedsTarget,
      `10K source should fit in URL budget (${result.fragmentLength} chars)`,
    );

    await assertRoundTrip({ src }, "large program");
  });

  it("all optional fields — full schema round-trip", async () => {
    await assertRoundTrip(
      {
        src: 'fn main() {\n    println("Hello")\n}',
        ver: "0.1.0",
        ex: "hello-world",
        f: { "diag-verbose": true, theme: "dark" },
      },
      "all optional fields",
    );
  });

  it("unknown fields — forward compatibility preservation (§3.3)", async () => {
    // Simulate a payload with an unknown field from a future version
    const payload: SharePayload = {
      src: "fn main() {}",
      ver: "0.2.0",
    };
    // Add unknown field directly
    (payload as Record<string, unknown>)["x"] = 42;

    const encoded = await encodeSharePayload(payload);
    const decoded = await decodeSharePayload(encoded.fragment);
    assert.ok(decoded.ok, "decode should succeed with unknown fields");
    if (!decoded.ok) return;

    assert.equal(decoded.payload.src, "fn main() {}");
    assert.equal(decoded.payload.ver, "0.2.0");
    // Unknown field must be preserved (§3.3)
    assert.equal(
      (decoded.payload as Record<string, unknown>)["x"],
      42,
      "unknown field 'x' should be preserved",
    );
  });

  it("special JSON chars — escaping correctness", async () => {
    const src = 'let s = "hello\\tworld\\n"\nlet q = "a\\"b"\n// null: \x00';
    await assertRoundTrip({ src }, "special JSON chars");
  });

  it("multiline source with various whitespace", async () => {
    const src = "fn main() {\n\tlet x = 1\n    let y = 2\r\n    x + y\n}\n";
    await assertRoundTrip({ src }, "multiline whitespace");
  });
});

// ---------------------------------------------------------------------------
// Canonical encoding (ADR-021 §6.2)
// ---------------------------------------------------------------------------

describe("Share link canonical encoding (ADR-021 §6.2)", () => {
  it("same state produces same fragment (deterministic)", async () => {
    const payload: SharePayload = {
      src: "fn main() {}",
      ver: "0.1.0",
      ex: "hello-world",
    };

    const result1 = await encodeSharePayload(payload);
    const result2 = await encodeSharePayload(payload);
    assert.equal(result1.fragment, result2.fragment, "same input → same output");
  });

  it("key order does not affect output (canonical sorting)", async () => {
    // Build two payloads with keys in different insertion order
    const a: SharePayload = { src: "fn main() {}", ver: "0.1.0", ex: "test" };

    const b = {} as SharePayload;
    b.ver = "0.1.0";
    b.ex = "test";
    b.src = "fn main() {}";

    const resultA = await encodeSharePayload(a);
    const resultB = await encodeSharePayload(b);
    assert.equal(resultA.fragment, resultB.fragment, "key order should not matter");
  });
});

// ---------------------------------------------------------------------------
// Encode result metadata
// ---------------------------------------------------------------------------

describe("Share encode result metadata", () => {
  it("fragment starts with #share/<version>/", async () => {
    const result = await encodeSharePayload({ src: "test" });
    assert.match(
      result.fragment,
      /^#share\/\d+\//,
      "fragment should match #share/<version>/<payload>",
    );
  });

  it("uses current share version", async () => {
    const result = await encodeSharePayload({ src: "" });
    assert.ok(
      result.fragment.startsWith(`#share/${CURRENT_SHARE_VERSION}/`),
      `should use version ${CURRENT_SHARE_VERSION}`,
    );
  });

  it("reports fragment length accurately", async () => {
    const result = await encodeSharePayload({ src: "fn main() {}" });
    assert.equal(result.fragmentLength, result.fragment.length);
  });

  it("exceedsTarget is false for small payloads", async () => {
    const result = await encodeSharePayload({ src: "fn main() {}" });
    assert.equal(result.exceedsTarget, false);
    assert.equal(result.exceedsHardLimit, false);
  });

  it("exports correct constants", () => {
    assert.equal(CURRENT_SHARE_VERSION, 1);
    assert.equal(SHARE_URL_TARGET_LENGTH, 8_192);
    assert.equal(SHARE_URL_HARD_LIMIT, 65_536);
  });
});

// ---------------------------------------------------------------------------
// Decode error handling (ADR-021 §6.3)
// ---------------------------------------------------------------------------

describe("Share link decode errors (ADR-021 §6.3)", () => {
  it("rejects non-share fragments", async () => {
    const result = await decodeSharePayload("#example/hello");
    assert.equal(result.ok, false);
  });

  it("rejects missing version separator", async () => {
    const result = await decodeSharePayload("#share/abc");
    assert.equal(result.ok, false);
  });

  it("rejects invalid version number", async () => {
    const result = await decodeSharePayload("#share/0/payload");
    assert.equal(result.ok, false);
  });

  it("rejects unsupported version", async () => {
    const result = await decodeSharePayload("#share/99/payload");
    assert.equal(result.ok, false);
    if (!result.ok) {
      assert.ok(result.error.includes("Unsupported"));
    }
  });

  it("rejects empty payload", async () => {
    const result = await decodeSharePayload("#share/1/");
    assert.equal(result.ok, false);
  });

  it("rejects corrupted base64url", async () => {
    const result = await decodeSharePayload("#share/1/!!!invalid!!!");
    assert.equal(result.ok, false);
  });

  it("rejects corrupted compressed data", async () => {
    // Valid base64url but invalid DEFLATE stream
    const result = await decodeSharePayload("#share/1/AAAA");
    assert.equal(result.ok, false);
  });

  it("handles fragment with leading # correctly", async () => {
    const payload: SharePayload = { src: "test" };
    const encoded = await encodeSharePayload(payload);

    // With #
    const r1 = await decodeSharePayload(encoded.fragment);
    assert.ok(r1.ok);

    // Without #
    const r2 = await decodeSharePayload(encoded.fragment.slice(1));
    assert.ok(r2.ok);
  });
});

// ---------------------------------------------------------------------------
// Fragment routing (ADR-021 §8)
// ---------------------------------------------------------------------------

describe("parseFragment (ADR-021 §8)", () => {
  it("empty fragment → none", () => {
    const result = parseFragment("");
    assert.equal(result.type, "none");
  });

  it("# only → none", () => {
    const result = parseFragment("#");
    assert.equal(result.type, "none");
  });

  it("share fragment → share action", () => {
    const result = parseFragment("#share/1/eNpLSS0u");
    assert.equal(result.type, "share");
    if (result.type === "share") {
      assert.equal(result.version, 1);
      assert.equal(result.encodedPayload, "eNpLSS0u");
    }
  });

  it("example fragment → example action", () => {
    const result = parseFragment("#example/hello-world");
    assert.equal(result.type, "example");
    if (result.type === "example") {
      assert.equal(result.id, "hello-world");
    }
  });

  it("example fragment without id → unknown", () => {
    const result = parseFragment("#example/");
    assert.equal(result.type, "unknown");
  });

  it("unrecognized prefix → unknown", () => {
    const result = parseFragment("#tutorial/lesson-1");
    assert.equal(result.type, "unknown");
    if (result.type === "unknown") {
      assert.equal(result.prefix, "tutorial/");
    }
  });

  it("works without leading #", () => {
    const result = parseFragment("example/fibonacci");
    assert.equal(result.type, "example");
    if (result.type === "example") {
      assert.equal(result.id, "fibonacci");
    }
  });

  it("share fragment without payload → unknown", () => {
    const result = parseFragment("#share/");
    assert.equal(result.type, "unknown");
  });

  it("share fragment without version slash → unknown", () => {
    const result = parseFragment("#share/abc");
    assert.equal(result.type, "unknown");
  });
});
