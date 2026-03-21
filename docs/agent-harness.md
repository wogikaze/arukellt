# Agent Harness

Minimum viable harness entrypoint for agent-driven repo hygiene in Arukellt.

Start here:

- Read [`issues/index.md`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/issues/index.md) for the executable queue state and the location of the issue you are working.
- Read [`AGENTS.md`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/AGENTS.md) for repository boundaries and extension order.
- Read [`docs/adr/ADR-0001-agent-harness-entrypoint.md`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/docs/adr/ADR-0001-agent-harness-entrypoint.md) for the current decision on short pointer docs plus deterministic guardrails.

Trust executable sources of truth over descriptive prose:

- queue state: [`issues/index.md`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/issues/index.md) and the matching file under [`issues/open/`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/issues/open) or [`issues/done/`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/issues/done)
- repository guardrails: [`crates/arktc/tests/workboard.rs`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/crates/arktc/tests/workboard.rs)
- language and CLI behavior: crate tests plus executable docs under [`docs/`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/docs)

Before claiming completion:

```bash
./scripts/verify-harness.sh
```

Then run any issue-specific verification recorded in the issue notes or `Done When` section, such as benchmark or target-specific commands.

Current clippy gate scope:

- `./scripts/verify-harness.sh` runs `cargo clippy --workspace --lib --bins -- -D warnings`
- test targets are intentionally not part of the failing clippy gate yet; track that separately in the queue when the repository is ready for it
