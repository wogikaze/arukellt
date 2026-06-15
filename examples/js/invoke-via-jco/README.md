# Invoke an Ark component via jco (JavaScript bindings)

[jco](https://github.com/bytecodealliance/jco) transpiles `.component.wasm` into ES modules.
This example documents the intended Node workflow:

1. Build `calculator.component.wasm` from Ark.
2. `npx @bytecodealliance/jco transpile` → `out/calculator.component.js`
3. Import and call exports from Node.

## Status

| Step | Status |
|------|--------|
| Ark → `.component.wasm` | ✅ |
| `jco transpile` on scalar calculator | ✅ (jco ≥ 1.23) |
| In-process `import` + invoke smoke | ⚠️ tracked in issue #036 |

Until the Node gate lands in CI, `run.sh` runs **transpile-only** when jco is available.

## Run

```bash
bash examples/js/invoke-via-jco/run.sh
```

## Manual transpile

```bash
bash examples/ark/export-library/run.sh
npx --yes @bytecodealliance/jco@1.23.0 transpile \
  .build/examples/ark-export/calculator.component.wasm \
  -o .build/examples/js-jco/out
ls .build/examples/js-jco/out
```

See also `issues/open/036-jco-javascript-interop.md` and `issues/blocked/037-jco-gc-support.md`.
