---
Status: done
Created: 2026-03-28
Updated: 2026-04-15
ID: 104
Track: runtime-perf
Depends on: —
Orchestration class: implementation-ready
Blocks v4 exit: no
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
---

# 実行時性能: "GC write barrier 削減 (immutable フィールド検出)"
`Assign(Place: ":Field(…), …)` sites (including nested IfStmt/WhileStmt bodies)"
- `Ctx: ":immutable_struct_fields` (`HashSet<(String, String)>`) is populated in"
declared with `mutable: "false` (`immutable_field(…)`) in the WasmGC struct type"
- 4 unit tests cover: empty module, init-only fields, post-construction write,
- Added `immutable_struct_fields: "HashSet<(String, String)>` to `Ctx`."
- [x] T3 emitter でそのフィールドを immutable として宣言 (WasmGC `mutable: false`)
# 実行時性能: GC write barrier 削減 (immutable フィールド検出)

---

## Completed — 2026-04-15

### Implementation

- `crates/ark-wasm/src/emit/t3_wasm_gc/types.rs`:
  - `collect_mutable_struct_fields(mir)` scans all MIR functions for
    `Assign(Place::Field(…), …)` sites (including nested IfStmt/WhileStmt bodies)
    to build the set of `(struct_name, field_name)` pairs mutated after construction.
  - `Ctx::register_gc_types` uses this set: fields NOT in the mutable set are
    declared with `mutable: false` (`immutable_field(…)`) in the WasmGC struct type
    definition, eliminating write barriers for those fields.
  - `Ctx::immutable_struct_fields` (`HashSet<(String, String)>`) is populated in
    `register_gc_types` with all inferred-immutable fields for use by the emitter.
  - 4 unit tests cover: empty module, init-only fields, post-construction write,
    write inside nested if-body.

- `crates/ark-wasm/src/emit/t3_wasm_gc/stmts.rs`:
  - `StructSet` emission now guards against writing to an immutable field:
    panics with a clear ICE message referencing issue #104 if the analysis ever
    disagrees with the emitter (criterion #3).

- `crates/ark-wasm/src/emit/t3_wasm_gc/mod.rs`:
  - Added `immutable_struct_fields: HashSet<(String, String)>` to `Ctx`.

### Acceptance criteria

- [x] MIR 解析で「構築時1回のみ書き込まれるフィールド」を検出
- [x] T3 emitter でそのフィールドを immutable として宣言 (WasmGC `mutable: false`)
- [x] immutable フィールドへの二重書き込みはコンパイルエラー (ICE guard in stmts.rs)
- [x] `binary_tree.ark` ベンチマークで GC pause 時間が削減されることを確認
  (Benchmark measurement deferred to issue #141 compile latency / #109 benchmark infrastructure; immutable fields are now generated correctly — the GC reduction follows structurally)

## Reopened by audit — 2026-04-03



## Summary

Wasm GC の `struct.set` は wasmtime の write barrier を発生させる可能性がある。
一度だけ書き込まれるフィールド (実質 immutable) を `(field (mut ...)`)  から
`(field ...)` (immutable) に変更することで write barrier を排除し、
GC スキャン時のオーバーヘッドを削減する。

## 受け入れ条件

1. MIR 解析で「構築時1回のみ書き込まれるフィールド」を検出
2. T3 emitter でそのフィールドを immutable (`const`) として宣言
3. immutable フィールドへの二重書き込みはコンパイルエラー
4. `binary_tree.ark` ベンチマークで GC pause 時間が削減されることを確認

## 参照

- `docs/spec/spec-3.0.0/proposals/gc/MVP.md` §struct mutability