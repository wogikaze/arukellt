---
Status: open
Created: 2026-05-14
Updated: 2026-05-14
ID: 618
Track: component-model
Depends on: 262, 476
Orchestration class: design-ready
Orchestration upstream: None
---

# WIT bindings round-trip regression surface

## Summary

Issue #262 closed the component-interop regression surface, but its future WIT
round-trip bullet covered a different workflow: generate WIT from Arukellt,
generate bindings back from that WIT, and verify those bindings execute through
the component pipeline. This issue tracks that workflow as its own open slice so
Issue #262 can remain complete without hiding the bindings-generation gap.

## Why this matters

- Component interop smoke tests can pass while generated WIT is not usable as an
  input to binding generation.
- `wasm-tools compose` coverage (#476) verifies composition, not Arukellt
  binding regeneration from emitted WIT.
- The generated WIT contract should be executable evidence, not only a text
  artifact.

## Acceptance

- [ ] A fixture emits WIT from Arukellt source and stores the expected WIT shape.
- [ ] A bindings-generation step consumes the emitted WIT and produces Arukellt
  bindings or an explicitly documented interim binding artifact.
- [ ] The generated bindings participate in a round-trip smoke test through the
  component pipeline.
- [ ] The workflow is wired into `tests/component-interop/` or an adjacent
  component test directory with a stable runner.
- [ ] `python scripts/manager.py verify` passes.

## Primary paths

- `tests/component-interop/`
- `crates/arukellt/src/`
- `crates/ark-wasm/src/`
- `docs/testing/test-categories.md`

## Close gate

All acceptance items checked with repo-internal evidence; #262 does not regain
unchecked future bullets.

---

## Design Spec

### 1. Architecture: What a WIT round-trip is

The round-trip workflow has three stages:

1. **Emit**: `arukellt compile --emit wit --target wasm32-wasi-p2 source.ark` produces WIT text from the compiler's `emit_wit_text_from_decls` function (in `src/compiler/driver.ark`).
2. **Import**: The emitted WIT text is parsed by an external WIT-aware tool (e.g. `wasm-tools component wit` or a WIT parser library) to produce bindings.
3. **Verify**: The bindings are used in a component that compiles and runs, proving the emitted WIT is a correct, consumable contract.

Currently stage 1 exists and is tested indirectly (the compiler can produce WIT text), but stages 2 and 3 have zero coverage. Stage 2 is the primary gap -- the emitted WIT text has never been fed back into a binding generator and compiled.

### 2. WIT type matrix and round-trip status

The following table describes every WIT type that the compiler's `emit_wit_text_from_decls` / `component_emitter.ark` binary generator can produce, its round-trip status, and what is missing.

Legend:
- **Implemented** = the binary component emitter (`component_emitter.ark`) encodes this type in the component binary, and the JCO interop smoke test passes.
- **WIT text** = `emit_wit_text_from_decls` (`driver.ark`) produces valid WIT text for this type.
- **Round-trip tested** = a test exists that (a) emits WIT, (b) imports it into a binding generator, and (c) verifies the generated bindings compile and run.

| WIT type | Binary emitter | WIT text | JCO interop test | Round-trip fixture | Notes |
|---|---|---|---|---|---|
| `bool` | yes | yes | `bool-logic`, `bool-renamed` | **missing** | |
| `u8` / `s8` | yes | yes | `u8-renamed`, `i8-renamed` | **missing** | |
| `u16` / `s16` | yes | yes | `u16-renamed`, `i16-renamed` | **missing** | |
| `u32` / `s32` | yes | yes | `int-widths`, `u32-renamed`, `i32-renamed` | **missing** | Covered by existing skeleton roundtrip.ark |
| `u64` / `s64` | yes | yes | `u64-renamed`, `i64-renamed` | **missing** | |
| `float32` / `float64` | yes | yes | `f32-*`, `f64-renamed`, `primitives-float` | **missing** | |
| `char` | yes | yes | `char-renamed` | **missing** | |
| `string` | yes | yes | 25+ string scenarios | **missing** | Most heavily tested JCO scenario; zero WIT round-trip coverage |
| `list<T>` | yes | yes (T only) | `list-*` | **missing** | WIT text works for list`<i32>`; nested/other element types need verification |
| `option<T>` | yes | yes (T only) | `option-*` | **missing** | WIT text works for option`<i32>`, option`<bool>`, option`<i64>` |
| `result<T, E>` | yes | yes | `result-*` | **missing** | WIT text works for result`<i32, i32>`, result`<i32, string>`, result`<bool, bool>` |
| `tuple<T1, T2>` | yes | yes | `tuple-*` | **missing** | WIT text works for concrete 2-element tuples |
| `record` | yes | yes | `record-*` | **missing** | Struct field names undergo driver_export_name (underscore to kebab-case) |
| `enum` (unit) | yes | yes | `enum-*` | **missing** | WIT text auto-detects unit vs payload variants |
| `variant` (payload) | yes | yes | `variant-*` | **missing** | Payload detection works; multi-payload variants in `Shape` verified |
| `flags` | binary only | **no** | **none** | **blocked** | WIT import surface explicitly rejects flags with E0090; text emitter does not emit `flags` declarations |
| `resource` / `own<T>` / `borrow<T>` | binary only | **no** | **none** | **blocked** | WIT import surface explicitly rejects resources with E0402; tracked by #473 |
| `stream<T>` / `future<T>` | binary only | **no** | **none** | **blocked** | WIT import surface explicitly rejects async with E0402; tracked by #474 |
| Multiple worlds | n/a | yes | **none** | **missing** | multi-world.ark and expected.wit exist in native/ but no runner |
| WIT `@rename` attribute | n/a | **no** | **partial** (JCO) | **missing** | JCO renamed-variant tests exist (e.g. `record-roundtrip-renamed`); the WIT text generator uses a fixed underscore-to-kebab conversion but does not produce `@rename` attributes, which means any name-independent-translation (NIT) mismatch in the binary emitter could produce WIT that decodes incorrectly |

### 3. Gap analysis

#### Gap A: No toolchain for binding generation from emitted WIT

The biggest gap is that there is no pipeline stage that feeds emitted WIT into an external binding generator and compiles the result. Two approaches exist:

**Option A1 -- `wasm-tools component wit` extract (recommended for v1):**
`wasm-tools component wit` can extract WIT text from a `.component.wasm` binary. The round-trip would be:
1. `arukellt compile --emit component --target wasm32-wasi-p2 source.ark -o out.component.wasm`
2. `wasm-tools component wit out.component.wasm > extracted.wit`
3. Compare `extracted.wit` against an expected WIT snapshot

This validates the binary encoding's WIT integrity without needing a binding generator.

**Option A2 -- JCO `jco wit` / `jco bindgen` (recommended for v2):**
JCO can generate JS bindings from a `.component.wasm`. The round-trip would be:
1. `arukellt compile --emit component --target wasm32-wasi-p2 source.ark -o out.component.wasm`
2. `jco wit out.component.wasm > roundtrip.wit`
3. `jco bindgen out.component.wasm --name test --output bindings/`
4. Use the generated bindings in a Node.js test

**Option A3 -- Direct WIT text parse and re-emit (interim):**
Use a standalone WIT parser (e.g. `wit-parser` via a small Rust tool) to parse the text produced by `--emit wit` and re-emit it, then check the two WIT texts are structurally equivalent (ignoring whitespace/comments).

#### Gap B: No schema of which Arukellt source shapes produce which WIT

The current `expected.wit` files in `tests/component-interop/native/` are hand-written and not tied to any automated comparison. Each fixture needs a triple:
- Source `.ark` (the input)
- Expected `.wit` (the textual WIT contract)
- A runner that calls `--emit wit`, captures output, and diffs against expected

#### Gap C: Named type reference in WIT text

The `emit_wit_type_defs` function emits inline type definitions for structs and enums, but when a function parameter references a named type (e.g. `Point`), the WIT text generator uses `driver_wit_type_name` to produce the kebab-case name. If the binary emitter uses a different name mangling, the round-trip breaks. The following potential mismatches need investigation:
- Struct fields named `value` vs WIT field naming
- Enum variant names with special characters
- Functions named `main` are excluded from export; how do named exports work in WIT?

#### Gap D: WIT `@rename` attribute

The binary component model uses name-independent-translation (NIT) which can rename exports. The WIT text generator currently has no mechanism to annotate renamed identifiers with `@rename(...)` attributes. If an Arukellt function is exported with a different name in the component binary than what the WIT text declares, bindings generated from the WIT text will not match the component's actual exports.

### 4. Fixture specifications

Each fixture directory should follow the pattern established by the native interop tests:
`tests/component-interop/roundtrip/<scenario>/`

Each fixture contains:

| File | Purpose |
|---|---|
| `<scenario>.ark` | Arukellt source with `pub fn` or `export fn` declarations |
| `<scenario>.expected.wit` | Expected WIT text output (golden file) |
| `<scenario>.component.wasm` | Expected component binary (cached; regenerated when changed) |
| `run.sh` | Shell runner that (a) compiles with `--emit wit`, (b) diffs against expected WIT, (c) optionally runs the component through `wasm-tools component wit` extraction |

#### Fixture catalog

**Phase 1 -- Foundational type-per-fixture (13 fixtures)**

These cover every scalar and simple composite type in isolation. Each fixture exports one `pub fn` that takes and returns the type under test.

| ID | Scenario | Arukellt type | WIT type | Notes |
|---|---|---|---|---|
| RT-01 | `scalar-bool` | `bool` | `bool` | Single bool param, bool return |
| RT-02 | `scalar-u8` | `u8` | `u8` | |
| RT-03 | `scalar-s8` | `i8` | `s8` | |
| RT-04 | `scalar-u16` | `u16` | `u16` | |
| RT-05 | `scalar-s16` | `i16` | `s16` | |
| RT-06 | `scalar-u32` | `u32` | `u32` | |
| RT-07 | `scalar-s32` | `i32` | `s32` | Replaces current skeleton roundtrip.ark |
| RT-08 | `scalar-u64` | `u64` | `u64` | |
| RT-09 | `scalar-s64` | `i64` | `s64` | |
| RT-10 | `scalar-float32` | `f32` | `f32` | |
| RT-11 | `scalar-float64` | `f64` | `f64` | |
| RT-12 | `scalar-char` | `char` | `char` | |
| RT-13 | `scalar-string` | `string` | `string` | Identity function |

**Phase 2 -- Composite types (8 fixtures)**

| ID | Scenario | Arukellt type | WIT type | Notes |
|---|---|---|---|---|
| RT-20 | `composite-record` | `struct Point { x: i32, y: i32 }` | `record point { x: s32, y: s32 }` | Struct to record mapping |
| RT-21 | `composite-enum` | `enum Color { Red, Green, Blue }` | `enum color { red, green, blue }` | Unit variant enum |
| RT-22 | `composite-variant` | `enum Shape { Circle(f64), Square(f64) }` | `variant shape { circle(float64), square(float64) }` | Payload variant |
| RT-23 | `composite-list` | `Vec<i32>` | `list<s32>` | List of primitives |
| RT-24 | `composite-option` | `Option<i32>` | `option<s32>` | Optional value |
| RT-25 | `composite-result` | `Result<i32, string>` | `result<s32, string>` | Result with ok and err |
| RT-26 | `composite-tuple` | `(i32, i32)` | `tuple<s32, s32>` | 2-element tuple |
| RT-27 | `composite-nested` | `struct Container { label: string, values: Vec<i32>, meta: Option<(i32, bool)> }` | `record container { label: string, values: list<s32>, meta: option<tuple<s32, bool>> }` | Nested composites |

**Phase 3 -- Rename and name resolution (2 fixtures)**

| ID | Scenario | Key concern |
|---|---|---|
| RT-30 | `rename-underscore` | Arukellt `snake_case` names produce WIT `kebab-case` names; verify the expected.wit reflects the correct kebab transform |
| RT-31 | `rename-export-alias` | Verify that if an exported function has a name that differs from its WIT declaration (potential `@rename` gap), the expected.wit captures the actual WIT output |

**Phase 4 -- World-level constructs (1 fixture)**

| ID | Scenario | Key concern |
|---|---|---|
| RT-40 | `multi-world` | Multiple export functions devolve into a single `world` with multiple exported interfaces; verify the WIT text structure matches expected.wit |

**Phase 5 -- Component binary round-trip (1 fixture)**

| ID | Scenario | Key concern |
|---|---|---|
| RT-50 | `binary-extract` | Compile to `.component.wasm`, extract WIT via `wasm-tools component wit`, and verify the extracted WIT is structurally equivalent to the golden WIT produced by `--emit wit` |

### 5. Runner design

The runner for all round-trip fixtures should be a single bash script at `tests/component-interop/roundtrip/run.sh` that:

1. Iterates all subdirectories of `tests/component-interop/roundtrip/`
2. For each scenario:
   a. Runs `arukellt compile --emit wit --target wasm32-wasi-p2 <scenario>.ark -o /tmp/rt-<scenario>.wit`
   b. Diffs the output against `<scenario>.expected.wit` using `diff` (ignoring trailing whitespace)
   c. If `wasm-tools` is available, also runs `wasm-tools component wit <scenario>.component.wasm` and checks equivalence
3. Reports pass/fail per scenario

Key runner requirements:
- **Skip gracefully** when `arukellt` is not built (exit 0, like JCO tests)
- **Skip gracefully** when `wasm-tools component wit` is not installed
- **Fail hard** on WIT text mismatch (exit 1)

The runner should be wired into `verification-component-interop` CI job, consistent with the existing JCO test pattern.

### 6. Implementation order

```
Phase 1 (Quick wins):
  [ ] Create `tests/component-interop/roundtrip/` directory
  [ ] Write `run.sh` with the fixture iteration loop
  [ ] Move the existing skeleton roundtrip.ark into RT-07 (scalar-s32)
  [ ] Write expected.wit for RT-07
  [ ] Verify runner passes for RT-07

Phase 2 (Expand scalar coverage):
  [ ] Add RT-01 through RT-06, RT-08 through RT-13
  [ ] Each: .ark + expected.wit; verify runner passes

Phase 3 (Composite types):
  [ ] Add RT-20 through RT-27
  [ ] Verify composite types produce correct WIT text
  [ ] Verify nested types produce correct WIT text

Phase 4 (Naming and structure):
  [ ] Add RT-30, RT-31 (rename fixtures)
  [ ] Add RT-40 (multi-world fixture)

Phase 5 (Binary round-trip):
  [ ] Add RT-50 (wasm-tools component wit extraction)
  [ ] Verify binary round-trip produces equivalent WIT
  [ ] Document any structural differences between --emit wit and extracted WIT

Phase 6 (CI integration):
  [ ] Wire roundtrip/run.sh into verification-component-interop
  [ ] Verify `python scripts/manager.py verify` passes
```

### 7. Known gaps and future work

| Gap | Status | Tracking |
|---|---|---|
| WIT `flags` type | Arukellt has no flags declaration syntax; WIT import rejects flags with E0090 | Separate feature |
| WIT `resource` / `own<T>` / `borrow<T>` | WIT import rejects with E0402; binary emitter encodes resource types but text emitter has no resource syntax | #473 |
| WIT `stream<T>` / `future<T>` | WIT import rejects with E0402; not in text emitter | #474 |
| WIT `@rename` attribute | Binary emitter may rename exports (NIT); text emitter has no `@rename(...)` annotation | New issue |
| WIT function imports | WIT import rejects function imports with E0401; binding generator cannot consume emitted WIT with imports | New issue |
| `jco bindgen` integration | JCO JS binding generation would test a real downstream consumer of the WIT contract | Future enhancement |
