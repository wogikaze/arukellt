# WAT feature probes

Status: research artifacts (not a product gate)

These are minimal WAT modules used to probe **feature-level** WebAssembly
support across local toolchains. They accompany
[`../target-runtime-verification.md`](../target-runtime-verification.md).

## Layout

| Directory | Scope |
|-----------|--------|
| `wasm10/` | Wasm 1.0 / MVP core |
| `wasm20/` | Wasm 2.0 additions |
| `wasm30/` | Wasm 3.0 additions (+ JS embedding probes) |
| `experimental/` | Threads/Atomics, legacy EH (not Wasm 3.0 Core) |

## Run

```bash
# Prefer a Node with current V8 (nvm Node 25+ recommended for try_table)
export PATH="$HOME/.nvm/versions/node/v25.2.1/bin:$PATH"
python3 docs/research/wat-probes/run-probes.py
```

Outputs:

- `results.json` — machine-readable stage results
- `results.md` — human-readable matrix

Stages per probe:

1. `wasm-tools.parse` / `wasm-tools.validate`
2. `wabt.wat2wasm` (`--enable-all`) / `wabt.validate`
3. `wasmtime run -W all-proposals=y --invoke test` (plus `shared-memory=y` for threads)
4. `iwasm -f test`
5. Node `WebAssembly.validate` / `instantiate` / invoke `test`

## Notes

- Binary magic/version remains `00 61 73 6d 01 00 00 00` even for Wasm 3.0 features.
- `wasmtime` results here use **opt-in** `-W all-proposals=y` (not default).
- Local `iwasm` was built with GC/Memory64/TailCall/MultiMemory **OFF**.
- Legacy `try`/`catch` text is rejected by current `wasm-tools`; Wasm 3.0 EH is `try_table`.
- Branch Hinting is not executable-semantics; not covered by a return-value probe.
