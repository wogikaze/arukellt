import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import type { IssueRecord, IssueStatus } from "../shared/types";
import { ISSUE_STATUSES } from "../shared/types";
import { REPO_ROOT } from "./repo";
import {
    countCheckboxes,
    firstHeading,
    inlineMeta,
    pick,
    splitFrontmatter,
    splitList,
    summaryParagraph,
} from "./markdown-meta";

/** `714-foo.md` -> `714`, `028b-foo.md` -> `028b`. */
const FILENAME_ID = /^(\d{1,4})([a-z])?[-_.]/;
/** Issue references inside a dependency field: `#714`, `714`, `issue 714`. */
const DEP_ID = /#?(\d{1,4})([a-z])?\b/;

/** Files the index generator itself writes into issues/open — not issues. */
const GENERATED_FILES = new Set(["index.md", "index-meta.json", "dependency-graph.md", "priority-table.md"]);

function normalizeId(digits: string, suffix: string | undefined): string {
    // Filenames zero-pad to three digits but frontmatter `ID:` does not, so a
    // single canonical form is required for dependency edges to resolve.
    const numeric = String(Number(digits));
    return suffix ? numeric + suffix : numeric;
}

function idFromFilename(name: string): string {
    const match = FILENAME_ID.exec(name);
    return match ? normalizeId(match[1], match[2]) : "";
}

function normalizeDeps(raw: string): string[] {
    const seen: string[] = [];
    for (const token of splitList(raw)) {
        // `035 (done)` and `ADR-035` both appear; only issue numbers are edges.
        if (/^adr[-\s]?\d+/i.test(token) || /^rfc[-\s]?\d+/i.test(token)) continue;
        const match = DEP_ID.exec(token);
        if (!match) continue;
        const id = normalizeId(match[1], match[2]);
        if (!seen.includes(id)) seen.push(id);
    }
    return seen;
}

/**
 * Drop a leading self-reference from a title.
 *
 * Headings are written inconsistently — `# 790 — Foo`, `# #790 Foo`, `# 790: Foo`
 * — and every surface here already renders the id next to the title, so leaving
 * it in produces `#790 790 — Foo` on cards and graph nodes.
 */
function stripLeadingId(title: string, id: string): string {
    const pattern = new RegExp(`^#?${id}\\s*(?:[—–\\-:.]\\s*)?`);
    const stripped = title.replace(pattern, "").trim();
    return stripped || title;
}

function normalizeTracks(raw: string): string[] {
    const tracks = splitList(raw.replace(/`/g, ""))
        .map((token) => token.toLowerCase())
        .filter((token) => token && token !== "none");
    return tracks.length ? [...new Set(tracks)] : ["untracked"];
}

function parsePriority(raw: string): number | null {
    const match = /(\d+)/.exec(raw);
    return match ? Number(match[1]) : null;
}

/**
 * `Status:` in frontmatter can disagree with the directory a file sits in
 * (issues are moved by `git mv`, and the field is sometimes not updated).
 * The directory is the operational truth, so it wins.
 */
function parseIssueFile(absPath: string, relPath: string, dirStatus: IssueStatus, name: string): IssueRecord {
    const text = readFileSync(absPath, "utf8");
    const { meta: frontmatter, body } = splitFrontmatter(text);
    const meta = { ...inlineMeta(body), ...frontmatter };

    const rawId = pick(meta, "ID", "Issue") || idFromFilename(name);
    const idMatch = /^#?(\d{1,4})([a-z])?/.exec(rawId);
    const id = idMatch ? normalizeId(idMatch[1], idMatch[2]) : idFromFilename(name) || name.replace(/\.md$/, "");

    return {
        id,
        sortKey: Number.parseInt(id, 10) || 0,
        title: stripLeadingId(firstHeading(body) || name.replace(/\.md$/, ""), id),
        path: relPath,
        status: dirStatus,
        tracks: normalizeTracks(pick(meta, "Track", "Tracks")),
        priority: parsePriority(pick(meta, "Priority")),
        dependsOn: normalizeDeps(pick(meta, "Depends on", "Depends On", "Blocked by", "Blocked By")),
        blocks: [],
        orchestrationClass: pick(meta, "Orchestration class", "Orchestration Class") || "unclassified",
        orchestrationUpstream: pick(meta, "Orchestration upstream", "Orchestration Upstream"),
        created: pick(meta, "Created"),
        updated: pick(meta, "Updated"),
        acceptance: countCheckboxes(text),
        summary: summaryParagraph(body),
    };
}

function listIssueFiles(dir: string): string[] {
    try {
        return readdirSync(dir)
            .filter((name) => name.endsWith(".md") && !GENERATED_FILES.has(name))
            .sort();
    } catch {
        return [];
    }
}

/**
 * Scan every `issues/<status>/` directory and resolve reverse dependencies.
 * Duplicate ids across directories keep the first occurrence in status order
 * (open before done) so an id resolves to its most actionable record.
 */
export function scanIssues(): IssueRecord[] {
    const byId = new Map<string, IssueRecord>();
    const all: IssueRecord[] = [];

    for (const status of ISSUE_STATUSES) {
        const dir = join(REPO_ROOT, "issues", status);
        for (const name of listIssueFiles(dir)) {
            const relPath = `issues/${status}/${name}`;
            const issue = parseIssueFile(join(dir, name), relPath, status, name);
            all.push(issue);
            if (!byId.has(issue.id)) byId.set(issue.id, issue);
        }
    }

    for (const issue of all) {
        for (const depId of issue.dependsOn) {
            const upstream = byId.get(depId);
            if (upstream && !upstream.blocks.includes(issue.id)) upstream.blocks.push(issue.id);
        }
    }

    all.sort((a, b) => b.sortKey - a.sortKey || a.id.localeCompare(b.id));
    return all;
}

/**
 * An open issue is ready when no dependency is still open or blocked.
 * Dependencies that do not resolve to a known issue cannot gate work, so they
 * are ignored rather than treated as blockers.
 */
export function isReady(issue: IssueRecord, byId: Map<string, IssueRecord>): boolean {
    if (issue.status !== "open") return false;
    return issue.dependsOn.every((depId) => {
        const dep = byId.get(depId);
        return !dep || dep.status === "done" || dep.status === "reject";
    });
}
