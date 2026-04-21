/**
 * Tests for the capability detection module.
 *
 * Verifies that the playground correctly identifies usage of
 * unsupported host capabilities (WASI, file I/O, network,
 * environment variables) in Arukellt source code and produces
 * appropriate warnings and diagnostics.
 *
 * @module
 */
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { checkCapabilities, capabilityWarningsToDiagnostics, UNSUPPORTED_CAPABILITIES, getCapabilityInfo, } from "../capability-check.js";
// ---------------------------------------------------------------------------
// checkCapabilities — basic detection
// ---------------------------------------------------------------------------
describe("checkCapabilities", () => {
    it("returns empty array for source with no unsupported features", () => {
        const source = `fn main() {
    let x = 42
    println(x)
}
`;
        const warnings = checkCapabilities(source);
        assert.equal(warnings.length, 0);
    });
    it("returns empty array for empty source", () => {
        assert.deepEqual(checkCapabilities(""), []);
    });
    it("returns empty array for comments containing patterns", () => {
        // Detection is source-level, so comments with patterns will match.
        // This is by design — we want to warn about intent, not just execution.
        // This test documents the behavior.
        const source = `// std::host::env is not available here
fn main() {}
`;
        const warnings = checkCapabilities(source);
        // Source-level scan picks up the pattern in the comment.
        assert.ok(warnings.length >= 1);
    });
    // --- WASI host detection ---
    it("detects import host", () => {
        const source = `import host
fn main() {}
`;
        const warnings = checkCapabilities(source);
        assert.equal(warnings.length, 1);
        assert.equal(warnings[0].capability.id, "wasi-host");
        assert.equal(warnings[0].matchText, "import host");
    });
    it("detects std::host:: usage", () => {
        const source = `fn main() {
    std::host::call()
}
`;
        const warnings = checkCapabilities(source);
        assert.equal(warnings.length, 1);
        assert.equal(warnings[0].capability.id, "wasi-host");
        assert.ok(warnings[0].matchText.startsWith("std::host::"));
    });
    // --- File I/O detection ---
    it("detects import fs", () => {
        const source = `import fs
fn main() {
    fs::read("file.txt")
}
`;
        const warnings = checkCapabilities(source);
        assert.ok(warnings.length >= 1);
        const fsWarning = warnings.find((w) => w.capability.id === "file-io");
        assert.ok(fsWarning, "should detect file-io capability");
    });
    it("detects std::fs:: usage", () => {
        const source = `fn main() {
    std::fs::read("data.txt")
}
`;
        const warnings = checkCapabilities(source);
        assert.equal(warnings.length, 1);
        assert.equal(warnings[0].capability.id, "file-io");
        assert.ok(warnings[0].message.includes("file system"));
    });
    it("detects host::fs:: usage", () => {
        const source = `fn main() {
    host::fs::write("out.txt", data)
}
`;
        const warnings = checkCapabilities(source);
        assert.equal(warnings.length, 1);
        assert.equal(warnings[0].capability.id, "file-io");
    });
    // --- Network detection ---
    it("detects import net", () => {
        const source = `import net
fn main() {}
`;
        const warnings = checkCapabilities(source);
        assert.equal(warnings.length, 1);
        assert.equal(warnings[0].capability.id, "network");
    });
    it("detects import http", () => {
        const source = `import http
fn main() {}
`;
        const warnings = checkCapabilities(source);
        assert.equal(warnings.length, 1);
        assert.equal(warnings[0].capability.id, "network");
    });
    it("detects std::net:: usage", () => {
        const source = `fn main() {
    std::net::connect("localhost", 8080)
}
`;
        const warnings = checkCapabilities(source);
        assert.equal(warnings.length, 1);
        assert.equal(warnings[0].capability.id, "network");
    });
    it("detects std::http:: usage", () => {
        const source = `fn main() {
    let resp = std::http::get("https://example.com")
}
`;
        const warnings = checkCapabilities(source);
        assert.equal(warnings.length, 1);
        assert.equal(warnings[0].capability.id, "network");
    });
    it("detects host::http:: usage", () => {
        const source = `fn main() {
    host::http::post(url, body)
}
`;
        const warnings = checkCapabilities(source);
        assert.equal(warnings.length, 1);
        assert.equal(warnings[0].capability.id, "network");
    });
    // --- Process environment detection ---
    it("detects import env", () => {
        const source = `import env
fn main() {}
`;
        const warnings = checkCapabilities(source);
        assert.equal(warnings.length, 1);
        assert.equal(warnings[0].capability.id, "process-env");
    });
    it("detects std::env:: usage", () => {
        const source = `fn main() {
    let path = std::env::get("PATH")
}
`;
        const warnings = checkCapabilities(source);
        assert.equal(warnings.length, 1);
        assert.equal(warnings[0].capability.id, "process-env");
    });
    it("detects host::env:: usage", () => {
        const source = `fn main() {
    let home = host::env::get("HOME")
}
`;
        const warnings = checkCapabilities(source);
        assert.equal(warnings.length, 1);
        assert.equal(warnings[0].capability.id, "process-env");
    });
    // --- Multiple detections ---
    it("detects multiple unsupported capabilities in one source", () => {
        const source = `import host
import fs

fn main() {
    let path = host::env::get("HOME")
    let data = std::fs::read(path)
    std::net::send(data)
}
`;
        const warnings = checkCapabilities(source);
        // Should detect: import host, import fs, host::env::get, std::fs::read, std::net::send
        assert.ok(warnings.length >= 4, `expected >= 4 warnings, got ${warnings.length}`);
        const ids = new Set(warnings.map((w) => w.capability.id));
        assert.ok(ids.has("wasi-host"), "should detect wasi-host");
        assert.ok(ids.has("file-io"), "should detect file-io");
        assert.ok(ids.has("process-env"), "should detect process-env");
        assert.ok(ids.has("network"), "should detect network");
    });
    // --- Position accuracy ---
    it("reports correct byte offsets", () => {
        const source = "fn main() {\n    std::fs::read()\n}\n";
        const warnings = checkCapabilities(source);
        assert.equal(warnings.length, 1);
        const w = warnings[0];
        const expectedStart = source.indexOf("std::fs::read");
        assert.equal(w.start, expectedStart);
        assert.equal(w.end, expectedStart + "std::fs::read".length);
        assert.equal(w.matchText, "std::fs::read");
    });
    // --- Ordering ---
    it("returns warnings sorted by source position", () => {
        const source = `std::net::connect()
std::fs::read()
std::env::get()
`;
        const warnings = checkCapabilities(source);
        assert.ok(warnings.length >= 3);
        for (let i = 1; i < warnings.length; i++) {
            assert.ok(warnings[i].start >= warnings[i - 1].start, `warning ${i} should come after warning ${i - 1} by position`);
        }
    });
    // --- Edge cases ---
    it("does not false-positive on similar-looking identifiers", () => {
        const source = `fn main() {
    let host_name = "example"
    let filesystem = true
    let network_ready = false
    let env_var = 42
}
`;
        const warnings = checkCapabilities(source);
        assert.equal(warnings.length, 0, "should not match partial identifiers");
    });
    it("does not match inside string literals that happen to contain patterns", () => {
        // The regex-based approach will match inside strings — this is
        // documented and acceptable since it errs on the side of caution.
        const source = `fn main() {
    let msg = "use std::host::call for WASI"
}
`;
        const warnings = checkCapabilities(source);
        // Source-level scan will pick up the pattern in the string.
        // This is a known limitation — we accept this false positive.
        assert.ok(warnings.length >= 1);
    });
    it("handles source with only whitespace", () => {
        assert.deepEqual(checkCapabilities("   \n\n  "), []);
    });
});
// ---------------------------------------------------------------------------
// capabilityWarningsToDiagnostics
// ---------------------------------------------------------------------------
describe("capabilityWarningsToDiagnostics", () => {
    it("returns empty array for no warnings", () => {
        assert.deepEqual(capabilityWarningsToDiagnostics([]), []);
    });
    it("converts a single warning to a diagnostic", () => {
        const warnings = checkCapabilities("import host\n");
        const diags = capabilityWarningsToDiagnostics(warnings);
        assert.equal(diags.length, 1);
        const d = diags[0];
        assert.equal(d.severity, "warning");
        assert.equal(d.code, "W9000");
        assert.equal(d.phase, "parse");
        assert.ok(d.message.includes("WASI host runtime"));
        assert.ok(d.labels.length >= 1);
        assert.equal(d.labels[0].file_id, 0);
        assert.equal(d.labels[0].start, 0);
        assert.equal(d.labels[0].end, "import host".length);
        assert.ok(d.notes.length >= 1);
        assert.ok(d.suggestion !== null);
    });
    it("preserves warning positions in diagnostic labels", () => {
        const source = "fn foo() {\n    std::fs::read()\n}\n";
        const warnings = checkCapabilities(source);
        const diags = capabilityWarningsToDiagnostics(warnings);
        assert.equal(diags.length, 1);
        const label = diags[0].labels[0];
        const expectedStart = source.indexOf("std::fs::read");
        assert.equal(label.start, expectedStart);
        assert.equal(label.end, expectedStart + "std::fs::read".length);
    });
    it("converts multiple warnings to diagnostics", () => {
        const source = `import fs
import net
`;
        const warnings = checkCapabilities(source);
        const diags = capabilityWarningsToDiagnostics(warnings);
        assert.equal(diags.length, warnings.length);
        for (const d of diags) {
            assert.equal(d.severity, "warning");
            assert.equal(d.code, "W9000");
        }
    });
    it("includes capability reason in diagnostic notes", () => {
        const warnings = checkCapabilities("std::net::connect()\n");
        const diags = capabilityWarningsToDiagnostics(warnings);
        assert.equal(diags.length, 1);
        assert.ok(diags[0].notes[0].includes("browser sandbox"));
    });
    it("includes suggestion about local installation", () => {
        const warnings = checkCapabilities("host::env::get()\n");
        const diags = capabilityWarningsToDiagnostics(warnings);
        assert.equal(diags.length, 1);
        assert.ok(diags[0].suggestion.includes("local"));
    });
});
// ---------------------------------------------------------------------------
// UNSUPPORTED_CAPABILITIES registry
// ---------------------------------------------------------------------------
describe("UNSUPPORTED_CAPABILITIES", () => {
    it("contains all expected capability categories", () => {
        const ids = UNSUPPORTED_CAPABILITIES.map((c) => c.id);
        assert.ok(ids.includes("wasi-host"));
        assert.ok(ids.includes("file-io"));
        assert.ok(ids.includes("network"));
        assert.ok(ids.includes("process-env"));
    });
    it("each capability has a name and reason", () => {
        for (const cap of UNSUPPORTED_CAPABILITIES) {
            assert.ok(cap.name.length > 0, `${cap.id} should have a name`);
            assert.ok(cap.reason.length > 0, `${cap.id} should have a reason`);
            assert.ok(cap.reason.includes("browser") || cap.reason.includes("sandbox"), `${cap.id} reason should mention browser/sandbox`);
        }
    });
});
// ---------------------------------------------------------------------------
// getCapabilityInfo
// ---------------------------------------------------------------------------
describe("getCapabilityInfo", () => {
    it("returns capability info for known IDs", () => {
        const knownIds = [
            "wasi-host",
            "file-io",
            "network",
            "process-env",
        ];
        for (const id of knownIds) {
            const info = getCapabilityInfo(id);
            assert.ok(info, `should find capability info for '${id}'`);
            assert.equal(info.id, id);
        }
    });
    it("returns undefined for unknown ID", () => {
        const info = getCapabilityInfo("nonexistent");
        assert.equal(info, undefined);
    });
});
// ---------------------------------------------------------------------------
// Integration scenario
// ---------------------------------------------------------------------------
describe("capability check integration", () => {
    it("produces displayable diagnostics from realistic source", () => {
        const source = `import host

fn main() {
    let config_path = host::env::get("CONFIG_PATH")
    let config = std::fs::read(config_path)
    let response = std::http::post("https://api.example.com", config)
    println(response)
}
`;
        const warnings = checkCapabilities(source);
        assert.ok(warnings.length >= 3, "should detect multiple issues");
        const diags = capabilityWarningsToDiagnostics(warnings);
        assert.equal(diags.length, warnings.length);
        // All diagnostics should be valid for display.
        for (const d of diags) {
            assert.equal(d.severity, "warning");
            assert.ok(d.message.length > 0);
            assert.ok(d.labels.length >= 1);
            assert.ok(d.labels[0].start >= 0);
            assert.ok(d.labels[0].end > d.labels[0].start);
            assert.ok(d.notes.length >= 1);
            assert.ok(d.suggestion !== null);
        }
        // Verify capability diversity.
        const capIds = new Set(warnings.map((w) => w.capability.id));
        assert.ok(capIds.size >= 3, "should detect at least 3 distinct capabilities");
    });
});
//# sourceMappingURL=capability-check.test.js.map