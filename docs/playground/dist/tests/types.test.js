/**
 * Type-level and structural tests for the playground API types.
 *
 * These tests verify that the TypeScript type definitions match the
 * JSON response shapes from `ark-playground-wasm`. They run under
 * Node.js `--test` runner and do not require a browser or Wasm module.
 *
 * @module
 */
import { describe, it } from "node:test";
import assert from "node:assert/strict";
// ---------------------------------------------------------------------------
// Helpers — simulate JSON.parse of wasm module output
// ---------------------------------------------------------------------------
function parseJson(json) {
    return JSON.parse(json);
}
// ---------------------------------------------------------------------------
// ParseResponse
// ---------------------------------------------------------------------------
describe("ParseResponse", () => {
    it("accepts a successful parse response", () => {
        const json = `{
      "ok": true,
      "module": {
        "docs": ["A test module"],
        "imports": [{"module_name": "io", "alias": null}],
        "items": [
          {"kind": "fn", "name": "main", "is_pub": false, "docs": []}
        ]
      },
      "diagnostics": [],
      "error_count": 0
    }`;
        const resp = parseJson(json);
        assert.equal(resp.ok, true);
        assert.equal(resp.error_count, 0);
        assert.ok(resp.module);
        assert.equal(resp.module.items.length, 1);
        assert.equal(resp.module.items[0].kind, "fn");
        assert.equal(resp.module.items[0].name, "main");
        assert.equal(resp.module.items[0].is_pub, false);
        assert.equal(resp.module.imports.length, 1);
        assert.equal(resp.module.imports[0].module_name, "io");
        assert.equal(resp.module.imports[0].alias, null);
    });
    it("accepts a parse response with errors", () => {
        const json = `{
      "ok": false,
      "module": {"docs": [], "imports": [], "items": []},
      "diagnostics": [{
        "code": "E0001",
        "severity": "error",
        "phase": "parse",
        "message": "unexpected token",
        "labels": [{"file_id": 0, "start": 3, "end": 5, "message": "here"}],
        "notes": ["expected identifier"],
        "suggestion": null
      }],
      "error_count": 1
    }`;
        const resp = parseJson(json);
        assert.equal(resp.ok, false);
        assert.equal(resp.error_count, 1);
        assert.equal(resp.diagnostics.length, 1);
        const diag = resp.diagnostics[0];
        assert.equal(diag.code, "E0001");
        assert.equal(diag.severity, "error");
        assert.equal(diag.phase, "parse");
        assert.equal(diag.message, "unexpected token");
        const label = diag.labels[0];
        assert.equal(label.file_id, 0);
        assert.equal(label.start, 3);
        assert.equal(label.end, 5);
        assert.equal(label.message, "here");
    });
    it("handles multiple item kinds", () => {
        const json = `{
      "ok": true,
      "module": {
        "docs": [],
        "imports": [],
        "items": [
          {"kind": "struct", "name": "Point", "is_pub": true, "docs": ["A point"]},
          {"kind": "enum", "name": "Color", "is_pub": true, "docs": []},
          {"kind": "trait", "name": "Drawable", "is_pub": true, "docs": []},
          {"kind": "impl", "name": "Point", "is_pub": false, "docs": []},
          {"kind": "fn", "name": "main", "is_pub": false, "docs": []}
        ]
      },
      "diagnostics": [],
      "error_count": 0
    }`;
        const resp = parseJson(json);
        const items = resp.module.items;
        assert.equal(items.length, 5);
        assert.equal(items[0].kind, "struct");
        assert.equal(items[1].kind, "enum");
        assert.equal(items[2].kind, "trait");
        assert.equal(items[3].kind, "impl");
        assert.equal(items[4].kind, "fn");
    });
    it("handles import aliases", () => {
        const json = `{
      "ok": true,
      "module": {
        "docs": [],
        "imports": [
          {"module_name": "io", "alias": null},
          {"module_name": "math", "alias": "m"}
        ],
        "items": []
      },
      "diagnostics": [],
      "error_count": 0
    }`;
        const resp = parseJson(json);
        const imports = resp.module.imports;
        assert.equal(imports[0].alias, null);
        assert.equal(imports[1].alias, "m");
    });
});
// ---------------------------------------------------------------------------
// FormatResponse
// ---------------------------------------------------------------------------
describe("FormatResponse", () => {
    it("accepts a successful format response", () => {
        const json = `{"ok": true, "formatted": "fn main() {\\n}\\n"}`;
        const resp = parseJson(json);
        assert.equal(resp.ok, true);
        assert.ok(resp.formatted);
        assert.ok(resp.formatted.includes("fn main()"));
    });
    it("accepts a format error response", () => {
        const json = `{"ok": false, "error": "source contains syntax errors"}`;
        const resp = parseJson(json);
        assert.equal(resp.ok, false);
        assert.equal(resp.error, "source contains syntax errors");
        assert.equal(resp.formatted, undefined);
    });
});
// ---------------------------------------------------------------------------
// TokenizeResponse
// ---------------------------------------------------------------------------
describe("TokenizeResponse", () => {
    it("accepts a successful tokenize response", () => {
        const json = `{
      "ok": true,
      "tokens": [
        {"kind": "Fn", "text": "fn", "start": 0, "end": 2},
        {"kind": "Ident", "text": "main", "start": 3, "end": 7},
        {"kind": "LParen", "text": "(", "start": 7, "end": 8},
        {"kind": "RParen", "text": ")", "start": 8, "end": 9}
      ],
      "diagnostics": []
    }`;
        const resp = parseJson(json);
        assert.equal(resp.ok, true);
        assert.equal(resp.tokens.length, 4);
        const token = resp.tokens[0];
        assert.equal(token.kind, "Fn");
        assert.equal(token.text, "fn");
        assert.equal(token.start, 0);
        assert.equal(token.end, 2);
    });
});
// ---------------------------------------------------------------------------
// Diagnostic with suggestion
// ---------------------------------------------------------------------------
describe("Diagnostic", () => {
    it("handles diagnostics with suggestions", () => {
        const json = `{
      "ok": false,
      "module": null,
      "diagnostics": [{
        "code": "W0001",
        "severity": "warning",
        "phase": "lex",
        "message": "unused variable",
        "labels": [],
        "notes": [],
        "suggestion": "prefix with underscore"
      }],
      "error_count": 0
    }`;
        const resp = parseJson(json);
        assert.equal(resp.diagnostics[0].suggestion, "prefix with underscore");
        assert.equal(resp.diagnostics[0].severity, "warning");
    });
});
// ---------------------------------------------------------------------------
// Worker protocol messages
// ---------------------------------------------------------------------------
describe("WorkerRequest", () => {
    it("validates init request shape", () => {
        const req = { id: 1, cmd: "init", wasmUrl: "/test.wasm" };
        assert.equal(req.cmd, "init");
        assert.equal(req.id, 1);
    });
    it("validates parse request shape", () => {
        const req = { id: 2, cmd: "parse", source: "fn main() {}" };
        assert.equal(req.cmd, "parse");
    });
    it("validates all command types", () => {
        const commands = [
            { id: 1, cmd: "init", wasmUrl: "/test.wasm" },
            { id: 2, cmd: "parse", source: "test" },
            { id: 3, cmd: "format", source: "test" },
            { id: 4, cmd: "tokenize", source: "test" },
            { id: 5, cmd: "version" },
        ];
        assert.equal(commands.length, 5);
    });
});
describe("WorkerResponse", () => {
    it("validates success response", () => {
        const resp = { id: 1, ok: true, result: { ok: true } };
        assert.equal(resp.ok, true);
    });
    it("validates error response", () => {
        const resp = { id: 1, ok: false, error: "init failed" };
        assert.equal(resp.ok, false);
    });
});
// ---------------------------------------------------------------------------
// ModuleSummary structure
// ---------------------------------------------------------------------------
describe("ModuleSummary", () => {
    it("validates full module structure", () => {
        const mod = {
            docs: ["Module doc"],
            imports: [{ module_name: "io", alias: null }],
            items: [{ kind: "fn", name: "main", is_pub: false, docs: [] }],
        };
        assert.equal(mod.docs.length, 1);
        assert.equal(mod.imports.length, 1);
        assert.equal(mod.items.length, 1);
    });
});
//# sourceMappingURL=types.test.js.map