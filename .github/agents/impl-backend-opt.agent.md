---
name: impl-backend-opt
description: >-
  Implements backend optimization passes and performance improvements
  for the T1/T3 emitters and MIR optimization pipeline.
domains:
- backend-opt
tracks:
- backend-opt
primary_paths:
- crates/arukellt-llvm/
- crates/arukellt-t3/
- crates/arukellt-mir-opt/
allowed_adjacent_paths:
- crates/arukellt-compiler/
- crates/arukellt-runtime/
- tests/benchmarks/
out_of_scope:
- Language design changes
- Frontend/parser changes
- Stdlib implementation
required_verification:
- `bash scripts/run/verify-harness.sh --quick`
- Benchmark regression checks
stop_if:
- Optimization pass breaks existing fixture parity
commit_discipline:
- One optimization pass per commit unless tightly coupled
- Include benchmark before/after in commit message
output_format:
- Changed files list
- Benchmark results
- Verification command output
- DONE_WHEN checklist
- Commit hash
