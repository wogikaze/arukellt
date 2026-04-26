# Open Issues Priority Table (Multi-Agent Scoring)

Generated automatically from `index-meta.json`.

Scoring criteria (0-5 each, total 25):
- **Blocker**: Number of downstream issues blocked.
- **Release**: V5 blocker status or release-track weight.
- **Readiness**: Implementation/design/verification readiness + acceptance progress.
- **Strategic**: Core language / compiler / selfhost / typechecker weight.
- **MA-Suit**: Multi-agent suitability (fewer deps, clear acceptance criteria).

| Rank | ID | Title | Track | Blocker | Release | Readiness | Strategic | MA-Suit | Total |
|------|----|-------|-------|---------|---------|-----------|-----------|---------|-------|
| 1 | 593 | Selfhost Phase 1: Multi-File Fixpoint | selfhost | 3 | 0 | 5 | 5 | 5 | 18 |
| 2 | 510 | T3 emitter: WASI P2 import-table switch (full P2-native c... | wasi-feature | 4 | 0 | 5 | 3 | 5 | 17 |
| 3 | 595 | Language Surface Uplift: Multi-Clause Function Definitions | selfhost-frontend / language-design | 4 | 0 | 3 | 5 | 5 | 17 |
| 4 | 604 | Stdlib Baseline: Contract and Facade Honesty | stdlib | 4 | 0 | 5 | 3 | 5 | 17 |
| 5 | 574 | 574 — Phase 7: Delete `crates/ark-lexer` | selfhost-retirement | 3 | 0 | 4 | 5 | 4 | 16 |
| 6 | 563 | 563 — Phase 5: Delete `crates/ark-stdlib` | selfhost-retirement | 2 | 0 | 4 | 5 | 4 | 15 |
| 7 | 598 | Language Surface Uplift: Expression-Level Comprehensions | selfhost-frontend / language-design | 2 | 0 | 3 | 5 | 5 | 15 |
| 8 | 600 | Type System Stage-Up: Soundness Floor | selfhost / typechecker | 2 | 0 | 3 | 5 | 5 | 15 |
| 9 | 614 | Error Handling Convergence: Compiler Structured Diagnostics | compiler / selfhost | 0 | 0 | 5 | 5 | 5 | 15 |
| 10 | 615 | Error Handling Convergence: Panic / ICE Policy | compiler / runtime / cli | 0 | 0 | 5 | 5 | 5 | 15 |
| 11 | 573 | 573 — Phase 7: Delete `crates/ark-dap` | selfhost-retirement | 2 | 0 | 4 | 5 | 4 | 15 |
| 12 | 580 | 580 — Phase 7: Delete `crates/ark-manifest` | selfhost-retirement | 2 | 0 | 4 | 5 | 4 | 15 |
| 13 | 121 | WASI P2: Canonical ABI ハンドリングの堅牢化 | wasi-feature | 2 | 0 | 5 | 3 | 4 | 14 |
| 14 | 099 | Selfhost compiler: incremental parse design slice | selfhost-frontend | 0 | 0 | 3 | 5 | 5 | 13 |
| 15 | 123 | import 構文と WIT パッケージ識別子の統一方針決定 | language-design | 0 | 0 | 3 | 5 | 5 | 13 |
| 16 | 125 | `compile()` のデフォルトを CoreHIR パスに移行 (Legacy パス廃止) | pipeline-refactor | 2 | 0 | 1 | 5 | 5 | 13 |
| 17 | 285 | Legacy lowering path を隔離・撤去する | corehir | 0 | 0 | 4 | 5 | 4 | 13 |
| 18 | 613 | Error Handling Convergence: Stdlib Result Surface | stdlib | 0 | 0 | 5 | 3 | 5 | 13 |
| 19 | 601 | Type System Stage-Up: Type Schemes and Controlled Let-Gen... | selfhost / typechecker | 3 | 0 | 1 | 5 | 4 | 13 |
| 20 | 520 | Stdlib: allocation / complexity / perf footgun を family 横... | stdlib | 0 | 0 | 4 | 3 | 5 | 12 |
| 21 | 611 | Optimization Uplift: T3-Safe Runtime Unlock | compiler / runtime-perf | 2 | 0 | 1 | 5 | 4 | 12 |
| 22 | 596 | Language Surface Uplift: Function-Level Guards | selfhost-frontend / language-design | 2 | 0 | 1 | 5 | 4 | 12 |
| 23 | 597 | Language Surface Uplift: Real `where` Clauses | selfhost-frontend / language-design | 2 | 0 | 1 | 5 | 4 | 12 |
| 24 | 074 | WASI P2 ネイティブ: P1 アダプタ不要のコンポーネント直接生成 | wasi-feature | 5 | 0 | 1 | 3 | 3 | 12 |
| 25 | 602 | Type System Stage-Up: Qualified Constraints and Coherent ... | selfhost / typechecker | 2 | 0 | 1 | 5 | 4 | 12 |
| 26 | 575 | 575 — Phase 7: Delete `crates/ark-parser` | selfhost-retirement | 4 | 0 | 0 | 5 | 3 | 12 |
| 27 | 576 | 576 — Phase 7: Delete `crates/ark-resolve` | selfhost-retirement | 4 | 0 | 0 | 5 | 3 | 12 |
| 28 | 577 | 577 — Phase 7: Delete `crates/ark-typecheck` | selfhost-retirement | 4 | 0 | 0 | 5 | 3 | 12 |
| 29 | 034 | CLI --wit flag, --emit component workflow, docs | component-model | 0 | 0 | 5 | 3 | 3 | 11 |
| 30 | 045 | std::collections: Deque、PriorityQueue | stdlib | 0 | 0 | 5 | 3 | 3 | 11 |
| 31 | 047 | std::collections: Arena、SlotMap、Interner ／ std::text: Rope | stdlib | 0 | 0 | 5 | 3 | 3 | 11 |
| 32 | 051 | std::time + std::random: 時刻・期間・乱数 | stdlib | 0 | 0 | 5 | 3 | 3 | 11 |
| 33 | 112 | ベンチマーク比較: C/Rust/Go/Grain との自動比較スクリプト | benchmark | 0 | 0 | 5 | 2 | 4 | 11 |
| 34 | 571 | 571 — Phase 6/D: src/ide/dap.ark — debug adapter scaffold... | selfhost-frontend | 2 | 0 | 0 | 5 | 4 | 11 |
| 35 | 512 | Stdlib: trait ベースの再利用可能 surface へ段階移行する | stdlib | 0 | 5 | 0 | 3 | 3 | 11 |
| 36 | 564 | 564 — Phase 5: Delete `crates/arukellt` | selfhost-retirement | 5 | 0 | 0 | 5 | 1 | 11 |
| 37 | 036 | jco JavaScript interop smoke test | component-model | 0 | 0 | 2 | 3 | 5 | 10 |
| 38 | 204 | 204-project-explain-build-explain-and-script-sandbox-surface | main | 0 | 0 | 3 | 2 | 5 | 10 |
| 39 | 205 | 205-docs-and-codebase-intelligence-surfaces | main | 0 | 0 | 3 | 2 | 5 | 10 |
| 40 | 436 | 436-playground-docs-site-integration | main | 0 | 0 | 3 | 2 | 5 | 10 |
| 41 | 468 | 468-playground-build-and-publish-path-proof | main | 0 | 0 | 3 | 2 | 5 | 10 |
| 42 | 469 | 469-extension-playground-surface-points-to-repo-proved-en... | main | 0 | 0 | 3 | 2 | 5 | 10 |
| 43 | 470 | 470-playground-feature-claims-match-implementation | main | 0 | 0 | 3 | 2 | 5 | 10 |
| 44 | 489 | 489-playground-user-visible-entrypoint-wiring | main | 0 | 0 | 3 | 2 | 5 | 10 |
| 45 | 500 | 500-playground-wasm-typecheck-export | main | 0 | 0 | 3 | 2 | 5 | 10 |
| 46 | 531 | Scripts Consolidation Epic: Python CLI Refactoring | main | 0 | 0 | 3 | 2 | 5 | 10 |
| 47 | 610 | Optimization Uplift: Lowering Bottleneck Reduction | compiler / selfhost | 0 | 0 | 1 | 5 | 4 | 10 |
| 48 | 126 | `run_frontend()` の二重 lower を解消 (遅延 lower) | pipeline-refactor | 0 | 0 | 1 | 5 | 4 | 10 |
| 49 | 594 | Selfhost Phase 2: Fixture and Diagnostic Parity | selfhost | 0 | 0 | 1 | 5 | 4 | 10 |
| 50 | 605 | Stdlib Baseline: Host Core-Platform Baseline | stdlib / wasi-feature | 2 | 0 | 1 | 3 | 4 | 10 |
| 51 | 606 | Stdlib Baseline: Structured Data and Semantics Baseline | stdlib | 2 | 0 | 1 | 3 | 4 | 10 |
| 52 | 607 | Stdlib Baseline: Collections Hash Hardening | stdlib | 2 | 0 | 1 | 3 | 4 | 10 |
| 53 | 076 | WASI P2 ネイティブ: wasi:filesystem ネイティブバインディング | wasi-feature | 2 | 0 | 2 | 3 | 3 | 10 |
| 54 | 124 | WIT コンポーネント import — ソース構文・ark.toml・型バインディング生成 | language-design | 0 | 0 | 1 | 5 | 4 | 10 |
| 55 | 543 | 543 — Benchmark: file I/O (I/O-heavy workloads) | benchmark | 0 | 0 | 4 | 2 | 4 | 10 |
| 56 | 578 | 578 — Phase 7: Delete `crates/ark-hir` | selfhost-retirement | 2 | 0 | 0 | 5 | 3 | 10 |
| 57 | 044 | std::collections::hash: HashMap\<K,V\> 汎用化と HashSet\<T\> | stdlib | 3 | 0 | 1 | 3 | 2 | 9 |
| 58 | 589 | Type System Stage-Up Plan (HM Core + Coherent Traits) (Op... | main | 0 | 0 | 2 | 2 | 5 | 9 |
| 59 | 591 | Optimization Uplift Plan (Compile / Run / Size) (Operatio... | main | 0 | 0 | 2 | 2 | 5 | 9 |
| 60 | 508 | Legacy path removal is blocked by CoreHIR lowerer stub | corehir | 0 | 0 | 0 | 5 | 4 | 9 |
| 61 | 612 | Optimization Uplift: Binary Size Squeeze | compiler / runtime-perf | 0 | 0 | 1 | 5 | 3 | 9 |
| 62 | 077 | WASI P2: `std::host::http` facade と runtime 検証 | wasi-feature | 2 | 0 | 1 | 3 | 3 | 9 |
| 63 | 139 | WASI P2: `std::host::sockets` facade と T3 実行検証 | wasi-feature | 2 | 0 | 1 | 3 | 3 | 9 |
| 64 | 603 | Type System Stage-Up: Monomorphization and Lowering Contr... | selfhost / typechecker / lowering | 0 | 0 | 1 | 5 | 3 | 9 |
| 65 | 581 | 581 — Phase 7: Delete `crates/ark-target` | selfhost-retirement | 2 | 0 | 0 | 5 | 2 | 9 |
| 66 | 529 | 100% Self-Hosting Transition Plan (Operational Guide) | main | 0 | 0 | 1 | 2 | 5 | 8 |
| 67 | 546 | Release: Binary Smoke Tests | main | 0 | 0 | 1 | 2 | 5 | 8 |
| 68 | 547 | Release: Determinism Check | main | 0 | 0 | 1 | 2 | 5 | 8 |
| 69 | 548 | Release: LSP E2E Tests | main | 0 | 0 | 1 | 2 | 5 | 8 |
| 70 | 549 | Release: Extension Activation Tests | main | 0 | 0 | 1 | 2 | 5 | 8 |
| 71 | 550 | Release: Formatter CLI-LSP Parity | main | 0 | 0 | 1 | 2 | 5 | 8 |
| 72 | 551 | Release: Failure Recovery Tests | main | 0 | 0 | 1 | 2 | 5 | 8 |
| 73 | 552 | Release: Post-Release Documentation | main | 0 | 0 | 1 | 2 | 5 | 8 |
| 74 | 553 | Release: Binary Distribution | main | 0 | 0 | 1 | 2 | 5 | 8 |
| 75 | 554 | Release: Extension Live Editor Tests | main | 0 | 0 | 1 | 2 | 5 | 8 |
| 76 | 555 | Release: Pre-Release CI Checks | main | 0 | 0 | 1 | 2 | 5 | 8 |
| 77 | 588 | Language Surface Uplift Plan (Operational Guide) | main | 0 | 0 | 1 | 2 | 5 | 8 |
| 78 | 590 | Stdlib Core Platform Baseline Plan (Operational Guide) | main | 0 | 0 | 1 | 2 | 5 | 8 |
| 79 | 592 | Error Handling Convergence Plan (Operational Guide) | main | 0 | 0 | 1 | 2 | 5 | 8 |
| 80 | 599 | Language Surface Uplift: Docs, Fixtures, and Rollout | docs / language-design | 0 | 0 | 1 | 5 | 2 | 8 |
| 81 | 579 | 579 — Phase 7: Delete `crates/ark-diagnostics` | selfhost-retirement | 2 | 0 | 0 | 5 | 1 | 8 |
| 82 | 485 | docs: arukellt component サブコマンド CLI リファレンス | docs | 0 | 0 | 1 | 2 | 4 | 7 |
| 83 | 214 | Extension quality / packaging / marketplace readiness | parallel | 0 | 0 | 2 | 2 | 2 | 6 |
| 84 | 054 | std::wit + std::component: WIT 型、resource handle、canonica... | stdlib | 0 | 0 | 1 | 3 | 2 | 6 |
| 85 | 055 | std::json + std::toml + std::csv: データ形式パーサ | stdlib | 0 | 0 | 1 | 3 | 2 | 6 |
| 86 | 608 | Stdlib Baseline: Docs, Verification, and Benchmark Closeout | stdlib / docs | 0 | 0 | 1 | 3 | 2 | 6 |
| 87 | 136 | ADR-011 に沿った `std::host` layer の段階的ロールアウト | wasi-feature | 0 | 0 | 1 | 3 | 2 | 6 |
| 88 | 582 | 582 — Phase 7 final: remove `Cargo.toml` and `Cargo.lock` | selfhost-retirement | 0 | 0 | 0 | 5 | 1 | 6 |
| 89 | 475 | `arukellt component` サブコマンド (v3 候補) | cli | 2 | 0 | 0 | 1 | 2 | 5 |
| 90 | 476 | `wasm-tools compose` 統合 (v3 候補) | wasm-feature | 0 | 0 | 0 | 3 | 2 | 5 |
