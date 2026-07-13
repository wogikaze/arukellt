# 到達記録（historical milestones）

ステータス: 履歴メモ（現行正本ではない）  
現行の verified state は [`docs/current-state.md`](../current-state.md)。

本ファイルは過去の到達・sweep・件数スナップショットを保管する。
「当時そう判断した」記録であり、現在の Data Model / ADR gaps と矛盾する場合は **current-state を優先**する。

---

## Recent Milestones

> 過去の到達記録。**現行の verified state は本文上部と Data Model / ADR gaps を優先**。
> 「fully GC-native」等の過去主張は後から不完全と判明したものがある。

- **Modular full-compile fixpoint reached (2026-06)** — the pinned bootstrap wasm is now built from the modular `src/compiler/**` tree and reproduces itself byte-for-byte (`sha256(pinned) == sha256(s2) == sha256(s3)`). Collision-aware export naming, CoreHIR i64 widening, shaped generic type annotations, binop operand type peeking, and a conditional `local.tee` peephole landed in the modular pipeline; the legacy monolithic emitter patches in `scripts/selfhost/checks.py` were removed. The bootstrap overlay now includes the `analysis`/`lsp`/`dap` namespaces, so the selfhost wasm serves the IDE gates (`ide-analyze`, `lsp`, `debug-adapter`); the LSP advertises completion, `signatureHelp`, and `codeAction` providers, stdlib definition/hover via manifest index (#334 baseline), AST-inferred hover with doc comments (#336), manifest-driven signature help (#337), and auto-import completion/code actions (#340). The lexer diagnostic position bug from the monolithic era is fixed (goldens updated).
- **Selfhost Phase 1 fixpoint achieved** — `sha256(s2) == sha256(s3)` passes (`attainment: reached`). The selfhost compiler (`src/compiler/main.ark`) reproducibly compiles itself. Multi-file module loading, qualified call resolution, and cross-module type handling are all working. See [Self-Hosting Bootstrap Status](../state/compiler.md#self-hosting-bootstrap-status).
- **`arukellt doc` subcommand added (issue 456)** — stdlib manifest lookup via `arukellt doc <symbol>`. Supports `--json`, `--target`, and fuzzy-match "did you mean?" for unknown symbols.
- **Host capability honesty (#633)** — `std::host::http`, `std::host::sockets`, and `std::host::udp` are not user-reachable on the current selfhost execution path (`call_host_io.ark` dispatches env/fs/process/stdio only). Manifest and [Capability surface](../platform/target-runtime-and-surfaces.md#capability-surface) now cross-link #446/#447/#077/#139. HTTPS is not supported for HTTP.
- **GC lowering on primary (`wasm32-gc`)** — Historical milestone claimed “fully GC-native”; **later found incomplete**. Current reality: partial GC struct/array lowering; `String`/`Vec`/enum still mixed (see Data Model). Do not treat this bullet as current verified state.
- **Component / WIT** — `--emit component`, `--emit wit`, and `--emit all` are available on `wasm32-gc` (legacy alias `wasm32-wasi-p2` may still appear in CLI/impl)
- **Stdlib roadmap issues 039–059** — closed; see `issues/done/`
- **Primary runtime correctness sweep (2026-04)** — wasmtime 29.x DRC GC bug mitigated (null collector workaround); peephole local.tee suppressed for GC-ref locals; nested concat scratch-local clobbering fixed; `eq`/`ne`/`split` builtins implemented. Fixture harness then **575/575 pass** with 31 new primary-run entries (historical `t3-run:`). With the wasmtime 46 upgrade (2026-06), the default GC collector switched from DRC to the copying collector, making the null-collector workaround unnecessary for new code paths.
- **Current open queue** — active work focuses on WASI / `std::host::*` rollout, Component Model depth, and trait-first stdlib (see `issues/open/`)
- **`std::host::process::exit` and `abort` available (issue 445)** — `__intrinsic_process_exit(i32)` and `__intrinsic_process_abort()` are wired into the `wasm32` and `wasm32-gc` WASI emitters via `wasi_snapshot_preview1/proc_exit`. Both are noreturn; the emitter emits `unreachable` after every call site. `abort()` uses `proc_exit(134)` (SIGABRT convention). `std::host::process` is no longer a stub.
- **Associated function syntax for builtin types (issue 701)** — `Vec::new<i32>()`, `String::from("hello")`, and `i32::from("42")` now compile and execute correctly. The parser desugars these to the corresponding intrinsic names (`Vec_new_i32`, `String_from`, `parse_i32`) at AST construction time, so the resolver, typechecker, and MIR lowering require no changes. Existing monomorphic constructors (`Vec_new_i32()`, `String_from()`) continue to work unchanged.
- **In-file test syntax (ADR-041)** — `test` is a contextual keyword (lexer keeps `TK_IDENT`) introducing three declaration forms: `test "name" { ... }` (standalone), `test <fn> "name" { ... }` (function-bound white-box), and `test mod "name" { test ... ; ... }` (1-level nested test module). Test bodies are resolved and type-checked in the enclosing file scope. `arukellt test <file>` discovers and lists tests, then type-checks the file. Test names are not registered as module symbols (not callable from production code). See `docs/adr/ADR-041-in-file-test-syntax.md`.
- **In-file test coverage targets (#715)** — Phase 1 adoption adds ≥180 `test` declarations under `std/` (core, collections, text, bytes pure helpers) and Phase 2 adds ≥60 under `src/compiler/` (lexer/parser/resolver/typechecker/mir/diagnostics pure helpers). In-file tests are white-box unit tests co-located with implementation; integration and side-effectful behavior (host, component, wasm emitter body) remain fixture-only. `python3 scripts/check/check-infile-test-adoption.py` reports progress (advisory, non-blocking). `arukellt test` allows lint warnings `W0005`–`W0007` during check-only discovery so assertion helpers do not fail the gate.
