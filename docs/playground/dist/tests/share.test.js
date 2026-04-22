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
import { encodeSharePayload, decodeSharePayload, parseFragment, checkVersionMismatch, encodeSharePayloadWithVersion, CURRENT_SHARE_VERSION, SHARE_URL_TARGET_LENGTH, SHARE_URL_HARD_LIMIT, REPRODUCIBILITY_CONTRACT, } from "../share.js";
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
/**
 * Assert round-trip: encode(payload) → fragment → decode(fragment) = payload.
 *
 * Compares fields semantically per ADR-021 §6.1.
 */
async function assertRoundTrip(payload, label) {
    const encoded = await encodeSharePayload(payload);
    assert.ok(encoded.fragment.startsWith("#share/"), `${label}: fragment should start with #share/`);
    const decoded = await decodeSharePayload(encoded.fragment);
    assert.ok(decoded.ok, `${label}: decode should succeed — ${decoded.ok ? "" : decoded.error}`);
    if (!decoded.ok)
        return; // unreachable after assert, satisfies TS
    // Semantic identity (ADR-021 §6.1)
    assert.equal(decoded.payload.src, payload.src, `${label}: src mismatch`);
    // Optional fields: compare if present, both absent is OK
    if (payload.ver !== undefined) {
        assert.equal(decoded.payload.ver, payload.ver, `${label}: ver mismatch`);
    }
    else {
        assert.equal(decoded.payload.ver, undefined, `${label}: ver should be absent`);
    }
    if (payload.ex !== undefined) {
        assert.equal(decoded.payload.ex, payload.ex, `${label}: ex mismatch`);
    }
    else {
        assert.equal(decoded.payload.ex, undefined, `${label}: ex should be absent`);
    }
    if (payload.f !== undefined) {
        assert.deepEqual(decoded.payload.f, payload.f, `${label}: f mismatch`);
    }
    else {
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
        await assertRoundTrip({ src: "// こんにちは\nfn main() {}" }, "Unicode");
    });
    it("large program — compression under URL limit", async () => {
        // Generate 10,000 characters of Arukellt-like source
        const lines = [];
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
        assert.ok(!result.exceedsTarget, `10K source should fit in URL budget (${result.fragmentLength} chars)`);
        await assertRoundTrip({ src }, "large program");
    });
    it("all optional fields — full schema round-trip", async () => {
        await assertRoundTrip({
            src: 'fn main() {\n    println("Hello")\n}',
            ver: "0.1.0",
            ex: "hello-world",
            f: { "diag-verbose": true, theme: "dark" },
        }, "all optional fields");
    });
    it("unknown fields — forward compatibility preservation (§3.3)", async () => {
        // Simulate a payload with an unknown field from a future version
        const payload = {
            src: "fn main() {}",
            ver: "0.2.0",
        };
        // Add unknown field directly
        payload["x"] = 42;
        const encoded = await encodeSharePayload(payload);
        const decoded = await decodeSharePayload(encoded.fragment);
        assert.ok(decoded.ok, "decode should succeed with unknown fields");
        if (!decoded.ok)
            return;
        assert.equal(decoded.payload.src, "fn main() {}");
        assert.equal(decoded.payload.ver, "0.2.0");
        // Unknown field must be preserved (§3.3)
        assert.equal(decoded.payload["x"], 42, "unknown field 'x' should be preserved");
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
        const payload = {
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
        const a = { src: "fn main() {}", ver: "0.1.0", ex: "test" };
        const b = {};
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
        assert.match(result.fragment, /^#share\/\d+\//, "fragment should match #share/<version>/<payload>");
    });
    it("uses current share version", async () => {
        const result = await encodeSharePayload({ src: "" });
        assert.ok(result.fragment.startsWith(`#share/${CURRENT_SHARE_VERSION}/`), `should use version ${CURRENT_SHARE_VERSION}`);
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
        const payload = { src: "test" };
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
// ---------------------------------------------------------------------------
// Version mismatch detection (ADR-021 §4)
// ---------------------------------------------------------------------------
describe("checkVersionMismatch (ADR-021 §4.2)", () => {
    it("same version → level 'none', no message", () => {
        const info = checkVersionMismatch({ src: "fn main() {}", ver: "0.1.0" }, "0.1.0");
        assert.equal(info.level, "none");
        assert.equal(info.linkVersion, "0.1.0");
        assert.equal(info.currentVersion, "0.1.0");
        assert.equal(info.message, null);
    });
    it("absent ver → level 'unknown', no message (§4.3)", () => {
        const info = checkVersionMismatch({ src: "fn main() {}" }, "0.1.0");
        assert.equal(info.level, "unknown");
        assert.equal(info.linkVersion, undefined);
        assert.equal(info.currentVersion, "0.1.0");
        assert.equal(info.message, null);
    });
    it("patch mismatch → level 'patch', has message", () => {
        const info = checkVersionMismatch({ src: "fn main() {}", ver: "0.1.0" }, "0.1.1");
        assert.equal(info.level, "patch");
        assert.equal(info.linkVersion, "0.1.0");
        assert.equal(info.currentVersion, "0.1.1");
        assert.ok(info.message !== null);
        assert.ok(info.message.includes("0.1.0"), "message includes link version");
        assert.ok(info.message.includes("0.1.1"), "message includes current version");
        assert.ok(info.message.includes("Behavior may differ"), "message includes behavior note");
    });
    it("minor mismatch → level 'minor', has message", () => {
        const info = checkVersionMismatch({ src: "fn main() {}", ver: "0.1.0" }, "0.2.0");
        assert.equal(info.level, "minor");
        assert.ok(info.message !== null);
    });
    it("major mismatch → level 'major', has message", () => {
        const info = checkVersionMismatch({ src: "fn main() {}", ver: "0.1.0" }, "1.0.0");
        assert.equal(info.level, "major");
        assert.ok(info.message !== null);
    });
    it("prerelease mismatch → level 'prerelease', has message", () => {
        const info = checkVersionMismatch({ src: "fn main() {}", ver: "0.1.0-dev" }, "0.1.0-rc1");
        assert.equal(info.level, "prerelease");
        assert.ok(info.message !== null);
        assert.ok(info.message.includes("0.1.0-dev"));
        assert.ok(info.message.includes("0.1.0-rc1"));
    });
    it("prerelease vs release → level 'prerelease'", () => {
        const info = checkVersionMismatch({ src: "fn main() {}", ver: "0.1.0-dev" }, "0.1.0");
        assert.equal(info.level, "prerelease");
        assert.ok(info.message !== null);
    });
    it("unparseable link version → level 'major' (safest default)", () => {
        const info = checkVersionMismatch({ src: "fn main() {}", ver: "custom-build" }, "0.1.0");
        assert.equal(info.level, "major");
        assert.ok(info.message !== null);
    });
    it("unparseable current version → level 'major' (safest default)", () => {
        const info = checkVersionMismatch({ src: "fn main() {}", ver: "0.1.0" }, "nightly");
        assert.equal(info.level, "major");
        assert.ok(info.message !== null);
    });
    it("both unparseable → level 'major'", () => {
        const info = checkVersionMismatch({ src: "fn main() {}", ver: "abc" }, "xyz");
        assert.equal(info.level, "major");
        assert.ok(info.message !== null);
    });
    it("message format matches ADR-021 §4.2 template", () => {
        const info = checkVersionMismatch({ src: "", ver: "0.1.0" }, "0.2.0");
        assert.equal(info.message, "This snippet was shared from version 0.1.0. " +
            "You are viewing it with version 0.2.0. Behavior may differ.");
    });
});
// ---------------------------------------------------------------------------
// Version-injected encoding (ADR-021 §4.2)
// ---------------------------------------------------------------------------
describe("encodeSharePayloadWithVersion", () => {
    it("injects version into payload", async () => {
        const result = await encodeSharePayloadWithVersion({ src: "fn main() {}" }, "0.1.0");
        assert.ok(result.fragment.startsWith("#share/"));
        const decoded = await decodeSharePayload(result.fragment);
        assert.ok(decoded.ok);
        if (!decoded.ok)
            return;
        assert.equal(decoded.payload.src, "fn main() {}");
        assert.equal(decoded.payload.ver, "0.1.0");
    });
    it("overwrites existing ver (re-sharing updates version)", async () => {
        const result = await encodeSharePayloadWithVersion({ src: "fn main() {}", ver: "0.0.1" }, "0.2.0");
        const decoded = await decodeSharePayload(result.fragment);
        assert.ok(decoded.ok);
        if (!decoded.ok)
            return;
        assert.equal(decoded.payload.ver, "0.2.0", "re-sharing should update ver to current");
    });
    it("preserves all other payload fields", async () => {
        const result = await encodeSharePayloadWithVersion({
            src: "fn main() {}",
            ex: "hello-world",
            f: { "diag-verbose": true },
        }, "0.1.0");
        const decoded = await decodeSharePayload(result.fragment);
        assert.ok(decoded.ok);
        if (!decoded.ok)
            return;
        assert.equal(decoded.payload.src, "fn main() {}");
        assert.equal(decoded.payload.ver, "0.1.0");
        assert.equal(decoded.payload.ex, "hello-world");
        assert.deepEqual(decoded.payload.f, { "diag-verbose": true });
    });
    it("preserves unknown fields when re-encoding with version", async () => {
        const payload = { src: "fn main() {}" };
        payload["x"] = 99;
        const result = await encodeSharePayloadWithVersion(payload, "0.1.0");
        const decoded = await decodeSharePayload(result.fragment);
        assert.ok(decoded.ok);
        if (!decoded.ok)
            return;
        assert.equal(decoded.payload.ver, "0.1.0");
        assert.equal(decoded.payload["x"], 99, "unknown field 'x' should be preserved");
    });
});
// ---------------------------------------------------------------------------
// Version pinning round-trip (encode → decode → mismatch check)
// ---------------------------------------------------------------------------
describe("Version pinning round-trip", () => {
    it("encode with version → decode → check mismatch (same version)", async () => {
        const encoded = await encodeSharePayloadWithVersion({ src: "fn main() {}" }, "0.1.0");
        const decoded = await decodeSharePayload(encoded.fragment);
        assert.ok(decoded.ok);
        if (!decoded.ok)
            return;
        const mismatch = checkVersionMismatch(decoded.payload, "0.1.0");
        assert.equal(mismatch.level, "none");
        assert.equal(mismatch.message, null);
    });
    it("encode with version → decode → check mismatch (different version)", async () => {
        const encoded = await encodeSharePayloadWithVersion({ src: "fn main() {}" }, "0.1.0");
        const decoded = await decodeSharePayload(encoded.fragment);
        assert.ok(decoded.ok);
        if (!decoded.ok)
            return;
        const mismatch = checkVersionMismatch(decoded.payload, "0.2.0");
        assert.equal(mismatch.level, "minor");
        assert.ok(mismatch.message !== null);
        assert.ok(mismatch.message.includes("0.1.0"));
        assert.ok(mismatch.message.includes("0.2.0"));
    });
    it("encode without version → decode → check mismatch → unknown", async () => {
        const encoded = await encodeSharePayload({ src: "fn main() {}" });
        const decoded = await decodeSharePayload(encoded.fragment);
        assert.ok(decoded.ok);
        if (!decoded.ok)
            return;
        const mismatch = checkVersionMismatch(decoded.payload, "0.1.0");
        assert.equal(mismatch.level, "unknown");
        assert.equal(mismatch.message, null);
    });
});
// ---------------------------------------------------------------------------
// Reproducibility contract (ADR-021 §4 + §6)
// ---------------------------------------------------------------------------
describe("REPRODUCIBILITY_CONTRACT", () => {
    it("exports the contract object", () => {
        assert.ok(REPRODUCIBILITY_CONTRACT !== null);
        assert.ok(typeof REPRODUCIBILITY_CONTRACT === "object");
    });
    it("guarantees src, ver, ex, f fields", () => {
        assert.ok(REPRODUCIBILITY_CONTRACT.guaranteedFields.includes("src"));
        assert.ok(REPRODUCIBILITY_CONTRACT.guaranteedFields.includes("ver"));
        assert.ok(REPRODUCIBILITY_CONTRACT.guaranteedFields.includes("ex"));
        assert.ok(REPRODUCIBILITY_CONTRACT.guaranteedFields.includes("f"));
    });
    it("lists non-guaranteed behaviors", () => {
        assert.ok(REPRODUCIBILITY_CONTRACT.notGuaranteed.length > 0);
        assert.ok(REPRODUCIBILITY_CONTRACT.notGuaranteed.includes("parse-diagnostics"));
        assert.ok(REPRODUCIBILITY_CONTRACT.notGuaranteed.includes("format-output"));
    });
    it("always loads — never refuses due to version mismatch", () => {
        assert.equal(REPRODUCIBILITY_CONTRACT.alwaysLoads, true);
    });
    it("re-sharing updates version", () => {
        assert.equal(REPRODUCIBILITY_CONTRACT.reshareUpdatesVersion, true);
    });
});
//# sourceMappingURL=share.test.js.map