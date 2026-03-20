# WORKBOARD

This file is the shared AI-managed task queue for the repository.
AI updates it; humans primarily read it.
It is the canonical place to park follow-up work, pick the next vertical slice, and record verified completion.

## Operating Rules

- Read this file before substantial planning or implementation work.
- Keep exactly one queue item in `Next`. If it becomes stale, promote the highest-priority unblocked item from `Ready`.
- Add newly discovered work to `Ready` unless a concrete dependency blocks it. Put dependency-gated work in `Blocked`.
- Keep task IDs stable as `WB-###`.
- Keep tasks as small vertical slices with one clear outcome.
- Move an item to `Done` only after the matching verification command or test has been run.
- When a task splits, add a follow-up item instead of mutating the old item beyond recognition.
- Keep `Done` entries concise and newest-first.
- Update this file in the same change when work starts, gets blocked, discovers follow-up tasks, or completes.

## Task Schema

Use this exact field order for every task:

### WB-000

title: Example task title
area: workflow
status: READY
priority: P2
owner: unassigned
depends_on: none
source: where this task came from
done_when:

- concrete verification outcome
notes:
- short context for future agents

Field rules:

- `status`: one of `NEXT`, `READY`, `BLOCKED`, `DONE`
- `priority`: one of `P0`, `P1`, `P2`, `P3`
- `owner`: `unassigned`, `ai`, or a short agent label
- `depends_on`: `none` or one or more `WB-###` identifiers
- `source`: file path, test name, user request, or other concrete origin
- `done_when`: 1 to 3 concrete checks
- `notes`: short bullets; newest note first if there are multiple notes

## Next

### WB-059

title: Expose the experimental `wasm-js-gc` backend contract through public build commands
area: arktc/chef/lang-backend-wasm
status: NEXT
priority: P3
owner: unassigned
depends_on: WB-056, WB-057
source: follow-up after `WB-057` internal GC lowering path landed
done_when:

- `arktc build` and `chef build` can opt into the experimental `wasm-js-gc` target behind an explicit contract gate instead of unconditional rejection
- docs/help explain that the target currently emits inspection-oriented GC WAT first and does not promise executable parity with `wasm-js`
- CLI regressions cover target selection, emitted contract text, and at least one focused GC WAT fixture
notes:
- `WB-057` added an internal GC-aware `Option<Int>` lowering path and richer wasm value representations, but the public CLI still hard-rejects the target

## Ready

## Blocked

### WB-008

title: Record a deployed GitHub Pages smoke URL for the docs shell
area: docs/release
status: BLOCKED
priority: P3
owner: unassigned
depends_on: repo-level GitHub Pages configuration
source: docs app shell exists, but deployment settings live outside the workspace
done_when:

- Pages source is configured
- the deployed URL is documented in the repo
- a smoke pass is recorded against the deployed site
notes:
- blocked on repository settings rather than code in this worktree

## Done

### WB-057

title: Introduce GC-aware wasm value representations in backend codegen
area: lang-backend-wasm
status: DONE
priority: P3
owner: unassigned
depends_on: WB-056
source: follow-up from `WB-054` Wasm GC feasibility spike
done_when:

- backend typing and expression emission can represent non-`i32` wasm values such as nullable refs and emitted GC type definitions
- at least one internal surface type (`Option<Int>` from the spike) lowers through the new representation path without `__alloc` or linear-memory option helpers
- focused backend tests assert both the new GC instructions/types and the continued behavior of the existing linear-memory targets
notes:
- added an internal `WasmTarget::JavaScriptHostGc` path, a richer emitted wasm type representation layer, and GC type emission for `Option<Int>` as `(ref null $__gc_option_i32)`
- `unwrap_or(Some/None)` for `Option<Int>` now lowers on the GC path without linear-memory option helpers or `__alloc`, while the existing `wasm-js` path stays on the prior representation
- verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p arktc --test cli --test help --test docs --test readme`, `cargo test -p chef --test cli --test help`, and `cargo test`

### WB-053

title: Introduce a backend-oriented wasm IR between High IR and WAT emission
area: lang-ir/lang-backend-wasm
status: DONE
priority: P2
owner: ai
depends_on: WB-051
source: user request: classify Wado architecture ideas for Arukellt
done_when:

- a new backend-facing IR exists for wasm-specific rewrites that are awkward in the current direct High-IR-to-WAT pipeline
- at least one existing codegen optimization is moved onto that IR
- `cargo test` passes or unrelated worktree failures are explicitly recorded
notes:
- added `lang-ir` wasm IR lowering in `crates/lang-ir/src/wasm.rs`, exported via `lang-ir`, with backend-facing `WasmModule` / `WasmFunctionBody` plus helper-usage analysis
- moved suffix-recursion specialization and the parse-or-zero fast path onto that wasm IR, and `lang-backend-wasm` now lowers through it before helper selection and WAT emission
- verified with `cargo test -p lang-ir --test lowering --quiet`, `cargo test -p lang-backend-wasm compact_suffix_recursion_uses_an_optimized_path_for_large_inputs --quiet`, and `cargo test -p lang-backend-wasm strip_suffix_option_map_unwrap_or_and_any_run_on_both_wasm_targets --quiet`
- full `cargo test --quiet` still has unrelated pre-existing failures in `crates/arktc/tests/docs_site.rs` (`docs_site_uses_relative_assets_and_known_routes`, `docs_site_runtime_helpers_work_in_node`)
- `cargo test -p lang-backend-wasm --tests --quiet` still has unrelated pre-existing closure/lambda failures in `javascript_target_supports_iter_unfold_take_for_int_sequences`, `wasi_target_supports_filter_with_lambda_callbacks`, `wasi_target_supports_iter_unfold_take_with_tuple_state`, and `wasi_target_supports_lambda_callback_values_via_apply`

### WB-058

title: Define the experimental Component Model target contract and naming
area: targets/lang-backend-wasm
status: DONE
priority: P3
owner: unassigned
depends_on: none
source: follow-up from `WB-055` Component Model evaluation
done_when:

- a concrete target name such as `wasm-component-js` or equivalent is chosen and documented
- CLI/help/docs state the host scenario, non-goals, and ABI boundary differences from `wasm-js` and `wasm-wasi`
- one target-selection or docs regression covers the new contract text
notes:
- chose `wasm-component-js` as the experimental Component Model contract for JS/Node hosts
- contract-only slice: current `arktc` and `chef` builds reject the target until a component-aware backend exists
- CLI/help/docs tests now pin the contract text and target selection failure path

### WB-054

title: Spike a separate Wasm GC backend for zero-runtime-wrapper data layouts
area: lang-backend-wasm
status: DONE
priority: P3
owner: ai
depends_on: none
source: user request: classify Wado's Wasm GC strategy for Arukellt
done_when:

- the repo records a concrete feasibility spike for representing at least one Arukellt surface type with Wasm GC instead of linear-memory helpers
- the spike identifies what would need to change in codegen, tests, and target contracts before broader adoption
- follow-up tasks are added for any realistic migration path
notes:
- recorded the feasibility spike in `docs/wasm-gc-spike.md`, centered on `Option<Int>` lowering to a nullable Wasm GC struct instead of a tag-plus-payload heap cell
- follow-up tasks now split the migration into a target-contract slice and a backend representation slice before any production rollout
- later backend cleanup slices restored `cargo test -p lang-backend-wasm`; this spike remains documentation-first and does not ship a GC target

### WB-049

title: Generalize feature-gated helper emission across the std surface
area: lang-backend-wasm
status: DONE
priority: P1
owner: ai
depends_on: WB-048
source: user request: classify Wado-style size tactics by Arukellt transferability
done_when:

- helper emission is driven by precise usage for newly added std-surface helpers rather than broad target-level defaults
- codegen tests assert that unused Option/list/string helpers do not appear in representative WAT output
- `cargo test -p lang-backend-wasm` passes
notes:
- helper gating now follows wasm-lowered helper usage, including option-vs-list map helpers and list-index-specific emission, and the crate is green again under `cargo test -p lang-backend-wasm`
- this slice now also keeps callback-only functions reachable through the optimizer so helper pruning does not break named/lambda callback paths

### WB-048

title: Trim dead generic helper emission when suffix-recursion specialization fires
area: lang-backend-wasm
status: DONE
priority: P2
owner: ai
depends_on: WB-047
source: follow-up after `WB-047`; specialized `abc049c` codegen still starts from the generic source shape and can over-emit helpers
done_when:

- specialized suffix-recursion modules stop emitting unused generic `strip_suffix` / closure helper baggage
- codegen tests assert the specialized WAT stays lean
- `cargo test -p lang-backend-wasm` passes
notes:
- specialized suffix-recursion functions now skip generic call scanning, closure collection, and named-callback collection so the emitted WAT only keeps the hidden `__suffix_rec_*` path plus `ends_with_at`

### WB-050

title: Add more source-shape peephole lowerings for common LLM-style pipelines
area: lang-backend-wasm
status: DONE
priority: P1
owner: ai
depends_on: WB-048
source: user request: classify Wado techniques by near-term value for Arukellt
done_when:

- at least two recurring high-level patterns beyond suffix recursion lower to leaner wasm paths without changing source syntax
- new tests prove both semantic parity and reduced helper/function emission for the chosen patterns
- `cargo test -p lang-backend-wasm` passes
notes:
- added a direct `parse.i64`-to-zero peephole for `parse_or_zero` wrappers and a non-list `split_whitespace()[idx]` lowering through `__split_whitespace_nth`
- helper usage analysis now understands the specialized source shapes, so generic `parse.i64`, `split_whitespace`, and `__list_get` baggage is not emitted for those paths
- fixed the optimizer/backend handoff so callback-only function references remain reachable and lambda closure ABI collection runs against the same wasm-lowered tree used for emission
- `cargo fmt && cargo test -p lang-backend-wasm`

### WB-047

title: Recover a non-allocating suffix fast path under the new Option surface
area: lang-backend-wasm
status: DONE
priority: P1
owner: ai
depends_on: WB-046
source: follow-up after `abc049c` moved back to whole-string `strip_suffix(...).map(...).unwrap_or(...)`
done_when:

- compact whole-string suffix recursion no longer materializes shrinking strings on active WASM targets
- a large `abc049c`-style input regression covers the optimized path
- `cargo test` passes with the optimized runtime path in place
notes:
- added backend pattern-matching for the compact suffix-recursion shape and lowered it to an iterative hidden helper over `(text, end)` without changing source syntax
- `cargo test -p lang-backend-wasm` and the relevant `arktc` CLI regression pass; full `cargo test` is still blocked by the pre-existing `docs_site` failures in this worktree

### WB-046

title: Add Option-first suffix and branch-compression surface
area: lang-surface/std
status: DONE
priority: P1
owner: ai
depends_on: none
source: user request: string recursion should avoid index plumbing and explicit Result matching
done_when:

- `strip_suffix(text, suffix)` returns `Option<String>` with `Some` / `None`
- `Option.map`, `Option.unwrap_or`, and `List.any` compile and run on interpreter and active WASM targets
- `abc049c` can be expressed in the compact whole-string recursive style and verified with `cargo test`
notes:
- updated `abc049c` to the compact whole-string recursive style and covered the new surface in `lang-core`, `lang-interp`, and `lang-backend-wasm`

### WB-045

title: Rewrite `abc049c` around whole-string `strip_suffix`
area: examples/abs/lang-surface
status: DONE
priority: P1
owner: ai
depends_on: none
source: user request: current example complexity comes from missing whole-string primitives, not the logic itself
done_when:

- `example/abs/src/abc049c.ar` no longer threads `end` indices through the recursion
- a compile regression covers the recursive `strip_suffix` style
- verified with `cargo fmt` and `cargo test`
notes:
- this keeps the current `Result`-shaped builtin surface but removes the manual index plumbing from the example

### WB-044

title: Replace `abc049c` substring peeling with index-based suffix checks
area: examples/abs/lang-surface
status: DONE
priority: P1
owner: ai
depends_on: none
source: user report: `ABC049C` hit MLE under the generated WASM path
done_when:

- the language exposes non-allocating suffix-check primitives needed by `abc049c`
- `example/abs/src/abc049c.ar` no longer materializes shrinking strings at each step
- verified with `cargo test`, ABS sample runs, and regenerated `abc049c.wat`
notes:
- added `len` and `ends_with_at` so `abc049c` can recurse on an integer end position instead of allocating substring copies

### WB-043

title: Close the remaining WASI gaps needed to emit WAT for all ABS examples
area: lang-backend-wasm/examples
status: DONE
priority: P2
owner: ai
depends_on: WB-042
source: user request to emit WAT for every file under `example/abs`
done_when:

- `stdin.read_line()` lowers on `wasm-wasi`
- ABS-style `List<String>` indexing and `strip_suffix` build on `wasm-wasi`
- verified with `cargo fmt`, `cargo test`, and WAT generation for every `example/abs/src/*.ar`
notes:
- added WASI helpers for incremental stdin line reads, dynamic list indexing, and `strip_suffix`, plus string equality support needed by `abc049c`

### WB-042

title: Implement the bundled AtCoder ABS example programs end to end
area: examples/abs
status: DONE
priority: P2
owner: ai
depends_on: WB-031
source: user request
done_when:

- every `example/abs/src/*.ar` task file has a concrete implementation
- the ABS sample inputs run successfully under `chef run`
- verified with `cargo fmt`, `cargo test`, and direct ABS example runs
notes:
- added the minimal `strip_suffix` pure helper so `abc049c` can be expressed directly in Arukel
- `abc085c` now uses a closed-form diophantine solve instead of deep recursion to avoid interpreter stack overflow

### WB-041

title: Grow wasm linear memory in `__alloc` for large list pipelines
area: lang-backend-wasm
status: DONE
priority: P1
owner: ai
depends_on: none
source: user report: `abc444b.wat` crashed with `Exception: out of bounds memory access` on `99999 45`
done_when:

- large `wasm-wasi` list pipelines grow memory instead of trapping at the initial page
- a regression runs an `abc444b`-shaped pipeline under `wasmer` with `99999 45`
- verified with `cargo fmt`, `cargo test -p arktc`, and `cargo test`
notes:
- `__alloc` now calls `memory.grow` before advancing `heap_ptr` when the requested heap end exceeds current linear memory

### WB-034

title: Add cross-target pure text helpers on WASM backends
area: lang-backend-wasm
status: DONE
priority: P1
owner: ai
depends_on: WB-037
source: target-support review for surface matrix
done_when:

- `text.split_whitespace()`, `parse.i64(text)`, and `parse.bool(text)` run on both `wasm-js` and `wasm-wasi`
- invalid integer input returns the same error shape on interpreter, `wasm-js`, and `wasm-wasi`
- `cargo test -p lang-backend-wasm && cargo test -p lang-interp && cargo test -p arktc` passes
notes:
- verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p lang-interp`, `cargo test -p arktc`, `cargo test -p chef`, and `cargo test`
- wasm backends now expose pure `split_whitespace`, `parse.i64`, and `parse.bool` helpers on both `wasm-js` and `wasm-wasi`

### WB-038

title: Add wasm-wasi stdin.read_text for command-style programs
area: lang-backend-wasm
status: DONE
priority: P1
owner: ai
depends_on: none
source: target-support review for surface matrix
done_when:

- `stdin.read_text()` lowers and runs on `wasm-wasi`
- a CLI/integration test proves `stdin.read_text().split_whitespace()` input can flow through the wasi target
- `cargo test -p lang-backend-wasm && cargo test -p arktc` passes
notes:
- verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p arktc`, and `cargo test`
- the user-provided `WB-033` identifier collided with an existing completed task, so this slice is recorded as `WB-038`
- keep `stdin.read_line()` out of scope for this slice
- target the full-text stdin ingestion path first

### WB-039

title: Add wasm-js List<i64> parity for literals, range, map/filter/sum, and iter.take
area: lang-backend-wasm
status: DONE
priority: P2
owner: ai
depends_on: WB-037
source: target-support review for surface matrix
done_when:

- `[1, 2, 3]` and `1..=3` lower to `List<i64>` on `wasm-js`
- `list.map(...)`, `list.filter(...)`, `list.sum()`, and `iter.unfold(...).take(n)` run on `wasm-js`
- `cargo test -p lang-backend-wasm` passes with parity coverage against interpreter results
notes:
- verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p arktc`, and `cargo test`
- the user-provided `WB-035` identifier collided with an existing completed task, so this slice is recorded as `WB-039`
- keep the scope to `List<i64>` only
- do not expand to generic collection lowering in this slice

### WB-040

title: Add list.join(String) on both WASM targets
area: lang-backend-wasm
status: DONE
priority: P2
owner: ai
depends_on: WB-037
source: target-support review for surface matrix
done_when:

- `list.join(", ")` lowers and runs on both `wasm-js` and `wasm-wasi`
- joined string output can be piped into `console.println` on both targets
- `cargo test -p lang-backend-wasm && cargo test -p arktc` passes
notes:
- verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p arktc`, and `cargo test`
- the user-provided `WB-036` identifier collided with an existing completed task, so this slice is recorded as `WB-040`
- keep separator handling identical across interpreter and both WASM targets

### WB-037

title: Add wasm-js string output path for console.println(String)
area: lang-backend-wasm
status: DONE
priority: P1
owner: ai
depends_on: none
source: target-support review for surface matrix
done_when:

- `string(i64)` lowers and runs on `wasm-js`
- `console.println(String)` lowers and runs on `wasm-js` through a host console bridge
- `cargo test -p lang-backend-wasm && cargo test -p arktc` passes with a wasm-js console output fixture
notes:
- keep the bridge minimal and capability-scoped
- do not introduce bare `println`; preserve `console.println`

### WB-036

title: Add an API-by-target support matrix for WASM-facing development
area: docs/dx
status: DONE
priority: P2
owner: ai
depends_on: none
source: user request after `parse.i64` failed under `chef build --target wasm-wasi`
done_when:

- `docs/std.md` documents the current API x target contract separately from the bundled-example matrix
- `arktc build` / `chef build` help or diagnostics point readers to that matrix
- verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p arktc`, `cargo test -p chef`, and `cargo test`
notes:
- the unsupported-call diagnostic now points directly to `docs/std.md#target-support-matrix`

### WB-035

title: Add `chef build` so run/test/build flows are available from one CLI
area: chef/cli
status: DONE
priority: P2
owner: ai
depends_on: none
source: user request
done_when:

- `chef build` supports the same `--target` / `--emit` / `--output` matrix as `arktc build`
- `chef` help, CLI tests, and README cover the new build path
- verified with `cargo fmt`, `cargo test -p chef`, and `cargo test`
notes:
- implemented as a thin wrapper over the existing WASM backend rather than shelling out to `arktc`

### WB-034

title: Add parser and typechecker diagnostics for canonical-style guidance
area: diagnostics
status: DONE
priority: P2
owner: ai
depends_on: WB-032
source: user request
done_when:

- unknown or discouraged forms such as `parse_int(x)`, `x.to_string()`, and redundant `|> x -> f(x)` produce structured guidance
- JSON diagnostics include concrete `suggested_fix` values for the canonical replacements
- `cargo test -p lang-core` passes with diagnostic golden tests
notes:
- verified with `cargo fmt`, `cargo test -p lang-core`, and `cargo test`
- parser/typechecker now emit canonical-style guidance for redundant pipe lambdas, `.to_string()`, and legacy `parse_int(...)`

### WB-032

title: Canonicalize parse API to parse.<type> and remove parse_* drift
area: lang-core
status: DONE
priority: P1
owner: ai
depends_on: none
source: user request
done_when:

- `parse.i64` and `parse.bool` are the canonical builtins, while `parse_int` / `parse_bool` now fall through to structured unknown-call diagnostics
- diagnostics suggest `parse.<type>` when legacy names are used
- verified with `cargo test -p lang-core`, `cargo test -p lang-interp`, `cargo test -p chef`, and `cargo test`
notes:
- keep `string(value)` as the only canonical value-to-string conversion
- no `parse_string` alias was added

### WB-033

title: Normalize example programs to one canonical pipeline/output style
area: examples
status: DONE
priority: P1
owner: ai
depends_on: WB-032
source: example/*.ar review
done_when:

- example programs use one output form consistently (`value |> console.println` or a single approved alternative)
- placeholder `_`, redundant pipe lambdas, and mixed range forms are removed from examples
- `cargo test -p chef` passes with updated snapshots or fixtures if needed
notes:
- verified with `cargo fmt`, `cargo test -p chef`, `cargo test -p arktc --test examples`, and `cargo test`
- `file_read.ar` and `result_error_handling.ar` now render to `String` first and use one final `|> console.println`, while `powers.ar` and `map_filter_sum.ar` use named helpers instead of equivalent inline lambdas

### WB-031

title: 文字列処理の最小競プロ surface を追加して `stdin.read_text().split_whitespace()` を成立させる
area: stdlib/lang-core
status: DONE
priority: P2
owner: ai
depends_on: WB-029, WB-030
source: `stdin.read_text` の I/O 経路はできたが、AtCoder 入力を実用的に分解する文字列 API がまだない
done_when:

- `split_whitespace() -> List<String>` と `parse_int() -> Result<Int, _>` が parser/typecheck/interpreter を通って動作する
- `practicea` 相当の入力を `stdin.read_text().split_whitespace()` 経由で解ける
- parser/typecheck/interpreter/CLI の対応テストが揃う
notes:
- verified with `cargo fmt`, `cargo test -p lang-core`, `cargo test -p lang-interp`, and `cargo test -p chef`; full `cargo test` currently still fails in unrelated `arktc` wasm-wasi regressions covered by WB-026
- `parse_int()` は `Result<Int, _>` を返し、invalid input は `Err(InvalidInt)` として明示的に失敗する

### WB-026

title: wasm-wasi の heap object codegen で nested allocation 時の base pointer 破壊を潰す
area: wasm-backend
status: DONE
priority: P2
owner: ai
depends_on: none
source: WB-013 で `emit_iter_step` が nested tuple allocation 後に誤った base pointer を返していた
done_when:

- nested allocation を含む heap-backed emitters が `heap_ptr - size` 再計算ではなく固定 base pointer で返る
- nested payload を持つ構築パスに回帰テストが追加される
notes:
- verified with `cargo fmt`, `cargo test -p arktc --test cli`, and `cargo test`
- `emit_construct`, `emit_tuple_literal`, and `emit_list_literal` now pin their base pointer in depth-scoped temp locals before emitting nested heap-backed children

### WB-030

title: CLI に標準入力注入オプションを追加して `arktc` / `chef` から競技入出力を試せるようにする
area: cli
status: DONE
priority: P3
owner: ai
depends_on: WB-029
source: `stdin.read_text` を追加しても CLI 側に入力注入手段がないと実用化できない
done_when:

- `chef run` が process stdin をそのまま `stdin.read_text()` に流し込める
- `chef run --help` と README に最小の pipe 使用例が追加される
- CLI テストが入力付き実行をカバーする
notes:
- verified with `cargo fmt`, `cargo test -p chef`, and `cargo test -p lang-interp`; full `cargo test` currently still fails in unrelated `arktc` wasm-wasi regressions covered by WB-026
- `--stdin-file` は追加せず、既存の pipe 経路を正式仕様として help/README/CLI tests で固定した

### WB-029

title: インタープリタに `stdin.read_text` を追加して競技プログラミング向け入力経路を作る
area: stdlib/interpreter
status: DONE
priority: P2
owner: ai
depends_on: none
source: ユーザー要求「abs/practiceA.arを解いてみよう」時に AtCoder 向けの標準入力 API が存在しなかった
done_when:

- `stdin.read_text()` が `lang-interp` で動作し、テストから入力文字列を注入できる
- `chef run` か同等の開発経路から標準入力付き実行を検証する回帰が追加される
- `practicea` 相当の 1 問が interpreter 経路で入出力つき完走できる
notes:
- verified with `cargo fmt`, `cargo test -p lang-interp`, and `cargo test -p chef`; full `cargo test` currently still fails in unrelated `arktc` wasm-wasi regressions covered by WB-026
- `lang-interp` now exposes injected stdin text to `stdin.read_text()`, and `chef run` forwards process stdin into the interpreter for competitive-programming style runs

### WB-028

title: arktc build の --target と --emit を分離し 2×3 の出力マトリクスを完成させる
area: arktc
status: DONE
priority: P2
owner: ai
depends_on: none
source: ユーザー指摘「--target wasm-js,wasm-wasi と --emit wat,wat-min で 2x3 のビルド先がある」
done_when:

- `--target wasm-js --emit wat` と `--target wasm-wasi --emit wat` が両方動作する
- `--emit wat-min` が空白を除去した 1 行 WAT を出力する
- `--target wat` は廃止または `--target wasm-js --emit wat` へのエイリアスとして維持する
- `cargo test -p arktc` が 2×3 のすべての有効な組み合わせをカバーするテストを持つ
notes:
- verified with `cargo fmt`, `cargo test -p arktc`, and `cargo test`
- `arktc build` now separates ABI selection (`--target wasm-js|wasm-wasi`) from output selection (`--emit wasm|wat|wat-min`), while keeping `--target wat` as a deprecated alias to JS-host WAT

### WB-027

title: arktc build に `--target wat` を追加し WAT テキストを直接出力できるようにする
area: arktc
status: DONE
priority: P2
owner: ai
depends_on: none
source: ユーザー要求「target に wat も欲しい」
done_when:

- `arktc build file.ar --target wat` が WAT テキストを stdout に出力する
- `arktc build file.ar --target wat --output out.wat` がファイルに書き出す
- `cargo test -p arktc` が通る新規テストが存在する
notes:
- verified with `cargo fmt`, `cargo test -p arktc`, and `cargo test`
- `lang-backend-wasm` now exposes `emit_wat`, and `arktc build --target wat` prints JS-host WAT to stdout by default or writes it via `--output`

### WB-013

title: wasm-wasi でイテレータ (Seq<T> / iter.unfold / take) を実装
area: wasm-backend
status: DONE
priority: P3
owner: ai
depends_on: WB-011, WB-012
source: infinite_iter.ar が `unsupported wasm type: Seq<Int>` で失敗
done_when:

- `infinite_iter.ar` が `arktc build --target wasm-wasi` で成功
- `wasmer` 実行結果が infinite_iter.stdout と一致
notes:
- verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p arktc --test examples`, `cargo run -p arktc -- build example/infinite_iter.ar --target wasm-wasi --output /tmp/infinite_iter.wasm`, `wasmer /tmp/infinite_iter.wasm`, and `cargo test`
- wasm-wasi now lowers `Seq<i64>` from `iter.unfold` into heap-backed iterator objects, materializes `take` into `List<i64>`, and supports tuple state plus `Next` / `Done` step objects

### WB-014

title: wasm-wasi で WASI ファイル I/O (fs.read_text) を実装
area: wasm-backend
status: DONE
priority: P3
owner: ai
depends_on: WB-012
source: file_read.ar が match 不一致エラーで失敗 (Result<String, ReadError> のペイロード ADT が前提)
done_when:

- `file_read.ar` が `arktc build --target wasm-wasi` で成功
- `wasmer` に hello.txt を渡した実行結果が file_read.stdout と一致
notes:
- verified with `cargo test -p lang-backend-wasm --test build --test examples`, `cargo run -p arktc -- build example/file_read.ar --target wasm-wasi --output /tmp/file_read.wasm`, `wasmer run --dir=. /tmp/file_read.wasm`, and `cargo test`
- wasm-wasi now lowers `fs.read_text` through WASI `path_open` + `fd_read` + `fd_close`, returning heap-backed `Result<String, ReadError>` values that existing match/ADT lowering can consume

### WB-023

title: wasm-wasi で List<i64> 向け map/filter/sum の組み込み lowering を追加
area: wasm-backend
status: DONE
priority: P1
owner: ai
depends_on: WB-021, WB-022
source: `map_filter_sum.ar` が `calls to \`sum\`` で失敗; list collection builtins 未実装
done_when:

- `map_filter_sum.ar` が `arktc build --target wasm-wasi` で成功する
- `wasmer` 実行結果が `map_filter_sum.stdout` と一致する
- backend tests cover `map`, `filter`, `sum` の `List<i64>` happy path
notes:
- verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p arktc --test examples`, `cargo run -p arktc -- build example/map_filter_sum.ar --target wasm-wasi --output /tmp/map_filter_sum.wasm`, `wasmer /tmp/map_filter_sum.wasm`, and `cargo test`
- wasm-wasi now lowers `List<i64>.map`, `List<i64>.filter`, and `List<i64>.sum` with closure-backed callbacks and linear-memory list buffers

### WB-010

title: wasm-wasi で List リテラルと基本コレクション操作 (map / filter / join / sum) を実装
area: wasm-backend
status: DONE
priority: P1
owner: ai
depends_on: WB-021, WB-022, WB-023, WB-024
source: powers.ar, fizz_buzz.ar, map_filter_sum.ar が `calls to join` / `calls to sum` で失敗
done_when:

- `powers.ar` が `arktc build --target wasm-wasi` で成功し `wasmer` 実行結果が powers.stdout と一致
- `fizz_buzz.ar` と `map_filter_sum.ar` も同様に一致
notes:
- umbrella slice is now closed by `WB-021` through `WB-024`; `powers.ar`, `fizz_buzz.ar`, and `map_filter_sum.ar` all build on wasm-wasi and match their stdout fixtures
- implementation landed as four vertical slices so callback ABI, list runtime, numeric collection ops, and string joining stayed independently verifiable

### WB-012

title: wasm-wasi でペイロード付き ADT (Result / 任意コンストラクタ) を実装
area: wasm-backend
status: DONE
priority: P2
owner: ai
depends_on: WB-009
source: result_error_handling.ar が `ADT payload fields are not yet supported in wasm backend` で失敗
done_when:

- `result_error_handling.ar` が `arktc build --target wasm-wasi` で成功
- `wasmer` 実行結果が result_error_handling.stdout と一致
notes:
- verified with `cargo fmt`, `cargo test`, `cargo run -p arktc -- build example/result_error_handling.ar --target wasm-wasi --output /tmp/result_error_handling.wasm`, and `wasmer /tmp/result_error_handling.wasm`
- wasm-wasi ADTs now lower to heap-backed `{tag, field0, ...}` objects, and match pattern bindings read payload fields by offset

### WB-025

title: arkli — GHCi 風インタラクティブ REPL の新規バイナリを追加する
area: repl
status: DONE
priority: P2
owner: ai
depends_on: none
source: ユーザー指摘「chef run で実行できるが Haskell インタープリタ風の arkli がない」
done_when:

- `cargo run -p arkli` で起動し、式を入力すると評価結果が表示される
- `:load file.ar` でファイルをロードし、その関数を呼び出せる
- `:quit` / `:q` で終了できる
- `cargo test -p arkli` が通る基本的な REPL ループテストが存在する
notes:
- verified with `cargo test -p arkli`, `printf 'let base = 40\nbase + 2\n:q\n' | cargo run -q -p arkli`, and `cargo test`
- added new `arkli` binary with expression evaluation, persistent session `let` bindings, `:load`, `:reload`, `:type`, and `:quit` / `:q`

### WB-024

title: wasm-wasi に heap-backed String 連結を追加し join を実装
area: wasm-backend
status: DONE
priority: P1
owner: ai
depends_on: WB-021, WB-022
source: `powers.ar` / `fizz_buzz.ar` が `calls to \`join\`` で失敗; 現状 String は literal pointer のみ
done_when:

- `List<String>.join(sep)` の wasm-wasi lowering が追加される
- `powers.ar` と `fizz_buzz.ar` が `arktc build --target wasm-wasi` で成功する
- `wasmer` 実行結果が各 `.stdout` fixture と一致する
notes:
- verified with `cargo fmt`, `cargo test`, `cargo run -p arktc -- build example/powers.ar --target wasm-wasi --output /tmp/powers.wasm`, `wasmer /tmp/powers.wasm`, `cargo run -p arktc -- build example/fizz_buzz.ar --target wasm-wasi --output /tmp/fizz_buzz.wasm`, and `wasmer /tmp/fizz_buzz.wasm`
- wasm-wasi strings now use heap-backed writable buffers for `string()` and `join()`, while `console.println` keeps consuming NUL-terminated UTF-8 pointers

### WB-022

title: wasm-wasi で関数参照と unary closure 呼び出しをコレクション組み込みから使えるようにする
area: wasm-backend
status: DONE
priority: P1
owner: ai
depends_on: WB-011
source: `map(fizz_buzz_label)` / `map(n -> ...)` / `filter(n -> ...)` が collection lowering の前提
done_when:

- named function referencesと unary closure values を collection helper 呼び出しに渡せる
- wasm backend tests cover one named-function and one lambda callback path
- 非対応の多引数/高階値ケースは明示的に拒否されたまま
notes:
- verified with `cargo test -p lang-backend-wasm --test build` and `cargo test`
- collection helpers now accept unary named-function references and lambda closures through the shared closure object ABI, while non-unary and higher-order callback cases remain hard errors

### WB-021

title: wasm-wasi に最小 List<i64> ランタイムを追加し list literal / range を lower する
area: wasm-backend
status: DONE
priority: P1
owner: ai
depends_on: WB-009
source: `HighExprKind::List` が wasm backend で即 bail されている; `powers.ar` / `map_filter_sum.ar` の前提
done_when:

- `List<i64>` を線形メモリ上の `{len, items_ptr}` 表現で保持できる
- list literal と `range_inclusive` の wasm-wasi codegen テストが追加される
- 依然未対応の higher-order / string collection 操作は明示的に失敗する
notes:
- verified with `cargo test -p lang-backend-wasm`, `cargo test -p arktc --test examples`, and `cargo test`
- list values now lower to heap-backed `{len, items_ptr}` objects on wasm-wasi
- backend tests cover both int list literals and `range_inclusive`

### WB-011

title: wasm-wasi でクロージャ (Lambda / Fn<A,B>) を実装
area: wasm-backend
status: DONE
priority: P2
owner: ai
depends_on: WB-009
source: closure.ar が `unsupported wasm type: Fn<Int, Int>` で失敗
done_when:

- `closure.ar` が `arktc build --target wasm-wasi` で成功
- `wasmer` 実行結果が closure.stdout と一致
notes:
- verified with `cargo run -p arktc -- build example/closure.ar --target wasm-wasi --output /tmp/closure.wasm` and `wasmer /tmp/closure.wasm`
- wasm backend now lowers unary closures as heap objects `{table_index, captures...}` and applies them via `call_indirect`

### WB-009

title: wasm-wasi で Let バインディングを実装し factorial/fibonacci 以外の単純パイプラインを解放する
area: wasm-backend
status: DONE
priority: P1
owner: ai
depends_on: none
source: HighExprKind::Let が emit_expr で bail! されている; powers.ar 等で必要
done_when:

- `let x = ...; body` 形式の HighExprKind::Let が WASI ターゲットで WAT local として emit される
- `cargo test -p lang-backend-wasm` が全 pass
notes:
- verified with `cargo fmt`, `cargo test -p lang-backend-wasm`, `cargo test -p arktc --test examples`, and `cargo test`
- wasm backend now lowers nested `HighExprKind::Let` bindings into declared locals with `local.set` / `local.get`, including shadow-safe local renaming

### WB-015

title: wasm-wasi で console.println と string() ビルトインを実装し hello_world / factorial / fibonacci を解放
area: wasm-backend
status: DONE
priority: P1
owner: ai
depends_on: none
source: ユーザー要求 "arktc build --target wasm-wasi で予想通りの出力ができるまで開発を進めてください"
done_when:

- `wasmer example/hello_world.wasm` → `Hello, world!`
- `wasmer example/factorial.wasm` → `3628800`
- `wasmer example/fibonacci.wasm` → `55`
notes:
- commit 61b3bd6: fd_write import + $console.println helper + $string helper 実装
- scratch memory 28 bytes (iovec + nwritten + newline + str_buf) を string table 末尾に確保
- matrix.json で 3 例を wasm_wasi_build: true に更新; 全テスト green
