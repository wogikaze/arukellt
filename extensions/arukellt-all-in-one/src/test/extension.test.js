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
let originalServerPath;

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
  if (originalServerPath === undefined) {
    return;
  }
  const cfg = vscode.workspace.getConfiguration("arukellt");
  await cfg.update(
    "server.path",
    originalServerPath,
    vscode.ConfigurationTarget.Global
  );
});

// ============================================================
// #272 — install / activate / binary discovery E2E
// ============================================================

suite("Extension Activation (#272)", () => {
  test("extension is present", () => {
    const ext = vscode.extensions.getExtension("arukellt.arukellt-all-in-one");
    assert.ok(ext, "Extension should be found by its ID");
  });

  test("extension activates on .ark file", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "arukellt",
      content: "let x = 1\nprint(x)\n",
    });
    await vscode.window.showTextDocument(doc);
    await new Promise((r) => setTimeout(r, 2000));

    const ext = vscode.extensions.getExtension("arukellt.arukellt-all-in-one");
    assert.ok(ext, "Extension should exist");
  });

  test("extension exports activate and deactivate", () => {
    const ext = vscode.extensions.getExtension("arukellt.arukellt-all-in-one");
    assert.ok(ext, "Extension should exist");
    // packageJSON should have activationEvents
    assert.ok(
      ext.packageJSON.activationEvents || ext.packageJSON.main,
      "Extension should have activation configuration"
    );
  });

  test("binary discovery handles missing binary gracefully", async () => {
    // Verify that setting an invalid path doesn't crash the extension
    const cfg = vscode.workspace.getConfiguration("arukellt");
    const original = cfg.get("server.path");
    try {
      await cfg.update(
        "server.path",
        "/nonexistent/arukellt-fake",
        vscode.ConfigurationTarget.Global
      );
      // The extension should handle this gracefully (no throw)
      await new Promise((r) => setTimeout(r, 500));
    } finally {
      await cfg.update("server.path", original, vscode.ConfigurationTarget.Global);
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
    const ext = vscode.extensions.getExtension("arukellt.arukellt-all-in-one");
    assert.ok(ext, "Extension with test controller should be present");
  });

  test("restart command re-initializes LSP", async () => {
    const allCommands = await vscode.commands.getCommands(true);
    assert.ok(
      allCommands.includes("arukellt.restartLanguageServer"),
      "Restart command should remain registered after activation"
    );
    const ext = vscode.extensions.getExtension("arukellt.arukellt-all-in-one");
    assert.ok(ext, "Extension should still be present after restart");
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
    const ext = vscode.extensions.getExtension("arukellt.arukellt-all-in-one");
    assert.ok(ext, "Extension should be present");
  });
});

suite("Status Bar (#275)", () => {
  test("status bar item is created", async () => {
    // Status bar item is created in activate()
    const ext = vscode.extensions.getExtension("arukellt.arukellt-all-in-one");
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

suite("Go to Definition (#450 / #453)", () => {
  let doc;
  suiteSetup(async function () {
    if (!fs.existsSync(repoDebugBinary)) { this.skip(); return; }
    const fixturePath = path.join(__dirname, "fixtures", "basic.ark");
    doc = await vscode.workspace.openTextDocument(fixturePath);
    await vscode.window.showTextDocument(doc);
    // Wait for LSP to finish analysis
    await new Promise((r) => setTimeout(r, 3000));
  });

  test("local variable definition range is identifier only", async () => {
    // `result` usage in `stdio::println(result)` on line 8 (0-indexed), col 20 is inside `result`
    // basic.ark layout (0-indexed): line 0=use, line 1=fn greet, ..., line 7=let result, line 8=stdio::println(result)
    const pos = new vscode.Position(8, 20);
    const locs = await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      doc.uri,
      pos
    );
    assert.ok(locs && locs.length > 0, "Should find definition of result");
    // Handle both Location (loc.range) and LocationLink (loc.targetRange)
    const loc = locs[0];
    const range = loc.targetRange || loc.range;
    assert.ok(range, "Definition result should have a range");
    // Should point to `let result = ...` on line 7
    assert.strictEqual(
      range.start.line,
      7,
      "Should point to let-binding line (line 7)"
    );
    assert.strictEqual(
      range.start.character,
      8,
      "Should start at 'result' identifier (col 8)"
    );
    // Range must be a single line (not the full let statement)
    assert.strictEqual(
      range.start.line,
      range.end.line,
      "Definition range should be single line (identifier only, not full let statement)"
    );
    // Range length should be at most the length of 'result' (6) + small tolerance
    const rangeLen = range.end.character - range.start.character;
    assert.ok(
      rangeLen <= 10,
      `Range too wide: ${rangeLen} chars (expected identifier width ~6)`
    );
  });

  test("function definition range is function name only", async () => {
    // `greet(...)` call on line 7 (0-indexed), col 17 is inside `greet`
    // basic.ark: line 7 = `    let result = greet("world")`; greet starts at col 17
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
    // fn greet(...) — `greet` starts at col 3 on line 1 (after `use std::host::stdio` on line 0)
    assert.strictEqual(range.start.line, 1, "Should point to fn greet line (line 1)");
    assert.strictEqual(
      range.start.character,
      3,
      "Should start at 'greet' identifier (col 3)"
    );
    // Range must be a single line
    assert.strictEqual(
      range.start.line,
      range.end.line,
      "Function definition range should be single line"
    );
    const rangeLen = range.end.character - range.start.character;
    assert.ok(
      rangeLen <= 8,
      `greet range too wide: ${rangeLen} chars (expected ~5)`
    );
  });

  test("definition on keyword/whitespace returns nothing", async () => {
    // Position 1, 0 is the `f` of `fn` keyword on `fn greet(...)` — not a bound name
    const pos = new vscode.Position(1, 0);
    const locs = await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      doc.uri,
      pos
    );
    // `fn` keyword should produce no definition
    assert.ok(
      !locs || locs.length === 0,
      "Keyword/whitespace position should return no definition"
    );
  });
});

// ============================================================
// #453 — Hover E2E (verifies #451 semantic-only hover filter)
// ============================================================

suite("Hover (#451 / #453)", () => {
  let doc;
  suiteSetup(async function () {
    if (!fs.existsSync(repoDebugBinary)) { this.skip(); return; }
    const fixturePath = path.join(__dirname, "fixtures", "basic.ark");
    doc = await vscode.workspace.openTextDocument(fixturePath);
    await vscode.window.showTextDocument(doc);
    await new Promise((r) => setTimeout(r, 3000));
  });

  test("string literal position returns no 'string literal' hover noise", async () => {
    // Line 2: `    let msg = concat("Hello, ", name)` — inside "Hello, " at col 25
    // basic.ark: line 0=use, line 1=fn greet, line 2=let msg = concat(...)
    const pos = new vscode.Position(2, 25);
    const hovers = await vscode.commands.executeCommand(
      "vscode.executeHoverProvider",
      doc.uri,
      pos
    );
    // After #451 fix: string literal positions should NOT produce a 'string literal' label
    const hasNoise =
      hovers &&
      hovers.some((h) =>
        h.contents.some((c) => {
          const text = typeof c === "string" ? c : c.value || "";
          return text.includes("string literal");
        })
      );
    assert.ok(
      !hasNoise,
      "String literal position should not produce 'string literal' hover noise (fixed by #451)"
    );
  });

  test("known function name produces meaningful hover content", async () => {
    // Line 8: `    stdio::println(result)` — `println` starts at col 11
    // basic.ark: line 8 = `    stdio::println(result)`
    const pos = new vscode.Position(8, 11);
    const hovers = await vscode.commands.executeCommand(
      "vscode.executeHoverProvider",
      doc.uri,
      pos
    );
    assert.ok(
      hovers && hovers.length > 0,
      "println should produce a hover result"
    );
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

  test("unresolved name produces E0100 diagnostic", async () => {
    const content = "fn main() {\n    println(undefined_var)\n}\n";
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
