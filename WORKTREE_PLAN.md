# Worktree Cleanup and Continuation Plan
Generated: 2026-07-27T19:54:43.536580+09:00
This file lists all worktrees, their state against origin/master, and whether they should be kept with a continuation plan or deleted.

## In-flight worktrees (keep and continue)

### /home/wogikaze/wgkz/arukellt
- **Branch**: master
- **HEAD**: f612754e6a2572f7a068de01e629dd549bff738e
- **Status vs origin/master**: 110 behind, 79 ahead (merged: False)
- **Dirty files**:
```
?? WORKTREE_PLAN.md
```
- **Last 5 commits**:
```
f612754e (HEAD -> master) fix(merge): repair wave ABI/import collisions after parallel merge.
fcfaa564 fix(merge): restore #822 core-ops after #807 overwrite.
342e6f71 chore(merge): finalize issue indexes and fixture accounting after wave merges.
ce89ddf9 merge(wave/834-bootstrap): resolve conflicts preferring lane changes.
c4d4b39a merge(wave/807-fixture-parity): resolve conflicts preferring lane changes.
```
- **Related plans / issues**:
(none found)
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arrukellt-aa6e-probe
- **Branch**: detached
- **HEAD**: aa6e04f8be166dfe83974ff387fc95460e7ca3ce
- **Status vs origin/master**: 228 behind, 28 ahead (merged: False)
- **Dirty files**:
```
?? .build-probe/
```
- **Last 5 commits**:
```
aa6e04f8 (HEAD) feat(native-cpp): Phase 1 shadow root liveness without emitting clears
a3733071 feat(native-cpp): Phase 0 receipt schema and GC phase timings
456f40ad docs: correct native-cpp root-clear status; open #833
780fa844 (origin/wave/native-cpp-recovery) chore(tooling): register .h family for native-cpp runtime headers
bb3b6ec7 docs(issue): close #832 native/wasmtime S3 equality; note high-RSS lane
```
- **Related plans / issues**:
(none found)
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arrukellt-native-recovery
- **Branch**: wave/native-cpp-recovery
- **HEAD**: a9dd39272d952a4870d5b989aa0c4254cc86c4fb
- **Status vs origin/master**: 16 behind, 0 ahead (merged: True)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
a9dd3927 (HEAD -> wave/native-cpp-recovery, origin/wave/native-cpp-general-readiness) docs(native-cpp): complete general readiness and unblock verify quick
1844ff27 fix(native-cpp): close expected-negative readiness gate in-scope
a10ce7a0 fix(component): restore export-shape validation on s2 bootstrap facade
edaa879a docs(plan): record native-cpp readiness gate progress
a846b185 test(native-cpp): refresh public corpus and readiness harness
```
- **Related plans / issues**:
(none found)
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/686-A-p2-adapter
- **Branch**: wave/686-A-p2-adapter
- **HEAD**: 7ede6b8738e70dda061e0c15dca865ead019d2e2
- **Status vs origin/master**: 134 behind, 2 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
7ede6b87 (HEAD -> wave/686-A-p2-adapter) docs(686): note Phase 4 adapter progress (47→65 pass)
302ab7b9 feat(686): GC canonical ABI adapters for string/record/variant
6a698660 docs(074): re-close WASI P2 native component parent gate.
02a0eae6 Merge branch 'wave/838-hashmap-string-fn'
7e2f545e chore(838): close HashMap<String, fn> funcref value ABI issue.
```
- **Related plans / issues**:
issues/open/686-wasm-gc-selfhost-implementation.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/686-B-verifier-gate
- **Branch**: wave/686-B-verifier-gate
- **HEAD**: c107e7b42d01d00ccadc274458abcd29845a25ee
- **Status vs origin/master**: 134 behind, 1 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
c107e7b4 (HEAD -> wave/686-B-verifier-gate) feat(686): hard-gate GC layout audit and remove name→offset emit fallback.
6a698660 docs(074): re-close WASI P2 native component parent gate.
02a0eae6 Merge branch 'wave/838-hashmap-string-fn'
7e2f545e chore(838): close HashMap<String, fn> funcref value ABI issue.
4c51053a feat(838): add HashMap<String, fn> GC funcref value ABI.
```
- **Related plans / issues**:
issues/open/686-wasm-gc-selfhost-implementation.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/686-merge
- **Branch**: wave/686-merge
- **HEAD**: a90c7327d0cb9a13a7e24249c6fc97978279c11a
- **Status vs origin/master**: 129 behind, 5 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
a90c7327 (HEAD -> wave/686-merge) Merge wave/686-A-p2-adapter into wave/686-merge
e57dc023 Merge wave/686-B-verifier-gate into wave/686-merge
7ede6b87 (wave/686-A-p2-adapter) docs(686): note Phase 4 adapter progress (47→65 pass)
302ab7b9 feat(686): GC canonical ABI adapters for string/record/variant
2187e9ae feat(839): resolve ? From conversion via SemanticTraitId.
```
- **Related plans / issues**:
issues/open/686-wasm-gc-selfhost-implementation.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/696-format
- **Branch**: wave/696-format
- **HEAD**: c9635670a345cd7c424928dfbb009898a97635c5
- **Status vs origin/master**: 134 behind, 2 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
c9635670 (HEAD -> wave/696-format) feat(696): wire f-string debug specifier and Debug-backed assert_eq into stdlib/runtime.
2679dd40 docs(696): add usage examples to issue #696.
6a698660 docs(074): re-close WASI P2 native component parent gate.
02a0eae6 Merge branch 'wave/838-hashmap-string-fn'
7e2f545e chore(838): close HashMap<String, fn> funcref value ABI issue.
```
- **Related plans / issues**:
docs/plans/696-debug-trait-format-macros.md
issues/done/696-debug-trait-format-macros.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/714-wrapper-retirement
- **Branch**: feature/714-wrapper-retirement
- **HEAD**: 0858d35fbbfed2472add6ad31fdb9a603569fea7
- **Status vs origin/master**: 484 behind, 6 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
0858d35f (HEAD -> feature/714-wrapper-retirement) refactor: retire host-linker, extract debug-tools as independent crate
c1bf4931 feat(compiler): bridged WASI P2 component path with stdout output
21bfb90a docs(research): add P2 bridged WASI path roadmap; update issue references
ad4a6416 fix: route P2 targets through stub-only emit path for library exports
1cdd4d88 refactor: remove Python P2 wrapper scripts and update gates
```
- **Related plans / issues**:
issues/done/714-wasi-p2-emitter-native-component-output.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/fixpoint-s3-green
- **Branch**: wave/730-fixpoint-s3-green
- **HEAD**: d54ec49f2a8ee1ff13682e836c9fb8571b3b131e
- **Status vs origin/master**: 260 behind, 0 ahead (merged: True)
- **Dirty files**:
```
?? .ark-debug/
```
- **Last 5 commits**:
```
d54ec49f (HEAD -> wave/730-fixpoint-s3-green) fix(wasm): stub http/exists GC emit that broke s3 validate
dcf471bd fix(loader): avoid fs::exists in find_package_root on wasm32-gc
7cda3c7c fix(selfhost): clear several s3 wasm32-gc validate blockers
7ac0e94e fix(mir): pass CoreHirMirView into fn-index facade
e763f20b test(selfhost): gate KEEP_CLOCK --time smoke and close mem64 clock docs
```
- **Related plans / issues**:
issues/done/730-bootstrap-wasm-4gb-memory-limit.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/lane-807-fail-list
- **Branch**: wave/807-fail-list-lane
- **HEAD**: 0155ad5216addcc6b5c844322f430b6c68c32dfc
- **Status vs origin/master**: 110 behind, 18 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
0155ad52 (HEAD -> wave/807-fail-list-lane) fix(807): L19 host_capability stubs, GC write, ro/deny flags
dbf9cd39 fix(807): L18 GC fs read and P2 open/read/close
4f31fbba fix(807): L17 host P2 args/clock/random and assert panic
a4cec131 fix(807): L16 P2 exit import index and host stub
55dbcb60 fix(807): L15 parse junk/exponent, BitSet, vec cap0
```
- **Related plans / issues**:
docs/plans/807-fixture-parity-remaining-failures.md
issues/done/807-fixture-parity-367-remaining-failures.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/lane-807-updated
- **Branch**: wave/807-updated-lane
- **HEAD**: c44cbee09f6bb7d0862c43e594a610c2075e3cec
- **Status vs origin/master**: 110 behind, 22 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
c44cbee0 (HEAD -> wave/807-updated-lane, wave/807-fixture-parity) docs(807): fix L21 tip hash and close wording
700b5335 docs(807): record L21 FAIL=0 tip fb3eb858
fb3eb858 fix(807): L21 fixture parity FAIL=0 and close issue
583d2839 fix(807): L20 trait/hashmap/core root-cause classes
0155ad52 (wave/807-fail-list-lane) fix(807): L19 host_capability stubs, GC write, ro/deny flags
```
- **Related plans / issues**:
docs/plans/807-fixture-parity-remaining-failures.md
issues/done/807-fixture-parity-367-remaining-failures.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/latency-emit
- **Branch**: wave/latency-emit-code
- **HEAD**: 9566727f19444c3554cc5bc3d9f95382acde146d
- **Status vs origin/master**: 233 behind, 1 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
9566727f (HEAD -> wave/latency-emit-code) perf(wasm): instrument emit.code locals vs insts split (#829)
08d4a1de fix(wasm): pass callee String to emit_i8_to_i32, not handler_id
734cbb91 docs: adopt native-cpp selfhost executor design
78b230a0 perf(mir): cache callee lookup in type propagate (#829).
41836842 fix(wasm): stop cloning i32 handler_id in simd/convert dispatch
```
- **Related plans / issues**:
(none found)
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/phase-a-fallback-handler-id
- **Branch**: wave/phase-a-fallback-handler-id
- **HEAD**: 314574bb684f6965c22583435866957b0287f1a2
- **Status vs origin/master**: 202 behind, 7 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
314574bb (HEAD -> wave/phase-a-fallback-handler-id) chore(#822): Phase F cleanup, restore P2 component emit, close issue
1e1f61f5 feat(stdlib): clear remaining legacy_emitter CoreOps (#822 A–E)
bcae1d59 feat(core-ops): add scalar.f64_trunc/nearest target_intrinsics
e8f329dd feat(stdlib): migrate parse/push_char/len/sqrt CoreOps off legacy_emitter
91bc0c37 feat(core-ops): Wave 0 — simd target_intrinsic + remove_i32 normal_call
```
- **Related plans / issues**:
(none found)
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-696-debug-trait
- **Branch**: wave/696-debug-trait
- **HEAD**: 1a0aecf0b22c14ea45756cd6905819d2bc7ce25f
- **Status vs origin/master**: 110 behind, 2 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
1a0aecf0 (HEAD -> wave/696-debug-trait) close(696): Debug f-string/assert_eq_debug docs and residual CoreOp note.
92310c77 feat(696): add f-string {:?}, Debug assert messages, and assert_eq_debug.
3efaed4f docs(plans): add parallel close wave plans for open issues.
4c73ac1e docs(727): record verify quick PASS on bridged close note.
cef65d2c fix(727): prefer cargo wasm-tools in absence gate.
```
- **Related plans / issues**:
docs/plans/696-debug-trait-format-macros.md
issues/done/696-debug-trait-format-macros.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-704-json-full
- **Branch**: wave/704-json-full
- **HEAD**: 94e1e96cb8c9e2c370fe61f4b1c56ae509be32f8
- **Status vs origin/master**: 110 behind, 2 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
94e1e96c (HEAD -> wave/704-json-full) close(704): delete LSP/DAP JSON facades; expand rfc8259 corpus
37a5467d feat(704): migrate LSP JSON helpers to std::json with RFC8259 escapes
3efaed4f docs(plans): add parallel close wave plans for open issues.
4c73ac1e docs(727): record verify quick PASS on bridged close note.
cef65d2c fix(727): prefer cargo wasm-tools in absence gate.
```
- **Related plans / issues**:
docs/plans/704-std-json-full-compliance.md
issues/done/704-std-json-full-compliance.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-705-toml-full
- **Branch**: wave/705-toml-full
- **HEAD**: a6a4759f07caa55f5bbb8b168336377ed21e9fa5
- **Status vs origin/master**: 110 behind, 2 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
a6a4759f (HEAD -> wave/705-toml-full) close(705): mark std::toml TOML 1.0 compliance done
0e15762b feat(stdlib): complete std::toml TOML 1.0 migration for #705
3efaed4f docs(plans): add parallel close wave plans for open issues.
4c73ac1e docs(727): record verify quick PASS on bridged close note.
cef65d2c fix(727): prefer cargo wasm-tools in absence gate.
```
- **Related plans / issues**:
docs/plans/705-std-toml-full-compliance.md
issues/done/705-std-toml-full-compliance.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-727-host-bridge
- **Branch**: wave/727-host-bridge
- **HEAD**: a0a3bf3ddeeec6484c1b3e214792ffeabb6da3b8
- **Status vs origin/master**: 110 behind, 2 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
a0a3bf3d (HEAD -> wave/727-host-bridge) docs(727): sync plans index and done-issue links.
66fb992f docs(727): mark host-bridge retirement plan verified.
3efaed4f docs(plans): add parallel close wave plans for open issues.
4c73ac1e docs(727): record verify quick PASS on bridged close note.
cef65d2c fix(727): prefer cargo wasm-tools in absence gate.
```
- **Related plans / issues**:
docs/plans/727-host-bridge-retirement.md
issues/done/727-arukellt-host-bridge-retirement.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-807-fixture-parity
- **Branch**: wave/807-fixture-parity
- **HEAD**: c44cbee09f6bb7d0862c43e594a610c2075e3cec
- **Status vs origin/master**: 110 behind, 22 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
c44cbee0 (HEAD -> wave/807-fixture-parity, wave/807-updated-lane) docs(807): fix L21 tip hash and close wording
700b5335 docs(807): record L21 FAIL=0 tip fb3eb858
fb3eb858 fix(807): L21 fixture parity FAIL=0 and close issue
583d2839 fix(807): L20 trait/hashmap/core root-cause classes
0155ad52 (wave/807-fail-list-lane) fix(807): L19 host_capability stubs, GC write, ro/deny flags
```
- **Related plans / issues**:
docs/plans/807-fixture-parity-remaining-failures.md
issues/done/807-fixture-parity-367-remaining-failures.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-809-wat-roundtrip
- **Branch**: wave/809-wat-roundtrip
- **HEAD**: 992566929c907d34e80da310d2aa171737acf6f3
- **Status vs origin/master**: 110 behind, 2 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
99256692 (HEAD -> wave/809-wat-roundtrip) close(809): WAT roundtrip green after match lowering fix.
4e07e2a6 fix(809): repair match lowering that emitted ill-formed Wasm if/else.
3efaed4f docs(plans): add parallel close wave plans for open issues.
4c73ac1e docs(727): record verify quick PASS on bridged close note.
cef65d2c fix(727): prefer cargo wasm-tools in absence gate.
```
- **Related plans / issues**:
docs/plans/809-wat-roundtrip-failure.md
issues/done/809-wat-roundtrip-failure.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-810-component-composite
- **Branch**: wave/810-component-composite
- **HEAD**: a8467f41e0f6fea85c6d8c871362889001e7dfa5
- **Status vs origin/master**: 128 behind, 10 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
a8467f41 (HEAD -> wave/810-component-composite) feat(810): GC Result/List/Tuple canonical ABI on library_component path.
7c6f5e0d feat(810): GC Option canonical ABI on library_component path.
6f967d88 Merge wave/686-merge into wave/810-component-composite.
9aaa3140 fix(bootstrap): align driver component-delegate patch with current driver source.
fc585d50 docs(rfc-007): add D5.1 component WIT type pool section.
```
- **Related plans / issues**:
docs/plans/810-component-interop-failures.md
issues/done/810-component-interop-failures.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-810-component-interop
- **Branch**: wave/810-component-interop
- **HEAD**: b913bf9d9f2493fd2aecd90cf3f232185c123045
- **Status vs origin/master**: 110 behind, 3 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
b913bf9d (HEAD -> wave/810-component-interop) fix(810): clear remaining component interop failures (fail 6→0).
f2b2a899 feat(810): GC Option/Result/List/Record/Tuple component ABI adapters.
21894f70 fix(810): restore p1-component for library interop under default wasi-p2.
3efaed4f docs(plans): add parallel close wave plans for open issues.
4c73ac1e docs(727): record verify quick PASS on bridged close note.
```
- **Related plans / issues**:
docs/plans/810-component-interop-failures.md
issues/done/810-component-interop-failures.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-810-lane-C-list
- **Branch**: wave/810-lane-C-list
- **HEAD**: 734e5d431d72afbdd459a7e209a7d39cec02ea49
- **Status vs origin/master**: 169 behind, 0 ahead (merged: True)
- **Dirty files**:
```
?? tests/component-interop/jco/list-first/list_first.wit
```
- **Last 5 commits**:
```
734e5d43 (HEAD -> wave/810-lane-C-list, wave/810-lane0-scalar, wave/810-lane-G-f32-misc, wave/810-lane-F-option-result, wave/810-lane-E-enum-variant, wave/810-lane-D-record-tuple) Lane A: narrow integer/bool/char scalar component interop fixes.
09de7d24 research(722): classify HOF call sites and prototype call_ref.
5c265642 Merge branch 'wave/1-string-adapter' into master
9595786f (wave/1-string-adapter) component-emit: add canonical ABI string adapter for single string greet.
bdfdbcfe Merge branch 'wave-2-723-exnref-eval'
```
- **Related plans / issues**:
docs/plans/810-component-interop-failures.md
issues/done/810-component-interop-failures.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-810-lane-D-record-tuple
- **Branch**: wave/810-lane-D-record-tuple
- **HEAD**: 734e5d431d72afbdd459a7e209a7d39cec02ea49
- **Status vs origin/master**: 169 behind, 0 ahead (merged: True)
- **Dirty files**:
```
?? tests/component-interop/jco/record-add/point_add.wit
?? tests/component-interop/jco/tuple-swap/tuple_swap.wit
```
- **Last 5 commits**:
```
734e5d43 (HEAD -> wave/810-lane-D-record-tuple, wave/810-lane0-scalar, wave/810-lane-G-f32-misc, wave/810-lane-F-option-result, wave/810-lane-E-enum-variant, wave/810-lane-C-list) Lane A: narrow integer/bool/char scalar component interop fixes.
09de7d24 research(722): classify HOF call sites and prototype call_ref.
5c265642 Merge branch 'wave/1-string-adapter' into master
9595786f (wave/1-string-adapter) component-emit: add canonical ABI string adapter for single string greet.
bdfdbcfe Merge branch 'wave-2-723-exnref-eval'
```
- **Related plans / issues**:
docs/plans/810-component-interop-failures.md
issues/done/810-component-interop-failures.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-810-lane-E-enum-variant
- **Branch**: wave/810-lane-E-enum-variant
- **HEAD**: 734e5d431d72afbdd459a7e209a7d39cec02ea49
- **Status vs origin/master**: 169 behind, 0 ahead (merged: True)
- **Dirty files**:
```
?? -
?? tests/component-interop/jco/enum-color-code/enum_color_code.wit
```
- **Last 5 commits**:
```
734e5d43 (HEAD -> wave/810-lane-E-enum-variant, wave/810-lane0-scalar, wave/810-lane-G-f32-misc, wave/810-lane-F-option-result, wave/810-lane-D-record-tuple, wave/810-lane-C-list) Lane A: narrow integer/bool/char scalar component interop fixes.
09de7d24 research(722): classify HOF call sites and prototype call_ref.
5c265642 Merge branch 'wave/1-string-adapter' into master
9595786f (wave/1-string-adapter) component-emit: add canonical ABI string adapter for single string greet.
bdfdbcfe Merge branch 'wave-2-723-exnref-eval'
```
- **Related plans / issues**:
docs/plans/810-component-interop-failures.md
issues/done/810-component-interop-failures.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-810-lane-F-option-result
- **Branch**: wave/810-lane-F-option-result
- **HEAD**: 734e5d431d72afbdd459a7e209a7d39cec02ea49
- **Status vs origin/master**: 169 behind, 0 ahead (merged: True)
- **Dirty files**:
```
?? option_bool.wit
```
- **Last 5 commits**:
```
734e5d43 (HEAD -> wave/810-lane-F-option-result, wave/810-lane0-scalar, wave/810-lane-G-f32-misc, wave/810-lane-E-enum-variant, wave/810-lane-D-record-tuple, wave/810-lane-C-list) Lane A: narrow integer/bool/char scalar component interop fixes.
09de7d24 research(722): classify HOF call sites and prototype call_ref.
5c265642 Merge branch 'wave/1-string-adapter' into master
9595786f (wave/1-string-adapter) component-emit: add canonical ABI string adapter for single string greet.
bdfdbcfe Merge branch 'wave-2-723-exnref-eval'
```
- **Related plans / issues**:
docs/plans/810-component-interop-failures.md
issues/done/810-component-interop-failures.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-810-lane-G-f32-misc
- **Branch**: wave/810-lane-G-f32-misc
- **HEAD**: 734e5d431d72afbdd459a7e209a7d39cec02ea49
- **Status vs origin/master**: 169 behind, 0 ahead (merged: True)
- **Dirty files**:
```
?? tests/component-interop/jco/multi-type-exports/abs_else_expr.ark
?? tests/component-interop/jco/multi-type-exports/abs_i64.ark
?? tests/component-interop/jco/multi-type-exports/abs_only.ark
?? tests/component-interop/jco/multi-type-exports/abs_sub.ark
?? tests/component-interop/jco/multi-type-exports/abs_swap.ark
?? tests/component-interop/jco/multi-type-exports/abs_top.ark
?? tests/component-interop/jco/multi-type-exports/identity.ark
?? tests/component-interop/jco/multi-type-exports/my_abs.ark
?? tests/component-interop/jco/multi-type-exports/neg.ark
?? tests/component-interop/jco/multi-type-exports/neg_if.ark
```
- **Last 5 commits**:
```
734e5d43 (HEAD -> wave/810-lane-G-f32-misc, wave/810-lane0-scalar, wave/810-lane-F-option-result, wave/810-lane-E-enum-variant, wave/810-lane-D-record-tuple, wave/810-lane-C-list) Lane A: narrow integer/bool/char scalar component interop fixes.
09de7d24 research(722): classify HOF call sites and prototype call_ref.
5c265642 Merge branch 'wave/1-string-adapter' into master
9595786f (wave/1-string-adapter) component-emit: add canonical ABI string adapter for single string greet.
bdfdbcfe Merge branch 'wave-2-723-exnref-eval'
```
- **Related plans / issues**:
docs/plans/810-component-interop-failures.md
issues/done/810-component-interop-failures.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-822-repr-stdlib
- **Branch**: wave/822-repr-stdlib
- **HEAD**: 16ead49c831ba0d8f77369128fe891bbaf3ab2b4
- **Status vs origin/master**: 110 behind, 16 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
16ead49c (HEAD -> wave/822-repr-stdlib) docs: refresh open issue index-meta timestamp
f09746c3 docs(822): sync close state, #698/ADR-037 carve-out, CoreOp counts
2288b453 migrate(822): finish Vec push/pop/get/new; carve SIMD to #698
232c9552 migrate(822): Vec len/set/get_unchecked off legacy via sealed raw
eb222430 docs(822): record typed set/sort migration; keep issue open
```
- **Related plans / issues**:
docs/plans/822-representation-dependent-stdlib-migration.md
issues/done/822-representation-dependent-stdlib-migration.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-824-early-body
- **Branch**: wave/824-early-body
- **HEAD**: b86e9e3b88385cfbf0fe76d61dc474fe07ba7211
- **Status vs origin/master**: 110 behind, 2 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
b86e9e3b (HEAD -> wave/824-early-body) docs(824): finish wontfix close sync and regenerate indexes
8e708f99 close(824): defer early body lowering after decl_emit gate miss
3efaed4f docs(plans): add parallel close wave plans for open issues.
4c73ac1e docs(727): record verify quick PASS on bridged close note.
cef65d2c fix(727): prefer cargo wasm-tools in absence gate.
```
- **Related plans / issues**:
docs/plans/824-early-body-lowering-worklist.md
issues/done/824-early-body-lowering-worklist.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-826-intern-clone
- **Branch**: wave/826-intern-clone
- **HEAD**: bac3c25e8072b26b0e1d4985f7443107ba2ea173
- **Status vs origin/master**: 110 behind, 2 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
bac3c25e (HEAD -> wave/826-intern-clone) close(826): move intern/clone audit to done with docs sync.
d54cf4ad fix(826): drop NameIndex probe clones; record intern audit.
3efaed4f docs(plans): add parallel close wave plans for open issues.
4c73ac1e docs(727): record verify quick PASS on bridged close note.
cef65d2c fix(727): prefer cargo wasm-tools in absence gate.
```
- **Related plans / issues**:
docs/plans/826-symbol-path-intern-clone-audit.md
issues/done/826-symbol-path-intern-clone-audit.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-834-bootstrap
- **Branch**: wave/834-bootstrap
- **HEAD**: dc776f73a3954591edd57ca92e7558d63dbe6a03
- **Status vs origin/master**: 110 behind, 5 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
dc776f73 (HEAD -> wave/834-bootstrap) fix(834): pin wasm32-gc/wasi-p2 bootstrap and close issue.
841e4319 fix(834): P2 FS host + GC emit fixes; hello compile under host-linker.
f94a030f fix(834): flat-src emit+validate; leave pin blocked on P2 fs I/O.
9c008809 fix(834): keep GC Result locals typed for cmd_init validate.
9e399548 fix(834): map P1/P2 WASI import indices for wasm32-gc emit.
```
- **Related plans / issues**:
docs/plans/834-wasm32-gc-bootstrap-pin.md
issues/done/834-wasm32-gc-bootstrap-pin.md
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-kanban-dashboard
- **Branch**: wave/kanban-dashboard
- **HEAD**: b6d48e679cce5c20d990a0267ce64af9f024e7ce
- **Status vs origin/master**: 196 behind, 1 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
b6d48e67 (HEAD -> wave/kanban-dashboard) Add kanban dashboard for issue/ADR/doc tracking.
61b16260 Update #714 and current-state for gate_510 fix.
331d2592 fix(wasm-gc): select Option/Result match payload by scrutinee T
32a5aa6e Add SourceLocation to MirFunction for source position tracking
02c27b48 feat(wasm-name): add local name subsection and --strip-debug flag
```
- **Related plans / issues**:
(none found)
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

### /home/wogikaze/wgkz/arukellt/.worktrees/wave-rust-fixture-isolation
- **Branch**: wave/rust-fixture-isolation
- **HEAD**: 12d654009d8db4ef4349b07049132d4c7a1811a7
- **Status vs origin/master**: 383 behind, 1 ahead (merged: False)
- **Dirty files**:
```

```
- **Last 5 commits**:
```
12d65400 (HEAD -> wave/rust-fixture-isolation) chore(verify): remove Cargo-built component fixtures from canonical gates
ed1729d1 perf(mir): reachability phase timing and size metrics (#823)
993b81d8 docs(issues): #823 reachability receipt and latency child issues
c46acb17 perf(mir): queue-BFS reachability with FunctionId→MirIndex map (#823)
0c2f0e58 docs(research): add agent tooling latency analysis and AGENTS.md efficiency guidelines
```
- **Related plans / issues**:
(none found)
- **Next steps**: Continue from current HEAD; rebase/merge onto origin/master when ready; run `python3 scripts/manager.py verify lane` or the relevant gate before opening a PR.

## Unnecessary worktrees (safe to delete)

The following worktrees are either already merged into origin/master, correspond to closed PRs, or are otherwise stale. Their HEADs are clean and behind origin/master.
- `/home/wogikaze/wgkz/arrukellt-native-recovery/.worktrees/land-local-master-ci` — `wave/land-local-master`, 110 commits behind origin/master
- `/home/wogikaze/wgkz/arrukellt-native-recovery/.worktrees/land-master-plus-native` — `wave/land-master-plus-native`, 16 commits behind origin/master
- `/home/wogikaze/wgkz/arrukellt-native-recovery/.worktrees/lane-error-enum` — `wave/native-error-enum-ctors`, 34 commits behind origin/master
- `/home/wogikaze/wgkz/arrukellt-native-recovery/.worktrees/lane-hashmap` — `wave/native-hashmap-family`, 32 commits behind origin/master
- `/home/wogikaze/wgkz/arrukellt-native-recovery/.worktrees/lane-neg-harness` — `wave/native-neg-harness`, 34 commits behind origin/master
- `/home/wogikaze/wgkz/arrukellt-native-recovery/.worktrees/lane-residual-scenarios` — `wave/native-residual-scenarios`, 31 commits behind origin/master
- `/home/wogikaze/wgkz/arukellt/.worktrees/714-p2-emitter-native` — `wave/714-p2-emitter-native`, 187 commits behind origin/master
- `/home/wogikaze/wgkz/arukellt/.worktrees/board` — `feature/docs-board`, 188 commits behind origin/master
- `/home/wogikaze/wgkz/arukellt/.worktrees/gate-speedup` — `wave/gate-speedup`, 272 commits behind origin/master
- `/home/wogikaze/wgkz/arukellt/.worktrees/lane-ci-root-cause` — `detached`, 0 commits behind origin/master
- `/home/wogikaze/wgkz/arukellt/.worktrees/wave-1-string-adapter` — `wave/1-string-adapter`, 188 commits behind origin/master
- `/home/wogikaze/wgkz/arukellt/.worktrees/wave-810-lane-B-string` — `wave/810-lane-B-string`, 168 commits behind origin/master
- `/home/wogikaze/wgkz/arukellt/.worktrees/wave-810-lane0-scalar` — `wave/810-lane0-scalar`, 169 commits behind origin/master

## Deletion command reference

To remove a worktree and its directory:

```bash
# For worktrees under the main repo:
git worktree remove <path>

# For worktrees under arrukellt-native-recovery:
# cd /home/wogikaze/wgkz/arrukellt-native-recovery && git worktree remove <path>
```
