# JavaScript interop examples

## [`invoke-component/`](invoke-component/README.md)

Call an Ark-exported component from JavaScript using **wasmtime CLI** subprocess
(`run.mjs`). Works today for scalar exports without a browser or jco.

## [`invoke-via-jco/`](invoke-via-jco/README.md)

Optional **jco transpile** path (`@bytecodealliance/jco`). Transpile is exercised when
Node + jco are installed; full in-process import remains tracked in issue #036 / #037.

Both examples share the Ark artifact from [`../ark/export-library/`](../ark/export-library/README.md).
