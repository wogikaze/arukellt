---
Status: open
Created: 2026-10-10
Updated: 2026-10-10
ID: 910
Track: compiler-internal
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: s3 build perf record 2026-10-10 - emit_function_body 47% profile
---

# 910 - emit_gc_local_val_type re-analyzes write sites per local decl

## Summary

emit_local_decls calls emit_gc_local_val_type for every local of every
emitted function. When the declared storage type is string/unknown it
falls into local_set_non_string_storage_type ->
copied_non_string_storage_type, which walks every write site of the
local and recurses into source locals (depth <= 3). The recursion has no
memoization for the copied path (only cached_direct_producer_storage
covers the sibling path), so the same local chains are re-derived for
every emit. In a perf record of the s2 -> s3 compile this subtree holds
~36% of total compile time under emit_local_decls (emit_function_body
~47% overall).

## Suggested direction

Memoize copied_non_string_storage_type per (function, local) in the
SelfEmitCtx fn cache the same way function_local_write_index and
cached_direct_producer_storage are already cached. Verify the cache key
invalidation matches the write-index lifetime.

## Evidence

- perf record of fixpoint stage-3 (tid 843202, 2026-10-10):
  emit_output 47.99%, emit_code_section 47.20%, emit_function_body
  47.06%, emit_local_decls 36.43%, emit_gc_local_val_type 36.29%,
  local_set_non_string_storage_type 11.52% under it.
