---
Status: done
Created: 2026-09-19
Updated: 2026-09-19
ID: 879
Parent: 852
Track: language-v2
Depends on: "871, 872, 874, 875"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 performance"
---

# 879 — Wasm GC / fallback の性能・サイズ receipt を取得する

## Scope

RFC-011: **performance**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] workload を固定する — runner の固定 20-workload suite
- [x] `.wasm` size/cold startup/steady runtime を両 backend で測る
- [x] allocation/GC time/peak memory を測る — RSS peak を両側で記録。GC pause telemetry は wasmtime CLI 非対応のため "unavailable" として明示
- [x] compiler compile time/RSS も必要に応じて測る — per-workload compile median_ms + max_rss_kb
- [x] commit/runtime/wasm-tools/host/commands/raw results を保存する
- [x] 不利な結果もそのまま記録する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する

## Progress receipt — 2026-09-20 (fail-closed current-s2 benchmark run)

- Commit `f499fccad` makes the benchmark runner fail closed for compile errors,
  runtime errors, incorrect output, and missing comparison metrics. It also
  selects the current selfhost s2 artifact and keeps compile/runtime paths
  repository-relative so the same artifact can be executed by Wasmtime.
- `python3 scripts/manager.py perf gate` was run with the current s2 wrapper
  on the fixed 20-workload suite. Raw output is in
  `tests/baselines/perf/current.json`, with the generated report in
  `docs/history/benchmarks/benchmark-results.md`; the envelope records the
  commit-independent tool/environment metadata and `wasm32` target.
- The run records 7 compile errors, 6 runtime errors, and 1 correctness
  failure (`parse_tree_distance`). Six workloads compile, run, and produce
  the expected result. The gate exits non-zero, as required; the 15-entry
  historic baseline was not rewritten to conceal the current compiler/runtime
  regressions. Wasmtime CLI still reports GC pause/allocation telemetry as
  unavailable.
- This is an honest fallback-target measurement receipt only. A complete
  `wasm32-gc` versus linear-memory comparison, allocation/GC instrumentation,
  and the dependent lowering/parity work remain open; #879 is not closed.

## Progress receipt — 2026-09-20 (typed closure workload parity)

- Commit `104b108d2` fixes the typed-capturing-closure workload that previously
  failed type checking with missing `Mul`/`Add` bounds for closure parameters.
- With the current s2 artifact, the fixed `closure_map` workload compiles as a
  `wasm32-gc` component (3447 bytes) and as a `wasm32` linear-memory core
  module (3099 bytes). `wasm-tools validate` passes for both artifacts, and
  Wasmtime executes both with the expected output `6017000`.
- This is a targeted positive workload receipt, not completion of the fixed
  20-workload gate. Allocation/GC time, peak memory, cold-start/steady-state
  measurements, and the remaining compile/runtime/correctness failures remain
  open; the historic baseline is unchanged.

## Progress receipt — 2026-09-27 (paired wasm32 / wasm32-gc full-suite run)

- Runner: `scripts/util/benchmark_runner.py` gained wasm32-gc component
  support in `ad393e064` (`--emit component` + `wasmtime run --wasm gc --wasm function-references --dir=.`).
  Stale workloads migrated to v2 APIs in `1fca2e1d9`;
  `bench_parse_tree_distance` triangular-offset bug fixed in `72429718a`
  (`Yes` verified against an independent Python simulation of the
  tree-metric check).
- Environment: commit `72429718a`, wasmtime 46.0.1, wasm-tools 1.252.0,
  Linux 6.18.33.2-microsoft-standard-WSL2 x86_64 (WSL2). Per-workload
  commands are recorded in each JSON `command` field.
- Raw results: `tests/baselines/perf/current-wasm32.json` and
  `current-wasm32-gc.json`; reports `docs/history/benchmarks/benchmark-879-wasm32.md` /
  `benchmark-879-wasm32-gc.md`. Historic `baselines.json` untouched.
- wasm32 linear: 20/20 compile ok; 12/20 run ok; 8/20 runtime trap
  (`unreachable`) — all allocation-heavy workloads, consistent with #896
  (64 KiB GC metadata arena exhausts at ~4096 records before the 64 MiB
  payload watermark can trigger entry-gated collect). `hash_ops` runs
  but correctness-fails (#895: `__hm_*` intrinsic store fixed at
  capacity 16; inserts silently dropped).
- wasm32-gc: 20/20 compile+run ok; 19/20 correctness pass — only
  `hash_ops` fails (#895 is backend-independent semantics).
- Cold startup (startup.ark probe, 20 iters, median): linear 11.003 ms,
  gc 11.14 ms — comparable; the component path adds no measurable
  startup penalty at this fixture size.
- Steady runtime (median ms): gc slower on small CPU-bound loops
  (fib 9.9 vs 10.9; time_ops 16.5 vs 9.3; template_engine 20.0 vs 10.4)
  but dramatically faster where the linear backend survives heavy
  allocation at all (binary_tree 17.4 vs 115.1; parse_tree_distance
  114.6 vs 549.0); 8 allocation-heavy workloads have no linear number
  (trap). GC wins wherever tracing-GC pressure exists; linear wins on
  plain scalar loops.
- Artifact size (binary_bytes): gc components carry ~8 KB envelope —
  small programs 4-5x larger (fib 2220->10258 B; vec_ops 3671->11949 B);
  large stdlib workloads comparable or smaller (toml_ops 72941<84667 B;
  json_parse 54484<59616 B).
- Compile time (median): linear 7177.7 ms vs gc 8136.2 ms (+13%,
  component packaging). Compiler RSS median ~170 MB both targets;
  hash_ops peaks ~280 MB on both (generic HashMap instantiation).
- Runtime RSS: linear median ~17.8 MB, gc median ~19.6 MB (+10%).
- GC pause / allocation-rate telemetry: unavailable via wasmtime CLI —
  recorded as "unavailable" with the runner note; only RSS peaks are
  captured. Finer GC telemetry remains a known tooling gap.
- Residual gaps transferred to tracked issues: #895 (intrinsic HashMap
  growth — both backends), #896 (linear GC metadata arena exhaustion —
  linear only). #874 remains open for the broader fallback lowering
  work; the traps above quantify that gap rather than conceal it.
