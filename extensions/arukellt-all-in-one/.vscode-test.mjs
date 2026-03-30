import { defineConfig } from "@vscode/test-cli";

export default defineConfig([
  {
    files: "src/test/**/*.test.js",
    workspaceFolder: "./src/test/fixtures",
    mocha: {
      timeout: 30000,
    },
  },
]);
