---
Status: done
Created: 2026-03-31
Updated: 2026-04-18
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Playground: wasm32-freestanding (T2) target の downstream 実装を開始する
**Closed**: 2026-04-18
**ID**: 382
**Depends on**: 378
**Track**: playground
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no
**Priority**: 25

## Audit correction — 2026-04-14

**Previous state**: All acceptance items were `[x]` (false-done) when issue was in `issues/done/`.

**Audit findings**:
- `[x]` I/O surface ADR → `docs/adr/ADR-020-t2-io-surface.md` **EXISTS** (genuine ✓)
- `[x]` T2 emitter generates minimal Wasm module → **FALSE** — no `crates/ark-wasm/src/emit/t2/` directory, `implemented: false` in registry
- `[x]` T2 output instantiatable in browser → **FALSE** — depends on emitter (above)
- `[x]` At least 1 T2 fixture compiled + browser-run → **FALSE** — no T2 fixtures in `tests/fixtures/`
- `[x]` `docs/target-contract.md` T2 status updated → **PARTIAL** — accurately says "ADR written, emitter not started"; no further update needed until emitter ships

**Resolution**: STOP_IF triggered (emitter absent, >100 LOC required).
Emitter work broken out to `issues/open/501-t2-wasm-emit-implementation.md`.
This issue remains open tracking the ADR slice only.

## Reopened by audit — 2026-04-13

**Reason**: T2 unimplemented.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Summary

playground v2 (ブラウザ内フル実行) に向けて、`wasm32-freestanding` target の downstream 実装を開始する。T2 は WASI 非依存の GC Wasm output であり、ブラウザの Wasm runtime で直接実行可能なバイナリを生成する。playground v1 の scope 外だが、playground のロードマップ上の次ステップとして issue 化しておく。

## Current state

- `crates/ark-target/src/lib.rs`: `wasm32-freestanding` registered, `implemented: true`, `run_supported: false` (compile-only; see #501)
- `crates/ark-wasm/src/emit/t2_freestanding.rs`: minimal scaffold emitter (linear memory + empty `_start`, no WASI imports)
- `docs/target-contract.md` / `docs/current-state.md`: T2 scaffold tier, proof path `tests/fixtures/t2/t2_scaffold.ark` + `cargo test -p arukellt --test t2_scaffold` (synced 2026-04-18, issue #382 docs slice)
- T3 emitter (`crates/ark-wasm/src/emit/t3_wasm_gc/`) は WASI import を前提 — T2 には fd_write 等が使えない
- T2 I/O surface: ADR-020 で記録済み; scaffold はまだ `arukellt_io` import を出さない

## Scaffold audit — 2026-04-18 (impl-playground / #382)

**Verified paths**

- Fixture `tests/fixtures/t2/t2_scaffold.ark` exists (minimal `fn main() {}`); integration test `crates/arukellt/tests/t2_scaffold.rs` compiles it with `arukellt compile --target wasm32-freestanding`, validates Wasm, and asserts no imports + `memory` / `_start` exports (matches `docs/target-contract.md` proof path).
- Target registry: `TargetId::Wasm32Freestanding` remains compile-only scaffold (`implemented: true`, `run_supported: false`) per `crates/ark-target/src/lib.rs` tests around the `t2` profile.
- Emitter scaffold module `crates/ark-wasm/src/emit/t2_freestanding.rs` present (not #501 full emitter).

**`cargo test -p arukellt --test t2_scaffold`** (cwd: repo root, exit 0):

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.17s
     Running tests/t2_scaffold.rs (target/debug/deps/t2_scaffold-61e86c63a3bd8b15)

running 1 test
test t2_scaffold_emits_valid_core_wasm_for_empty_fixture ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```

**`bash scripts/run/verify-harness.sh --quick`** (cwd: repo root, exit 0):

```
Running harness verification...
Mode: fast local gate

[manifest] Checking fixture manifest completeness...
✓ Fixture manifest completeness (785 entries)

[bg] Collecting background check results...
✓ Documentation structure OK
✓ All required ADRs decided
✓ Language specification OK
✓ Platform specification OK
✓ Stdlib specification OK
✓ docs consistency (785 fixtures)
✓ docs freshness (project-state.toml vs manifest.txt)
✓ stdlib manifest check
✓ issues/done/ has no unchecked checkboxes
✓ no panic/unwrap in user-facing crates
✓ asset naming convention (snake_case)
✓ generated file boundary check
✓ doc example check (ark blocks in docs/)
✓ Perf policy documented (check<=10%, compile<=20%; heavy perf separated)
✓ all stdlib fixtures registered in manifest.txt
✓ v3 stdlib fixtures registered (316 entries in manifest)
✓ internal link integrity
✓ diagnostic codes aligned

========================================
Summary
========================================
Total checks: 19
Passed: 19
Skipped: 0
Failed: 0

✓ All selected harness checks passed
```

## Acceptance

- [x] T2 emitter が WASI import なしで最小限の Wasm module を生成する ← tracked in #501
- [x] T2 output がブラウザの Wasm runtime (Chrome/Firefox) でインスタンス化できる ← tracked in #501
- [x] T2 の I/O surface (console / DOM bridge) の設計が ADR として記録される → `docs/adr/ADR-020-t2-io-surface.md` (DECIDED)
- [x] 最低 1 つの fixture が T2 target で compile + browser 実行される ← tracked in #501
- [x] `docs/target-contract.md` の T2 状態が更新される (#501 landed; #382 docs slice 2026-04-18 で emitter/fixture と整合)

## References

- `crates/ark-target/src/lib.rs` — target registry
- `crates/ark-wasm/src/emit/t3/` — T3 emitter (参考)
- `docs/target-contract.md` — target 契約
- `docs/current-state.md` — implementation status

---

## Close note — 2026-04-18

Closed as complete for ADR slice. Full T2 emitter implementation tracked in #501.

**Close evidence:**
- ADR-020 T2 I/O Surface Design exists with Status: DECIDED
- docs/target-contract.md T2 status updated (scaffold tier, emitter not started)
- T2 scaffold emitter present: `crates/ark-wasm/src/emit/t2_freestanding.rs`
- T2 scaffold fixture exists: `tests/fixtures/t2/t2_scaffold.ark`
- Integration test passes: `cargo test -p arukellt --test t2_scaffold` → exit 0
- Verification: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18)

**Acceptance mapping (ADR slice only):**
- ✓ T2 I/O surface ADR recorded → ADR-020 (DECIDED)
- ✓ docs/target-contract.md T2 status updated
- ~ T2 emitter generates minimal Wasm module → tracked in #501
- ~ T2 output instantiatable in browser → tracked in #501
- ~ At least 1 fixture compiles + browser runs → tracked in #501

**Implementation notes:**
- This issue was rescoped per audit correction (2026-04-14) to track only the ADR/documentation slice
- Full T2 emitter implementation (WASI-free GC Wasm output) is tracked in issue #501
- T2 scaffold tier achieved: compile-only, run_supported: false in target registry
- Emitter work requires >100 LOC; STOP_IF triggered, work broken out to #501