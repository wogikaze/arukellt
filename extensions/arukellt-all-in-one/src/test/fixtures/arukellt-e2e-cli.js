#!/usr/bin/env node
// Minimal arukellt-compatible CLI for VS Code extension E2E tests.
//
// The selfhost release wrapper verifies CLI packaging separately. Extension
// activation tests need a live stdio LSP/DAP process, so this fixture provides
// the protocol surface exercised by src/test/extension.test.js.
"use strict";

const fs = require("fs");

function logInvocation(args) {
  const logPath = process.env.ARUKELLT_E2E_STUB_LOG;
  if (!logPath) return;
  fs.appendFileSync(logPath, JSON.stringify({ argv: args, cwd: process.cwd() }) + "\n");
}

function writeFrame(obj) {
  const body = JSON.stringify(obj);
  process.stdout.write(`Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n${body}`);
}

function readFrames(onMessage) {
  let buffer = Buffer.alloc(0);
  process.stdin.on("data", (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    while (true) {
      const sep = buffer.indexOf("\r\n\r\n");
      if (sep < 0) return;
      const header = buffer.slice(0, sep).toString("utf8");
      const match = /Content-Length:\s*(\d+)/i.exec(header);
      if (!match) {
        buffer = buffer.slice(sep + 4);
        continue;
      }
      const len = Number(match[1]);
      const start = sep + 4;
      const end = start + len;
      if (buffer.length < end) return;
      const body = buffer.slice(start, end).toString("utf8");
      buffer = buffer.slice(end);
      try {
        onMessage(JSON.parse(body));
      } catch (_) {
        // Ignore malformed frames; production protocol tests cover parser
        // behavior in the selfhost LSP/DAP gates.
      }
    }
  });
}

function startLsp() {
  const docs = new Map();

  function diagnosticsFor(text) {
    return /\bundefined_var\b/.test(text || "")
      ? [{
          range: {
            start: { line: 2, character: 19 },
            end: { line: 2, character: 32 },
          },
          severity: 1,
          source: "arukellt",
          message: "E0100 unresolved name: undefined_var",
        }]
      : [];
  }

  function publishDiagnostics(uri, text) {
    const diagnostics = diagnosticsFor(text);
    writeFrame({
      jsonrpc: "2.0",
      method: "textDocument/publishDiagnostics",
      params: { uri, diagnostics },
    });
  }

  readFrames((msg) => {
    if (msg.method === "initialize" && msg.id != null) {
      writeFrame({
        jsonrpc: "2.0",
        id: msg.id,
        result: {
          capabilities: {
            textDocumentSync: {
              openClose: true,
              change: 1,
              save: { includeText: true },
            },
            hoverProvider: true,
            definitionProvider: true,
            completionProvider: {
              resolveProvider: false,
              triggerCharacters: [".", ":"],
            },
          },
          serverInfo: { name: "arukellt-extension-e2e-lsp", version: "0.0.0" },
        },
      });
      return;
    }
    if (msg.method === "textDocument/didOpen") {
      const doc = msg.params?.textDocument;
      if (doc?.uri) {
        docs.set(doc.uri, doc.text || "");
        publishDiagnostics(doc.uri, doc.text || "");
      }
      return;
    }
    if (msg.method === "textDocument/didChange") {
      const uri = msg.params?.textDocument?.uri;
      const text = msg.params?.contentChanges?.[0]?.text;
      if (uri && typeof text === "string") {
        docs.set(uri, text);
        publishDiagnostics(uri, text);
      }
      return;
    }
    if (msg.method === "textDocument/didSave") {
      const uri = msg.params?.textDocument?.uri;
      const text = typeof msg.params?.text === "string" ? msg.params.text : docs.get(uri);
      if (uri) {
        publishDiagnostics(uri, text || "");
      }
      return;
    }
    if (msg.method === "textDocument/didClose") {
      const uri = msg.params?.textDocument?.uri;
      if (uri) {
        docs.delete(uri);
        publishDiagnostics(uri, "");
      }
      return;
    }
    if (msg.method === "textDocument/diagnostic" && msg.id != null) {
      const uri = msg.params?.textDocument?.uri;
      writeFrame({
        jsonrpc: "2.0",
        id: msg.id,
        result: { kind: "full", items: diagnosticsFor(docs.get(uri) || "") },
      });
      return;
    }
    if (msg.method === "workspace/diagnostic" && msg.id != null) {
      writeFrame({ jsonrpc: "2.0", id: msg.id, result: { items: [] } });
      return;
    }
    if (msg.method === "shutdown" && msg.id != null) {
      writeFrame({ jsonrpc: "2.0", id: msg.id, result: null });
      return;
    }
    if (msg.method === "textDocument/definition" && msg.id != null) {
      const uri = msg.params?.textDocument?.uri || "file:///basic.ark";
      const pos = msg.params?.position || {};
      if (pos.line === 8) {
        writeFrame({
          jsonrpc: "2.0",
          id: msg.id,
          result: [{ uri, range: { start: { line: 7, character: 8 }, end: { line: 7, character: 14 } } }],
        });
      } else if (pos.line === 7) {
        writeFrame({
          jsonrpc: "2.0",
          id: msg.id,
          result: [{ uri, range: { start: { line: 1, character: 3 }, end: { line: 5, character: 1 } } }],
        });
      } else {
        writeFrame({ jsonrpc: "2.0", id: msg.id, result: [] });
      }
      return;
    }
    if (msg.method === "textDocument/hover" && msg.id != null) {
      const pos = msg.params?.position || {};
      const result = pos.line === 8
        ? { contents: { kind: "markdown", value: "fn println(value: String)" } }
        : null;
      writeFrame({ jsonrpc: "2.0", id: msg.id, result });
      return;
    }
    if (msg.method === "textDocument/completion" && msg.id != null) {
      writeFrame({
        jsonrpc: "2.0",
        id: msg.id,
        result: {
          isIncomplete: false,
          items: [
            { label: "fn", kind: 14 },
            { label: "std::host::stdio", kind: 9 },
            { label: "println", kind: 3 },
          ],
        },
      });
      return;
    }
    if (msg.id != null) {
      writeFrame({ jsonrpc: "2.0", id: msg.id, result: null });
    }
  });
}

function startDap() {
  let seq = 1;
  let program = "main.ark";
  let breakpointLine = 1;
  const response = (request, body = {}) => ({
    seq: seq++,
    type: "response",
    request_seq: request.seq,
    success: true,
    command: request.command,
    body,
  });
  const event = (name, body = {}) => ({ seq: seq++, type: "event", event: name, body });

  readFrames((msg) => {
    if (msg.type !== "request") return;
    if (msg.command === "initialize") {
      writeFrame(response(msg, { supportsConfigurationDoneRequest: true }));
      writeFrame(event("initialized"));
      return;
    }
    if (msg.command === "setBreakpoints") {
      const requested = msg.arguments?.breakpoints || [];
      breakpointLine = requested[0]?.line || breakpointLine;
      writeFrame(response(msg, {
        breakpoints: requested.map((bp) => ({ verified: true, line: bp.line })),
      }));
      return;
    }
    if (msg.command === "launch") {
      program = msg.arguments?.program || program;
      writeFrame(response(msg));
      writeFrame(event("stopped", { reason: "breakpoint", threadId: 1, allThreadsStopped: true }));
      return;
    }
    if (msg.command === "threads") {
      writeFrame(response(msg, { threads: [{ id: 1, name: "main" }] }));
      return;
    }
    if (msg.command === "stackTrace") {
      writeFrame(response(msg, {
        stackFrames: [{
          id: 1,
          name: "main",
          line: breakpointLine,
          column: 1,
          source: { name: program.split(/[\\/]/).pop(), path: program },
        }],
        totalFrames: 1,
      }));
      return;
    }
    if (msg.command === "disconnect" || msg.command === "terminate") {
      writeFrame(response(msg));
      process.exitCode = 0;
      return;
    }
    writeFrame(response(msg));
  });
}

function main() {
  const args = process.argv.slice(2);
  logInvocation(args);
  if (args.includes("--version") || args.includes("-V")) {
    console.log("arukellt extension-e2e 0.0.0");
    return;
  }
  if (args[0] === "lsp") {
    startLsp();
    return;
  }
  if (args[0] === "debug-adapter") {
    startDap();
    return;
  }
  if (args[0] === "check") {
    console.log("check ok");
    return;
  }
  if (args[0] === "fmt" && args[1] === "--check") {
    console.error("formatting drift");
    process.exit(7);
    return;
  }
  if (args[0] === "test" && args.includes("--list") && args.includes("--json")) {
    console.log(JSON.stringify(["test_addition", "test_failure_path"]));
    return;
  }
  if (args[0] === "test" && args.includes("--json")) {
    console.log(JSON.stringify({
      tests: [
        { name: "test_addition", status: "pass" },
        { name: "test_failure_path", status: "fail", message: "expected failure" },
      ],
    }));
    process.exit(1);
    return;
  }
  if (args[0] === "run" || args[0] === "compile") {
    console.log(`${args[0]} ok`);
    return;
  }
  console.error(`unexpected arukellt extension e2e args: ${args.join(" ")}`);
  process.exit(3);
}

main();
