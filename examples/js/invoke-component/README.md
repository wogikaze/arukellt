# Invoke an Ark component from JavaScript

`run.mjs` shells out to **wasmtime** (same as the Rust example) so you can script Ark
components from Node without writing a custom Wasm host.

## Run

```bash
bash examples/js/invoke-component/run.sh
```

Or directly:

```bash
node examples/js/invoke-component/run.mjs
```

## Manual

```bash
bash examples/ark/export-library/run.sh
node -e "
import { execSync } from 'node:child_process';
const wasm = '.build/examples/ark-export/calculator.component.wasm';
const out = execSync(\`wasmtime run --wasm gc --wasm component-model --invoke 'mul(6,7)' \${wasm}\`, { encoding: 'utf8' });
console.log(out.trim());
"
```

For in-process JS bindings, see [`../invoke-via-jco/`](../invoke-via-jco/README.md).
