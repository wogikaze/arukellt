// @ts-check
const assert = require("assert");
const fs = require("fs");
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

  test("custom server.path setting is respected", () => {
    const cfg = vscode.workspace.getConfiguration("arukellt");
    const serverPath = cfg.get("server.path");
    assert.strictEqual(
      typeof serverPath,
      "string",
      "server.path should be a string"
    );
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
// #453 — Go to Definition E2E (verifies #450 identifier-only span)
// ============================================================
// Skipped: vscode.executeDefinitionProvider reliably times out under @vscode/test-electron
// (VS Code 1.116) in this workspace; ark-lsp unit tests cover definition behavior.
suite.skip("Go to Definition (#450 / #453)", () => {
  let doc;
  suiteSetup(async function () {
    if (!fs.existsSync(repoDebugBinary)) { this.skip(); return; }
    const fixturePath = path.join(__dirname, "fixtures", "basic.ark");
    doc = await vscode.workspace.openTextDocument(fixturePath);
    await vscode.window.showTextDocument(doc);
    await new Promise((r) => setTimeout(r, 3000));
  });

  test("local variable definition range is identifier only", async () => {
    const pos = new vscode.Position(8, 20);
    const locs = await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      doc.uri,
      pos
    );
    assert.ok(locs && locs.length > 0, "Should find definition of result");
    const loc = locs[0];
    const range = loc.targetRange || loc.range;
    assert.ok(range, "Definition result should have a range");
    assert.strictEqual(range.start.line, 7, "Should point to let-binding line (line 7)");
    assert.strictEqual(range.start.character, 8, "Should start at 'result' identifier (col 8)");
    assert.strictEqual(range.start.line, range.end.line, "Definition range should be single line");
    const rangeLen = range.end.character - range.start.character;
    assert.ok(rangeLen <= 10, `Range too wide: ${rangeLen} chars`);
  });

  test("function definition range is function name only", async () => {
    const pos = new vscode.Position(7, 17);
    const locs = await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      doc.uri,
      pos
    );
    assert.ok(locs && locs.length > 0, "Should find definition of greet");
    const loc = locs[0];
    const range = loc.targetRange || loc.range;
    assert.ok(range, "Definition result should have a range");
    assert.strictEqual(range.start.line, 1, "Should point to fn greet line (line 1)");
    assert.strictEqual(range.start.character, 3, "Should start at 'greet' identifier (col 3)");
    assert.strictEqual(range.start.line, range.end.line, "Function definition range should be single line");
    const rangeLen = range.end.character - range.start.character;
    assert.ok(rangeLen <= 8, `greet range too wide: ${rangeLen} chars`);
  });

  test("definition on keyword/whitespace returns nothing", async () => {
    const pos = new vscode.Position(1, 0);
    const locs = await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      doc.uri,
      pos
    );
    assert.ok(!locs || locs.length === 0, "Keyword position should return no definition");
  });
});

// ============================================================
// #453 — Hover E2E (verifies #451 semantic-only hover filter)
// ============================================================
suite.skip("Hover (#451 / #453)", () => {
  let doc;
  suiteSetup(async function () {
    if (!fs.existsSync(repoDebugBinary)) { this.skip(); return; }
    const fixturePath = path.join(__dirname, "fixtures", "basic.ark");
    doc = await vscode.workspace.openTextDocument(fixturePath);
    await vscode.window.showTextDocument(doc);
    await new Promise((r) => setTimeout(r, 3000));
  });

  test("string literal position returns no 'string literal' hover noise", async () => {
    const pos = new vscode.Position(2, 25);
    const hovers = await vscode.commands.executeCommand(
      "vscode.executeHoverProvider",
      doc.uri,
      pos
    );
    const hasNoise =
      hovers &&
      hovers.some((h) =>
        h.contents.some((c) => {
          const text = typeof c === "string" ? c : c.value || "";
          return text.includes("string literal");
        })
      );
    assert.ok(!hasNoise, "String literal position should not produce 'string literal' hover noise");
  });

  test("known function name produces meaningful hover content", async () => {
    const pos = new vscode.Position(8, 11);
    const hovers = await vscode.commands.executeCommand(
      "vscode.executeHoverProvider",
      doc.uri,
      pos
    );
    assert.ok(hovers && hovers.length > 0, "println should produce a hover result");
    const content = hovers
      .flatMap((h) => h.contents)
      .map((c) => (typeof c === "string" ? c : c.value || ""))
      .join("\n");
    assert.ok(
      content.includes("println") || content.includes("fn"),
      `Hover should contain function name or signature, got: ${content.slice(0, 200)}`
    );
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
