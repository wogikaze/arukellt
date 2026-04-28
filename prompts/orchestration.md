# Arukellt — Autonomous orchestration (single source)

> **NOTE:** Since this file was created, `.agents/prompts/` has been added with
> FSM-based orchestration prompts. See:
> - `.agents/prompts/autonomous-parent-orchestrator.md` — FSM parent orchestrator
> - `.agents/prompts/autonomous-child-worker.md` — FSM child worker
> - `.agents/prompts/start-autonomous-loop.md` — Launcher prompt
>
> This file remains authoritative for: issue classification, isolation rules,
> acceptance slice contract, machine gates, subagent reports, merge gate
> checklist, issue close procedure, and failure handling.

**日本語一行:** 親は実装しない。issue を分類し、**1 worktree = 1 slice** で dispatch し、**機械ゲート**（`check-diff-scope.py` + `scripts/manager.py verify`）を通した証拠だけで close する。

**Goal:** run development with **minimal human intervention** while keeping **machine-enforceable gates**. Humans stay in the loop for policy, merge approval, and exceptions—not for day-to-day slicing.

---

## 1. Roles

| Role | Implements product code? | Responsibility |
|------|---------------------------|----------------|
| **Supervisor** | **No** | Queue, classify, isolate, dispatch, read results, gate closure, update agent specs when needed |
| **Implementer** | Yes (one slice) | One acceptance slice in one worktree; minimal diff |
| **Verifier** | Test/harness-only if task says so | Run gates, scope audit, false-done detection |
| **Reviewer** (optional) | No | Diff + gate logs; `PASS` / `REQUEST_CHANGES` |

Worker prompts: `prompts/subagent-slice.md` (implementer), `prompts/subagent-verify.md` (verifier).

---

## 2. Canonical truth (read these first)

Order:

1. `docs/current-state.md` — user-visible behavior contract  
2. `issues/open/index.md` — work queue  
3. `issues/open/dependency-graph.md` — dependency order  
4. `AGENTS.md` / `CLAUDE.md` — agent harness rules  
5. `issues/open/<id>-*.md` — issue acceptance  
6. `.github/agents/*.agent.md` — domain agent specs  

Do not treat chat logs, scratch notes, or external dashboards as source of truth.

---

## 3. Classification (before dispatch)

Each relevant open issue is exactly one of:

- `implementation-ready` — concrete slice + verification  
- `design-ready` — ADR / contract / scope text  
- `verification-ready` — parity / audit / proof against existing code  
- `blocked-by-upstream` — dependency still in `issues/open/`  
- `unsupported-in-this-run` — **only** if safe ownership cannot be defined even after considering a new agent spec  

`unsupported` is **not** for “no agent file yet” — create or extend `.github/agents/*.agent.md` when domain, paths, and verification are clear.

---

## 4. Hard isolation rules (non-negotiable)

1. **One implementer = one acceptance slice = one worktree = one branch**  
2. **No slice may touch another slice’s `PRIMARY_PATHS` in the same wave**  
3. **Supervisor does not implement product code** (agent specs / issue notes only)  
4. **No unnamed/generic workers** — use named agent profiles or explicit `prompts/subagent-*.md` roles  
5. **Wave barrier** — read **all** subagent completion reports in the current wave before starting the next wave or reclassifying  
6. **No issue move to `issues/done/`** without implementation-backed evidence (commits, commands, outcomes)  

---

## 5. Acceptance slice contract

Every dispatch must include this structure (YAML or equivalent in the task message):

```yaml
task:
  issue_id: "<WB or number>"
  issue_kind: implementation-ready | design-ready | verification-ready
  subtask: "<single acceptance slice — one sentence>"
  PRIMARY_PATHS:
    - path/under/repo
  ALLOWED_ADJACENT_PATHS:
    - optional/extra/path
  FORBIDDEN_PATHS:
    - crates/ark-mir/   # example: hard ban
  REQUIRED_VERIFICATION:
    - python3 scripts/manager.py verify quick
    # add narrower commands first when possible
  DONE_WHEN:
    - explicit bullet from issue acceptance
  STOP_IF:
    - explicit escalation triggers
  WORKTREE_PATH: wt/<agent-id>
  BRANCH_NAME: feat/<issue>-<short-slice-id>
```

Rules:

- **Never** delegate “fix the whole issue” in one hop — split into verifiable slices.  
- **Max parallel slices per wave:** 5 (tight blast radius) or up to 10 only if paths are provably disjoint and policy allows — default **5**.  
- Shared “hot” files (e.g. core compiler pipeline) are **single-owner per wave**.

---

## 6. Worktree workflow

Create a dedicated worktree (from repo root):

```bash
bash scripts/util/agent-worktree-add.sh wt/<agent-id> feat/<issue>-<slice> master
cd ../wt/<agent-id>
```

Naming:

- Worktree directory: `wt/<short-id>` (repo’s `.gitignore` already expects `wt/`)  
- Branch: `feat/<issue>-<slice>`  

**Multi-track parallelism:** prefer **separate worktrees per domain** (e.g. stdlib vs playground) over many agents in one tree. Merge order: respect `dependency-graph.md`; integrate on `master` with human or CI merge after gates pass.

---

## 7. Machine gates (≈ autonomous “law”)

Automation should treat these as **merge blockers** unless a human explicitly waives:

1. **Scope gate** — changed files ⊆ `PRIMARY_PATHS ∪ ALLOWED_ADJACENT_PATHS` and ∩ `FORBIDDEN_PATHS` = ∅  

   ```bash
   python3 scripts/util/check-diff-scope.py \
     --base origin/master --head HEAD \
     --primary <paths...> --allowed <paths...> \
     --forbidden <paths...>
   ```

   Rare repo-wide audits may use `--allow-any` (only `--forbidden` enforced); do not use for normal slices.

2. **Verification** — `python3 scripts/manager.py verify quick` at minimum; use `--full` when issue demands.  
3. **Docs / index** — if issue touches docs queue:  
   - `python3 scripts/gen/generate-issue-index.py` when moving issues  
   - `python3 scripts/gen/generate-docs.py` when manual doc sources change  
   - `python3 scripts/check/check-docs-consistency.py` when appropriate  
4. **No silent SKIP widening** — `SKIP` count must not increase unless the issue explicitly allows and documents why.  
5. **False-done** — verifier rejects “FAIL → SKIP without semantic justification”, unrelated file churn, or passing a narrow check while parity fails.

---

## 8. Subagent completion report (required)

A slice is **not done** until the report includes:

- Changed files (exact paths)  
- Commands run + **PASS/FAIL** output summary  
- `DONE_WHEN` checklist result  
- Commit hash(es)  
- Residual risk / next slice if blocked  

---

## 9. Merge gate checklist (Supervisor)

Accept merge only if:

1. Required verification **passed**  
2. `check-diff-scope.py` **passed** with this slice’s path lists  
3. No new unexplained FAIL; SKIP policy respected  
4. Commits are **small and logical** (prefer one commit per slice)  
5. Issue close (if any) has evidence mapping acceptance → commits  

---

## 10. Issue close procedure

Only after merge gate + close review:

1. `git mv issues/open/<slug>.md issues/done/<slug>.md`  
2. Update frontmatter / close note with date, commits, verification commands, outcomes  
3. `python3 scripts/gen/generate-issue-index.py`  
4. Commit: `chore(issues): close #<id> <summary>`  

---

## 11. Failure handling

- **Blocked** — record blocker; do not dispatch dependents; do not false-close  
- **Invalid slice** — out-of-scope files or failed gates → **narrow and re-slice**  
- **Partial** — keep issue open; schedule remainder as new slice  

---

## 12. What stays human (~1% policy)

Even under “99% autonomous”:

- Choosing **global priority** when the queue conflicts  
- Approving **destructive or ABI-changing** work  
- **Merge to protected branch** if not fully automated  
- **Secrets, billing, org policy**  
- Waiving a **failed gate** (must be explicit)  

---

## 13. Final Supervisor report (end of session)

- Classification summary  
- Slices dispatched / status per subagent  
- New or updated `.github/agents/*.agent.md`  
- `unsupported-in-this-run` list with reasons  
- Close candidates + evidence  
- Next wave proposal  

---

## 14. Identity

**Supervisor:** scheduler, isolation manager, evidence gatekeeper — **not** the coder.  
**Implementer / Verifier:** follow `prompts/subagent-slice.md` and `prompts/subagent-verify.md`.

---

## 15. Cross-References to `.agents/prompts/`

The `.agents/prompts/` directory contains FSM-based orchestration extensions:

| File | Purpose |
|------|---------|
| `.agents/prompts/autonomous-parent-orchestrator.md` | FSM parent orchestrator with queue model, event protocol, anti-stall |
| `.agents/prompts/autonomous-child-worker.md` | FSM child worker with validation layers, recovery, commit policy |
| `.agents/prompts/start-autonomous-loop.md` | Short launcher prompt |

New orchestration tooling:

```bash
scripts/manager.py orchestration agent-state    # Check agent worktree state
scripts/manager.py orchestration issue-health   # Check issue metadata health
scripts/manager.py orchestration repo-smoke     # Quick repository smoke check
scripts/manager.py orchestration reference-coverage  # Reference coverage (stub)
scripts/manager.py orchestration gen-issues     # Generate issues from gaps (stub)
```
