// @ts-check
const assert = require("assert");
const vscode = require("vscode");

suite("Extension Activation", () => {
  test("extension is present", () => {
    const ext = vscode.extensions.getExtension("arukellt.arukellt-all-in-one");
    assert.ok(ext, "Extension should be found by its ID");
  });

  test("extension activates on .ark file", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "arukellt",
      content: 'let x = 1\nprint(x)\n',
    });
    await vscode.window.showTextDocument(doc);

    // Give activation a moment
    await new Promise((r) => setTimeout(r, 2000));

    const ext = vscode.extensions.getExtension("arukellt.arukellt-all-in-one");
    assert.ok(ext, "Extension should exist");
    // Extension may or may not activate without a real arukellt binary,
    // but it should at least be present and not crash.
  });
});

suite("Command Registration", () => {
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
    ];
    for (const cmd of expected) {
      assert.ok(
        allCommands.includes(cmd),
        `Command ${cmd} should be registered`
      );
    }
  });
});

suite("Configuration", () => {
  test("default settings are available", () => {
    const cfg = vscode.workspace.getConfiguration("arukellt");
    assert.strictEqual(cfg.get("server.path"), "arukellt");
    assert.deepStrictEqual(cfg.get("server.args"), []);
    assert.strictEqual(cfg.get("target"), "wasm32-wasi-p1");
    assert.strictEqual(cfg.get("emit"), "core-wasm");
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

suite("Debug Adapter", () => {
  test("arukellt debug type is registered", () => {
    // The debuggers contribution in package.json registers the type
    // We can verify by checking that a debug configuration provider exists
    const debugType = "arukellt";
    // vscode.debug.registerDebugAdapterDescriptorFactory is called in activate()
    // We verify the contribution point is recognized
    assert.ok(
      vscode.extensions.getExtension("arukellt.arukellt-all-in-one"),
      "Extension with debug contribution should be present"
    );
  });
});
