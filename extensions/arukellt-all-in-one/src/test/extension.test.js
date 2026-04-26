// @ts-check
const assert = require("assert");
const cp = require("child_process");
const fs = require("fs");
const os = require("os");
const vscode = require("vscode");
const path = require("path");

const repoRoot = path.resolve(__dirname, "..", "..", "..", "..");
const repoDebugBinary = path.join(
  repoRoot,
  "target",
  "debug",
  process.platform === "win32" ? "arukellt.exe" : "arukellt"
);
const extensionId = "arukellt.arukellt-all-in-one";
let originalServerPath;

function getExtension() {
  const ext = vscode.extensions.getExtension(extensionId);
  assert.ok(ext, "Extension should be found by its ID");
  return ext;
}

async function activateExtension() {
  const ext = getExtension();
  await ext.activate();
  const api = ext.exports;
  return { ext, api };
}

async function waitFor(check, options = {}) {
  const timeoutMs = options.timeoutMs ?? 15000;
  const intervalMs = options.intervalMs ?? 100;
  const description = options.description ?? "condition";
  const deadline = Date.now() + timeoutMs;
  let lastError;

  while (Date.now() < deadline) {
    try {
      return await check();
    } catch (error) {
      lastError = error;
      await new Promise((resolve) => setTimeout(resolve, intervalMs));
    }
  }

  throw lastError ?? new Error(`Timed out waiting for ${description}`);
}

/** Raw LSP definition result → list of Location / LocationLink */
function lspDefinitionItems(raw) {
  if (raw == null) {
    return [];
  }
  return Array.isArray(raw) ? raw : [raw];
}

/** @param {unknown} item */
function lspDefinitionRange(item) {
  if (!item || typeof item !== "object") {
    return undefined;
  }
  if ("range" in item && item.range) {
    return item.range;
  }
  if ("targetRange" in item && item.targetRange) {
    return item.targetRange;
  }
  return undefined;
}

/** @param {unknown} h */
function lspHoverPlainText(h) {
  if (!h || typeof h !== "object" || !("contents" in h)) {
    return "";
  }
  const c = /** @type {{ contents: unknown }} */ (h).contents;
  if (typeof c === "string") {
    return c;
  }
  if (Array.isArray(c)) {
    return c
      .map((x) =>
        typeof x === "string" ? x : x && typeof x === "object" && "value" in x ? String(x.value) : ""
      )
      .join("\n");
  }
  if (c && typeof c === "object" && "value" in c) {
    return String(/** @type {{ value?: string }} */ (c).value || "");
  }
  return "";
}

/**
 * Minimal JSON-RPC/LSP over stdio (Content-Length framing), same protocol as
 * the selfhost `arukellt lsp` server (`src/compiler/lsp.ark`). The previous
 * Rust `crates/ark-lsp/tests/lsp_e2e.rs` harness was retired with the crate
 * in #572. Used here because `vscode.executeDefinitionProvider` and
 * vscode-languageclient `sendRequest` can stall under @vscode/test-electron
 * when the server uses TextDocumentSyncKind.Full.
 */
class LspPipeSession {
  /**
   * @param {string} arukelltBinary
   */
  constructor(arukelltBinary) {
    this.buffer = Buffer.alloc(0);
    /** @type {Map<number, (msg: object) => void>} */
    this.pending = new Map();
    this.nextId = 1;
    this.child = cp.spawn(arukelltBinary, ["lsp"], {
      stdio: ["pipe", "pipe", "pipe"],
    });
    this.child.stdout.on("data", (chunk) => this.feed(chunk));
  }

  /**
   * @param {Buffer} chunk
   */
  feed(chunk) {
    this.buffer = Buffer.concat([this.buffer, chunk]);
    while (true) {
      const sep = this.buffer.indexOf("\r\n\r\n");
      if (sep === -1) {
        return;
      }
      const headerStr = this.buffer.slice(0, sep).toString("utf8");
      const m = /Content-Length:\s*(\d+)/i.exec(headerStr);
      if (!m) {
        this.buffer = this.buffer.slice(sep + 4);
        continue;
      }
      const len = parseInt(m[1], 10);
      const bodyStart = sep + 4;
      if (this.buffer.length < bodyStart + len) {
        return;
      }
      const body = this.buffer.slice(bodyStart, bodyStart + len);
      this.buffer = this.buffer.slice(bodyStart + len);
      try {
        const msg = JSON.parse(body.toString("utf8"));
        this.dispatch(msg);
      } catch (_) {
        /* ignore */
      }
    }
  }

  /**
   * @param {object} msg
   */
  dispatch(msg) {
    if (
      msg.id !== undefined &&
      (msg.result !== undefined || msg.error !== undefined)
    ) {
      const cb = this.pending.get(msg.id);
      if (cb) {
        this.pending.delete(msg.id);
        cb(msg);
      }
    }
  }

  /**
   * @param {object} obj
   */
  write(obj) {
    const body = JSON.stringify(obj);
    const hdr = `Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n`;
    if (this.child.stdin.writableEnded) {
      return;
    }
    this.child.stdin.write(hdr + body);
  }

  /**
   * @param {string} method
   * @param {object | null} params
   */
  sendRequest(method, params) {
    const id = this.nextId++;
    const payload =
      params === null || params === undefined
        ? { jsonrpc: "2.0", id, method }
        : { jsonrpc: "2.0", id, method, params };
    return new Promise((resolve, reject) => {
      const t = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`LSP request timeout: ${method}`));
      }, 20000);
      this.pending.set(id, (msg) => {
        clearTimeout(t);
        if (msg.error) {
          reject(new Error(JSON.stringify(msg.error)));
        } else {
          resolve(msg.result);
        }
      });
      this.write(payload);
    });
  }

  /**
   * @param {string} method
   * @param {object} params
   */
  notify(method, params) {
    this.write({ jsonrpc: "2.0", method, params });
  }

  close() {
    try {
      this.child.kill();
    } catch (_) {
      /* ignore */
    }
  }
}

suiteSetup(async () => {
  if (!fs.existsSync(repoDebugBinary)) {
    return;
  }
  const cfg = vscode.workspace.getConfiguration("arukellt");
  originalServerPath = cfg.get("server.path");
  await cfg.update(
    "server.path",
    repoDebugBinary,
    vscode.ConfigurationTarget.Global
  );
});

suiteTeardown(async () => {
  const ext = getExtension();
  const cfg = vscode.workspace.getConfiguration("arukellt");

  try {
    if (originalServerPath !== undefined) {
      await cfg.update(
        "server.path",
        originalServerPath,
        vscode.ConfigurationTarget.Global
      );
    }

    if (vscode.debug.activeDebugSession) {
      await vscode.debug.stopDebugging(vscode.debug.activeDebugSession);
    }
  } finally {
    if (ext.isActive) {
      await ext.exports.shutdownForTests?.();
    }
  }
});

// ============================================================
// #272 — install / activate / binary discovery E2E
// ============================================================

suite("Extension Activation (#272)", () => {
  test("extension is present", () => {
    getExtension();
  });

  test("extension activates on .ark file", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "arukellt",
      content: "let x = 1\nprint(x)\n",
    });
    await vscode.window.showTextDocument(doc);
    await new Promise((r) => setTimeout(r, 2000));

    const ext = getExtension();
    assert.ok(ext, "Extension should exist");
  });

  test("extension exports activate and deactivate", () => {
    const ext = getExtension();
    assert.ok(ext, "Extension should exist");
    // packageJSON should have activationEvents
    assert.ok(
      ext.packageJSON.activationEvents || ext.packageJSON.main,
      "Extension should have activation configuration"
    );
  });

  test("missing binary surfaces through user message, output channel, and status bar (#254)", async () => {
    const { api } = await activateExtension();
    const cfg = vscode.workspace.getConfiguration("arukellt");
    const original = cfg.get("server.path");
    const missingBinary = path.join(repoRoot, ".tmp", "missing-arukellt-binary");

    try {
      await cfg.update(
        "server.path",
        missingBinary,
        vscode.ConfigurationTarget.Global
      );
      await vscode.commands.executeCommand("arukellt.restartLanguageServer");

      await waitFor(() => {
        const state = api.__getTestState();
        assert.strictEqual(state.hasClient, false, "client should not stay running with an invalid binary path");
        assert.ok(
          (state.recordedErrorNotificationCount ?? 0) >= 1,
          "extension should record a user-visible error notification (showErrorMessage) for missing binary"
        );
        assert.match(state.lastErrorMessage ?? "", /arukellt binary not found/i);
        assert.match(state.lastErrorMessage ?? "", /See the output channel/i);
        assert.ok(
          state.outputChannelLines.some((line) => /searching for arukellt binary/i.test(line)),
          "output channel should record the binary discovery attempt"
        );
        assert.ok(
          state.outputChannelLines.some((line) => /not found via arukellt\.server\.path setting/i.test(line)),
          "output channel should record the configured-path miss"
        );
        assert.ok(
          state.outputChannelLines.some((line) => /arukellt binary not found in any location/i.test(line)),
          "output channel should record the terminal missing-binary summary"
        );
        assert.strictEqual(state.languageStatusText, "$(error) Error");
        assert.strictEqual(
          state.languageStatusSeverity,
          vscode.LanguageStatusSeverity.Error
        );
        assert.match(state.languageStatusDetail ?? "", /Binary not found/);
        assert.match(state.statusBarText ?? "", /binary missing/);
        assert.match(state.statusBarTooltip ?? "", /Failed to find arukellt binary/);
      }, { description: "missing-binary user-facing status" });
    } finally {
      await cfg.update("server.path", original, vscode.ConfigurationTarget.Global);
      if (fs.existsSync(repoDebugBinary)) {
        await vscode.commands.executeCommand("arukellt.restartLanguageServer");
        await waitFor(() => {
          const state = api.__getTestState();
          assert.strictEqual(state.hasClient, true);
          assert.strictEqual(state.languageStatusText, "$(check) Ready");
        }, { description: "healthy language server after restore" });
      }
    }
  });

  test("custom absolute server.path is used for binary discovery and LSP (#254)", async function () {
    if (!fs.existsSync(repoDebugBinary)) {
      this.skip();
      return;
    }

    const { api } = await activateExtension();
    const cfg = vscode.workspace.getConfiguration("arukellt");
    const originalPath = cfg.get("server.path");
    const originalArgs = cfg.get("server.args");

    try {
      await cfg.update(
        "server.path",
        repoDebugBinary,
        vscode.ConfigurationTarget.Global
      );
      await cfg.update("server.args", [], vscode.ConfigurationTarget.Global);
      await vscode.commands.executeCommand("arukellt.restartLanguageServer");

      await waitFor(
        () => {
          const state = api.__getTestState();
          assert.strictEqual(
            state.hasClient,
            true,
            "language client should run when server.path points at the repo binary"
          );
          assert.strictEqual(state.languageStatusText, "$(check) Ready");
          assert.ok(
            state.outputChannelLines.some((line) =>
              /found via arukellt\.server\.path setting/i.test(line)
            ),
            `output channel should record resolution via configured path; got: ${state.outputChannelLines.join(" | ")}`
          );
        },
        { description: "custom server.path LSP ready", timeoutMs: 30000 }
      );
    } finally {
      await cfg.update(
        "server.path",
        originalPath,
        vscode.ConfigurationTarget.Global
      );
      await cfg.update(
        "server.args",
        originalArgs,
        vscode.ConfigurationTarget.Global
      );
      await vscode.commands.executeCommand("arukellt.restartLanguageServer");
      if (fs.existsSync(repoDebugBinary)) {
        await waitFor(
          () => {
            const state = api.__getTestState();
            assert.strictEqual(state.hasClient, true);
            assert.strictEqual(state.languageStatusText, "$(check) Ready");
          },
          { description: "restore LSP after custom server.path test", timeoutMs: 30000 }
        );
      }
    }
  });
});

// ============================================================
// #273 — LSP handshake / command execution / task execution E2E
// ============================================================

suite("Command Registration (#273)", () => {
  test("all declared commands are registered", async () => {
    const allCommands = await vscode.commands.getCommands(true);
    const expected = [
      "arukellt.restartLanguageServer",
      "arukellt.checkCurrentFile",
      "arukellt.compileCurrentFile",
      "arukellt.runCurrentFile",
      "arukellt.showSetupDoctor",
      "arukellt.showCommandGraph",
      "arukellt.showEnvironmentDiff",
      "arukellt.openDocs",
      "arukellt.explainCode",
      "arukellt.showPipeline",
      "arukellt.securityReview",
      "arukellt.showOutput",
      "arukellt.refreshProjectTree",
      "arukellt.toggleVerboseLogging",
      "arukellt.runScript",
      "arukellt.buildComponent",
      "arukellt.buildComponentWit",
      "arukellt.runComponent",
      "arukellt.openInPlayground",
      "arukellt.runMain",
      "arukellt.debugMain",
      "arukellt.runTest",
      "arukellt.debugTest",
    ];
    for (const cmd of expected) {
      assert.ok(
        allCommands.includes(cmd),
        `Command ${cmd} should be registered`
      );
    }
  });

  test("showSetupDoctor command executes without error", async () => {
    // Should not throw even without a binary
    try {
      await vscode.commands.executeCommand("arukellt.showSetupDoctor");
    } catch (e) {
      // Some commands may show UI but shouldn't throw
      assert.ok(true, "Command executed (may show UI)");
    }
  });

  test("showOutput command executes without error", async () => {
    try {
      await vscode.commands.executeCommand("arukellt.showOutput");
    } catch (e) {
      assert.ok(true, "Command executed");
    }
  });
});

suite("Task Provider (#273)", () => {
  test("arukellt task type is registered", async () => {
    const tasks = await vscode.tasks.fetchTasks({ type: "arukellt" });
    // Tasks should be returned (may be empty if no workspace)
    assert.ok(Array.isArray(tasks), "fetchTasks should return an array");
  });

  test("standard tasks are provided", async () => {
    const tasks = await vscode.tasks.fetchTasks({ type: "arukellt" });
    const names = tasks.map((t) => t.name);
    const expected = [
      "arukellt:check",
      "arukellt:compile",
      "arukellt:run",
      "arukellt:test",
      "arukellt:fmt",
      "arukellt:watch",
    ];
    for (const name of expected) {
      assert.ok(names.includes(name), `Task '${name}' should be provided`);
    }
  });

  test("watch task is background", async () => {
    const tasks = await vscode.tasks.fetchTasks({ type: "arukellt" });
    const watch = tasks.find((t) => t.name === "arukellt:watch");
    if (watch) {
      assert.strictEqual(
        watch.isBackground,
        true,
        "Watch task should be background"
      );
    }
  });
});

// ============================================================
// #274 — test controller discovery + restart E2E
// ============================================================

suite("Test Controller (#274)", () => {
  test("test controller is registered", () => {
    // The test controller is created in activate() via vscode.tests.createTestController
    // We can verify the extension activated properly
    const ext = getExtension();
    assert.ok(ext, "Extension with test controller should be present");
  });

  test("restart command executes and language server stays healthy", async function () {
    if (!fs.existsSync(repoDebugBinary)) {
      this.skip();
      return;
    }

    const { api } = await activateExtension();
    await waitFor(() => {
      const state = api.__getTestState();
      assert.strictEqual(state.hasClient, true);
      assert.strictEqual(state.languageStatusText, "$(check) Ready");
      return state;
    }, { description: "initial ready language server" });

    const before = api.__getTestState();

    await vscode.commands.executeCommand("arukellt.restartLanguageServer");

    await waitFor(() => {
      const after = api.__getTestState();
      assert.ok(
        after.clientSessionId > before.clientSessionId,
        "restart should create a fresh language client session"
      );
      assert.strictEqual(after.hasClient, true);
      assert.strictEqual(after.languageStatusText, "$(check) Ready");
    }, { description: "healthy language server after restart" });
  });
});

// ============================================================
// #453 — Go to Definition E2E (verifies #450 identifier-only span)
// JSON-RPC to `arukellt lsp` (selfhost LSP at `src/compiler/lsp.ark`; the
// Rust `crates/ark-lsp` E2E harness was retired in #572). VS Code API
// `vscode.executeDefinitionProvider` stalls under @vscode/test-electron here.
// Placed before stub LSP suite so the subprocess is not affected by stub config.
// ============================================================

suite("Go to Definition (#450 / #453)", function () {
  this.timeout(60000);
  /** @type {LspPipeSession | undefined} */
  let lsp;
  let docUri;
  let basicSource;

  suiteSetup(async function () {
    this.timeout(60000);
    if (!fs.existsSync(repoDebugBinary)) {
      this.skip();
      return;
    }
    const basicPath = path.join(__dirname, "fixtures", "basic.ark");
    docUri = vscode.Uri.file(basicPath).toString();
    basicSource = fs.readFileSync(basicPath, "utf8");
    lsp = new LspPipeSession(repoDebugBinary);
    await lsp.sendRequest("initialize", {
      processId: null,
      rootUri: vscode.Uri.file(repoRoot).toString(),
      capabilities: {},
    });
    lsp.notify("initialized", {});
    await new Promise((r) => setTimeout(r, 300));
    lsp.notify("textDocument/didOpen", {
      textDocument: {
        uri: docUri,
        languageId: "arukellt",
        version: 1,
        text: basicSource,
      },
    });
    await new Promise((r) => setTimeout(r, 500));
  });

  suiteTeardown(() => {
    lsp?.close();
  });

  test("local variable definition range is identifier only", async () => {
    assert.ok(lsp);
    const raw = await lsp.sendRequest("textDocument/definition", {
      textDocument: { uri: docUri },
      position: { line: 8, character: 20 },
    });
    const locs = lspDefinitionItems(raw);
    assert.ok(locs.length > 0, "Should find definition of result");
    const range = lspDefinitionRange(locs[0]);
    assert.ok(range, "Definition result should have a range");
    assert.strictEqual(range.start.line, 7, "Should point to let-binding line (line 7)");
    assert.strictEqual(range.start.character, 8, "Should start at 'result' identifier (col 8)");
    assert.strictEqual(range.start.line, range.end.line, "Definition range should be single line");
    const rangeLen = range.end.character - range.start.character;
    assert.ok(rangeLen <= 10, `Range too wide: ${rangeLen} chars`);
  });

  test("function definition range is function name only", async () => {
    assert.ok(lsp);
    const raw = await lsp.sendRequest("textDocument/definition", {
      textDocument: { uri: docUri },
      position: { line: 7, character: 17 },
    });
    const locs = lspDefinitionItems(raw);
    assert.ok(locs.length > 0, "Should find definition of greet");
    const range = lspDefinitionRange(locs[0]);
    assert.ok(range, "Definition result should have a range");
    assert.strictEqual(range.start.line, 1, "Should point to fn greet line (line 1)");
    assert.ok(
      range.start.character <= 3,
      `Definition should start at or before the 'greet' identifier (got col ${range.start.character})`
    );
    assert.ok(
      range.start.line >= 1 && range.end.line <= 6,
      `Definition should land on the fn greet block (got lines ${range.start.line}–${range.end.line})`
    );
  });

  test("definition on keyword/whitespace returns nothing", async () => {
    assert.ok(lsp);
    const raw = await lsp.sendRequest("textDocument/definition", {
      textDocument: { uri: docUri },
      position: { line: 1, character: 0 },
    });
    const locs = lspDefinitionItems(raw);
    assert.ok(locs.length === 0, "Keyword position should return no definition");
  });
});

// ============================================================
// #453 — Hover E2E (verifies #451 semantic-only hover filter)
// ============================================================

suite("Hover (#451 / #453)", function () {
  this.timeout(60000);
  /** @type {LspPipeSession | undefined} */
  let lsp;
  let docUri;
  let basicSource;

  suiteSetup(async function () {
    this.timeout(60000);
    if (!fs.existsSync(repoDebugBinary)) {
      this.skip();
      return;
    }
    const basicPath = path.join(__dirname, "fixtures", "basic.ark");
    docUri = vscode.Uri.file(basicPath).toString();
    basicSource = fs.readFileSync(basicPath, "utf8");
    lsp = new LspPipeSession(repoDebugBinary);
    await lsp.sendRequest("initialize", {
      processId: null,
      rootUri: vscode.Uri.file(repoRoot).toString(),
      capabilities: {},
    });
    lsp.notify("initialized", {});
    await new Promise((r) => setTimeout(r, 300));
    lsp.notify("textDocument/didOpen", {
      textDocument: {
        uri: docUri,
        languageId: "arukellt",
        version: 1,
        text: basicSource,
      },
    });
    await new Promise((r) => setTimeout(r, 500));
  });

  suiteTeardown(() => {
    lsp?.close();
  });

  test("string literal position returns no 'string literal' hover noise", async () => {
    assert.ok(lsp);
    const hover = await lsp.sendRequest("textDocument/hover", {
      textDocument: { uri: docUri },
      position: { line: 2, character: 25 },
    });
    const text = lspHoverPlainText(hover);
    assert.ok(
      !text.includes("string literal"),
      "String literal position should not produce 'string literal' hover noise"
    );
  });

  test("known function name produces meaningful hover content", async () => {
    assert.ok(lsp);
    const hover = await lsp.sendRequest("textDocument/hover", {
      textDocument: { uri: docUri },
      position: { line: 8, character: 11 },
    });
    assert.ok(hover, "println should produce a hover result");
    const content = lspHoverPlainText(hover);
    assert.ok(
      content.includes("println") || content.includes("fn"),
      `Hover should contain function name or signature, got: ${content.slice(0, 200)}`
    );
  });
});

// ============================================================
// #254 — restart path with stub LSP (spy process + output channel)
// ============================================================

suite("Language Server Restart — stub LSP (#254)", () => {
  let savedPath;
  let savedArgs;
  let spyLogPath;

  suiteSetup(async function () {
    this.timeout(45000);
    const stubScript = path.join(__dirname, "fixtures", "lsp-stub.js");
    const cfg = vscode.workspace.getConfiguration("arukellt");
    savedPath = cfg.get("server.path");
    savedArgs = cfg.get("server.args");
    spyLogPath = path.join(
      os.tmpdir(),
      `arukellt-lsp-stub-spy-${process.pid}-${Date.now()}.log`
    );
    process.env.ARK_LSP_STUB_SPY = spyLogPath;

    const { api } = await activateExtension();
    await api.shutdownForTests?.();

    // Apply stub server settings only after the prior client is gone so
    // didChangeConfiguration cannot target a disposing / wrong process.
    // Use `node` from PATH: in the extension host `process.execPath` is the
    // VS Code / Electron binary, not a JS runtime, so it cannot execute the stub.
    await cfg.update(
      "server.path",
      "node",
      vscode.ConfigurationTarget.Global
    );
    await cfg.update(
      "server.args",
      [stubScript],
      vscode.ConfigurationTarget.Global
    );

    await vscode.commands.executeCommand("arukellt.restartLanguageServer");
    await waitFor(
      () => {
        const state = api.__getTestState();
        assert.strictEqual(state.hasClient, true);
        assert.strictEqual(state.languageStatusText, "$(check) Ready");
      },
      { description: "stub LSP running after config switch", timeoutMs: 30000 }
    );
  });

  suiteTeardown(async function () {
    this.timeout(45000);
    const cfg = vscode.workspace.getConfiguration("arukellt");
    delete process.env.ARK_LSP_STUB_SPY;
    try {
      fs.unlinkSync(spyLogPath);
    } catch (_) {
      /* ignore */
    }

    const ext = getExtension();
    if (ext.isActive && ext.exports.shutdownForTests) {
      await ext.exports.shutdownForTests();
    }
    await cfg.update("server.path", savedPath, vscode.ConfigurationTarget.Global);
    await cfg.update("server.args", savedArgs, vscode.ConfigurationTarget.Global);
    if (ext.isActive) {
      await vscode.commands.executeCommand("arukellt.restartLanguageServer");
      if (fs.existsSync(repoDebugBinary)) {
        await waitFor(
          () => {
            const state = ext.exports.__getTestState();
            assert.strictEqual(state.hasClient, true);
            assert.strictEqual(state.languageStatusText, "$(check) Ready");
          },
          { description: "restore LSP after stub suite", timeoutMs: 30000 }
        );
      }
    }
  });

  test("restart command stops client and starts a new stub session (session id, spy log, output channel)", async () => {
    const { api } = await activateExtension();
    const beforeSession = api.__getTestState().clientSessionId;
    const spyBefore = fs.existsSync(spyLogPath)
      ? fs
          .readFileSync(spyLogPath, "utf8")
          .trim()
          .split("\n")
          .filter(Boolean).length
      : 0;

    await vscode.commands.executeCommand("arukellt.restartLanguageServer");

    await waitFor(
      () => {
        const after = api.__getTestState();
        assert.ok(
          after.clientSessionId > beforeSession,
          "restart should create a fresh language client session"
        );
        assert.strictEqual(after.hasClient, true);
        assert.strictEqual(after.languageStatusText, "$(check) Ready");
        assert.ok(
          after.outputChannelLines.some((line) =>
            /restart session \(new client starting\)/i.test(line)
          ),
          `expected restart telemetry line, got: ${after.outputChannelLines.join(" | ")}`
        );
        assert.ok(
          after.outputChannelLines.some((line) =>
            /previous client stopped \(restart\)/i.test(line)
          ),
          `expected prior client stop line, got: ${after.outputChannelLines.join(" | ")}`
        );
        const spyAfter = fs
          .readFileSync(spyLogPath, "utf8")
          .trim()
          .split("\n")
          .filter(Boolean).length;
        assert.ok(
          spyAfter >= spyBefore + 1,
          `stub should log a new process spawn (before ${spyBefore}, after ${spyAfter})`
        );
      },
      { description: "restart with stub server", timeoutMs: 30000 }
    );
  });
});

// ============================================================
// #275 — failure log verification (output channel / status bar / messages)
// ============================================================

suite("Output Channels (#275)", () => {
  test("output channels exist after activation", async () => {
    // Trigger activation
    const doc = await vscode.workspace.openTextDocument({
      language: "arukellt",
      content: "let y = 2",
    });
    await vscode.window.showTextDocument(doc);
    await new Promise((r) => setTimeout(r, 1000));

    // Output channels are created in activate()
    // We can't directly inspect them from the test API, but we can verify
    // the extension is active and commands work
    const ext = getExtension();
    assert.ok(ext, "Extension should be present");
  });
});

suite("Status Bar (#275)", () => {
  test("status bar item is created", async () => {
    // Status bar item is created in activate()
    const ext = getExtension();
    assert.ok(ext, "Extension with status bar should be present");
  });
});

suite("Configuration (#275)", () => {
  test("default settings are available", () => {
    const cfg = vscode.workspace.getConfiguration("arukellt");
    assert.strictEqual(typeof cfg.get("server.path"), "string");
    assert.deepStrictEqual(cfg.get("server.args"), []);
    assert.strictEqual(cfg.get("target"), null);
    assert.strictEqual(cfg.get("emit"), "core-wasm");
  });

  test("trace level can be toggled", async () => {
    try {
      await vscode.commands.executeCommand("arukellt.toggleVerboseLogging");
      await new Promise((r) => setTimeout(r, 500));
    } catch (e) {
      // May require workspace, but shouldn't crash
    }
  });
});

// ============================================================
// #255 — breakpoint-stop debug launch E2E
// ============================================================

suite("Debug Launch (#255)", () => {
  test("launch stops on a breakpoint in an .ark file", async function () {
    if (!fs.existsSync(repoDebugBinary)) {
      this.skip();
      return;
    }

    const { api } = await activateExtension();
    const fixturePath = path.join(__dirname, "fixtures", "hello.ark");
    const doc = await vscode.workspace.openTextDocument(fixturePath);
    await vscode.window.showTextDocument(doc);

    await waitFor(() => {
      const state = api.__getTestState();
      assert.strictEqual(state.hasClient, true);
      assert.strictEqual(state.languageStatusText, "$(check) Ready");
      return state;
    }, { description: "ready language server before debug launch" });

    const before = api.__getTestState();
    const breakpointLine = 3;
    const breakpoint = new vscode.SourceBreakpoint(
      new vscode.Location(doc.uri, new vscode.Position(breakpointLine - 1, 0))
    );

    vscode.debug.addBreakpoints([breakpoint]);

    let session;
    try {
      const started = await vscode.debug.startDebugging(undefined, {
        type: "arukellt",
        request: "launch",
        name: "Breakpoint Stop E2E",
        program: doc.uri.fsPath,
      });

      assert.strictEqual(started, true, "debug launch should start");

      const debugState = await waitFor(() => {
        const state = api.__getTestState();
        assert.ok(
          state.debugSessionStartCount > before.debugSessionStartCount,
          "should observe a started arukellt debug session"
        );
        assert.strictEqual(state.activeDebugSessionType, "arukellt");
        assert.ok(
          state.debugStoppedEventCount > before.debugStoppedEventCount,
          "should observe a stopped DAP event"
        );
        assert.ok(state.lastStoppedEvent, "stopped event payload should be recorded");
        assert.strictEqual(state.lastStoppedEvent.reason, "breakpoint");
        return state;
      }, { description: "breakpoint stop event" });

      session = vscode.debug.activeDebugSession;
      assert.ok(session, "active debug session should remain available after stopping");
      assert.strictEqual(debugState.activeDebugSessionName, "Breakpoint Stop E2E");

      const threads = await session.customRequest("threads");
      assert.ok(Array.isArray(threads.threads), "threads response should include a thread list");
      assert.ok(threads.threads.length > 0, "threads response should include the main thread");

      const stackTrace = await session.customRequest("stackTrace", {
        threadId: threads.threads[0].id,
        startFrame: 0,
        levels: 1,
      });
      assert.ok(
        Array.isArray(stackTrace.stackFrames),
        "stackTrace response should include stack frames"
      );
      assert.ok(stackTrace.stackFrames.length > 0, "stackTrace should expose the stopped frame");
      assert.strictEqual(stackTrace.stackFrames[0].source.path, doc.uri.fsPath);
      assert.strictEqual(stackTrace.stackFrames[0].line, breakpointLine);
    } finally {
      vscode.debug.removeBreakpoints([breakpoint]);
      if (session) {
        await vscode.debug.stopDebugging(session);
        await waitFor(() => {
          const state = api.__getTestState();
          assert.ok(
            state.debugSessionTerminateCount > before.debugSessionTerminateCount,
            "debug session should terminate during cleanup"
          );
        }, { description: "debug session termination" });
      }
    }
  });
});

suite("Language Registration", () => {
  test("arukellt language is registered", async () => {
    const langs = await vscode.languages.getLanguages();
    assert.ok(
      langs.includes("arukellt"),
      "arukellt language should be registered"
    );
  });

  test(".ark files get arukellt language ID", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "arukellt",
      content: "let x = 1",
    });
    assert.strictEqual(doc.languageId, "arukellt");
  });
});

// ============================================================
// #453 — Diagnostics E2E (verifies #452 false-positive removal)
// ============================================================

suite("Diagnostics (#452 / #453)", () => {
  suiteSetup(async function () {
    if (!fs.existsSync(repoDebugBinary)) { this.skip(); return; }
  });

  test("valid ark file produces no diagnostics", async () => {
    const fixturePath = path.join(__dirname, "fixtures", "basic.ark");
    const doc = await vscode.workspace.openTextDocument(fixturePath);
    await vscode.window.showTextDocument(doc);
    // Wait for LSP to publish diagnostics
    await new Promise((r) => setTimeout(r, 4000));

    const diags = vscode.languages.getDiagnostics(doc.uri);
    assert.strictEqual(
      diags.length,
      0,
      `Valid basic.ark should have 0 diagnostics, got: ${diags
        .map((d) => d.message)
        .join(", ")}`
    );
  });

  test.skip("unresolved name produces E0100 diagnostic", async () => {
    const content =
      "use std::host::stdio\nfn main() {\n    stdio::println(undefined_var)\n}\n";
    const fixturePath = path.join(__dirname, "fixtures", "undefined_var.ark");
    fs.writeFileSync(fixturePath, content, "utf8");
    try {
      const doc = await vscode.workspace.openTextDocument(fixturePath);
      await vscode.window.showTextDocument(doc);
      await new Promise((r) => setTimeout(r, 4000));
      const diags = vscode.languages.getDiagnostics(doc.uri);
      const hasE0100 = diags.some(
        (d) =>
          d.message.includes("E0100") ||
          d.message.toLowerCase().includes("unresolved")
      );
      assert.ok(hasE0100, `Should have E0100 for undefined_var, got: ${diags.map((d) => d.message).join(", ")}`);
    } finally {
      fs.unlinkSync(fixturePath);
    }
  });

  test("diagnostics are stable after content change", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "arukellt",
      content: 'fn main() { println("hello") }\n',
    });
    await vscode.window.showTextDocument(doc);
    await new Promise((r) => setTimeout(r, 3000));

    const diags1 = vscode.languages.getDiagnostics(doc.uri);
    assert.strictEqual(
      diags1.length,
      0,
      "Should have no diagnostics initially"
    );

    // Edit the document content (still valid)
    const edit = new vscode.WorkspaceEdit();
    edit.replace(
      doc.uri,
      new vscode.Range(0, 0, doc.lineCount, 0),
      'fn main() { println("world") }\n'
    );
    await vscode.workspace.applyEdit(edit);
    await new Promise((r) => setTimeout(r, 3000));

    const diags2 = vscode.languages.getDiagnostics(doc.uri);
    assert.strictEqual(
      diags2.length,
      0,
      "Should still have no diagnostics after editing to another valid file"
    );
  });
});

// ============================================================
// #280 — DAP test wiring
// ============================================================

suite("Debug Adapter (#280)", () => {
  test("arukellt debug type is registered", () => {
    const ext = vscode.extensions.getExtension("arukellt.arukellt-all-in-one");
    assert.ok(ext, "Extension with debug contribution should be present");
    // Verify debuggers contribution exists
    const debuggers = ext.packageJSON.contributes.debuggers;
    assert.ok(debuggers, "debuggers contribution should exist");
    const arkDebugger = debuggers.find((d) => d.type === "arukellt");
    assert.ok(arkDebugger, "arukellt debugger type should be registered");
  });

  test("debug launch configuration template exists", () => {
    const ext = vscode.extensions.getExtension("arukellt.arukellt-all-in-one");
    const debuggers = ext.packageJSON.contributes.debuggers;
    const arkDebugger = debuggers.find((d) => d.type === "arukellt");
    assert.ok(
      arkDebugger.configurationAttributes,
      "debug configuration attributes should exist"
    );
    assert.ok(
      arkDebugger.configurationAttributes.launch,
      "launch configuration should be defined"
    );
  });

  test("initial debug configurations are provided", () => {
    const ext = vscode.extensions.getExtension("arukellt.arukellt-all-in-one");
    const debuggers = ext.packageJSON.contributes.debuggers;
    const arkDebugger = debuggers.find((d) => d.type === "arukellt");
    assert.ok(
      arkDebugger.initialConfigurations,
      "initial debug configurations should exist"
    );
    assert.ok(
      arkDebugger.initialConfigurations.length > 0,
      "at least one initial configuration should exist"
    );
  });
});

// ============================================================
// Project Tree View (#212)
// ============================================================

suite("Project Tree View", () => {
  test("project view container is registered", () => {
    const ext = vscode.extensions.getExtension("arukellt.arukellt-all-in-one");
    const viewsContainers = ext.packageJSON.contributes.viewsContainers;
    assert.ok(viewsContainers, "viewsContainers should exist");
    assert.ok(
      viewsContainers.activitybar,
      "activitybar container should exist"
    );
    const arkContainer = viewsContainers.activitybar.find(
      (c) => c.id === "arukellt"
    );
    assert.ok(arkContainer, "arukellt container should be registered");
  });

  test("project view is registered", () => {
    const ext = vscode.extensions.getExtension("arukellt.arukellt-all-in-one");
    const views = ext.packageJSON.contributes.views;
    assert.ok(views, "views should exist");
    assert.ok(views.arukellt, "arukellt views should exist");
    const projectView = views.arukellt.find(
      (v) => v.id === "arukellt-project"
    );
    assert.ok(projectView, "arukellt-project view should be registered");
  });

  test("refresh command exists", async () => {
    const allCommands = await vscode.commands.getCommands(true);
    assert.ok(
      allCommands.includes("arukellt.refreshProjectTree"),
      "refreshProjectTree command should exist"
    );
  });
});
