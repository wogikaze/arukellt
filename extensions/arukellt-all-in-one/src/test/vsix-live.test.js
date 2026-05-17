// @ts-check
const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const vscode = require("vscode");

const extensionId = "arukellt.arukellt-all-in-one";
const repoRoot = path.resolve(__dirname, "..", "..", "..", "..");
const sourceExtensionRoot = path.join(repoRoot, "extensions", "arukellt-all-in-one");
const repoDebugBinary = path.join(
  repoRoot,
  "target",
  "debug",
  process.platform === "win32" ? "arukellt.exe" : "arukellt"
);

let originalServerPath;
let originalServerArgs;

function mark(name) {
  if (process.env.ARUKELLT_VSIX_LIVE_MARKER) {
    fs.appendFileSync(process.env.ARUKELLT_VSIX_LIVE_MARKER, `${name}\n`);
  }
}

async function waitFor(check, options = {}) {
  const timeoutMs = options.timeoutMs ?? 20000;
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

function getExtension() {
  const ext = vscode.extensions.getExtension(extensionId);
  assert.ok(ext, "installed Arukellt extension should be discoverable");
  return ext;
}

async function activateInstalledExtension() {
  const ext = getExtension();
  const api = await ext.activate();
  return { ext, api };
}

async function waitUntilReady(api, description = "installed VSIX language server ready") {
  return waitFor(() => {
    const state = api.__getTestState();
    assert.strictEqual(state.hasClient, true);
    assert.strictEqual(state.languageStatusText, "$(check) Ready");
    return state;
  }, { description, timeoutMs: 30000 });
}

async function withTimeout(promise, description, timeoutMs = 15000) {
  let timeout;
  const timer = new Promise((_, reject) => {
    timeout = setTimeout(() => reject(new Error(`Timed out waiting for ${description}`)), timeoutMs);
  });
  try {
    return await Promise.race([promise, timer]);
  } finally {
    clearTimeout(timeout);
  }
}

suite("VSIX Live Editor Release Gate (#554)", function () {
  suiteSetup(async function () {
    if (!fs.existsSync(repoDebugBinary)) {
      throw new Error(`missing extension E2E CLI fixture at ${repoDebugBinary}`);
    }

    const cfg = vscode.workspace.getConfiguration("arukellt");
    originalServerPath = cfg.get("server.path");
    originalServerArgs = cfg.get("server.args");
    await cfg.update("server.path", repoDebugBinary, vscode.ConfigurationTarget.Global);
    await cfg.update("server.args", [], vscode.ConfigurationTarget.Global);
  });

  suiteTeardown(async function () {
    const cfg = vscode.workspace.getConfiguration("arukellt");
    if (originalServerPath !== undefined) {
      await cfg.update("server.path", originalServerPath, vscode.ConfigurationTarget.Global);
    }
    if (originalServerArgs !== undefined) {
      await cfg.update("server.args", originalServerArgs, vscode.ConfigurationTarget.Global);
    }
    const ext = getExtension();
    if (ext.isActive) {
      await ext.exports.shutdownForTests?.();
    }
  });

  test("VSIX installs, activates, and reaches Ready language status", async function () {
    mark("ready:start");
    const { ext, api } = await activateInstalledExtension();

    assert.notStrictEqual(
      path.resolve(ext.extensionPath),
      path.resolve(sourceExtensionRoot),
      "release gate must exercise the installed VSIX, not the source extensionDevelopmentPath"
    );

    const doc = await vscode.workspace.openTextDocument({
      language: "arukellt",
      content: "fn main() {\n  println(\"hello\")\n}\n",
    });
    await vscode.window.showTextDocument(doc);

    await waitUntilReady(api);
    mark("ready:pass");
  });

  test("diagnostics appear for a saved file with a type error", async function () {
    mark("diagnostics:start");
    const { api } = await activateInstalledExtension();
    await waitUntilReady(api, "installed VSIX language server ready before diagnostics");
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "arukellt-vsix-live-"));
    const file = path.join(dir, "type_error.ark");
    fs.writeFileSync(
      file,
      "use std::host::stdio\nfn main() {\n    stdio::println(undefined_var)\n}\n",
      "utf8"
    );

    const doc = await vscode.workspace.openTextDocument(file);
    await vscode.window.showTextDocument(doc);
    await doc.save();

    await waitFor(() => {
      const state = api.__getTestState();
      assert.strictEqual(state.languageStatusText, "$(check) Ready");
      const diagnostics = vscode.languages.getDiagnostics(doc.uri);
      assert.ok(
        diagnostics.some((diag) => /E0100|undefined_var|unresolved/i.test(diag.message)),
        `expected a type-error diagnostic, got: ${diagnostics.map((diag) => diag.message).join(" | ")}`
      );
      return diagnostics;
    }, { description: "type-error diagnostics from installed VSIX", timeoutMs: 30000 });
    mark("diagnostics:pass");
  });

  test("completion, hover, and go-to-definition work through the live editor API", async function () {
    mark("editor:start");
    const { api } = await activateInstalledExtension();
    await waitUntilReady(api, "installed VSIX language server ready before editor API checks");
    const basicPath = path.join(__dirname, "fixtures", "basic.ark");
    const doc = await vscode.workspace.openTextDocument(basicPath);
    await vscode.window.showTextDocument(doc);

    const completionList = await withTimeout(
      vscode.commands.executeCommand(
        "vscode.executeCompletionItemProvider",
        doc.uri,
        new vscode.Position(7, 0)
      ),
      "completion provider"
    );
    const completionLabels = (completionList?.items || []).map((item) => String(item.label));
    assert.ok(completionLabels.includes("fn"), `completion labels should include fn: ${completionLabels.join(", ")}`);
    assert.ok(
      completionLabels.includes("println"),
      `completion labels should include println: ${completionLabels.join(", ")}`
    );

    const hovers = await withTimeout(
      vscode.commands.executeCommand(
        "vscode.executeHoverProvider",
        doc.uri,
        new vscode.Position(8, 16)
      ),
      "hover provider"
    );
    const hoverText = (hovers || [])
      .flatMap((hover) => hover.contents || [])
      .map((content) => typeof content === "string" ? content : content.value || "")
      .join("\n");
    assert.match(hoverText, /println|fn/i);

    const definitions = await withTimeout(
      vscode.commands.executeCommand(
        "vscode.executeDefinitionProvider",
        doc.uri,
        new vscode.Position(8, 10)
      ),
      "definition provider"
    );
    assert.ok(Array.isArray(definitions), "definition provider should return a location array");
    assert.ok(definitions.length > 0, "definition provider should return at least one location");
    mark("editor:pass");
  });
});
