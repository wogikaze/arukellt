/**
 * Wire contract between the board server and the SPA.
 *
 * The server is the only component that touches the repository, so every field
 * here is already normalized: the SPA never re-parses markdown frontmatter.
 */

export type IssueStatus = "open" | "blocked" | "done" | "reject";

export const ISSUE_STATUSES: readonly IssueStatus[] = ["open", "blocked", "done", "reject"];

/** Acceptance-criteria checkbox tally scraped from an issue body. */
export interface AcceptanceCount {
    checked: number;
    total: number;
}

export interface IssueRecord {
    /** Canonical id such as `714` or `028b`; unique across all status dirs. */
    id: string;
    /** Numeric prefix used for sorting; `028b` sorts as 28. */
    sortKey: number;
    title: string;
    /** Repository-relative path, e.g. `issues/open/714-....md`. */
    path: string;
    status: IssueStatus;
    tracks: string[];
    priority: number | null;
    dependsOn: string[];
    /** Reverse edges, resolved server-side across every status dir. */
    blocks: string[];
    orchestrationClass: string;
    orchestrationUpstream: string;
    created: string;
    updated: string;
    acceptance: AcceptanceCount;
    /** First prose paragraph of the `## Summary` section, trimmed for cards. */
    summary: string;
}

/** Documents grouped by the directory that owns them. */
export type DocKind = "adr" | "rfc" | "plan" | "research" | "process" | "state" | "guide";

export const DOC_KINDS: readonly DocKind[] = [
    "adr",
    "rfc",
    "plan",
    "research",
    "process",
    "state",
    "guide",
];

/**
 * Lifecycle of a decision document. ADR statuses are binding per AGENTS.md:
 * only `ACCEPTED` constrains implementation.
 */
export type DocStatus = "ACCEPTED" | "PROPOSED" | "SUPERSEDED" | "DRAFT" | "REJECTED" | "UNKNOWN";

export interface DocRecord {
    id: string;
    kind: DocKind;
    title: string;
    path: string;
    status: DocStatus;
    /** Raw status line as written, kept because ADRs annotate rationale inline. */
    statusText: string;
    decidedOn: string;
    /** Issue ids referenced as `#123` in the body, for cross-linking. */
    relatedIssues: string[];
    /** Other decision docs referenced as `ADR-013` / `RFC-008`. */
    relatedDocs: string[];
    summary: string;
}

export interface TreeNode {
    name: string;
    path: string;
    type: "dir" | "file";
    children?: TreeNode[];
}

export interface BoardStats {
    issuesByStatus: Record<IssueStatus, number>;
    docsByStatus: Record<string, number>;
    trackCount: number;
    /** Open issues whose dependencies are all satisfied — the actionable set. */
    readyCount: number;
}

export interface BoardData {
    generatedAt: string;
    repoRoot: string;
    repoName: string;
    branch: string;
    issues: IssueRecord[];
    docs: DocRecord[];
    tree: TreeNode[];
    tracks: string[];
    orchestrationClasses: string[];
    stats: BoardStats;
}

export interface FileContent {
    path: string;
    absolutePath: string;
    text: string;
    /** Present for issues and decision docs so detail views can show metadata. */
    issue: IssueRecord | null;
    doc: DocRecord | null;
}

export interface SearchHit {
    path: string;
    title: string;
    kind: "issue" | "doc";
    /** Status of the issue or decision doc, for badge rendering. */
    status: string;
    score: number;
    /** Matching lines with `line` (1-based) and surrounding text. */
    excerpts: { line: number; text: string }[];
}

export interface SearchResponse {
    query: string;
    total: number;
    hits: SearchHit[];
}

export interface ApiError {
    error: string;
}
