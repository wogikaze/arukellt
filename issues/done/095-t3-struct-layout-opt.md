---
Status: done
Created: 2026-03-28
Updated: 2026-04-15
ID: 095
Track: backend-opt
Depends on: —
Orchestration class: implementation-ready
Orchestration upstream: —
---

# T3: struct フィールドレイアウト最適化 (アクセス頻度ベース)
**Blocks v4 exit**: no

---

## Completion Note

Closed 2026-04-15.

- `crates/ark-wasm/src/emit/t3_wasm_gc/layout_opt.rs` computes deterministic
 per-struct field reorder permutations from reachable MIR field-access
 frequency.
- `crates/ark-wasm/src/emit/t3_wasm_gc/mod.rs` applies the optimization only at
 `opt_level >= 2`, reorders `self.struct_layouts`, and preserves definition
 order for equal-frequency ties.
- `crates/ark-wasm/src/emit/t3_wasm_gc/types.rs`,
 `crates/ark-wasm/src/emit/t3_wasm_gc/operands.rs`, and
 `crates/ark-wasm/src/emit/t3_wasm_gc/stmts.rs` all resolve emitted field
 order from `self.struct_layouts`, so `struct.new`, `struct.get`, and
 `struct.set` use the same remapped layout.
- Regression coverage in `crates/ark-wasm/src/emit/t3_wasm_gc/layout_opt.rs`
 proves that the hottest field is moved to the front and that equal-frequency
 ties remain in declaration order.

## Summary

Wasm GC の struct 型はフィールドインデックスでアクセスされるため、
フィールドの並び順を変えても意味論は変わらない。
アクセス頻度の高いフィールドを低インデックスに置くことで、
wasmtime の JIT コードがより小さいオフセット即値を使えるようになる。
将来の WasmGC struct packing 最適化への布石にもなる。

## 受け入れ条件

- [x] MIR 走査から struct ごとのフィールドアクセス頻度を集計する
- [x] T3 emit 時にアクセス頻度の高いフィールドを前方に並べ替えた型定義を生成する
- [x] フィールド順序のリマッピングが `struct.get` / `struct.set` / `struct.new` に一貫して反映される
- [x] `--opt-level 2` でのみ有効
- [x] 回帰テストでホットフィールド優先と同頻度 tie の安定順序を確認する

## Verification

- `cargo test -p ark-wasm layout_opt -- --nocapture`
- `bash scripts/run/verify-harness.sh --quick`
- `cargo build --workspace --exclude ark-llvm`

## 参照

- `docs/spec/spec-3.0.0/proposals/gc/MVP.md` §struct

---

## Queue closure verification — 2026-04-18

- **Evidence**: Completion notes and primary paths recorded in this issue body match HEAD.
- **Verification**: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18).
- **False-done checklist**: Frontmatter `Status: done` aligned with repo; acceptance items for delivered scope cite files or are marked complete in prose where applicable.

**Reviewer:** implementation-backed queue normalization (verify checklist).
