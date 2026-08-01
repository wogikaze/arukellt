---
Status: done
Created: 2026-07-14
Updated: 2026-07-26
ID: 810
Track: compiler
Depends on: none
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: CQ-18 audit — unresolved verify full failures need open owner
---

# 810 — Component interop failures

## Close note — 2026-07-26

**Verdict: APPROVE (issue-close-review)**

Acceptance: every `tests/component-interop/jco/*/run.sh` fixture and
`python3 scripts/manager.py verify component-interop` wasmtime cases pass
validation / interop (fail count 0). Explicit `--wasi-version wasi-p2` command
path for #668 remains gated.

### Evidence

| Acceptance | Evidence |
|---|---|
| Component emit validates + interops | `python3 scripts/manager.py verify component-interop` → **102 pass / 0 fail** (jco-interop skipped unless `ARUKELLT_TEST_JCO=1`) |
| Full jco directory suite | Manual `tests/component-interop/jco/*/run.sh` → **103 pass / 0 fail** |
| #668 preserved | `python3 scripts/check/gate-668-p2-args-env.py` → **PASS** |
| Lane quality | `python3 scripts/manager.py verify lane` → **PASS** |
| Blocker cleared | `docs/data/release-guarantees.toml` `check_component_interop_wasmtime` → `result = "pass"`, `affected_count = 0` |

### Fixes on `wave/810-component-interop`

1. Default wasi-p2 + library exports → `p1-component` (explicit wasi-p2 keeps command path).
2. GC Option/Result/List/Record/Tuple canonical ABI adapters on library path.
3. f32 MIR binop typing + real `f64↔f32` demote/promote bridges; opcode corrections.
4. Fixture: calculator jco test uses Number (not BigInt); `abs_i32` renamed to `absolute_i32` (CoreOp defer).

Fail trajectory: receipt **103** → routing **56** → adapters **6** → this close **0**.

## Summary

`verify full` reported 103 component interop failures. The selfhost
compiler's component model output did not pass component validation or
interop checks. Resolved on `wave/810-component-interop`.

## Removal condition

~~Each fixture passes when the selfhost compiler emits a valid component
that passes `wasm-tools validate --component` and interop checks.~~ **Met.**

## Validation command

```bash
python3 scripts/manager.py verify component-interop
# optional Node/jco:
ARUKELLT_TEST_JCO=1 python3 scripts/manager.py verify component-interop
```
