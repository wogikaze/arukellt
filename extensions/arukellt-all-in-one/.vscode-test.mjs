import { defineConfig } from "@vscode/test-cli";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const downloadedCode = path.join(
  __dirname,
  ".vscode-test",
  "vscode-linux-x64-1.120.0",
  "bin",
  "code"
);
const localDownloadedInstall = fs.existsSync(downloadedCode)
  ? { useInstallation: { fromPath: downloadedCode } }
  : {};

export default defineConfig([
  {
    files: "src/test/**/*.test.js",
    workspaceFolder: "./src/test/fixtures",
    launchArgs: ["--no-sandbox"],
    ...localDownloadedInstall,
    mocha: {
      timeout: 30000,
    },
  },
]);
