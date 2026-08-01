# Native-cpp Experimental Promotion Autonomous Mission

Act as the implementation owner for the native-cpp selfhost executor experimental promotion.

Read first:

1. `AGENTS.md`
2. `docs/plans/native-cpp-experimental-promotion.md`
3. `issues/done/847-native-cpp-root-liveness-reenable.md`
4. `issues/done/848-native-cpp-experimental-promotion.md`
5. `docs/current-state.md`
6. ADR-049 and RFC-008

Current starting point:

- Phase 0 COMPLETE
- Phase 1 COMPLETE
- commit `aa6e04f8`
- root liveness shadow analyzed=8361, skipped=0, planned clears=3,265,155, emitted=0
- current-source S2/S3 hash `8b71d684…`
- next work is Phase 2

Execute Phase 2 through the Final Experimental Promotion Checklist autonomously.

Rules:

- Do not end after a phase, commit, issue, test, or performance improvement.
- After each completed phase, update the canonical plan and immediately start the next phase.
- Commit each validated work unit without asking.
- Do not wait for user approval between rollout stages.
- A failed test is a debugging task, not a stop signal.
- A failed performance gate selects the next measured optimization branch.
- Closing #847 must be followed by continuing #848.
- Never accept `--allow-high-rss`, stress-only green, or one strict PASS as completion.
- Use the hard-stop conditions in the canonical plan. When one task is blocked, continue all independent tasks.
- Preserve old baselines and create timestamped new receipts; never rewrite Phase 0 evidence.
- Do not weaken tests, thresholds, equality, validation, or determinism requirements.
- Do not silently skip functions or introduce fallback success.
- Do not broaden the goal into public native target completion.

Required final output only after every final checkbox is complete:

```text
NATIVE_CPP_EXPERIMENTAL_PROMOTION: COMPLETE
STRICT_RUNS: 3/3 PASS
HIGH_RSS_OVERRIDE: false
ISSUE_833: CLOSED
ISSUE_834: CLOSED
```

Any earlier status must end with `MISSION_STATUS: CONTINUE` unless a canonical hard blocker is fully documented.
