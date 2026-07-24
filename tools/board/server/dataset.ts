import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

import type { BoardData, BoardStats, FileContent, IssueRecord, IssueStatus } from "../shared/types";
import { ISSUE_STATUSES } from "../shared/types";
import { currentBranch, REPO_NAME, REPO_ROOT, resolveRepoFile, toRepoRelative } from "./repo";
import { isReady, scanIssues } from "./scan-issues";
import { scanDocs, scanDocsTree } from "./scan-docs";

/** Directories whose contents define the dataset; also the cache-invalidation scope. */
const WATCHED_DIRS = [
    "issues/open",
    "issues/blocked",
    "issues/done",
    "issues/reject",
    "docs/adr",
    "docs/rfcs",
    "docs/plans",
    "docs/research",
    "docs/process",
    "docs/state",
];

/**
 * Cheap change detector: file count plus newest mtime per watched directory.
 * Directory mtime alone misses in-place edits, which is the common case while
 * an author is editing an issue with the board open.
 */
function datasetSignature(): string {
    const parts: string[] = [];
    for (const dir of WATCHED_DIRS) {
        const abs = join(REPO_ROOT, dir);
        let newest = 0;
        let count = 0;
        try {
            for (const name of readdirSync(abs)) {
                if (!name.endsWith(".md")) continue;
                count += 1;
                const mtime = statSync(join(abs, name)).mtimeMs;
                if (mtime > newest) newest = mtime;
            }
        } catch {
            continue;
        }
        parts.push(`${dir}:${count}:${Math.round(newest)}`);
    }
    return parts.join("|");
}

function buildStats(issues: IssueRecord[], byId: Map<string, IssueRecord>, docStatuses: string[]): BoardStats {
    const issuesByStatus = Object.fromEntries(ISSUE_STATUSES.map((s) => [s, 0])) as Record<IssueStatus, number>;
    const tracks = new Set<string>();
    let readyCount = 0;
    for (const issue of issues) {
        issuesByStatus[issue.status] += 1;
        for (const track of issue.tracks) tracks.add(track);
        if (isReady(issue, byId)) readyCount += 1;
    }

    const docsByStatus: Record<string, number> = {};
    for (const status of docStatuses) docsByStatus[status] = (docsByStatus[status] ?? 0) + 1;

    return { issuesByStatus, docsByStatus, trackCount: tracks.size, readyCount };
}

function buildDataset(): BoardData {
    const issues = scanIssues();
    const docs = scanDocs();
    const byId = new Map(issues.map((issue) => [issue.id, issue]));

    const tracks = [...new Set(issues.flatMap((issue) => issue.tracks))].sort();
    const orchestrationClasses = [...new Set(issues.map((issue) => issue.orchestrationClass))].sort();

    return {
        generatedAt: new Date().toISOString(),
        repoRoot: REPO_ROOT,
        repoName: REPO_NAME,
        branch: currentBranch(),
        issues,
        docs,
        tree: scanDocsTree(),
        tracks,
        orchestrationClasses,
        stats: buildStats(issues, byId, docs.map((doc) => doc.status)),
    };
}

let cached: { signature: string; data: BoardData } | null = null;

export function getDataset(forceRefresh = false): BoardData {
    const signature = datasetSignature();
    if (!forceRefresh && cached && cached.signature === signature) return cached.data;
    const data = buildDataset();
    cached = { signature, data };
    return data;
}

/** Raw markdown for one repository file, enriched with its parsed record when known. */
export function getFile(relPath: string): FileContent | null {
    const abs = resolveRepoFile(relPath);
    if (!abs) return null;
    const normalized = toRepoRelative(abs);
    const data = getDataset();
    return {
        path: normalized,
        absolutePath: abs,
        text: readFileSync(abs, "utf8"),
        issue: data.issues.find((issue) => issue.path === normalized) ?? null,
        doc: data.docs.find((doc) => doc.path === normalized) ?? null,
    };
}
