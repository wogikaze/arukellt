# Arukellt All-in-One Release Procedure

This document is the extension packaging checklist used before publishing a
VSIX or Marketplace release.

## Prerequisites

- Node.js and npm are available.
- `node_modules/` is installed with `npm ci`.
- The repository root contains a runnable `arukellt` binary for extension tests.
- For headless activation tests, no other VS Code instance may hold the
  `.vscode-test` instance lock.
- On WSL, set `DONT_PROMPT_WSL_INSTALL=1` to suppress VS Code's WSL install
  prompt, and ensure `/tmp/.X11-unix` is owned by `root:root` with mode `1777`
  before relying on the exact `xvfb-run` gate. Native Linux CI is preferred for
  release evidence.
- The repository CI `extension-tests` job runs the same npm install, metadata,
  build, and `xvfb-run` activation sequence on native Ubuntu.

## Verification

```bash
cd extensions/arukellt-all-in-one
npm ci
npm run test:marketplace-metadata
npm run build
xvfb-run -a npm test
xvfb-run -a npm run test:vsix-live
```

Expected outputs:

- `npm run test:marketplace-metadata` exits 0.
- `npm run build` produces `arukellt-all-in-one-<version>.vsix`.
- `xvfb-run -a npm test` passes the activation and workflow suite.
- `xvfb-run -a npm run test:vsix-live` installs the packaged VSIX and passes
  the live editor smoke checks for activation, Ready status, diagnostics, and
  completion/hover/definition.

## Marketplace Metadata

The package must keep these fields populated before publishing:

- `publisher`
- `icon`
- `galleryBanner`
- `categories`
- `keywords`
- `repository`
- `bugs`
- `homepage`
- `engines.vscode`

The icon is `media/icon.png`. README media should remain relative so VS Code
Marketplace can render it from the packaged extension.

## Compatibility

- Desktop VS Code: supported.
- VS Code Remote / Dev Containers / Codespaces: supported when `arukellt` is
  available inside the remote environment or `arukellt.server.path` points to
  a valid remote binary.
- Web extension host: not supported yet. The extension launches `arukellt lsp`
  as a local or remote process, which is unavailable in the browser extension
  host.

## Publishing

1. Update `version` in `package.json`.
2. Add a matching section to `CHANGELOG.md`.
3. Run the verification commands above.
4. Package with `npm run build`.
5. Publish the generated VSIX using the project Marketplace publisher account.
