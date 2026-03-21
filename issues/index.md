# Issues Index

This file is the shared AI-managed queue for repository work.
Read this first for the current active set, dependency shape, and the location of each detailed issue file.

## Operating Rules

- Read this file before substantial planning or implementation work.
- Keep `active` limited to the tasks that are intentionally in flight; use `priority` and dependency order instead of a single `NEXT` slot.
- Add newly discovered work to `issues/open/` and register it in the JSON index block below in the same change.
- Move completed work from `issues/open/` to `issues/done/`, set `status` to `done`, and update this file in the same change.
- Preserve `WB-###` IDs once assigned. Keep any migration-specific history in `Notes` rather than extra metadata fields.
- Keep summaries short so cross-issue reasoning usually fits in this file without reopening every detailed issue file.

## Active

- WB-059: Expose the experimental `wasm-js-gc` backend contract through public build commands (`p3`; depends on WB-056, WB-057)

## Ready

- none

## Blocked

- WB-008: Record a deployed GitHub Pages smoke URL for the docs shell (`p3`; blocked on repo-level GitHub Pages configuration)

## Recently Done

- WB-063: Allow shebang-executable `.ar` programs through `chef run`
- WB-062: Add parser and typechecker diagnostics for canonical-style guidance
- WB-061: Split lang-backend-wasm into target, ABI, helper analysis, emitter, runtime, closure, and postprocess modules
- WB-060: Share compiler build driver between `arktc` and `chef`
- WB-058: Define the experimental Component Model target contract and naming
- WB-057: Introduce GC-aware wasm value representations in backend codegen
- WB-056: Placeholder for historical `WB-056` dependency reference
- WB-054: Spike a separate Wasm GC backend for zero-runtime-wrapper data layouts
- WB-053: Introduce a backend-oriented wasm IR between High IR and WAT emission
- WB-051: Placeholder for historical `WB-051` dependency reference
- WB-050: Add more source-shape peephole lowerings for common LLM-style pipelines
- WB-049: Generalize feature-gated helper emission across the std surface
- WB-048: Trim dead generic helper emission when suffix-recursion specialization fires

## Index Data

```json
[
  {
    "id": "WB-059",
    "title": "Expose the experimental `wasm-js-gc` backend contract through public build commands",
    "status": "active",
    "priority": "p3",
    "area": [
      "arktc",
      "chef",
      "lang-backend-wasm"
    ],
    "depends_on": [
      "WB-056",
      "WB-057"
    ],
    "blocked_on": [],
    "source": "follow-up after `WB-057` internal GC lowering path landed",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/open/WB-059.md",
    "summary": "`WB-057` added an internal GC-aware `Option<Int>` lowering path and richer wasm value representations, but the public CLI still hard-rejects the target"
  },
  {
    "id": "WB-008",
    "title": "Record a deployed GitHub Pages smoke URL for the docs shell",
    "status": "blocked",
    "priority": "p3",
    "area": [
      "docs",
      "release"
    ],
    "depends_on": [],
    "blocked_on": [
      "repo-level GitHub Pages configuration"
    ],
    "source": "docs app shell exists, but deployment settings live outside the workspace",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/open/WB-008.md",
    "summary": "blocked on repository settings rather than code in this worktree"
  },
  {
    "id": "WB-009",
    "title": "wasm-wasi で Let バインディングを実装し factorial/fibonacci 以外の単純パイプラインを解放する",
    "status": "done",
    "priority": "p1",
    "area": [
      "wasm-backend"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "HighExprKind::Let が emit_expr で bail! されている; powers.ar 等で必要",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-009.md",
    "summary": "verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p arktc --test examples`, and `cargo test`"
  },
  {
    "id": "WB-010",
    "title": "wasm-wasi で List リテラルと基本コレクション操作 (map / filter / join / sum) を実装",
    "status": "done",
    "priority": "p1",
    "area": [
      "wasm-backend"
    ],
    "depends_on": [
      "WB-021",
      "WB-022",
      "WB-023",
      "WB-024"
    ],
    "blocked_on": [],
    "source": "powers.ar, fizz_buzz.ar, map_filter_sum.ar が `calls to join` / `calls to sum` で失敗",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-010.md",
    "summary": "umbrella slice is now closed by `WB-021` through `WB-024`; `powers.ar`, `fizz_buzz.ar`, and `map_filter_sum.ar` all build on wasm-wasi and match their stdout fixtures"
  },
  {
    "id": "WB-015",
    "title": "wasm-wasi で console.println と string() ビルトインを実装し hello_world / factorial / fibonacci を解放",
    "status": "done",
    "priority": "p1",
    "area": [
      "wasm-backend"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "ユーザー要求 \"arktc build --target wasm-wasi で予想通りの出力ができるまで開発を進めてください\"",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-015.md",
    "summary": "commit 61b3bd6: fd_write import + $console.println helper + $string helper 実装"
  },
  {
    "id": "WB-021",
    "title": "wasm-wasi に最小 List<i64> ランタイムを追加し list literal / range を lower する",
    "status": "done",
    "priority": "p1",
    "area": [
      "wasm-backend"
    ],
    "depends_on": [
      "WB-009"
    ],
    "blocked_on": [],
    "source": "`HighExprKind::List` が wasm backend で即 bail されている; `powers.ar` / `map_filter_sum.ar` の前提",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-021.md",
    "summary": "verified with `cargo test -p lang-backend-wasm`, `cargo test -p arktc --test examples`, and `cargo test`"
  },
  {
    "id": "WB-022",
    "title": "wasm-wasi で関数参照と unary closure 呼び出しをコレクション組み込みから使えるようにする",
    "status": "done",
    "priority": "p1",
    "area": [
      "wasm-backend"
    ],
    "depends_on": [
      "WB-011"
    ],
    "blocked_on": [],
    "source": "`map(fizz_buzz_label)` / `map(n -> ...)` / `filter(n -> ...)` が collection lowering の前提",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-022.md",
    "summary": "verified with `cargo test -p lang-backend-wasm --test build` and `cargo test`"
  },
  {
    "id": "WB-023",
    "title": "wasm-wasi で List<i64> 向け map/filter/sum の組み込み lowering を追加",
    "status": "done",
    "priority": "p1",
    "area": [
      "wasm-backend"
    ],
    "depends_on": [
      "WB-021",
      "WB-022"
    ],
    "blocked_on": [],
    "source": "`map_filter_sum.ar` が `calls to \\`sum\\`` で失敗; list collection builtins 未実装",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-023.md",
    "summary": "verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p arktc --test examples`, `cargo run -p arktc -- build example/map_filter_sum.ar --target wasm-wasi --output /tmp/map_filter_sum.wasm`, `wasmer /tmp/map_filter_sum.wasm`, and `cargo test`"
  },
  {
    "id": "WB-024",
    "title": "wasm-wasi に heap-backed String 連結を追加し join を実装",
    "status": "done",
    "priority": "p1",
    "area": [
      "wasm-backend"
    ],
    "depends_on": [
      "WB-021",
      "WB-022"
    ],
    "blocked_on": [],
    "source": "`powers.ar` / `fizz_buzz.ar` が `calls to \\`join\\`` で失敗; 現状 String は literal pointer のみ",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-024.md",
    "summary": "verified with `cargo fmt`, `cargo test`, `cargo run -p arktc -- build example/powers.ar --target wasm-wasi --output /tmp/powers.wasm`, `wasmer /tmp/powers.wasm`, `cargo run -p arktc -- build example/fizz_buzz.ar --target wasm-wasi --output /tmp/fizz_buzz.wasm`, and `wasmer /tmp/fizz_buzz.wasm`"
  },
  {
    "id": "WB-032",
    "title": "Canonicalize parse API to parse.<type> and remove parse_* drift",
    "status": "done",
    "priority": "p1",
    "area": [
      "lang-core"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "user request",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-032.md",
    "summary": "keep `string(value)` as the only canonical value-to-string conversion"
  },
  {
    "id": "WB-033",
    "title": "Normalize example programs to one canonical pipeline/output style",
    "status": "done",
    "priority": "p1",
    "area": [
      "examples"
    ],
    "depends_on": [
      "WB-032"
    ],
    "blocked_on": [],
    "source": "example/*.ar review",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-033.md",
    "summary": "verified with `cargo fmt`, `cargo test -p chef`, `cargo test -p arktc --test examples`, and `cargo test`"
  },
  {
    "id": "WB-034",
    "title": "Add cross-target pure text helpers on WASM backends",
    "status": "done",
    "priority": "p1",
    "area": [
      "lang-backend-wasm"
    ],
    "depends_on": [
      "WB-037"
    ],
    "blocked_on": [],
    "source": "target-support review for surface matrix",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-034.md",
    "summary": "verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p lang-interp`, `cargo test -p arktc`, `cargo test -p chef`, and `cargo test`"
  },
  {
    "id": "WB-037",
    "title": "Add wasm-js string output path for console.println(String)",
    "status": "done",
    "priority": "p1",
    "area": [
      "lang-backend-wasm"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "target-support review for surface matrix",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-037.md",
    "summary": "keep the bridge minimal and capability-scoped"
  },
  {
    "id": "WB-038",
    "title": "Add wasm-wasi stdin.read_text for command-style programs",
    "status": "done",
    "priority": "p1",
    "area": [
      "lang-backend-wasm"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "target-support review for surface matrix",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-038.md",
    "summary": "verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p arktc`, and `cargo test`"
  },
  {
    "id": "WB-041",
    "title": "Grow wasm linear memory in `__alloc` for large list pipelines",
    "status": "done",
    "priority": "p1",
    "area": [
      "lang-backend-wasm"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "user report: `abc444b.wat` crashed with `Exception: out of bounds memory access` on `99999 45`",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-041.md",
    "summary": "`__alloc` now calls `memory.grow` before advancing `heap_ptr` when the requested heap end exceeds current linear memory"
  },
  {
    "id": "WB-044",
    "title": "Replace `abc049c` substring peeling with index-based suffix checks",
    "status": "done",
    "priority": "p1",
    "area": [
      "examples",
      "abs",
      "lang-surface"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "user report: `ABC049C` hit MLE under the generated WASM path",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-044.md",
    "summary": "added `len` and `ends_with_at` so `abc049c` can recurse on an integer end position instead of allocating substring copies"
  },
  {
    "id": "WB-045",
    "title": "Rewrite `abc049c` around whole-string `strip_suffix`",
    "status": "done",
    "priority": "p1",
    "area": [
      "examples",
      "abs",
      "lang-surface"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "user request: current example complexity comes from missing whole-string primitives, not the logic itself",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-045.md",
    "summary": "this keeps the current `Result`-shaped builtin surface but removes the manual index plumbing from the example"
  },
  {
    "id": "WB-046",
    "title": "Add Option-first suffix and branch-compression surface",
    "status": "done",
    "priority": "p1",
    "area": [
      "lang-surface",
      "std"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "user request: string recursion should avoid index plumbing and explicit Result matching",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-046.md",
    "summary": "updated `abc049c` to the compact whole-string recursive style and covered the new surface in `lang-core`, `lang-interp`, and `lang-backend-wasm`"
  },
  {
    "id": "WB-047",
    "title": "Recover a non-allocating suffix fast path under the new Option surface",
    "status": "done",
    "priority": "p1",
    "area": [
      "lang-backend-wasm"
    ],
    "depends_on": [
      "WB-046"
    ],
    "blocked_on": [],
    "source": "follow-up after `abc049c` moved back to whole-string `strip_suffix(...).map(...).unwrap_or(...)`",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-047.md",
    "summary": "added backend pattern-matching for the compact suffix-recursion shape and lowered it to an iterative hidden helper over `(text, end)` without changing source syntax"
  },
  {
    "id": "WB-049",
    "title": "Generalize feature-gated helper emission across the std surface",
    "status": "done",
    "priority": "p1",
    "area": [
      "lang-backend-wasm"
    ],
    "depends_on": [
      "WB-048"
    ],
    "blocked_on": [],
    "source": "user request: classify Wado-style size tactics by Arukellt transferability",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-049.md",
    "summary": "helper gating now follows wasm-lowered helper usage, including option-vs-list map helpers and list-index-specific emission, and the crate is green again under `cargo test -p lang-backend-wasm`"
  },
  {
    "id": "WB-050",
    "title": "Add more source-shape peephole lowerings for common LLM-style pipelines",
    "status": "done",
    "priority": "p1",
    "area": [
      "lang-backend-wasm"
    ],
    "depends_on": [
      "WB-048"
    ],
    "blocked_on": [],
    "source": "user request: classify Wado techniques by near-term value for Arukellt",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-050.md",
    "summary": "added a direct `parse.i64`-to-zero peephole for `parse_or_zero` wrappers and a non-list `split_whitespace()[idx]` lowering through `__split_whitespace_nth`"
  },
  {
    "id": "WB-060",
    "title": "Share compiler build driver between `arktc` and `chef`",
    "status": "done",
    "priority": "p1",
    "area": [
      "arktc",
      "chef"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "user request: make `chef` relate to `arktc` more like `cargo` to `rustc`",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-060.md",
    "summary": "verified with `cargo fmt` and `cargo test`"
  },
  {
    "id": "WB-011",
    "title": "wasm-wasi でクロージャ (Lambda / Fn<A,B>) を実装",
    "status": "done",
    "priority": "p2",
    "area": [
      "wasm-backend"
    ],
    "depends_on": [
      "WB-009"
    ],
    "blocked_on": [],
    "source": "closure.ar が `unsupported wasm type: Fn<Int, Int>` で失敗",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-011.md",
    "summary": "verified with `cargo run -p arktc -- build example/closure.ar --target wasm-wasi --output /tmp/closure.wasm` and `wasmer /tmp/closure.wasm`"
  },
  {
    "id": "WB-012",
    "title": "wasm-wasi でペイロード付き ADT (Result / 任意コンストラクタ) を実装",
    "status": "done",
    "priority": "p2",
    "area": [
      "wasm-backend"
    ],
    "depends_on": [
      "WB-009"
    ],
    "blocked_on": [],
    "source": "result_error_handling.ar が `ADT payload fields are not yet supported in wasm backend` で失敗",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-012.md",
    "summary": "verified with `cargo fmt`, `cargo test`, `cargo run -p arktc -- build example/result_error_handling.ar --target wasm-wasi --output /tmp/result_error_handling.wasm`, and `wasmer /tmp/result_error_handling.wasm`"
  },
  {
    "id": "WB-025",
    "title": "arkli — GHCi 風インタラクティブ REPL の新規バイナリを追加する",
    "status": "done",
    "priority": "p2",
    "area": [
      "repl"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "ユーザー指摘「chef run で実行できるが Haskell インタープリタ風の arkli がない」",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-025.md",
    "summary": "verified with `cargo test -p arkli`, `printf 'let base = 40\\nbase + 2\\n:q\\n' | cargo run -q -p arkli`, and `cargo test`"
  },
  {
    "id": "WB-026",
    "title": "wasm-wasi の heap object codegen で nested allocation 時の base pointer 破壊を潰す",
    "status": "done",
    "priority": "p2",
    "area": [
      "wasm-backend"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "WB-013 で `emit_iter_step` が nested tuple allocation 後に誤った base pointer を返していた",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-026.md",
    "summary": "verified with `cargo fmt`, `cargo test -p arktc --test cli`, and `cargo test`"
  },
  {
    "id": "WB-027",
    "title": "arktc build に `--target wat` を追加し WAT テキストを直接出力できるようにする",
    "status": "done",
    "priority": "p2",
    "area": [
      "arktc"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "ユーザー要求「target に wat も欲しい」",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-027.md",
    "summary": "verified with `cargo fmt`, `cargo test -p arktc`, and `cargo test`"
  },
  {
    "id": "WB-028",
    "title": "arktc build の --target と --emit を分離し 2×3 の出力マトリクスを完成させる",
    "status": "done",
    "priority": "p2",
    "area": [
      "arktc"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "ユーザー指摘「--target wasm-js,wasm-wasi と --emit wat,wat-min で 2x3 のビルド先がある」",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-028.md",
    "summary": "verified with `cargo fmt`, `cargo test -p arktc`, and `cargo test`"
  },
  {
    "id": "WB-029",
    "title": "インタープリタに `stdin.read_text` を追加して競技プログラミング向け入力経路を作る",
    "status": "done",
    "priority": "p2",
    "area": [
      "stdlib",
      "interpreter"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "ユーザー要求「abs/practiceA.arを解いてみよう」時に AtCoder 向けの標準入力 API が存在しなかった",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-029.md",
    "summary": "verified with `cargo fmt`, `cargo test -p lang-interp`, and `cargo test -p chef`; full `cargo test` currently still fails in unrelated `arktc` wasm-wasi regressions covered by WB-026"
  },
  {
    "id": "WB-031",
    "title": "文字列処理の最小競プロ surface を追加して `stdin.read_text().split_whitespace()` を成立させる",
    "status": "done",
    "priority": "p2",
    "area": [
      "stdlib",
      "lang-core"
    ],
    "depends_on": [
      "WB-029",
      "WB-030"
    ],
    "blocked_on": [],
    "source": "`stdin.read_text` の I/O 経路はできたが、AtCoder 入力を実用的に分解する文字列 API がまだない",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-031.md",
    "summary": "verified with `cargo fmt`, `cargo test -p lang-core`, `cargo test -p lang-interp`, and `cargo test -p chef`; full `cargo test` currently still fails in unrelated `arktc` wasm-wasi regressions covered by WB-026"
  },
  {
    "id": "WB-035",
    "title": "Add `chef build` so run/test/build flows are available from one CLI",
    "status": "done",
    "priority": "p2",
    "area": [
      "chef",
      "cli"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "user request",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-035.md",
    "summary": "implemented as a thin wrapper over the existing WASM backend rather than shelling out to `arktc`"
  },
  {
    "id": "WB-036",
    "title": "Add an API-by-target support matrix for WASM-facing development",
    "status": "done",
    "priority": "p2",
    "area": [
      "docs",
      "dx"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "user request after `parse.i64` failed under `chef build --target wasm-wasi`",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-036.md",
    "summary": "the unsupported-call diagnostic now points directly to `docs/std.md#target-support-matrix`"
  },
  {
    "id": "WB-039",
    "title": "Add wasm-js List<i64> parity for literals, range, map/filter/sum, and iter.take",
    "status": "done",
    "priority": "p2",
    "area": [
      "lang-backend-wasm"
    ],
    "depends_on": [
      "WB-037"
    ],
    "blocked_on": [],
    "source": "target-support review for surface matrix",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-039.md",
    "summary": "verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p arktc`, and `cargo test`"
  },
  {
    "id": "WB-040",
    "title": "Add list.join(String) on both WASM targets",
    "status": "done",
    "priority": "p2",
    "area": [
      "lang-backend-wasm"
    ],
    "depends_on": [
      "WB-037"
    ],
    "blocked_on": [],
    "source": "target-support review for surface matrix",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-040.md",
    "summary": "verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p arktc`, and `cargo test`"
  },
  {
    "id": "WB-042",
    "title": "Implement the bundled AtCoder ABS example programs end to end",
    "status": "done",
    "priority": "p2",
    "area": [
      "examples",
      "abs"
    ],
    "depends_on": [
      "WB-031"
    ],
    "blocked_on": [],
    "source": "user request",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-042.md",
    "summary": "added the minimal `strip_suffix` pure helper so `abc049c` can be expressed directly in Arukel"
  },
  {
    "id": "WB-043",
    "title": "Close the remaining WASI gaps needed to emit WAT for all ABS examples",
    "status": "done",
    "priority": "p2",
    "area": [
      "lang-backend-wasm",
      "examples"
    ],
    "depends_on": [
      "WB-042"
    ],
    "blocked_on": [],
    "source": "user request to emit WAT for every file under `example/abs`",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-043.md",
    "summary": "added WASI helpers for incremental stdin line reads, dynamic list indexing, and `strip_suffix`, plus string equality support needed by `abc049c`"
  },
  {
    "id": "WB-048",
    "title": "Trim dead generic helper emission when suffix-recursion specialization fires",
    "status": "done",
    "priority": "p2",
    "area": [
      "lang-backend-wasm"
    ],
    "depends_on": [
      "WB-047"
    ],
    "blocked_on": [],
    "source": "follow-up after `WB-047`; specialized `abc049c` codegen still starts from the generic source shape and can over-emit helpers",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-048.md",
    "summary": "specialized suffix-recursion functions now skip generic call scanning, closure collection, and named-callback collection so the emitted WAT only keeps the hidden `__suffix_rec_*` path plus `ends_with_at`"
  },
  {
    "id": "WB-053",
    "title": "Introduce a backend-oriented wasm IR between High IR and WAT emission",
    "status": "done",
    "priority": "p2",
    "area": [
      "lang-ir",
      "lang-backend-wasm"
    ],
    "depends_on": [
      "WB-051"
    ],
    "blocked_on": [],
    "source": "user request: classify Wado architecture ideas for Arukellt",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-053.md",
    "summary": "added `lang-ir` wasm IR lowering in `crates/lang-ir/src/wasm.rs`, exported via `lang-ir`, with backend-facing `WasmModule` / `WasmFunctionBody` plus helper-usage analysis"
  },
  {
    "id": "WB-061",
    "title": "Split lang-backend-wasm into target, ABI, helper analysis, emitter, runtime, closure, and postprocess modules",
    "status": "done",
    "priority": "p2",
    "area": [
      "lang-backend-wasm"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "user request: split the oversized wasm backend crate",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-061.md",
    "summary": "verified with `cargo fmt`, `cargo test -p lang-backend-wasm --quiet`, and `cargo test`"
  },
  {
    "id": "WB-062",
    "title": "Add parser and typechecker diagnostics for canonical-style guidance",
    "status": "done",
    "priority": "p2",
    "area": [
      "diagnostics"
    ],
    "depends_on": [
      "WB-032"
    ],
    "blocked_on": [],
    "source": "user request",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-062.md",
    "summary": "migrated from a duplicate `WB-034` entry during the `issues/` transition; the original ID is preserved here for historical searchability"
  },
  {
    "id": "WB-063",
    "title": "Allow shebang-executable `.ar` programs through `chef run`",
    "status": "done",
    "priority": "p2",
    "area": [
      "lang-core",
      "chef",
      "examples"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "user request: permit `#!/usr/bin/env -S chef run` at the top of `.ar` files and add one bundled example that exercises it",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-063.md",
    "summary": "verified with `cargo fmt`, `cargo test -p lang-core`, `cargo test -p arktfmt`, `cargo test -p chef`, `cargo test -p arktc --test examples`, `cargo test -p arktc --test workboard`, and `cargo test`"
  },
  {
    "id": "WB-013",
    "title": "wasm-wasi でイテレータ (Seq<T> / iter.unfold / take) を実装",
    "status": "done",
    "priority": "p3",
    "area": [
      "wasm-backend"
    ],
    "depends_on": [
      "WB-011",
      "WB-012"
    ],
    "blocked_on": [],
    "source": "infinite_iter.ar が `unsupported wasm type: Seq<Int>` で失敗",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-013.md",
    "summary": "verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p arktc --test examples`, `cargo run -p arktc -- build example/infinite_iter.ar --target wasm-wasi --output /tmp/infinite_iter.wasm`, `wasmer /tmp/infinite_iter.wasm`, and `cargo test`"
  },
  {
    "id": "WB-014",
    "title": "wasm-wasi で WASI ファイル I/O (fs.read_text) を実装",
    "status": "done",
    "priority": "p3",
    "area": [
      "wasm-backend"
    ],
    "depends_on": [
      "WB-012"
    ],
    "blocked_on": [],
    "source": "file_read.ar が match 不一致エラーで失敗 (Result<String, ReadError> のペイロード ADT が前提)",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-014.md",
    "summary": "verified with `cargo test -p lang-backend-wasm --test build --test examples`, `cargo run -p arktc -- build example/file_read.ar --target wasm-wasi --output /tmp/file_read.wasm`, `wasmer run --dir=. /tmp/file_read.wasm`, and `cargo test`"
  },
  {
    "id": "WB-030",
    "title": "CLI に標準入力注入オプションを追加して `arktc` / `chef` から競技入出力を試せるようにする",
    "status": "done",
    "priority": "p3",
    "area": [
      "cli"
    ],
    "depends_on": [
      "WB-029"
    ],
    "blocked_on": [],
    "source": "`stdin.read_text` を追加しても CLI 側に入力注入手段がないと実用化できない",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-030.md",
    "summary": "verified with `cargo fmt`, `cargo test -p chef`, and `cargo test -p lang-interp`; full `cargo test` currently still fails in unrelated `arktc` wasm-wasi regressions covered by WB-026"
  },
  {
    "id": "WB-051",
    "title": "Placeholder for historical `WB-051` dependency reference",
    "status": "done",
    "priority": "p3",
    "area": [
      "historical",
      "workboard"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "migration placeholder created because `WB-053` depends on `WB-051`, but the original task body is missing from `WORKBOARD.md`",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-051.md",
    "summary": "no standalone `WB-051` entry exists in the last `WORKBOARD.md`; keep this placeholder until the original task history is recovered"
  },
  {
    "id": "WB-054",
    "title": "Spike a separate Wasm GC backend for zero-runtime-wrapper data layouts",
    "status": "done",
    "priority": "p3",
    "area": [
      "lang-backend-wasm"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "user request: classify Wado's Wasm GC strategy for Arukellt",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-054.md",
    "summary": "recorded the feasibility spike in `docs/wasm-gc-spike.md`, centered on `Option<Int>` lowering to a nullable Wasm GC struct instead of a tag-plus-payload heap cell"
  },
  {
    "id": "WB-056",
    "title": "Placeholder for historical `WB-056` dependency reference",
    "status": "done",
    "priority": "p3",
    "area": [
      "historical",
      "workboard"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "migration placeholder created because `WB-057` and `WB-059` depend on `WB-056`, but the original task body is missing from `WORKBOARD.md`",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-056.md",
    "summary": "no standalone `WB-056` entry exists in the last `WORKBOARD.md`; keep this placeholder until the original task history is recovered"
  },
  {
    "id": "WB-057",
    "title": "Introduce GC-aware wasm value representations in backend codegen",
    "status": "done",
    "priority": "p3",
    "area": [
      "lang-backend-wasm"
    ],
    "depends_on": [
      "WB-056"
    ],
    "blocked_on": [],
    "source": "follow-up from `WB-054` Wasm GC feasibility spike",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-057.md",
    "summary": "added an internal `WasmTarget::JavaScriptHostGc` path, a richer emitted wasm type representation layer, and GC type emission for `Option<Int>` as `(ref null $__gc_option_i32)`"
  },
  {
    "id": "WB-058",
    "title": "Define the experimental Component Model target contract and naming",
    "status": "done",
    "priority": "p3",
    "area": [
      "targets",
      "lang-backend-wasm"
    ],
    "depends_on": [],
    "blocked_on": [],
    "source": "follow-up from `WB-055` Component Model evaluation",
    "created_at": "2026-03-21",
    "updated_at": "2026-03-21",
    "file": "issues/done/WB-058.md",
    "summary": "chose `wasm-component-js` as the experimental Component Model contract for JS/Node hosts"
  }
]
```
