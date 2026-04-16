#!/usr/bin/env node
/**
 * Minimal JSON-RPC LSP over stdio for VS Code extension E2E tests.
 * Invoked as: node lsp-stub.js lsp [--stdio] (vscode-languageclient appends --stdio).
 * Spy: set ARK_LSP_STUB_SPY to a file path; each session appends one line.
 */
"use strict";

const fs = require("fs");

function appendSpy() {
  const p = process.env.ARK_LSP_STUB_SPY;
  if (!p) return;
  try {
    fs.appendFileSync(p, `${Date.now()} ${process.pid} lsp-session\n`);
  } catch (_) {
    /* ignore */
  }
}

function sendMessage(obj) {
  const body = JSON.stringify(obj);
  const header = `Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n`;
  process.stdout.write(header + body);
}

function handleMessage(msg) {
  if (msg && msg.method === "initialize" && msg.id != null) {
    sendMessage({
      jsonrpc: "2.0",
      id: msg.id,
      result: {
        capabilities: {
          textDocumentSync: 0,
        },
        serverInfo: { name: "arukellt-lsp-stub", version: "0-e2e" },
      },
    });
    return;
  }
  if (msg && msg.method === "shutdown" && msg.id != null) {
    sendMessage({ jsonrpc: "2.0", id: msg.id, result: null });
    return;
  }
  if (msg && msg.method === "workspace/configuration" && msg.id != null) {
    sendMessage({ jsonrpc: "2.0", id: msg.id, result: [] });
    return;
  }
  if (msg && Object.prototype.hasOwnProperty.call(msg, "id") && msg.id != null) {
    sendMessage({ jsonrpc: "2.0", id: msg.id, result: {} });
  }
}

function main() {
  const argv = process.argv.slice(2);
  // vscode-languageclient always appends `--stdio` after our args when
  // transport is TransportKind.stdio, so argv is e.g. [stub.js, lsp, --stdio].
  if (!argv.includes("lsp")) {
    process.stderr.write(
      "lsp-stub: expected 'lsp' in argv (language client adds --stdio after it)\n"
    );
    process.exit(1);
  }

  appendSpy();

  let buf = Buffer.alloc(0);
  process.stdin.on("data", (chunk) => {
    buf = Buffer.concat([buf, chunk]);
    while (true) {
      const sep = buf.indexOf("\r\n\r\n");
      if (sep === -1) break;
      const headerText = buf.slice(0, sep).toString("utf8");
      const m = /Content-Length:\s*(\d+)/i.exec(headerText);
      if (!m) {
        buf = buf.slice(sep + 4);
        continue;
      }
      const len = parseInt(m[1], 10);
      const total = sep + 4 + len;
      if (buf.length < total) break;
      const body = buf.slice(sep + 4, total).toString("utf8");
      buf = buf.slice(total);
      try {
        handleMessage(JSON.parse(body));
      } catch (_) {
        /* ignore malformed */
      }
    }
  });

  process.stdin.on("end", () => process.exit(0));
}

main();
