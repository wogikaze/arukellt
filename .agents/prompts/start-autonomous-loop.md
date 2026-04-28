# Start Autonomous Loop

Start autonomous multi-worktree compiler development using the parent orchestrator.

```md
Act as the parent orchestrator for the Arukellt autonomous development loop.

Use `.agents/prompts/autonomous-parent-orchestrator.md`.

Maximize safe subagent/worktree expansion.
Keep child agents continuously supplied with issue lists.
Each child must work in its own worktree, close or progress all assigned issues,
commit validated work, send or defer webhook reports, and request merge.

Do not stop when one issue blocks.
Do not stop when one child fails.
Do not stop when Ready issues run out before generating more reference-backed issues.
If issues are exhausted, generate more from reference coverage and continue.
If no safe work can be generated, write a clean stop report.

Use subagents for implementation and review.
Do not use subagents as search engines.

End each parent cycle with:
ORCHESTRATOR_STATUS: CONTINUE
or a justified clean stop status.
```
