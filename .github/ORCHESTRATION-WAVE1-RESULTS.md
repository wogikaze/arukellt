# Wave 1 barrier — subagent completion summary

Read completed for all five slices before any Wave 2 dispatch.

| WO | AGENT | ISSUE | Status | Commit |
|----|-------|-------|--------|--------|
| 1 | impl-compiler | 064 | completed (implementation) | `7903928c18deb1af80c96e0d502ec5a0bdfa2e9f` |
| 2 | impl-compiler | 282 | completed (verification + tests) | `dd4d0ccd293d3acea655d6f4bfbfec90f9e13a28` |
| 3 | impl-component-model | 032 | completed (fix + regression) | `70ba206a945a8ef34d65739906f7035547fa44e1` |
| 4 | impl-benchmark | 110 | completed (baselines + harness hints) | `4e2e2bf8e4a417270325aa3bbb53e756f478a656` |
| 5 | impl-vscode-ide | 453 | completed (E2E unskipped + LSP pipe) | `9d42055d6dfbe4bb34b01b9a163747717c89056a` |

## Close candidates (parent review only)

Per orchestration rules, **`issues/done/` moves only with implementation-backed full-issue evidence.** These slices advance work but may not satisfy every acceptance line:

- **#032**: Export validation slice fixed; full issue checklist still has many `[x]` from before — verify remaining reopen criteria (component export E2E) before close.
- **#064**: Branch hint section populated; criterion 3 (source `@likely`) may still be deferred — review issue body.
- **#282**: Audit contradiction resolved with tests; issue may still be “open” until parent updates issue text / acceptance.
- **#110**: Perf gate and baselines refreshed; confirm CI expectations for baseline churn.
- **#453**: E2E enabled; confirm issue acceptance table vs current tests.

## Next wave proposal (max 5 parallel)

1. **impl-selfhost** — #499 first open acceptance item (closure capture), single path in `src/compiler/**`.
2. **impl-playground** — #382 one downstream T2 slice (`PRIMARY_PATHS` from issue).
3. **impl-compiler** — #039 resolver slice 2+ (after reading issue “実装タスク”; avoid conflicting with #028 in same wave if both touch `ark-resolve`).
4. **impl-component-model** — #028 / #028b follow-up: one `--wit` pipeline gap if still open.
5. **impl-verification-infra** — #154 bounded scaffold slice OR #268 parity CI slice (register agent from `.github/ORCHESTRATION-RUN-REGISTRY.md`).

Re-run classification from `.github/ORCHESTRATION-CLASSIFICATION.txt` after any issue file moves; refresh with `bash scripts/gen/generate-issue-index.sh` if queue metadata changes.
