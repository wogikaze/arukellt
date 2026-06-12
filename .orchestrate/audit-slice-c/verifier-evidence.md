# Verifier evidence — audit-slice-c (2026-06-12)

Branch: `cursor/audit-slice-c-component-wit-wasi-2d78`

## verify quick

```text
python3 scripts/manager.py verify quick
→ exit 1; 146/149 passed (wasmtime 30.0.2 in PATH)
Failures (3, pre-existing / unrelated to Slice C):
  - doc example check (arukellt not in PATH for check-doc-examples.py)
  - docs consistency (generated docs stale; needs generate-docs.py)
  - false-done hygiene (#487 STATUS_MISMATCH in issues/done/)
```

Without wasmtime (upstream VM): 143/149 passed (6 failures including wasmtime-gated LSP/DAP/analysis).

## component interop

```text
python3 scripts/manager.py verify component
→ exit 0; 101/101 passed (wasmtime)
```

## component-compile manifest spot-check

- `component-compile:` entries in manifest.txt: **101** (handoff cited 102; off-by-one)
- `src/compiler/component/*.ark` modules: **115**
- Batch `arukellt run` on all 101 `component-compile:` fixtures: **96 pass / 5 fail**
  - Fails: `export_string_{i64,u64}*` (type errors), `import_scalar_func` (parse error)
- `tests/fixtures/wit_import/`: **absent** (confirms #034 reopen)
- `import_scalar_func.diag` golden: **E0401** (runtime currently emits E0001 parse error; still non-success)

## reopened issues (7)

Present under `issues/open/` with reopen notes; absent from `issues/done/`:
034, 073, 117, 118, 138, 443, 618

## key claim spot-checks

- #443: `src/compiler/main/commands.ark` `compose` → `CMD_NOT_YET()`
- #618: only `tests/component-interop/native/roundtrip/roundtrip.ark` skeleton; no run.sh
- #073: manifest has 3 `module-run:stdlib_host/wasi_{clock,random,args}.ark` only
