# CI perf gate: コンパイル時間・実行時間・バイナリサイズ閾値チェック

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-18
**Closed**: 2026-04-18
**ID**: 110
**Depends on**: 109
**Track**: benchmark
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v4 exit**: yes

## Historical: reopened by audit — 2026-04-13

**Reason (obsolete)**: Queue audit thought `scripts/update-baselines.sh` was missing.

**Repo truth (2026-04-18)**: Updater, baselines JSON, `--perf-gate`, and `scripts/check/perf-gate.sh` are present; see audit below. This section is kept for traceability only.

## Summary

`scripts/run/verify-harness.sh` を拡張して、
コンパイル時間・実行時間・バイナリサイズの回帰を CI で自動検知する perf gate を追加する。
roadmap-v4.md §6 item 8 および §7 で明示的に要求されている。

## 閾値定義

| 軸 | 閾値 (ベースライン比) | アクション |
|----|---------------------|-----------|
| コンパイル時間 | +20% | failure |
| 実行時間 | +10% | failure |
| バイナリサイズ | +15% | failure |

## 受け入れ条件

1. `tests/baselines/perf/` に JSON 形式のベースラインファイル
2. `scripts/run/verify-harness.sh` に `--perf-gate` オプション追加
3. `scripts/update-baselines.sh` でベースラインを手動更新
4. CI で perf gate が失敗した場合にわかりやすいエラーメッセージ

## 参照

- roadmap-v4.md §6 item 8, §7, §8

## Landed implementation — 2026-04-14 (not a queue “close”)

Earlier body used “Closed” while **Status** stayed `open` in the generated index — contradictory. This section is the implementation record only; queue state remains `open` until maintainers move the file to `issues/done/` and regenerate indexes.

**Changes (landed)**:
- `scripts/check/perf-gate.sh`: invokes `scripts/util/benchmark_runner.py` with baseline/current paths
- `scripts/update-baselines.sh`: baseline refresh, supports `--dry-run`
- `tests/baselines/perf/`: `baselines.json` (5 benchmarks), `current.json`, `hello-wasm-size.json`
- `scripts/run/verify-harness.sh`: `--perf-gate` runs the check above

## Audit — 2026-04-18 (impl-benchmark #110)

Acceptance re-checked. CI messaging tightened: `perf-gate.sh` prints remediation on non-zero exit; verify-harness shows up to 120 lines of perf output on failure (baseline compare was previously easy to miss after `tail -30`).

**Commands**

```text
$ bash scripts/run/verify-harness.sh --quick
Total checks: 19
Passed: 19
Failed: 0
✓ All selected harness checks passed

$ bash scripts/update-baselines.sh --dry-run
[dry-run] Would run benchmark_runner.py --mode update-baseline
[dry-run] Would write: .../tests/baselines/perf/baselines.json
[dry-run] Would write: .../tests/baselines/perf/current.json
[dry-run] Would write: .../docs/process/benchmark-results.md

$ bash scripts/check/perf-gate.sh
(Outcome environment-dependent: on 2026-04-18 audit host, runs alternated between PASS and FAIL on compile/run variance vs frozen baseline; failure path prints thresholds + `update-baselines.sh` hint. Opt-in gate remains appropriate for CI hardware profiles.)

---

## Close note — 2026-04-18

Closed as complete. CI perf gate implementation landed with all acceptance criteria met.

**Close evidence:**
- `scripts/check/perf-gate.sh`: invokes `scripts/util/benchmark_runner.py` with baseline/current paths
- `scripts/update-baselines.sh`: baseline refresh, supports `--dry-run`
- `tests/baselines/perf/`: `baselines.json` (5 benchmarks), `current.json`, `hello-wasm-size.json`
- `scripts/run/verify-harness.sh`: `--perf-gate` runs the check above
- CI messaging tightened: perf-gate.sh prints remediation on non-zero exit
- verify-harness shows up to 120 lines of perf output on failure

**Acceptance mapping:**
- ✓ `tests/baselines/perf/` has JSON baseline files
- ✓ `scripts/run/verify-harness.sh` has `--perf-gate` option
- ✓ `scripts/update-baselines.sh` updates baselines manually
- ✓ CI perf gate failure shows clear error messages

**Implementation notes:**
- Implementation landed 2026-04-14
- Thresholds: compile time +20%, runtime +10%, binary size +15%
- Opt-in gate appropriate for CI hardware profiles
- `--quick` does not run `--perf-gate` by default for speed
```
