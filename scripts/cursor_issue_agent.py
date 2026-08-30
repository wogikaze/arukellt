#!/usr/bin/env python3
"""Local GitHub Issue -> Cursor Agent -> PR runner.

The Cursor Agent only edits an isolated worktree. Git/GitHub mutations are performed
by this deterministic harness after the generated patch has been inspected.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
from typing import Any

DEFAULT_INTERVAL = 60
DEFAULT_CHECK_INTERVAL = 30
DEFAULT_RESTRICTED_PATHS = (
    ".github/workflows/",
    ".github/actions/",
    ".github/cursor/",
    ".cursor/",
    ".agents/",
    ".claude/",
    "AGENTS.md",
    "CLAUDE.md",
    "scripts/cursor_issue_agent.py",
)
CONVENTIONAL_TITLE = re.compile(
    r"^(?:build|chore|ci|docs|feat|fix|perf|refactor|revert|style|test)(?:\([^)]+\))?!?:\s+",
    re.IGNORECASE,
)


class AgentError(RuntimeError):
    pass


def run(
    argv: list[str],
    *,
    cwd: Path | None = None,
    env: dict[str, str] | None = None,
    input_text: str | None = None,
    check: bool = True,
    capture: bool = True,
) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        argv,
        cwd=cwd,
        env=env,
        input=input_text,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE if capture else None,
        check=False,
    )
    if check and result.returncode != 0:
        command = " ".join(shlex.quote(part) for part in argv)
        detail = (result.stderr or result.stdout or "").strip()
        raise AgentError(f"command failed ({result.returncode}): {command}\n{detail}")
    return result


def require_binary(name: str, aliases: tuple[str, ...] = ()) -> str:
    for candidate in (name, *aliases):
        resolved = shutil.which(candidate)
        if resolved:
            return resolved
    names = ", ".join((name, *aliases))
    raise AgentError(f"required executable not found: {names}")


def repo_root(git_bin: str) -> Path:
    return Path(run([git_bin, "rev-parse", "--show-toplevel"]).stdout.strip()).resolve()


def repo_slug(repo: str) -> str:
    return re.sub(r"[^A-Za-z0-9._-]+", "-", repo).strip("-")


def slug(text: str, max_length: int = 48) -> str:
    value = re.sub(r"[^A-Za-z0-9]+", "-", text.lower()).strip("-")
    return (value or "issue")[:max_length].rstrip("-")


def gh_json(gh_bin: str, argv: list[str], *, cwd: Path) -> Any:
    output = run([gh_bin, *argv], cwd=cwd).stdout
    return json.loads(output)


def resolve_repo(gh_bin: str, root: Path, override: str | None) -> str:
    if override:
        return override
    return run(
        [gh_bin, "repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"],
        cwd=root,
    ).stdout.strip()


def resolve_base(gh_bin: str, root: Path, override: str | None) -> str:
    if override:
        return override
    return run(
        [
            gh_bin,
            "repo",
            "view",
            "--json",
            "defaultBranchRef",
            "--jq",
            ".defaultBranchRef.name",
        ],
        cwd=root,
    ).stdout.strip()


def state_dir(git_bin: str, root: Path, repo: str) -> Path:
    raw = run([git_bin, "rev-parse", "--git-common-dir"], cwd=root).stdout.strip()
    common = Path(raw)
    if not common.is_absolute():
        common = (root / common).resolve()
    directory = common / "cursor-issue-agent" / repo_slug(repo)
    directory.mkdir(parents=True, exist_ok=True)
    return directory


def state_file(directory: Path) -> Path:
    return directory / "state.json"


def load_state(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {"initialized": False, "seen_issues": [], "processed_issues": {}}
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise AgentError(f"cannot read state file {path}: {exc}") from exc
    value.setdefault("initialized", False)
    value.setdefault("seen_issues", [])
    value.setdefault("processed_issues", {})
    return value


def save_state(path: Path, state: dict[str, Any]) -> None:
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(state, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(path)


def mark_seen(state: dict[str, Any], issue_number: int) -> None:
    if issue_number not in state["seen_issues"]:
        state["seen_issues"].append(issue_number)
        state["seen_issues"].sort()


def issue_labels(issue: dict[str, Any]) -> list[str]:
    labels = issue.get("labels") or []
    return [item.get("name", "") for item in labels if isinstance(item, dict)]


def issue_names(items: Any) -> str:
    if not items:
        return "(none)"
    names: list[str] = []
    for item in items:
        if isinstance(item, dict):
            names.append(item.get("login") or item.get("name") or "")
        else:
            names.append(str(item))
    return ", ".join(name for name in names if name) or "(none)"


def list_open_issues(gh_bin: str, root: Path, repo: str) -> list[dict[str, Any]]:
    return gh_json(
        gh_bin,
        [
            "issue",
            "list",
            "--repo",
            repo,
            "--state",
            "open",
            "--limit",
            "1000",
            "--json",
            "number,title,url,labels",
        ],
        cwd=root,
    )


def get_issue(gh_bin: str, root: Path, repo: str, number: int) -> dict[str, Any]:
    return gh_json(
        gh_bin,
        [
            "issue",
            "view",
            str(number),
            "--repo",
            repo,
            "--json",
            "number,title,body,author,labels,assignees,state,url",
        ],
        cwd=root,
    )


def load_config(root: Path) -> dict[str, Any]:
    path = root / ".github" / "cursor" / "issue-agent.json"
    if not path.exists():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise AgentError(f"cannot read {path}: {exc}") from exc


def is_non_closing_issue(issue: dict[str, Any], config: dict[str, Any]) -> bool:
    numbers = {int(value) for value in config.get("non_closing_issues", [])}
    labels = {str(value).lower() for value in config.get("non_closing_labels", [])}
    return int(issue["number"]) in numbers or bool(
        labels.intersection(label.lower() for label in issue_labels(issue))
    )


def build_pr_title(issue: dict[str, Any]) -> str:
    title = " ".join(str(issue["title"]).split())
    if CONVENTIONAL_TITLE.match(title):
        return title[:120]
    lowered_labels = {label.lower() for label in issue_labels(issue)}
    if lowered_labels.intersection({"bug", "type: bug", "security"}):
        kind = "fix"
    elif lowered_labels.intersection({"enhancement", "feature", "type: feature"}):
        kind = "feat"
    elif lowered_labels.intersection({"documentation", "docs"}):
        kind = "docs"
    elif lowered_labels.intersection({"performance", "perf"}):
        kind = "perf"
    elif lowered_labels.intersection({"test", "testing"}):
        kind = "test"
    else:
        kind = "chore"
    return f"{kind}: {title}"[:120].rstrip()


def render_prompt(root: Path, issue: dict[str, Any], repo: str, base: str, base_sha: str) -> str:
    template_path = root / ".github" / "cursor" / "prompts" / "implement-issue.md"
    if not template_path.exists():
        raise AgentError(f"missing prompt template: {template_path}")
    template = template_path.read_text(encoding="utf-8")
    replacements = {
        "REPOSITORY": repo,
        "BASE_BRANCH": base,
        "BASE_SHA": base_sha,
        "ISSUE_NUMBER": str(issue["number"]),
        "ISSUE_TITLE": str(issue["title"]),
        "ISSUE_AUTHOR": (issue.get("author") or {}).get("login", "(unknown)"),
        "ISSUE_LABELS": issue_names(issue.get("labels")),
        "ISSUE_ASSIGNEES": issue_names(issue.get("assignees")),
        "ISSUE_URL": str(issue["url"]),
        "ISSUE_BODY": str(issue.get("body") or "(empty)"),
    }
    return re.sub(r"\{\{([A-Z_]+)\}\}", lambda match: replacements.get(match.group(1), ""), template)


def write_cursor_config(directory: Path) -> Path:
    config_dir = directory / "cursor-config"
    config_dir.mkdir(parents=True, exist_ok=True)
    config = {
        "version": 1,
        "editor": {"vimMode": False},
        "permissions": {
            "allow": ["Read(**)", "Write(**)", "Shell(*)", "WebFetch(*)"],
            "deny": [
                "Read(.env*)",
                "Read(**/.env*)",
                "Write(.env*)",
                "Write(**/.env*)",
                "Write(.github/workflows/**)",
                "Write(.github/actions/**)",
                "Write(.github/cursor/**)",
                "Write(.cursor/**)",
                "Write(.agents/**)",
                "Write(.claude/**)",
                "Write(AGENTS.md)",
                "Write(CLAUDE.md)",
                "Write(scripts/cursor_issue_agent.py)",
            ],
        },
        "attribution": {
            "attributeCommitsToAgent": False,
            "attributePRsToAgent": False,
        },
    }
    (config_dir / "cli-config.json").write_text(
        json.dumps(config, indent=2) + "\n", encoding="utf-8"
    )
    return config_dir


def write_readonly_shims(directory: Path, git_bin: str, gh_bin: str) -> Path:
    shim_dir = directory / "readonly-bin"
    shim_dir.mkdir(parents=True, exist_ok=True)
    git_script = f'''#!/usr/bin/env python3
import subprocess, sys
REAL = {git_bin!r}
ALLOWED = {{"status", "diff", "show", "log", "rev-parse", "ls-files", "grep", "blame", "describe", "cat-file", "ls-tree", "merge-base", "for-each-ref", "name-rev"}}
args = sys.argv[1:]
if args and args[0] in ALLOWED:
    raise SystemExit(subprocess.call([REAL, *args]))
print("cursor issue agent: mutating git commands are disabled inside Cursor Agent", file=sys.stderr)
raise SystemExit(126)
'''
    gh_script = f'''#!/usr/bin/env python3
import subprocess, sys
REAL = {gh_bin!r}
ALLOWED = {{
    "repo": {{"view"}},
    "issue": {{"list", "view", "status"}},
    "pr": {{"list", "view", "checks", "status", "diff"}},
    "run": {{"list", "view"}},
    "workflow": {{"list", "view"}},
    "release": {{"list", "view"}},
    "search": {{"issues", "prs", "commits", "code"}},
}}
args = sys.argv[1:]
if len(args) >= 2 and args[0] in ALLOWED and args[1] in ALLOWED[args[0]]:
    raise SystemExit(subprocess.call([REAL, *args]))
print("cursor issue agent: mutating GitHub CLI commands are disabled inside Cursor Agent", file=sys.stderr)
raise SystemExit(126)
'''
    for name, content in (("git", git_script), ("gh", gh_script)):
        path = shim_dir / name
        path.write_text(content, encoding="utf-8")
        path.chmod(0o755)
    return shim_dir


def changed_files(git_bin: str, worktree: Path) -> list[str]:
    run([git_bin, "add", "-N", "."], cwd=worktree)
    output = run([git_bin, "diff", "--name-only", "HEAD"], cwd=worktree).stdout
    return [line.strip() for line in output.splitlines() if line.strip()]


def is_restricted(path: str, restricted_paths: tuple[str, ...]) -> bool:
    normalized = path.replace("\\", "/")
    for restricted in restricted_paths:
        if restricted.endswith("/"):
            if normalized.startswith(restricted):
                return True
        elif normalized == restricted:
            return True
    return False


def create_worktree(
    git_bin: str,
    root: Path,
    repo: str,
    base: str,
    issue: dict[str, Any],
) -> tuple[str, Path, str]:
    run([git_bin, "fetch", "origin", base, "--no-tags"], cwd=root, capture=False)
    base_sha = run([git_bin, "rev-parse", f"origin/{base}"], cwd=root).stdout.strip()
    branch = f"cursor/issue-{issue['number']}-{slug(str(issue['title']))}-{int(time.time())}"
    worktree_root = Path(tempfile.gettempdir()) / "cursor-issue-agent" / repo_slug(repo)
    worktree_root.mkdir(parents=True, exist_ok=True)
    worktree = worktree_root / f"issue-{issue['number']}-{int(time.time() * 1000)}"
    run(
        [git_bin, "worktree", "add", "-b", branch, str(worktree), f"origin/{base}"],
        cwd=root,
        capture=False,
    )
    return branch, worktree, base_sha


def cleanup_worktree(git_bin: str, root: Path, branch: str, worktree: Path) -> None:
    run([git_bin, "worktree", "remove", "--force", str(worktree)], cwd=root, check=False)
    run([git_bin, "branch", "-D", branch], cwd=root, check=False)


def write_pr_body(
    directory: Path,
    issue: dict[str, Any],
    cursor_output: str,
    relation: str,
) -> Path:
    output = cursor_output.strip()
    if len(output) > 12000:
        output = output[:12000] + "\n\n[Cursor final message truncated by issue agent]"
    body = f"""## Summary

- Implements the GitHub Issue through the local Cursor Issue Agent.
- Cursor edited an isolated worktree; deterministic git/GitHub operations were performed by the harness.

## Cursor final message

{output or '(no final message)'}

## Automation safety

- Issue text was treated as untrusted task context.
- Cursor was prevented from using mutating `git` / `gh` commands.
- Changes to agent instructions, Cursor configuration, GitHub workflows/actions, and this harness are rejected.

{relation}
"""
    path = directory / f"pr-body-{issue['number']}-{int(time.time() * 1000)}.md"
    path.write_text(body, encoding="utf-8")
    return path


def process_issue(context: dict[str, Any], issue_number: int) -> dict[str, Any]:
    root: Path = context["root"]
    git_bin: str = context["git"]
    gh_bin: str = context["gh"]
    agent_bin: str = context["agent"]
    repo: str = context["repo"]
    base: str = context["base"]
    config: dict[str, Any] = context["config"]
    args = context["args"]
    directory: Path = context["state_dir"]

    issue = get_issue(gh_bin, root, repo, issue_number)
    if str(issue.get("state", "")).upper() != "OPEN":
        print(f"Issue #{issue_number} is not open; skipping.")
        return {"status": "skipped", "branch": None, "pr_url": None}

    branch, worktree, base_sha = create_worktree(git_bin, root, repo, base, issue)
    print(f"Issue #{issue_number}: Cursor Agent on {branch}")
    prompt = render_prompt(worktree, issue, repo, base, base_sha)
    config_dir = write_cursor_config(directory)
    shim_dir = write_readonly_shims(directory, git_bin, gh_bin)
    agent_env = os.environ.copy()
    agent_env["CURSOR_CONFIG_DIR"] = str(config_dir)
    agent_env["PATH"] = str(shim_dir) + os.pathsep + agent_env.get("PATH", "")

    command = [agent_bin, "-p", prompt, "--output-format", "text"]
    if args.model:
        command.extend(["--model", args.model])

    agent_result = run(command, cwd=worktree, env=agent_env, check=False, capture=True)
    cursor_output = (agent_result.stdout or "").strip()
    if cursor_output:
        print(cursor_output)
    if agent_result.returncode != 0:
        detail = (agent_result.stderr or "").strip()
        print(f"Cursor Agent exited with {agent_result.returncode}: {detail}", file=sys.stderr)
        files = changed_files(git_bin, worktree)
        if files:
            print(f"Worktree preserved for inspection: {worktree}", file=sys.stderr)
            return {
                "status": "agent-failed",
                "branch": branch,
                "pr_url": None,
                "worktree": str(worktree),
            }
        cleanup_worktree(git_bin, root, branch, worktree)
        return {"status": "agent-failed", "branch": branch, "pr_url": None}

    files = changed_files(git_bin, worktree)
    if not files:
        cleanup_worktree(git_bin, root, branch, worktree)
        return {"status": "no-patch", "branch": branch, "pr_url": None}

    restricted_paths = tuple(DEFAULT_RESTRICTED_PATHS) + tuple(config.get("restricted_paths", []))
    restricted = [path for path in files if is_restricted(path, restricted_paths)]
    if restricted:
        print("Refusing to publish because restricted files changed:", file=sys.stderr)
        for path in restricted:
            print(f"  {path}", file=sys.stderr)
        print(f"Worktree preserved for inspection: {worktree}", file=sys.stderr)
        return {
            "status": "restricted",
            "branch": branch,
            "pr_url": None,
            "worktree": str(worktree),
        }

    title = build_pr_title(issue)
    run([git_bin, "add", "-A"], cwd=worktree, capture=False)
    run([git_bin, "commit", "-m", title], cwd=worktree, capture=False)
    head_sha = run([git_bin, "rev-parse", "HEAD"], cwd=worktree).stdout.strip()
    run([git_bin, "push", "-u", "origin", branch], cwd=worktree, capture=False)

    relation = (
        f"Part of #{issue_number}"
        if is_non_closing_issue(issue, config)
        else f"Closes #{issue_number}"
    )
    body_file = write_pr_body(directory, issue, cursor_output, relation)
    pr_url = run(
        [
            gh_bin,
            "pr",
            "create",
            "--repo",
            repo,
            "--base",
            base,
            "--head",
            branch,
            "--title",
            title,
            "--body-file",
            str(body_file),
        ],
        cwd=root,
    ).stdout.strip()
    print(f"Created {pr_url}")
    run(
        [
            gh_bin,
            "issue",
            "comment",
            str(issue_number),
            "--repo",
            repo,
            "--body",
            f"Cursor Issue Agent created pull request: {pr_url}",
        ],
        cwd=root,
        check=False,
    )

    status = "pr-created"
    checks_passed = False
    if not args.no_watch_ci:
        print(f"Watching checks for {pr_url}")
        checks = run(
            [
                gh_bin,
                "pr",
                "checks",
                pr_url,
                "--repo",
                repo,
                "--watch",
                "--interval",
                str(args.check_interval),
            ],
            cwd=root,
            check=False,
            capture=False,
        )
        checks_passed = checks.returncode == 0
        status = "checks-passed" if checks_passed else "checks-failed"

    if args.merge and checks_passed:
        merge_flag = {"squash": "--squash", "merge": "--merge", "rebase": "--rebase"}[
            args.merge_method
        ]
        merged = run(
            [
                gh_bin,
                "pr",
                "merge",
                pr_url,
                "--repo",
                repo,
                merge_flag,
                "--delete-branch",
            ],
            cwd=root,
            check=False,
            capture=False,
        )
        status = "merged" if merged.returncode == 0 else "merge-failed"

    cleanup_worktree(git_bin, root, branch, worktree)
    return {
        "status": status,
        "branch": branch,
        "pr_url": pr_url,
        "head_sha": head_sha,
    }


def record_result(state: dict[str, Any], issue_number: int, result: dict[str, Any]) -> None:
    state["processed_issues"][str(issue_number)] = {
        "at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        **result,
    }


def execute_one(context: dict[str, Any], state: dict[str, Any], issue_number: int) -> None:
    mark_seen(state, issue_number)
    save_state(context["state_file"], state)
    try:
        result = process_issue(context, issue_number)
    except Exception as exc:
        result = {"status": "failed", "error": str(exc), "branch": None, "pr_url": None}
        print(f"Issue #{issue_number} failed: {exc}", file=sys.stderr)
    record_result(state, issue_number, result)
    save_state(context["state_file"], state)


def drain(context: dict[str, Any], state: dict[str, Any]) -> None:
    issues = list_open_issues(context["gh"], context["root"], context["repo"])
    pending = [
        issue
        for issue in issues
        if str(issue["number"]) not in state["processed_issues"]
    ]
    if not pending:
        print("No unprocessed open issues to process.")
        return
    print(f"Processing {len(pending)} unprocessed open issue(s).")
    for issue in pending:
        execute_one(context, state, int(issue["number"]))


def watch(context: dict[str, Any], state: dict[str, Any]) -> None:
    args = context["args"]
    if not state["initialized"] and not args.backfill:
        issues = list_open_issues(context["gh"], context["root"], context["repo"])
        for issue in issues:
            mark_seen(state, int(issue["number"]))
        state["initialized"] = True
        save_state(context["state_file"], state)
        print(f"Seeded {len(issues)} existing open issue(s); watching only for new issues.")
    else:
        state["initialized"] = True
        save_state(context["state_file"], state)

    while True:
        issues = list_open_issues(context["gh"], context["root"], context["repo"])
        if args.backfill:
            pending = [
                issue
                for issue in issues
                if str(issue["number"]) not in state["processed_issues"]
            ]
        else:
            pending = [issue for issue in issues if issue["number"] not in state["seen_issues"]]
        for issue in pending:
            execute_one(context, state, int(issue["number"]))
        time.sleep(args.interval)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Run Cursor Agent locally against GitHub Issues and publish reviewed patches as PRs."
    )
    parser.add_argument("command", choices=("list", "run", "drain", "watch"))
    parser.add_argument("--issue", type=int, help="Issue number for the run command")
    parser.add_argument("--repo", help="Override owner/name repository detection")
    parser.add_argument("--base", help="Override the repository default branch")
    parser.add_argument("--interval", type=int, default=DEFAULT_INTERVAL, help="watch poll interval seconds")
    parser.add_argument(
        "--check-interval",
        type=int,
        default=DEFAULT_CHECK_INTERVAL,
        help="gh pr checks polling interval seconds",
    )
    parser.add_argument("--backfill", action="store_true", help="watch existing open issues too")
    parser.add_argument("--no-watch-ci", action="store_true", help="create PR without waiting for CI")
    parser.add_argument("--merge", action="store_true", help="merge after CI succeeds")
    parser.add_argument(
        "--merge-method", choices=("squash", "merge", "rebase"), default="squash"
    )
    parser.add_argument("--model", help="Cursor model name; default uses the CLI's configured model")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    if args.command == "run" and args.issue is None:
        raise AgentError("run requires --issue NUMBER")
    if args.interval < 5:
        raise AgentError("--interval must be >= 5 seconds")
    if args.check_interval < 10:
        raise AgentError("--check-interval must be >= 10 seconds")
    if args.merge and args.no_watch_ci:
        raise AgentError("--merge cannot be combined with --no-watch-ci")

    git_bin = require_binary("git")
    gh_bin = require_binary("gh")
    root = repo_root(git_bin)
    repo = resolve_repo(gh_bin, root, args.repo)
    base = resolve_base(gh_bin, root, args.base)
    directory = state_dir(git_bin, root, repo)
    path = state_file(directory)
    state = load_state(path)

    if args.command == "list":
        for issue in list_open_issues(gh_bin, root, repo):
            print(f"#{issue['number']} {issue['title']}")
        return 0

    agent_bin = require_binary("agent", ("cursor-agent",))
    run([agent_bin, "--version"], cwd=root)
    config = load_config(root)
    context = {
        "args": args,
        "root": root,
        "git": git_bin,
        "gh": gh_bin,
        "agent": agent_bin,
        "repo": repo,
        "base": base,
        "state_dir": directory,
        "state_file": path,
        "config": config,
    }

    if args.command == "run":
        execute_one(context, state, int(args.issue))
    elif args.command == "drain":
        drain(context, state)
    else:
        watch(context, state)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        print("Interrupted.", file=sys.stderr)
        raise SystemExit(130)
    except AgentError as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(2)
