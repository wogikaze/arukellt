# Prelude vs Module Import: Canonical Access Paths

> Defines which stdlib names are prelude-only, which are module-only,
> and the canonical access path for each.

## Principle

Every stdlib function has exactly one **canonical access path**:

- **Prelude functions** (`prelude = true`): Available without import. Canonical path is the bare name.
- **Module functions**: Require `use std::xxx` import. Canonical path is `std::xxx::name`.

There are **no dual-exposed functions** — no function has both `prelude = true` and a `module` field in the manifest.

## Prelude Categories

Prelude functions are grouped by `doc_category` for documentation purposes only.
These categories do **not** correspond to importable modules:

| doc_category | Count | Examples | Importable? |
|---|---|---|---|
| `string` | 19 | `concat`, `trim`, `split` | No — prelude only |
| `collections` | 36 | `push`, `pop`, `len`, `sort_i32` | No — prelude only |
| `conversion` | 8 | `to_string`, `parse_i32` | No — prelude only |
| `math` | 5 | `sqrt`, `abs`, `min`, `max` | No — prelude only |
| `option_result` | 15 | `unwrap`, `is_some`, `is_ok` | No — prelude only |
| `assert` | 5 | `assert`, `assert_eq` | No — prelude only |
| `box` | 2 | `Box_new`, `unbox` | No — prelude only |
| `control` | 1 | `panic` | No — prelude only |
| `io` | 3 | `println`, `print`, `eprintln` | No — prelude only |
| *(none)* | 7 | `Vec_new`, `HashMap_new` | No — prelude only |

> **Important**: `std::math`, `std::string`, `std::collections` are **not** importable
> modules. The LSP may suggest them as virtual modules for completion, but they are
> prelude function categories, not actual module paths.

## Actual Importable Modules

These modules exist in `std/manifest.toml` and require `use` to access:

| Module | Count | Examples |
|---|---|---|
| `std::host::stdio` | 3 | `println`, `print`, `eprintln` |
| `std::host::fs` | 3 | `read_file`, `write_file`, `file_exists` |
| `std::host::env` | 5 | `var`, `set_var`, `vars`, `remove_var`, `has_var` |
| `std::host::clock` | 1 | `now_ms` |
| `std::host::random` | 3 | `random_i32`, `random_f64`, `random_bool` |
| `std::host::process` | 2 | `exit`, `abort` |
| `std::host::http` | 2 | `get`, `request` (stub) |
| `std::host::sockets` | 1 | `connect` (stub) |
| `std::path` | 6 | `join`, `basename`, `dirname` |
| `std::time` | 3 | `format_timestamp`, `parse_timestamp` |
| `std::json` | 6 | `stringify`, `parse` |
| `std::text` | 15 | `pad_left`, `pad_right`, `repeat` |
| `std::seq` | 8 | `range`, `enumerate`, `zip` |
| `std::test` | 16 | `test`, `skip`, `bench` |
| `std::bytes` | 20 | `bytes_new`, `bytes_len` |
| `std::random` | 3 | `seed`, `next_i32` |
| `std::toml` | 1 | `parse_line` |
| `std::csv` | 1 | `parse_line` |
| `std::wasm` | 19 | `section_id`, `valtype` |
| `std::wit` | 14 | `type_id`, `canonical_abi` |
| `std::component` | 2 | `abi_version`, `model_version` |
| `std::core` | 3 | `hash`, `ordering` |
| `std::collections::hash` | 5 | `HashMap_new`, `HashMap_insert` |
| `std::collections::linear` | 13 | `Vec_new`, `deque_new` |
| `std::collections::ordered` | 10 | `BTreeMap_new` |

## Resolver Behavior

The resolver resolves names in this order:
1. Local scope (let bindings, function parameters)
2. Module-level definitions (fn, struct, enum in current file)
3. Imported names (`use` declarations)
4. Prelude names (auto-imported)

When a name exists in both an import and the prelude, the explicit import takes precedence.

## LSP Implications

- **Completion**: Should show both prelude names (without import) and module names (with auto-import suggestion)
- **Go to definition**: Prelude functions resolve to their builtin descriptor; module functions resolve to their source file
- **Auto-import**: Should only suggest actual importable modules, not doc_category virtual modules

## See Also

- [Expansion Policy](expansion-policy.md) — which modules accept new APIs
- [Stability Policy](stability-policy.md) — stability tiers
- [API Reference](reference.md) — full function listing
