# #604 Phase 0 Gap Ledger: Contract vs Actual Behavior

This ledger captures the Phase 0 baseline for issue #604 before broad API-surface changes.

| Family | Current Contract Claim | Observed Actual Behavior | Evidence | Disposition |
|---|---|---|---|---|
| `std::host::fs` | Filesystem helper facade | `exists(path)` is a read-probe backed by `__intrinsic_fs_read_file`; not a general path existence query | `std/host/fs.ark` module docs and `exists` implementation | Keep behavior, clarify contract, route true path semantics to #605 |
| `std::json` | JSON parser/serializer | Bounded behavior with tagged `JsonValue`; nested access re-parses raw text; legacy helpers preserved | `std/json/mod.ark` module docs and `JsonValue` comments | Keep current subset, document constraints clearly |
| `std::toml` | TOML parser/serializer | Bounded subset only (simple `key=value`, comments/blank lines); unsupported grammar returns `Err` | `std/toml/mod.ark` module docs and `toml_parse` comments | Keep subset, explicit partial semantics labeling |
| `std::collections::hash` | Hash map/set user-facing helpers | Raw and facade-level concerns are still co-located in a single module | `std/collections/hash.ark` module docs and open #607 | Defer deep hardening to #607, keep naming honesty in #604 |
| `std::host::http` | Host HTTP helpers | Host/runtime-backed HTTP only; HTTPS not supported in current capability surface | `std/host/http.ark`, `std/manifest.toml` availability notes | Keep explicit HTTPS limitation; no over-claim |
| `std::host::sockets` | Host socket helpers | Minimal host-bound socket surface; full socket lifecycle remains deferred | `std/host/sockets.ark`, open issue chain under #074/#139 | Keep provisional framing; no stable over-claim |
| `std::text` | Extended text helpers | Largely byte/ASCII-oriented operations; not full Unicode semantics | `std/text/mod.ark` docs and existing stdlib docs | Keep explicit semantics language |
| `std::time` | Time helpers | Pure duration math; host clock reads live in `std::host::clock` | `std/time/mod.ark` module docs | Preserve split and document boundary |

## Phase 0 Exit Check

- A single baseline table exists for all targeted families.
- Each row has evidence and disposition.
- No new runtime capability has been introduced by this ledger artifact.
