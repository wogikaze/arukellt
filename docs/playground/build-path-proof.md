# Playground Build and Publish Path - Proof Record

**Issue**: #468
**Status**: PROVED (2026-04-25)
**Evidence gathered by**: impl-playground agent

---

## Build Commands

Run in order from repo root:

    # 1. Build the Wasm module
    cd playground && npm run build:wasm
    # output: crates/ark-playground-wasm/pkg/ark_playground_wasm_bg.wasm (568 KB)

    # 2. Build playground JS + copy all assets into docs/playground/
    cd playground && npm run build:app
    # output: docs/playground/dist/  (compiled TypeScript)
    #         docs/playground/wasm/  (wasm pkg: .wasm + .js glue + .d.ts)

Both commands exit 0. Total build time: ~21 s (dominated by Rust compile).

---

## Output Paths

- crates/ark-playground-wasm/pkg/ark_playground_wasm_bg.wasm  (568248 bytes)
- crates/ark-playground-wasm/pkg/ark_playground_wasm.js        (wasm-bindgen glue)
- docs/playground/wasm/ark_playground_wasm_bg.wasm             (deployed copy)
- docs/playground/dist/                                        (compiled TS)

---

## Artifact Validation

Built with wasm-pack 0.13.1 and wasm-opt optimization pass (exit 0).
File size: 568248 bytes. Wasm magic bytes 0061736d confirmed valid.

---

## Publish Path

.github/workflows/pages.yml uploads ./docs to GitHub Pages:

    - uses: actions/upload-pages-artifact@v3
      with:
        path: ./docs
    - uses: actions/deploy-pages@v4

docs/playground/wasm/ is within ./docs and is therefore included in every
GitHub Pages deployment. The workflow triggers on changes to playground/**
and crates/ark-playground-wasm/**.

---

## Toolchain Requirements

- Rust stable + wasm32-unknown-unknown target (rustup target add wasm32-unknown-unknown)
- wasm-pack >= 0.13 (cargo install wasm-pack)
- wasm-opt is bundled by wasm-pack
- Node.js >= 18

---

## Notes

- build:app uses || true when copying pkg/, so it silently skips the wasm copy
  if build:wasm has not been run first. Always run build:wasm before build:app.
- scripts/gen/stamp-playground-assets.sh is referenced by build:app but does
  not exist on disk; cache-busting is inactive (tracked by #437).
- No local dev-server (npm run dev) exists in playground/package.json.
