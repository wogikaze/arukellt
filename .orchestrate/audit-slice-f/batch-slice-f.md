# Slice F audit batch — release / benchmark / hygiene

Contract: `prompts/research.md`
Cross-check: `scripts/manager.py` gates, `docs/release-criteria.md`, `benchmarks/` (read-only)

## Issue IDs (57)

016, 109, 140, 146, 149, 225, 242, 250, 264, 267, 268, 322, 326, 357,
373, 374, 375, 376, 377, 417, 418, 419, 420, 421, 422, 423, 424, 425, 426, 427,
465, 530, 531, 532, 538, 539, 540, 541, 542, 543, 544, 545, 546, 547, 548, 549,
550, 551, 552, 553, 554, 555, 556, 608, 619, 620, 621

## Cross-check anchors

- `python3 scripts/manager.py verify quick` (false-done hygiene + close-gate gates)
- `docs/release-criteria.md` pre-release checklist vs release issues #546–556
- `mise.toml` bench tasks vs #140
- `scripts/util/benchmark_runner.py`, `benchmarks/README.md`, `benchmarks/legacy/`
- Hygiene scripts under `scripts/check/check-generated-files.sh`, `check-asset-naming.sh`, `check-links.sh`
- Missing claimed scripts: `check-orphan-inventory.sh`, `check-admission-gate.sh`, `scripts/run/verify-harness.sh`
