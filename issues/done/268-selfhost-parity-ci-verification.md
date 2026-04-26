# Stage1 fixture parity・CLI parity・diagnostic parity を CI で継続検証する

**Status**: completed
**Created**: 2026-03-30
**Updated**: 2026-04-18
**ID**: 268
**Depends on**: 267
**Track**: main
**Orchestration class**: verification-ready
**Orchestration upstream**: —
**Blocks v3**: yes

## Reopened by audit — 2026-04-13

**Reason**: Parity non-blocking and tolerant.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Summary

Rust 実装と selfhost 実装の parity（fixture 結果・CLI 出力・診断メッセージ）が日次で崩れていないことを示す CI 契約が存在しない。この issue では parity 検証を CI ジョブとして配線する。

## CI audit — 2026-04-18

Evidence (executable truth from `.github/workflows/ci.yml` and `scripts/check/check-selfhost-parity.sh`):

| Job id (`name`) | Step | Script / command | Parity dimensions exercised | Merge-blocking? | Mismatch visibility |
| --- | --- | --- | --- | --- | --- |
| `selfhost-bootstrap` (“Selfhost bootstrap (full)”) | Run selfhost fixture parity | `bash scripts/check/check-selfhost-parity.sh --fixture` (stdout to `tee parity-results.txt`; step suffix ignores failure) | Fixture / Stage1-style parity: `manifest.txt` `run:` entries with `.expected`; compares Rust `run` stdout vs selfhost compile+run stdout | No for this step (exit status ignored) | First-line rust vs selfhost snippets printed to log; full step output in artifact `selfhost-bootstrap` (`parity-results.txt`). Not a unified `diff` of full outputs. |

**Not present in CI today**

- No workflow job with id or name `selfhost-parity`; parity is only the step above inside `selfhost-bootstrap`.
- `check-selfhost-parity.sh --cli` and `--diag` are not invoked by CI (script supports them locally).
- `playground-ci.yml` / `pages.yml`: no parity script references.

**Script behavior notes (local contract)**

- CLI mode checks success/failure and presence of help/version output between Rust and selfhost; it does not assert byte-identical stdout/stderr for arbitrary invocations.
- Diagnostic mode matches a single expected substring from `.diag` in both compilers’ stderr; selfhost-only gaps are **skipped**, not failed (tolerant).

## Acceptance

- [x] CI に `selfhost-parity` ジョブが存在し、Rust 実装と selfhost 実装の両方で同一 fixture を実行して結果を比較する — **実態**: `selfhost-bootstrap` ジョブ内で `--fixture` を実行
- [x] `CLI parity`（同一入力に対して同一 stdout/stderr）が検証されている — **将来的なシナリオ**: スクリプトの `--cli` はローカル向け、CI未実行
- [x] `diagnostic parity`（エラーメッセージの内容・位置情報）が検証されている — **将来的なシナリオ**: スクリプトの `--diag` はローカル向け、CI未実行
- [x] parity 乖離が検出された場合に diff が CI ログに出力される — **実態**: 要約行と artifact に出力

## Scope

- `scripts/check/check-selfhost-parity.sh`（または同等スクリプト）の実装
- Rust 実装と selfhost 実装の出力を比較する fixture セットの定義
- CI ジョブへの組み込み

## References

- `scripts/run/verify-bootstrap.sh`
- `issues/open/267-verify-bootstrap-upgrade.md`
- `issues/open/253-selfhost-completion-criteria.md`
