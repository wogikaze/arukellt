#!/usr/bin/env python3
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path
from collections import defaultdict, deque

try:
    import frontmatter
except ImportError:
    frontmatter = None


class SimplePost:
    def __init__(self, metadata, content):
        self.metadata = metadata
        self.content = content


def _parse_scalar(value):
    value = value.strip()
    if value.startswith("#"):
        return ""
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {'"', "'"}:
        return value[1:-1]
    lowered = value.lower()
    if lowered in {"true", "yes"}:
        return True
    if lowered in {"false", "no"}:
        return False
    return value


def load_issue_frontmatter(path):
    if frontmatter is not None:
        return frontmatter.load(path)

    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        return SimplePost({}, text)

    parts = text.split("---\n", 2)
    if len(parts) < 3:
        return SimplePost({}, text)

    raw_meta = parts[1]
    metadata = {}
    current_key = ""
    for raw_line in raw_meta.splitlines():
        if not raw_line.strip() or raw_line.lstrip().startswith("#"):
            continue
        if raw_line[0].isspace():
            if current_key:
                metadata[current_key] = f"{metadata[current_key]} {raw_line.strip()}".strip()
            continue
        if ":" not in raw_line:
            continue
        key, value = raw_line.split(":", 1)
        current_key = key.strip()
        metadata[current_key] = _parse_scalar(value)

    return SimplePost(metadata, parts[2].lstrip("\n"))

ROOT = Path(__file__).parent.parent.parent
OPEN_DIR = ROOT / "issues/open"
BLOCKED_DIR = ROOT / "issues/blocked"
DONE_DIR = ROOT / "issues/done"
INDEX_OUT = OPEN_DIR / "index.md"
GRAPH_OUT = OPEN_DIR / "dependency-graph.md"
PRIORITY_OUT = OPEN_DIR / "priority-table.md"

open_dir = OPEN_DIR
blocked_dir = BLOCKED_DIR
done_dir = DONE_DIR
index_out = INDEX_OUT
graph_out = GRAPH_OUT
priority_out = PRIORITY_OUT
meta_out = open_dir / "index-meta.json"

issue_files = sorted(p for p in open_dir.glob('*.md') if re.match(r'^\d', p.name))
blocked_files = sorted(blocked_dir.glob('*.md')) if blocked_dir.exists() else []
done_files = sorted(done_dir.glob('*.md')) if done_dir.exists() else []
issues = {}
blocked_issues = {}
done_issues = {}
reverse = defaultdict(list)

def normalize_dep_token(token):
    token = token.strip()
    if not token or token in {'none', 'なし', '—', '-'}:
        return None
    # Suffix issues e.g. 028b (done/open file 028b-*.md), not the same as 028
    m = re.match(r'^#?(\d{3})([a-z])\b', token)
    if m:
        return m.group(1) + m.group(2)
    if token.startswith('#'):
        m = re.match(r'^#?(\d+)', token)
        if m:
            return m.group(1)
    m = re.match(r'^(\d+)', token)
    if m:
        return m.group(1)
    return token

def normalize_deps(raw):
    if raw.strip() in {'', 'none', 'なし', '—', '-'}:
        return []
    deps = []
    for token in raw.split(','):
        dep = normalize_dep_token(token)
        if dep:
            deps.append(dep)
    return deps

def merge_blockquote_metadata(metadata, lines):
    """Backfill legacy issue metadata written as '> **Track:** release'."""
    merged = dict(metadata)
    for line in lines[:12]:
        m = re.match(r'^>\s*\*\*([^:*]+):\*\*\s*(.+?)\s*$', line)
        if not m:
            continue
        key = m.group(1).strip()
        if key not in {"Status", "Track", "Type"} or key in merged:
            continue
        merged[key] = _parse_scalar(m.group(2))
    return merged

def issue_sort_key(issue_id):
    m = re.match(r'^(\d+)([a-z]*)$', str(issue_id))
    if not m:
        return (999999, str(issue_id))
    return (int(m.group(1)), m.group(2))

def score_release(issue):
    title = str(issue["title"]).lower()
    track = str(issue["track"]).lower()
    if title.startswith("release:"):
        return 4
    if "release" in title or "release" in track:
        return 3
    return 0

def score_readiness(issue):
    checked = int(issue.get("checked") or 0)
    unchecked = int(issue.get("unchecked") or 0)
    if unchecked == 0 and checked > 0:
        return 5
    if unchecked == 0:
        return 1
    total = checked + unchecked
    return max(1, min(5, round((checked / total) * 5)))

def score_strategic(issue):
    text = f'{issue["title"]} {issue["track"]}'.lower()
    if any(term in text for term in ("selfhost", "compiler", "type", "language", "phase")):
        return 5
    if any(term in text for term in ("stdlib", "wasi", "component", "runtime", "wasm")):
        return 3
    if "main" in text or "docs" in text or "benchmark" in text:
        return 2
    return 1

def score_multi_agent(issue):
    dep_count = len(issue.get("deps") or [])
    unchecked = int(issue.get("unchecked") or 0)
    if dep_count == 0 and unchecked <= 2:
        return 5
    if dep_count <= 1 and unchecked <= 5:
        return 4
    if dep_count <= 2:
        return 3
    return 2

def truncate(text, width=62):
    return text if len(text) <= width else text[: width - 3] + "..."

def parse_file(path):
    post = load_issue_frontmatter(path)
    content = post.content
    lines = content.splitlines()
    title = lines[0][2:].strip() if lines and lines[0].startswith('# ') else path.stem
    meta = merge_blockquote_metadata(post.metadata, lines)

    issue_id = str(meta.get('ID', path.name.split('-')[0]))
    deps_raw = str(meta.get('Depends on', 'none'))
    deps = normalize_deps(deps_raw)
    status = meta.get('Status', 'open')
    track = meta.get('Track', 'main')
    blocked_by = meta.get('Blocked by', '')
    orchestration_class = meta.get('Orchestration class', '')
    orchestration_upstream = meta.get('Orchestration upstream', '')
    acceptance_unchecked = sum(1 for line in lines if line.startswith('- [ ]'))
    acceptance_checked = sum(1 for line in lines if line.startswith('- [x]') or line.startswith('- [X]'))
    return {
        'id': issue_id,
        'title': title,
        'path': path.name,
        'deps': deps,
        'status': status,
        'track': track,
        'blocked_by': blocked_by,
        'unchecked': acceptance_unchecked,
        'checked': acceptance_checked,
        'orchestration_class': orchestration_class,
        'orchestration_upstream': orchestration_upstream,
    }

for path in issue_files:
    try:
        data = parse_file(path)
        issues[data['id']] = data
    except Exception as e:
        print(f"Warning: Failed to parse {path}: {e}", file=sys.stderr)

for path in blocked_files:
    data = parse_file(path)
    blocked_issues[str(data['id'])] = data

for path in done_files:
    try:
        data = parse_file(path)
        done_issues[str(data['id'])] = data
    except Exception as e:
        print(f"Warning: Failed to parse {path}: {e}", file=sys.stderr)

for issue_id, data in issues.items():
    for dep in data['deps']:
        reverse[str(dep)].append(str(issue_id))

# topological order among known deps
indegree = {iid: 0 for iid in issues}
for iid, data in issues.items():
    for dep in data['deps']:
        if dep in issues:
            indegree[iid] += 1
queue = deque(sorted([iid for iid, deg in indegree.items() if deg == 0]))
order = []
while queue:
    iid = queue.popleft()
    order.append(iid)
    for child in sorted(reverse.get(iid, [])):
        indegree[child] -= 1
        if indegree[child] == 0:
            queue.append(child)
if len(order) != len(issues):
    remaining = [iid for iid in issues if iid not in order]
    order.extend(sorted(remaining))

# index markdown
lines = []
lines.append('# Open Issues Index')
lines.append('')
lines.append('Auto-generated by `scripts/gen/generate-issue-index.py`. Do not edit manually.')
lines.append('')
lines.append('## Summary')
lines.append('')
lines.append(f'- Total open issues: {len(issues)}')
lines.append(f'- Blocked issues: {len(blocked_issues)}')
lines.append(f'- Done issues: {len(done_issues)}')
lines.append(f'- Main-track issues: {sum(1 for i in issues.values() if i["track"] == "main")}')
lines.append(f'- Parallel-track issues: {sum(1 for i in issues.values() if i["track"] == "parallel")}')
lines.append('')

# Track statistics
lines.append('## Track Statistics')
lines.append('')
lines.append('| Track | Open | Done | Blocked | Total |')
lines.append('|-------|------|------|---------|-------|')
all_issues = {**issues, **blocked_issues, **done_issues}
track_stats = {}
for iid, data in all_issues.items():
    track = data['track']
    if track not in track_stats:
        track_stats[track] = {'open': 0, 'done': 0, 'blocked': 0}
    if iid in issues:
        track_stats[track]['open'] += 1
    elif iid in blocked_issues:
        track_stats[track]['blocked'] += 1
    elif iid in done_issues:
        track_stats[track]['done'] += 1

for track in sorted(track_stats.keys()):
    stats = track_stats[track]
    total = stats['open'] + stats['done'] + stats['blocked']
    lines.append(f'| {track} | {stats["open"]} | {stats["done"]} | {stats["blocked"]} | {total} |')
lines.append('')
lines.append('Machine-readable metadata (orchestration + deps + acceptance counts): `index-meta.json` (generated alongside this file).')
lines.append('')
lines.append('## Dependency order')
lines.append('')
for idx, iid in enumerate(order, 1):
    data = issues[iid]
    lines.append(f'{idx}. [{iid} — {data["title"]}]({data["path"]})')
lines.append('')
lines.append('## Issue table')
lines.append('')
lines.append('| ID | Title | Track | Depends on | Blocks | Acceptance | Orchestration | Orch notes | |')
lines.append('|----|-------|-------|------------|--------|------------|---------------|------------|-|')
for iid in order:
    data = issues[iid]
    deps = ', '.join(data['deps']) if data['deps'] else 'none'
    blocks = ', '.join(sorted(reverse.get(iid, []))) if reverse.get(iid) else 'none'
    progress = f'{data["checked"]} checked / {data["unchecked"]} open'
    orch = data.get('orchestration_class', '') or '—'
    orch_note = str(data.get('orchestration_upstream', '') or '—').replace('|', '/')
    lines.append(f'| {iid} | [{data["title"]}]({data["path"]}) | {data["track"]} | {deps} | {blocks} | {progress} | {orch} | {orch_note} | |')

if blocked_issues:
    lines.append('')
    lines.append('## Blocked issues')
    lines.append('')
    lines.append('Issues in `issues/blocked/` — waiting on external dependencies.')
    lines.append('')
    lines.append('| ID | Title | Track | Blocked by | |')
    lines.append('|----|-------|-------|------------|--|')
    for iid in sorted(blocked_issues):
        data = blocked_issues[iid]
        blocked_by = data['blocked_by'] or 'see issue'
        lines.append(f'| {iid} | [{data["title"]}](../../issues/blocked/{data["path"]}) | {data["track"]} | {blocked_by} | |')

index_out.write_text(re.sub(r'\n{3,}', '\n\n', '\n'.join(lines)) + '\n')

# graph markdown
all_for_graph = {**issues, **blocked_issues}
mermaid = ['graph LR']
for iid in order:
    mermaid.append(f'  I{iid}["{iid} {issues[iid]["title"]}"]')
for iid in sorted(blocked_issues):
    mermaid.append(f'  I{iid}["{iid} {blocked_issues[iid]["title"]} ⛔"]')
for iid in list(order) + sorted(blocked_issues):
    for dep in all_for_graph[iid]['deps']:
        if dep in all_for_graph:
            mermaid.append(f'  I{dep} --> I{iid}')

graph = []
graph.append('# Issue Dependency Graph')
graph.append('')
graph.append('Auto-generated by `scripts/gen/generate-issue-index.py`. Do not edit manually.')
graph.append('')
graph.append('## Mermaid graph')
graph.append('')
graph.append('```mermaid')
graph.extend(mermaid)
graph.append('```')
graph.append('')
graph.append('## Adjacency list')
graph.append('')
for iid in order:
    deps = ', '.join(issues[iid]['deps']) if issues[iid]['deps'] else 'none'
    blocks = ', '.join(sorted(reverse.get(iid, []))) if reverse.get(iid) else 'none'
    graph.append(f'- **{iid}** depends on: {deps}; blocks: {blocks}')
if blocked_issues:
    graph.append('')
    graph.append('### Blocked')
    graph.append('')
    for iid in sorted(blocked_issues):
        data = blocked_issues[iid]
        deps = ', '.join(data['deps']) if data['deps'] else 'none'
        graph.append(f'- **{iid}** ⛔ blocked — depends on: {deps}; blocked by: {data["blocked_by"] or "external"}')

graph_out.write_text(re.sub(r'\n{3,}', '\n\n', '\n'.join(graph)) + '\n')

meta_payload = {
    'schema': 'arukellt-issue-index-meta-v1',
    'generated_at': datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),
    'generator': 'scripts/gen/generate-issue-index.py',
    'open_issues': [
        {
            'id': iid,
            'path': issues[iid]['path'],
            'title': issues[iid]['title'],
            'track': issues[iid]['track'],
            'status': issues[iid]['status'],
            'depends_on': issues[iid]['deps'],
            'blocks_issue_ids': sorted(reverse.get(iid, [])),
            'acceptance': {
                'checked': issues[iid]['checked'],
                'unchecked': issues[iid]['unchecked'],
            },
            'orchestration': {
                'class': issues[iid].get('orchestration_class') or None,
                'upstream_notes': issues[iid].get('orchestration_upstream') or None,
            },
        }
        for iid in order
    ],
    'blocked_external': [
        {
            'id': iid,
            'path': blocked_issues[iid]['path'],
            'title': blocked_issues[iid]['title'],
            'track': blocked_issues[iid]['track'],
            'blocked_by': blocked_issues[iid]['blocked_by'],
            'depends_on': blocked_issues[iid]['deps'],
        }
        for iid in sorted(blocked_issues)
    ],
}
meta_out.write_text(json.dumps(meta_payload, indent=2, ensure_ascii=False) + '\n')

# priority table
priority_rows = []
for iid in order:
    data = issues[iid]
    blocker_score = min(5, len(reverse.get(iid, [])))
    release_score = score_release(data)
    readiness_score = score_readiness(data)
    strategic_score = score_strategic(data)
    ma_score = score_multi_agent(data)
    total = blocker_score + release_score + readiness_score + strategic_score + ma_score
    priority_rows.append(
        (
            -total,
            issue_sort_key(iid),
            iid,
            data,
            blocker_score,
            release_score,
            readiness_score,
            strategic_score,
            ma_score,
            total,
        )
    )

priority_rows.sort()
priority_lines = [
    "# Open Issues Priority Table (Multi-Agent Scoring)",
    "",
    "Generated by `scripts/gen/generate-issue-index.py` from `index-meta.json` inputs.",
    "",
    "Scoring criteria (0-5 each, total 25):",
    "- **Blocker**: Number of open downstream issues blocked.",
    "- **Release**: Release-track weight.",
    "- **Readiness**: Acceptance progress and executable checklist clarity.",
    "- **Strategic**: Core language / compiler / selfhost / stdlib weight.",
    "- **MA-Suit**: Multi-agent suitability (fewer deps, clear acceptance criteria).",
    "",
    "| Rank | ID | Title | Track | Blocker | Release | Readiness | Strategic | MA-Suit | Total |",
    "|------|----|-------|-------|---------|---------|-----------|-----------|---------|-------|",
]
for rank, row in enumerate(priority_rows, 1):
    (
        _negative_total,
        _sort_key,
        iid,
        data,
        blocker_score,
        release_score,
        readiness_score,
        strategic_score,
        ma_score,
        total,
    ) = row
    priority_lines.append(
        f'| {rank} | {iid} | {truncate(data["title"])} | {data["track"]} | '
        f"{blocker_score} | {release_score} | {readiness_score} | "
        f"{strategic_score} | {ma_score} | {total} |"
    )

priority_out.write_text("\n".join(priority_lines) + "\n")
