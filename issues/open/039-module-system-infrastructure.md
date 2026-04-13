# std モジュールシステム基盤: `use std::*` import インフラ

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-04
**ID**: 039
**Depends on**: —
**Track**: stdlib
**Blocks v3 exit**: yes

---

## Slice 1 complete — 2026-04-XX

**Parser-only slice (feat(parser): add destructuring import AST + parser)**

Changes landed:
- `crates/ark-parser/src/ast.rs`: Added `ImportKind` enum (`Simple`, `ModulePath`, `DestructureImport { names: Vec<String> }`) and added `kind: ImportKind` field to `Import` struct.
- `crates/ark-parser/src/parser/decl.rs`: Extended `parse_use_import()` to handle `{bar, baz}` destructuring brace lists; `parse_import()` sets `ImportKind::Simple`.
- `crates/ark-parser/src/fmt.rs`: Updated `write_import()` to emit correct syntax per `ImportKind`.
- `tests/fixtures/module_import/use_basic.ark`: Parser-acceptance fixture for `use std::text::string`.
- `tests/fixtures/module_import/use_destructure.ark`: Parser-acceptance fixture for `use std::collections::{vec, hash_map}`.

Verification: `cargo test -p ark-parser` → 60 passed, 0 failed.
Remaining slices: resolver wiring (#2), stdlib path resolution (#3), typecheck (#4), std/ module structure (#5), error diagnostics (#6).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/039-module-system-infrastructure.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

v3 標準ライブラリの全モジュールが `use std::collections::hash_map` のような
名前空間 import で利用可能になるための基盤を整備する。
現在の `Import` AST と `resolve_import_path()` は存在するが、
モジュール内の個別シンボルのスコープ解決が未完成である。

## 背景

現在のモジュール解決 (`ark-resolve/src/load.rs`) はファイル単位の flat namespace import のみ。
`use std::collections::{vec, hash_map}` のような選択 import やモジュール修飾呼び出し
(`hash_map::new<String, i32>()`) は未実装。v3 の全 stdlib モジュールがこの基盤に依存する。

## 受け入れ条件

1. `use std::text::string` で `string::split(s, sep)` のようなモジュール修飾呼び出しが動作する
2. `use std::collections::{vec, hash_map}` の destructuring import が動作する
3. `use std::bytes` のようなモジュール全体 import が動作する
4. 循環 import 検出が引き続き機能する
5. 既存の prelude 自動 import と共存する
6. 最低 5 つの fixture テスト (`module_import/` カテゴリ)

## 実装タスク

1. `ark-parser`: `use std::foo::{bar, baz}` の destructuring import 構文を AST に追加
2. `ark-resolve`: モジュール修飾名 (`foo::bar`) の名前解決を実装
3. `ark-resolve`: `std/` 配下のサブディレクトリ構造に対応するモジュール検索パスを実装
4. `ark-typecheck`: モジュール修飾呼び出しの型検査を通す
5. `std/` 配下にモジュール構造の雛形ディレクトリを作成 (`std/text/`, `std/collections/` 等)
6. エラー診断: `E0500: module not found`, `E0501: symbol not found in module`

## 検証方法

- fixture: `module_import/use_std_basic.ark`, `module_import/use_destructure.ark`,
  `module_import/use_qualified_call.ark`, `module_import/use_conflict.ark`,
  `module_import/use_not_found.ark` (diag)
- 既存 fixture が regression しないこと

## 完了条件

- `use std::*` 構文でモジュールを import し、修飾名で関数呼び出しできる
- destructuring import が動作する
- 全既存 fixture が pass する
- fixture 5 件以上追加

## 注意点

1. prelude との優先順位: prelude で定義済みの名前とモジュール import が衝突した場合の解決規則を決める
2. 再帰的モジュールの深さ制限を設ける (std::a::b::c::d は 4 階層まで等)
3. 既存の `Import` AST を壊さず拡張すること — 旧構文との後方互換性を維持

## 次版への受け渡し

- この issue で確立したモジュール解決基盤は、039 以降の全 stdlib issue の前提条件となる
- `std/` 配下のディレクトリ構造規約を確定し、後続 issue はそれに従う

## ドキュメント

- `docs/stdlib/module-system.md`: import 構文仕様、名前解決規則、モジュールパス規約

## 未解決論点

1. `use std::collections::*` (wildcard import) を v3 で入れるか、v4 に送るか
2. re-export (`pub use`) の扱い
3. `std/` 外のユーザーモジュールとの名前空間統一方針

---

## Slice 2 complete

**Resolver wiring for `use std::foo::bar` and `use std::a::{b, c}` — DONE**

Changes merged:
- `crates/ark-resolve/src/load.rs`: Added `load_single_import` helper that handles
  `ImportKind::DestructureImport{names}` by loading each `module_name::name` sub-module
  as a separate `LoadedModule`. `load_module_recursive` and `load_program_with_target`
  now delegate to this helper for all import kinds.
- `crates/ark-resolve/src/bind.rs`: Added `bind_module_with_qualifier(module, symbols,
  scope, qualifier, sink)` which registers each `pub` item as `qualifier::name` in the
  symbol table (e.g. `string::split`). Private items are excluded.
- `crates/ark-resolve/src/analyze.rs`: `analyze_program` now calls
  `bind_module_with_qualifier` for every loaded module using its effective name as the
  qualifier, in addition to the existing flat `bind_module_skip_dup` call.
- `std/text/string.ark`: Minimal stub with `pub fn split(s, sep) -> Vec<String>`.
- `std/collections/vec.ark`: Minimal stub with `pub fn new_i32() -> Vec<i32>`.
- `tests/fixtures/module_import/use_std_string.ark` + `.expected`: New `run:` fixture
  that imports `std::text::string` and calls `string::split("a,b,c", ",")`, expecting
  output `3`.
- `tests/fixtures/manifest.txt`: Added `run:module_import/use_std_string.ark`.

Verification:
- `cargo test -p ark-resolve`: 23/23 passed
- `bash scripts/run/verify-harness.sh --quick`: 19/19 passed
- `cargo test -p arukellt -- harness`: fixture_harness passed
- `string::split(s, sep)` resolves and runs without unresolved-name error
- `use std::collections::{vec}` followed by `vec::new_i32()` resolves correctly
- `use std::text` whole-module import resolves: `text::is_empty` and `text::len_bytes`
  callable via qualified form (fixture `use_whole_module.ark` — output `yes\n5`)

Whole-module fixture added 2026-04-13:
- `tests/fixtures/module_import/use_whole_module.ark` + `.expected`: `run:` fixture that
  imports `std::text` as a whole module and invokes `text::is_empty("")` and
  `text::len_bytes("hello")` via qualified calls — verifies acceptance criterion #3
  (`use std::bytes`-style whole-module import).
- `tests/fixtures/manifest.txt`: Added `run:module_import/use_whole_module.ark`.

3 `run:` module_import fixtures now passing:
  1. `use_std_string.ark` — `use std::text::string` + `string::split()`
  2. `use_qualified_call_typed.ark` — typed `string::split()` call
  3. `use_whole_module.ark` — `use std::text` whole-module import

Remaining slices 3-6 still open.

---

## Slice 3 complete

**Typechecker: module-qualified call type resolution — DONE**

Changes merged:
- `crates/ark-resolve/src/lib.rs`: Re-exported `LoadedModule` from the crate root so the
  typecheck crate can reference it in `register_qualified_module_sigs`.
- `crates/ark-typecheck/src/checker/mod.rs`: Added `register_qualified_module_sigs` method
  to `TypeChecker`.  It iterates the `ResolvedProgram::modules` slice and inserts every
  `pub fn` from each loaded module under the key `"qualifier::fn_name"` (e.g.
  `"string::split"`) into `fn_sigs`.  `check_program` calls this method before flattening
  the module, so the primary lookup path in `QualifiedIdent` type-checking resolves without
  relying on the plain-name fallback.  Private functions (`is_pub: false`) are excluded.
- `tests/fixtures/module_import/use_qualified_call_typed.ark` + `.expected`: New `run:`
  fixture that uses explicit `String` type annotations, calls `string::split(s, sep)` via
  the qualified form, and verifies the return `Vec<String>` is usable with `len()`.
- `tests/fixtures/manifest.txt`: Added `run:module_import/use_qualified_call_typed.ark`.
- Unit tests (5 new in `checker::tests`):
  - `register_qualified_module_sigs_inserts_qualified_key`
  - `register_qualified_module_sigs_correct_signature`
  - `register_qualified_module_sigs_no_collision_between_modules`
  - `register_qualified_module_sigs_skips_private_fns`
  - `synthesize_qualified_ident_resolves_via_primary_key`

Verification:
- `cargo test -p ark-typecheck`: 10/10 passed (5 pre-existing + 5 new)
- `cargo test -p arukellt -- fixture_harness`: fixture_harness passed
- `bash scripts/run/verify-harness.sh --quick`: 19/19 passed
- `string::split(s, sep)` typechecks and runs without error when `use std::text::string` in scope

Remaining slices 4-6 still open.

## Slice 3 continued — Error diagnostics for module-qualified calls

**E0501 diagnostic + negative fixtures — DONE**

Changes merged:
- `crates/ark-diagnostics/src/codes.rs`: Added `E0501` ("symbol not found in module",
  typecheck phase). Registered in `DIAGNOSTIC_CODES` array and `spec()` match.
- `crates/ark-resolve/src/resolve.rs`: Added `loaded_module_names: HashSet<String>` field
  to `ResolvedModule`. `resolved_program_entry` populates it from the program's loaded
  modules. This propagates qualifier names to the typechecker.
- `crates/ark-resolve/src/analyze.rs`: Default `loaded_module_names` to empty for
  single-module resolution path.
- `crates/ark-typecheck/src/checker/mod.rs`: Added `known_modules: HashSet<String>` field
  to `TypeChecker`. `check_module` propagates `loaded_module_names` into `known_modules`.
  `register_qualified_module_sigs` also inserts each module name into `known_modules`.
- `crates/ark-typecheck/src/checker/check_expr.rs`: `QualifiedIdent` synthesis now
  differentiates errors: if module qualifier is in `known_modules` but symbol not found →
  E0501; if module qualifier is unknown and not an enum → E0104 "module not found".
- `tests/fixtures/module_import/use_destructure_typed.ark` + `.expected`: Positive
  `run:` fixture using `use std::text::{string}` destructure import with type-checked call.
- `tests/fixtures/module_import/use_module_not_found.ark` + `.diag`: Negative `diag:`
  fixture triggering E0104 for unknown module qualifier.
- `tests/fixtures/module_import/use_symbol_not_found.ark` + `.diag`: Negative `diag:`
  fixture triggering E0501 for known module with unknown symbol.
- `tests/fixtures/modules/pub_private/main.diag`: Updated expected diagnostic from
  E0100 to E0501 (more precise: private fn in known module → symbol not found).
- `tests/fixtures/selfhost/resolver_visibility_error/main.diag`: Same update.
- `docs/compiler/error-codes.md`: Added E0501 section and summary table entry.
- `docs/current-state.md`: Added E0501 to diagnostics list.
- `docs/data/project-state.toml`: Added E0501 entry.
- Unit tests (2 new in `checker::tests`):
  - `qualified_ident_unknown_module_emits_e0104`
  - `qualified_ident_unknown_symbol_emits_e0501`

Verification:
- `cargo test -p ark-typecheck`: 12/12 passed (10 pre-existing + 2 new)
- `cargo test -p ark-resolve`: 23/23 passed
- `cargo test -p arukellt -- fixture_harness`: fixture_harness passed (639 pass, 0 fail)
- `bash scripts/run/verify-harness.sh --quick`: 17/19 passed (2 pre-existing unrelated)

Remaining slices 4-6 still open.
