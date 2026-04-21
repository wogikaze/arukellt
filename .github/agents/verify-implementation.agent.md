---
name: verify-implementation
description: >-
  Verifies implementation completeness, runs parity checks, validates
  close candidates, and performs implementation-backed close verification.
domains:
- verification
tracks:
- main
- runtime-perf
- selfhost
primary_paths:
- scripts/run/
- tests/
- benchmarks/
allowed_adjacent_paths:
- crates/
- std/
- docs/
out_of_scope:
- New feature implementation
- Design work
required_verification:
- `bash scripts/run/verify-harness.sh`
- Specific verification commands from issue
stop_if:
- Verification fails with unclear blocker
commit_discipline:
- Close evidence commits only
- Issue-only updates as separate chore(issue) commits
output_format:
- Verification results
- Changed files (if any)
- Close evidence mapping
- DONE_WHEN checklist
- Commit hash
