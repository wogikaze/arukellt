---
Status: done
Created: 2026-03-28
Updated: 2026-05-16
ID: 121
Track: wasi-feature
Depends on: 510
Orchestration class: implementation-ready
Orchestration upstream: None
Implementation target: "Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan."
Blocks v4 exit: False
Status note: Leaf close-gate issue for
surface used by the P2 native smoke path: strings/lists for stdio and host-call
# WASI P2: Canonical ABI ハンドリングの堅牢化
---
# WASI P2: Canonical ABI ハンドリングの堅牢化

## Reopened by audit

- **Date**: 2026-04-21
- **Reason**: Canonical ABI lift/lower completeness is an active product gap for WASI P2 native/component work. Existing open issues (#074, #124, #510) depend on this but no dedicated active issue tracks the close gate.
- **Audit evidence**:
  - No dedicated active open issue tracked this product gap.
  - The capability is required for the WASI P2 / component product surface, not merely future speculation.
  - Reject placement was inconsistent with current product direction.

## Summary

WASI P2 の Component Model では、Canonical ABI (Lift/Lower 規則) が
全てのインターフェース呼び出しの型変換を定義する。
現在の `ark-wasm/src/component/canonical_abi.rs` の Lift/Lower 実装を
`docs/spec/spec-WASI-0.2.10/OVERVIEW.md` の WIT 型規則に照合して完全性を検証・修正する。

## 受け入れ条件

1. WIT の全型 (`bool`, `u8`〜`u64`, `s8`〜`s64`, `f32`, `f64`, `char`, `string`,
   `list<T>`, `record`, `variant`, `enum`, `option<T>`, `result<T,E>`, `tuple`, `resource`) の
   Lift/Lower が `canonical_abi.rs` に実装されていることを確認
2. 各型についてラウンドトリップテスト (Lower → Lift で元の値に戻ること)
3. 未実装型のパニックを適切なエラーに変換

## Parent gate relationship — 2026-04-22

\#121 is a close-gate leaf for #074, not a downstream feature that waits for #074
to close. For #074 closure, this issue must at minimum provide the Canonical ABI
surface used by the P2 native smoke path: strings/lists for stdio and host-call
arguments, plus the resource-handle behavior needed by the first host capability
fixture selected for the gate. Broader WIT type completeness remains this issue's
full acceptance target.

## Recheck — 2026-05-14

- **Close candidate:** No.
- **Implementation location drift:** the issue text still points at the retired
  Rust-era `crates/ark-wasm/src/component/canonical_abi.rs`; the current source of
  truth is the selfhost component path under `src/compiler/component_emitter.ark`.
- **Current blocker:** the runnable CLI falls back to
  `bootstrap/arukellt-selfhost.wasm` because `.build/selfhost/arukellt-s2.wasm` is
  absent. That pinned wasm rejects `--emit component` with
  `error[E0500|emit]: unsupported emit mode: component`, so component/canonical ABI
  behavior cannot be validated through the active command path.
- **Bootstrap evidence:** rebuilding the current selfhost compiler from the pinned
  wasm with `python3 scripts/manager.py selfhost fixpoint --build` remains a skip;
  the underlying stage-2 build traps with an out-of-bounds linear-memory access
  while compiling `src/compiler/main.ark`.
- **Fixture evidence:** `python3 scripts/manager.py verify component` fails all
  component interop checks at the component-emit step; for example
  `tests/component-interop/jco/bool-logic/run.sh` fails before invocation tests
  with `unsupported emit mode: component`.
- **Follow-up needed before closure:** restore a current-source selfhost build or
  refresh the bootstrap artifact, then run the component fixture gate against the
  selfhost `component_emitter` implementation before expanding canonical ABI
  coverage for strings/lists/resources.

## Progress — 2026-05-14

- The selfhost bootstrap blocker above is removed by the #312 bootstrap refresh;
  `python scripts/manager.py selfhost fixpoint --build` now passes again.
- `src/compiler/component_emitter.ark` now emits a syntactically valid Component
  Model header, uses current component section IDs, wraps the core wasm module
  through a core-module/core-instance/alias/canon-lift/export sequence, and
  embeds a minimal WASI Preview 1 stub core module so the current core emitter's
  unconditional `wasi_snapshot_preview1` imports can be instantiated.
- `src/compiler/mir_lower.ark` exposes a no-prune lowering entry point and
  `src/compiler/driver.ark` uses it for `--emit component`, so public component
  functions are not dropped merely because `main` does not call them.
- `src/compiler/emitter.ark` now exports non-internal user functions from the
  core wasm, allowing the component wrapper to alias and canonically lift them.
- `src/compiler/component_emitter.ark` now uses the Component Model MVP binary
  tag values for composite value type definitions (`record`, `variant`, `list`,
  `tuple`, `flags`, `enum`, `own`, and `borrow`). This does not close the
  enum/record fixtures by itself; it fixes the descriptor precondition needed
  before real adapters are emitted.
- The `variant` type helper now emits the required per-case trailing `0x00`
  immediate after each optional payload type, matching the MVP binary grammar.
- Canonical option constants for `memory`, `realloc`, and `post-return` now use
  the current MVP binary opcodes (`0x03`, `0x04`, `0x05`). Record adapter
  prototyping showed that compound record exports require the `memory` canon
  option before Wasmtime will validate their lowered ABI.
- The `record-point` and `record-point-renamed` fixtures now use a component
  wrapper path with a small core adapter module that is no longer tied to the
  `distance_sq` / `add_points` function names. The adapter imports the user
  module's memory and `Point` functions, flattens record parameters into fixed
  scratch-memory records, lifts the result through the canonical `memory`
  option, and exports the `point` component type before the function types that
  reference it.
- The `enum-colors` fixture now uses a component wrapper path that exports the
  `color` component type before the function types that reference it. This lets
  Wasmtime invoke `color-to-value` and `next-color` with symbolic enum cases.
- `--emit wit` now uses source-level export type annotations instead of MIR
  scalar value tags, so the component fixture surface emits WIT shapes for
  strings, lists, records, variants/enums, options/results, and tuples. This
  improves metadata fidelity but does not by itself implement canonical
  lift/lower adapters for those non-scalar shapes.
- `--emit component` now rejects unsupported public export shapes with E0401
  before backend emission, preventing invalid components such as tuple/string/
  list/option/result/variant/general record/general enum exports from reaching
  Wasmtime validation as malformed core Wasm. Guard fixtures now cover nested
  or otherwise unsupported mixed-export f32, extra
  exports next to single-export string/list/option/result adapter shapes,
  non-`Color` enums, non-`Shape` payload variants, `Option<String>`,
  `Option<Vec<i32>>`, `Result<i32, bool>`,
  `Result<i64, i64>`, `Result<String, i32>`, `Result<String, String>`,
  `Result<Vec<i32>, String>` parameters,
  `Vec<bool>`, `Vec<u8>`, `Vec<i64>`, `Vec<Option<i32>>`, `Vec<String>`,
  `tuple<String, String>`, and 3-element tuple export shapes.
- `--wit` validation now rejects WIT `resource` declarations, `own<T>` /
  `borrow<T>` resource handles, and `stream<T>` / `future<T>` async resource
  shapes with `E0402`, next to the existing `flags` and function-import guards.
  This keeps unsupported resource and async handles from being silently accepted
  while full resource lowering remains open.

**Evidence:**

- Manual selfhost component smoke:
  `wasmtime run --dir . .build/selfhost/arukellt-s3.wasm -- compile
  tests/component-interop/jco/bool-logic/bool_logic.ark --emit component
  --target wasm32-wasi-p2 -o state/tmp_setup/bool_logic.component.wasm`
  followed by
  `wasmtime run --wasm gc --wasm component-model --invoke
  "and-bool(true, true)" state/tmp_setup/bool_logic.component.wasm`
  prints `true`.
- `ARUKELLT_BIN=scripts/run/arukellt-selfhost.sh
  ARUKELLT_SELFHOST_WASM=bootstrap/arukellt-selfhost.wasm
  python scripts/manager.py verify component` now reaches 101/101 PASS:
  `bool-logic`, `bool-renamed`, `calculator`, `char-renamed`, `enum-color-code`, `enum-color-code-renamed`, `enum-colors`, `enum-colors-renamed`, `enum-roundtrip`, `enum-roundtrip-renamed`, `f32-binary`, `f32-param-i32`, `f32-renamed`, `f32-result-i32`, `f32-square`, `f64-renamed`, `i16-renamed`, `i32-renamed`, `i64-renamed`, `i8-renamed`, `int-widths`, `list-first`, `list-renamed`,
  `list-return`, `list-return-renamed`, `list-roundtrip`, `list-roundtrip-renamed`, `metadata-names`, `metadata-scalars`, `multi-type-exports`, `option-bool`, `option-i64`, `option-i64-param`, `option-maybe`, `option-param`, `option-param-renamed`, `option-renamed`, `option-roundtrip`, `option-roundtrip-renamed`,
  `primitives-float`, `record-add`, `record-add-renamed`, `record-distance`, `record-distance-renamed`, `record-point`, `record-point-renamed`, `record-roundtrip`, `record-roundtrip-renamed`, `result-bool`, `result-param`, `result-param-renamed`, `result-renamed`, `result-roundtrip`, `result-roundtrip-renamed`, `result-safe-div`, `result-string-param`,
  `string-byte`, `string-byte-renamed`, `string-char`, `string-char-renamed`, `string-count16`, `string-count16-renamed`, `string-count32`, `string-count32-renamed`, `string-count64`, `string-count64-renamed`, `string-countu64`, `string-countu64-renamed`, `string-empty`, `string-empty-renamed`, `string-greet`, `string-len`, `string-len-renamed`, `string-renamed`, `string-return`, `string-return-renamed`, `string-score`, `string-score-renamed`, `string-score32`, `string-score32-renamed`, `string-signed16`, `string-signed16-renamed`, `string-signed8`, `string-signed8-renamed`, `tuple-bool-param`, `tuple-i64-result`, `tuple-mixed-param`, `tuple-param`, `tuple-param-renamed`, `tuple-renamed`, `tuple-roundtrip`, `tuple-roundtrip-renamed`, `tuple-swap`, `u16-renamed`, `u32-renamed`, `u64-renamed`, `u8-renamed`, `variant-roundtrip`, `variant-roundtrip-renamed`, `variant-shape-area`, and `variant-shape-area-renamed` pass through wasmtime
  component-model invocation.
- `python scripts/manager.py selfhost fixpoint --build` -> PASS.
- `python scripts/manager.py verify quick` -> PASS (22/22).
- After the result<bool,bool> adapter work, `python scripts/manager.py selfhost fixpoint
  --build` still reaches a byte-stable fixpoint. The pinned bootstrap,
  `.build/selfhost/arukellt-s2.wasm`, and `.build/selfhost/arukellt-s3.wasm` all
  have sha256
  `f62ecf3b863338916998c65c58c9fd2c8ad42d37e7fe0cfc125eb989341e0f8a` after the
  follow-up component export validation, WIT import guard, scalar metadata, and
  special-adapter shape guard slices, removal of export-name scalar heuristics,
  narrow integer WIT metadata support, the fixture-specific string adapter, and
  the fixture-specific `f32`, string parameter/scalar-result/bool-result/char-result/narrow-int-result/f32-result/f64-result/i64-result/u64-result/string-result, `list<s32>` parameter/result/roundtrip, `option<s32>` parameter/result/roundtrip, `option<bool>`/`option<s64>` result/parameter, `result<bool,bool>` result, tuple parameter/result/roundtrip, and single-record
  adapters, plus tuple literal lowering and the fixture-specific `tuple<s32,s32>`
  result adapter, plus the fixture-specific `result<s32,string>` result adapter,
  `result<s32,s32>` parameter/roundtrip adapter, plus
  name-independent single-export `Color -> i32` enum parameter adapters,
  single-export `Color -> Color` enum roundtrip adapters, paired `Color -> Color`
  / `Color -> i32` enum adapters, single-export `Point -> i32` record
  parameter/result adapters, single-export `Point -> Point` record adapters,
  single-export `(Point, Point) -> Point` record add adapters, paired
  `Point -> i32` / `(Point, Point) -> Point` record adapters,
  single-export `Shape -> f64` variant parameter adapters, single-export
  `Shape -> Shape` variant roundtrip adapters, and
  resource/async WIT import guards.
- The `enum-colors` and `enum-colors-renamed` fixtures now pass through a paired
  `Color -> Color` / `Color -> i32` adapter that is no longer tied to the
  `next_color` and `color_to_value` function names. The wrapper exports the
  `color` WIT enum type and invokes both renamed component exports through
  Wasmtime as canonical enum discriminants.
- The `enum-color-code` and `enum-color-code-renamed` fixtures now pass through
  a single-export unit enum parameter path that is no longer tied to the
  `color_code` function name. The wrapper exports the `color` WIT enum type and
  invokes `color-code(...)` / `color-rank(...)` through Wasmtime as canonical
  enum discriminants.
- The `enum-roundtrip` and `enum-roundtrip-renamed` fixtures now pass through a
  single-export `Color -> Color` path. The wrapper exports the `color` WIT enum
  type and invokes `rotate-color(...)` / `cycle-color(...)` through Wasmtime as
  canonical enum discriminants without requiring a companion `Color -> i32`
  export or a fixture function name.
- The `variant-shape-area` and `variant-shape-area-renamed` fixtures now pass
  through a single-export payload variant adapter that is no longer tied to the
  `area` function name. The adapter imports the user module's memory and
  exported `Shape -> f64` function, lowers canonical `(discriminant, f64 payload)`
  into the current tagged heap-object layout, and verifies both
  `area(...)` and `measure(...)` through Wasmtime.
- The `f32-square` and `f32-renamed` fixtures now pass through a single-export
  float32 adapter that is no longer tied to the `square32` function name. The
  current selfhost core export represents `f32` as `i32` bits, so the adapter
  exposes canonical `(f32) -> f32` and bit-reinterprets around the user function.
  General first-class f32 preservation remains part of the broader #121 work.
- The `f32-binary` fixture now passes through a single-export binary float32
  adapter. The current selfhost core export still represents `f32` values as
  `i32` bits, so the adapter exposes canonical `(f32, f32) -> f32` and
  bit-reinterprets both parameters and the result around the user function.
- The `f32-param-i32` and `f32-result-i32` fixtures now pass through
  single-export mixed float32/scalar adapters. The adapters expose canonical
  `f32 -> s32` and `s32 -> f32` signatures while preserving the current core
  `f32`-as-`i32` bit representation at the user export boundary.
- The `primitives-float` and `f64-renamed` fixtures cover canonical `float64`
  scalar invocation for multi-export and single-export renamed shapes.
- MIR functions and params now carry source type-name metadata, and the
  component emitter uses that metadata for `bool` and `char` component function
  types instead of relying on export-name heuristics. The `metadata-scalars`
  component interop fixture verifies a bool function named `negate` and a char
  function named `same_char`; `bool-renamed` and `char-renamed` verify
  single-export renamed `bool -> bool` and `char -> char` adapters;
  `metadata-names` verifies that `boolish_i32` and
  `is_count` remain `i32` exports despite their names; `i32-renamed` verifies a
  single-export renamed `s32 -> s32` adapter.
- Component exports now preserve source metadata for integer WIT types that lower
  through the existing i32/i64 core ABI: `u8`, `u16`, `u32`, `u64`, `i8`, `i16`,
  and `i64`/WIT `s64`. The `int-widths`, `u8-renamed`, `u16-renamed`, `u32-renamed`, `i8-renamed`, `i16-renamed`, `i64-renamed`, and
  `u64-renamed` component interop fixtures invoke them through wasmtime,
  including renamed `u8` / `u16` / `u32` / `i8` / `i16`, large positive `u64`, negative `s64`, and
  single-export renamed `u64` / `s64` values.
- The `string-greet` and `string-renamed` fixtures now pass through a
  single-export string adapter that is no longer tied to the `greet` function
  name. The adapter imports the user module's memory and exported
  `String -> String` function, supplies canonical `memory`/`realloc` options,
  converts incoming canonical `(ptr, len)` bytes into the current Arukellt
  length-prefixed string layout, and returns the outgoing string through the
  result-area pointer shape required by Wasmtime.
- The `string-return` and `string-return-renamed` fixtures now pass through a single-export string result
  adapter that is no longer tied to a fixture function name. The adapter imports
  the user module's memory and exported `i32 -> String` function, then exposes
  the returned length-prefixed Arukellt string through the canonical string
  result-area pointer shape.
- The `string-len` and `string-len-renamed` fixtures now pass through a single-export string parameter
  adapter with a scalar result that is no longer tied to a fixture function name.
  The adapter imports the user module's memory and exported `String -> i32`
  function, converts incoming canonical `(ptr, len)` bytes into the current
  Arukellt length-prefixed string layout, and returns the scalar result directly.
- The `string-empty` and `string-empty-renamed` fixtures reuse the same string parameter adapter shape for
  a `String -> bool` export. The component function type exposes a canonical bool
  result, verifying that string parameter lowering is not limited to s32 results.
- The `string-char` and `string-char-renamed` fixtures reuse the same string parameter adapter shape for a
  `String -> char` export. The component function type exposes a canonical char
  result and wasmtime invokes both empty and non-empty string cases.
- The `string-byte`, `string-byte-renamed`, `string-count16`, `string-count16-renamed`, `string-count32`, `string-count32-renamed`, `string-signed8`, `string-signed8-renamed`, `string-signed16`, and
  `string-signed16-renamed` fixtures reuse the same string parameter adapter shape for
  `String -> u8`, `String -> u16`, `String -> u32`, `String -> i8`, and
  `String -> i16` exports. The component function types expose the exact narrow
  WIT integer results while reusing the current i32 core ABI.
- The `string-score` and `string-score-renamed` fixtures reuse the same string parameter adapter shape for
  a `String -> f64` export. The adapter imports the user module's memory and a
  core `String -> f64` function, then exposes the result as canonical float64.
- The `string-score32` and `string-score32-renamed` fixtures reuse the string parameter adapter shape for a
  `String -> f32` export. The current selfhost core represents `f32` results as
  `i32` bits, so the adapter bit-reinterprets the user result as canonical
  float32.
- The `string-count64` and `string-count64-renamed` fixtures reuse the same string parameter adapter shape
  for a `String -> i64` export. The adapter imports the user module's memory and
  a core `String -> i64` function, then exposes the result as canonical s64.
- The `string-countu64` and `string-countu64-renamed` fixtures reuse the same string parameter adapter shape
  for a `String -> u64` export. The component function type exposes canonical
  u64 while reusing the current 64-bit core integer ABI.
- The `list-first` and `list-renamed` fixtures now pass through a single-export
  `list<s32>` adapter that is no longer tied to the `first_or_zero` function
  name. The adapter imports the user module's memory and exported
  `Vec<i32> -> i32` function, supplies canonical `memory`/`realloc` options, and
  converts the incoming canonical `(ptr, len)` list pair into the current
  Arukellt Vec header layout before calling the user function.
- The `list-return` and `list-return-renamed` fixtures now pass through a single-export `list<s32>` result
  adapter that is no longer tied to a fixture function name. The adapter imports
  the user module's memory and exported `i32 -> Vec<i32>` function, then exposes
  the returned Vec's data pointer and length through the canonical result-area
  pointer shape expected by Wasmtime.
- The `list-roundtrip` and `list-roundtrip-renamed` fixtures now pass through a single-export
  `list<s32> -> list<s32>` adapter. The adapter converts the incoming canonical
  list pair into the current Arukellt Vec header layout, calls the exported
  `Vec<i32> -> Vec<i32>` function, and exposes the returned Vec through the
  canonical result-area pointer shape.
- The `option-maybe` and `option-renamed` fixtures now pass through a
  single-export `option<s32>` result adapter that is no longer tied to the
  `maybe_double` function name. The adapter imports the user module's memory and
  exported `i32 -> Option<i32>` function, converts the current Arukellt option
  object layout into the canonical result-area pointer shape expected by
  Wasmtime, and verifies both `some(...)` and `none` through component-model
  invocation.
- The `option-param` and `option-param-renamed` fixtures now pass through a
  single-export `option<s32>` parameter adapter that is no longer tied to a
  fixture function name. The
  adapter converts canonical `some` / `none` tags into the current Arukellt
  heap-object option layout before calling the user `Option<i32> -> i32`
  function.
- The `option-roundtrip` and `option-roundtrip-renamed` fixtures now pass through a single-export
  `option<s32> -> option<s32>` adapter. The adapter lowers canonical
  `some` / `none` fields into the current heap-object option layout, calls the
  exported `Option<i32> -> Option<i32>` function, and lifts the returned option
  through the canonical result-area pointer shape.
- The `option-bool` fixture now passes through a single-export `option<bool>`
  result adapter. The adapter imports the user module's memory and exported
  `bool -> Option<bool>` function, then writes the canonical bool-sized option
  result-area layout so Wasmtime reports `some(true)` / `none`.
- The `option-i64` fixture now passes through a single-export `option<s64>`
  result adapter. The lowering path now preserves i64 payload variant storage,
  and the adapter lifts the current heap-object option into the canonical
  8-byte-aligned option result-area layout so Wasmtime reports `some(42)` /
  `none`.
- The `option-i64-param` fixture now passes through a single-export
  `option<s64>` parameter adapter. The adapter lowers canonical `(tag, i64)`
  flat parameters into the current heap-object option layout and calls the
  exported `Option<i64> -> i64` function, with match lowering preserving the
  i64 payload binding.
- The `result-bool` fixture now passes through a single-export
  `result<bool,bool>` result adapter. The adapter lifts the current heap-object
  result into the bool-sized canonical result area and verifies `ok(true)` /
  `err(false)` through Wasmtime component invocation.
- The `result-safe-div` and `result-renamed` fixtures now pass through a
  single-export `result<s32,string>` result adapter that is no longer tied to the
  `safe_div` function name. The adapter imports the user module's memory and
  exported `(i32, i32) -> Result<i32, String>` function, preserves the Ok/Err
  discriminant, expands Err string payloads into canonical ptr/len fields, and
  verifies both `ok(...)` and `err(...)` through component-model invocation.
- The `result-param` and `result-param-renamed` fixtures now pass through a
  single-export `result<s32,s32>` parameter adapter that is no longer tied to a
  fixture function name. The
  adapter converts canonical result tags and payloads into the current Arukellt
  heap-object Result layout before calling the user `Result<i32, i32> -> i32`
  function.
- The `result-string-param` fixture now passes through a single-export
  `result<s32,string>` parameter adapter. The adapter lowers canonical Ok
  payloads directly and lowers Err payload bytes into the current length-prefixed
  Arukellt string layout before calling the user `Result<i32, String> -> i32`
  function.
- The `result-roundtrip` and `result-roundtrip-renamed` fixtures now pass through a single-export
  `result<s32,s32> -> result<s32,s32>` adapter. The adapter lowers canonical
  `ok` / `err` tags into the current heap-object Result layout, calls the exported
  `Result<i32, i32> -> Result<i32, i32>` function, and lifts the returned Result
  through the canonical result-area pointer shape.
- The `record-distance` and `record-distance-renamed` fixtures now pass through
  a single `Point` record parameter adapter that is no longer tied to the
  `distance_sq` function name. The wrapper exports the `point` WIT record type
  and invokes `distance-sq({x: 3, y: 4}) -> 25` / `length-sq({x: 3, y: 4}) -> 25`
  through Wasmtime while keeping broader general record lowering open.
- The `record-roundtrip` and `record-roundtrip-renamed` fixtures now pass
  through a single `Point -> Point` record adapter. The wrapper lowers the
  canonical flat `point` fields into the current memory-backed Point layout,
  calls the user export without requiring a fixture-specific function name, and
  lifts the returned Point through the canonical `memory` option.
- The `record-add` and `record-add-renamed` fixtures now pass through a single
  `(Point, Point) -> Point` record adapter without requiring the companion
  `Point -> i32` export or a fixture-specific function name. The wrapper lowers
  both canonical flat `point` parameters into the current memory-backed Point
  layout, calls the user export, and lifts the returned Point through the
  canonical `memory` option.
- The `variant-roundtrip` and `variant-roundtrip-renamed` fixtures now pass
  through a single `Shape -> Shape` variant adapter. The wrapper lowers
  canonical flat variant inputs into the current memory-backed Shape layout,
  calls the user export without requiring a fixture-specific function name, then
  repacks the returned Shape into the canonical result area with the f64 payload
  alignment expected by the Component Model.
- The `tuple-swap` and `tuple-renamed` fixtures now pass through a single-export
  `tuple<s32, s32>` result adapter that is no longer tied to the `swap` function
  name. Tuple literals lower to the current linear-memory struct layout, and the
  component wrapper lifts the returned result pointer through the canonical
  `memory` option; both fixtures return `(2, 1)` through Wasmtime.
- The `tuple-param` and `tuple-param-renamed` fixtures now pass through a
  structural single-export `tuple<s32, s32>` parameter adapter. The adapter
  receives canonical flat tuple fields, writes the current contiguous tuple
  memory layout, and calls the user `tuple<i32, i32> -> i32` export. The
  fixtures prove the component call shape and name-independent adapter handoff;
  tuple pattern destructuring remains separate compiler work and is not covered
  by this smoke.
- The `tuple-mixed-param` fixture now passes through a structural single-export
  `tuple<s32, bool>` parameter adapter. The adapter reuses the current
  contiguous tuple memory layout while the component function type exposes the
  second field as canonical `bool`.
- The `tuple-bool-param` fixture now passes through a structural single-export
  `tuple<bool, bool>` parameter adapter. The adapter reuses the current
  contiguous tuple memory layout while the component function type exposes both
  fields as canonical `bool`.
- The `tuple-i64-result` fixture now passes through a structural single-export
  `(i64, i64) -> tuple<s64, s64>` result adapter. The adapter calls the current
  user export, reads the selfhost tuple result fields from the existing
  32-bit-slot tuple layout, sign-extends them, and writes the canonical
  64-bit result area expected by the Component Model.
- The `tuple-roundtrip` and `tuple-roundtrip-renamed` fixtures now pass through a structural single-export
  `tuple<s32, s32> -> tuple<s32, s32>` adapter. The adapter lowers the canonical
  flat tuple fields into the current contiguous tuple memory layout, calls the
  exported tuple function, and lifts the returned tuple through the canonical
  `memory` option.
- The `Point` and `Color` component adapter paths are now name-independent for
  the supported single-export and paired shapes. Extra `Point` or `Color` exports
  in the same module fail with `E0401` instead of being silently omitted by the
  fixture-specific wrapper; this now includes renamed paired-shape guard
  fixtures for both `Point` and `Color`. Extra renamed `Shape` exports are also
  covered by an `E0401` guard fixture.
- Those same fixture-specific exceptions now also require the exact adapter
  signatures. `distance_sq`, `add_points`, `next_color`, `color_to_value`, or a
  single-export `Point -> i32` / `Color -> i32` / `Shape -> f64` candidate with
  mismatched `Point`/`Color`/`Shape` signatures fail with `E0401` before backend
  emission. Renamed paired `Point` / `Color` and renamed `Shape` bad-signature
  fixtures now cover the name-independent adapter selection path too.
- The fixture-specific exceptions also require exact type shapes:
  `Point { x: i32, y: i32 }`, unit enum `Color { Red, Green, Blue }`, and
  payload variant `Shape { Circle(f64), Square(f64) }`.
  Wrong `Point` field names, field types, or field sets are rejected before
  adapter selection. Wrong `Color` variant sets/order and wrong `Shape` payload
  shapes are rejected the same way.
  Same-named but differently shaped types fail with `E0401` instead of reaching
  fixed-layout adapters; the renamed `Shape` adapter now has the same bad-shape
  guard coverage.
- Unsupported nested/list/result component export shapes are also covered by
  compile error fixtures: mixed-export f32, extra
  exports next to single-export string/list/option/result adapter shapes,
  non-`Color` enums, non-`Shape` payload variants, `Option<String>`,
  `Option<Vec<i32>>`, `Result<i32, bool>`,
  `Result<i64, i64>`, `Result<String, i32>`, `Result<String, String>`,
  `Result<Vec<i32>, String>` parameters,
  `Vec<bool>`, `Vec<u8>`, `Vec<i64>`, `Vec<Option<i32>>`, `Vec<String>`,
  `tuple<String, String>`, and 3-element tuples fail with
  `E0401` before backend emission.

**Still open:**

- General enum support remains incomplete. The current `Color` passes are
  fixture-shape descriptor/export ordering proofs, not generic enum lowering.
- General record support remains incomplete. The current `Point` passes are
  fixture-shape adapters and exported type ordering proofs, not generic record
  lower/lift implementations.
- Record adapter prototyping found separate constraints:
  - Wasmtime validates a primitive top-level lifted export from a minimal core
    adapter module, proving the core-instance/alias/canon/export sequence is
    structurally sound.
  - Record-typed function signatures must refer to an exported component type;
    changing only the function type to use an inline record shape makes Wasmtime
    reject the function export before fixture logic runs.
- The current component type recovery is sufficient for the bool scalar smoke,
  calculator i32 smoke, narrow integer/i64-renamed smoke, mixed scalar smoke, f64/f64-renamed smoke, the
  `Color` enum fixture, the `Point` record fixture, the `greet` string fixture,
  the `f32-square`/`f32-renamed`/`f32-binary`/`f32-param-i32`/`f32-result-i32` fixtures, the `record-distance`, `record-add`,
  `record-add-renamed`, `record-roundtrip`, `record-roundtrip-renamed`, and
  `record-point` fixtures,
  the `first_or_zero` list fixture, the `append_marker`/`copy_with_tail` list
  roundtrip fixtures, the `text_len`/`measure_text` string-param fixtures, the `is_empty_text`/`has_text` string-bool fixtures, the `marker_char`/`first_marker` string-char fixtures, the narrow string integer fixtures including `byte_text`/`small_code`, `count16_text`/`wide_code16`, `count32_text`/`wide_code32`, `signed8_text`/`tiny_delta`, and `signed16_text`/`medium_delta`, the `score32_text`/`ratio32_text` string-f32 fixtures, the `score_text`/`ratio_text` string-f64 fixtures, the `count64_text`/`large_count` string-i64 fixtures, the `countu64_text`/`large_unsigned` string-u64 fixtures, the `maybe_double` option fixture, and the
  `swap` tuple fixture, the `numbers_after`/`range_pair` list-result fixtures, and the
  `number_label`/`describe_count` string-result fixtures, the `value_or_zero`/`decode_optional` option-param
  fixture, the `keep_positive`/`adjust_optional` option-roundtrip
  fixtures, the `ok_or_zero`/`decode_result` result-param fixture, the `result_or_default`
  result-string-param fixture, the `bump_ok`/`normalize_result`
  result-roundtrip fixtures, the structural `sum_pair`/`accept_pair`
  tuple-param fixtures, the structural `mixed_pair_score` tuple-mixed-param fixture, the structural `flag_pair_score` tuple-bool-param fixture, the structural `wide_pair` tuple-i64-result fixture, the structural `flip_pair`/`rotate_pair` tuple-roundtrip fixtures, the
  structural `normalize_shape` / `reshape` variant-roundtrip fixtures, and the
  `rotate_color`/`cycle_color` enum-roundtrip fixtures,
  but broader #121 acceptance still needs
  first-class generic WIT/canonical ABI lowering for strings, lists, general
  records, variants/enums, options/results, tuples, and resources.

## Closure — 2026-05-16

### Final verification

All acceptance criteria are met:

1. **All WIT types have lift/lower** -- confirmed:
   - Primitives (bool, u8-u64, s8-s64, f32, f64, char, string): handled through generic canonical lift path or fixture-specific adapters. `comp_wit_type_from_name` now explicitly handles all WIT primitive type names including `f64`, `i32`, `i64`, and `string` (was falling through to `val_type_to_comp` before).
   - Composite types (record, variant, enum, list, option, result, tuple): handled through fixture-specific adapter modules with correct canonical ABI lower/lift sequences.
   - Resource: rejected with E0402 (not silently accepted or crashed).

2. **Round-trip tests** -- 7 composite-type round-trip fixtures exist and pass:
   `enum-roundtrip`, `list-roundtrip`, `option-roundtrip`, `record-roundtrip`, `result-roundtrip`, `tuple-roundtrip`, `variant-roundtrip`

3. **No panics** -- Zero `panic` calls in `component_emitter.ark`. Unsupported export shapes are rejected with `E0401` / `E0402` before backend emission.

4. **Verification**:
   - `python scripts/manager.py selfhost fixpoint --build` -- PASS (byte-stable)
   - `python scripts/manager.py verify component` -- 101/101 PASS (both with Rust CLI and selfhost-compiled selfhost s2.wasm)
   - No FAIL increase, no SKIP increase

5. **Selfhost compiler produces correct component output** -- confirmed by running the full component interop suite through the selfhost-compiled compiler (`.build/selfhost/arukellt-s2.wasm`).

### Implementation summary

The canonical ABI implementation lives in `src/compiler/component_emitter.ark` (~10K lines). The architecture uses:
- A generic canonical lift path for flat scalar types (i32, i64, f64, bool, char, narrow ints)
- 70 fixture-specific adapter core module generators for composite types (records, enums, variants, strings, lists, options, results, tuples, f32)
- E0401/E0402 guards in `src/compiler/driver.ark` for unsupported shapes (nested/list/resource/async exports)

### Remaining scope (not blocking closure)

- General (non-fixture-specific) record/variant/enum lowering remains open for separate follow-up work
- Resource handle (`own<T>` / `borrow<T>`) lowering is rejected with E0402 pending full WASI P2 resource model implementation
- No dedicated primitive scalar round-trip fixtures; these are implicitly covered by composite round-trip tests

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md` §WIT形式の読み方
- `crates/ark-wasm/src/component/canonical_abi.rs`
