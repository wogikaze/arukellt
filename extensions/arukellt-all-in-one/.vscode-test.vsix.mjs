import { defineConfig } from "@vscode/test-cli";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const downloadedCode = path.join(
  __dirname,
  ".vscode-test",
  "vscode-linux-x64-1.120.0",
  "code"
);
const localDownloadedInstall = fs.existsSync(downloadedCode)
  ? { useInstallation: { fromPath: downloadedCode } }
  : {};

export default defineConfig([
  {
    files: "src/test/vsix-live.test.js",
    extensionDevelopmentPath: "./src/test/vsix-runner",
    workspaceFolder: "./src/test/fixtures",
    installExtensions: [path.join(__dirname, "arukellt-all-in-one-0.0.1.vsix")],
    launchArgs: ["--no-sandbox"],
    env: {
      ARUKELLT_VSIX_LIVE_MARKER: process.env.ARUKELLT_VSIX_LIVE_MARKER,
    },
    ...localDownloadedInstall,
    mocha: {
      timeout: 45000,
    },
  },
]);
