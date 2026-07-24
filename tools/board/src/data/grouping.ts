import type { IssueRecord, IssueStatus } from "../../shared/types";

export type BoardAxis = "status" | "readiness" | "orchestration" | "track" | "priority";

export const BOARD_AXES: { id: BoardAxis; label: string; hint: string }[] = [
    { id: "status", label: "Status", hint: "issues/ のディレクトリ = 運用上の正" },
    { id: "readiness", label: "Readiness", hint: "依存が解けているか。着手可能な issue を選ぶ用" },
    { id: "orchestration", label: "Orchestration", hint: "Orchestration class ヘッダ。エージェント投入の分類" },
    { id: "track", label: "Track", hint: "Track ヘッダ。複数 track の issue は複数レーンに出る" },
    { id: "priority", label: "Priority", hint: "Priority ヘッダ。未設定は末尾" },
];

export type Readiness = "ready" | "waiting" | "blocked" | "done" | "reject";

export interface Lane {
    id: string;
    label: string;
    /** CSS colour token driving the lane accent and card status dot. */
    color: string;
    hint: string;
    issues: IssueRecord[];
}

export interface BoardFilters {
    statuses: IssueStatus[];
    tracks: string[];
    orchestrationClasses: string[];
    query: string;
    /** Restrict to open issues whose dependencies are all resolved. */
    onlyReady: boolean;
}

export const DEFAULT_FILTERS: BoardFilters = {
    // 700+ done issues would bury the actionable ones, so history is opt-in.
    statuses: ["open", "blocked"],
    tracks: [],
    orchestrationClasses: [],
    query: "",
    onlyReady: false,
};

const STATUS_COLORS: Record<IssueStatus, string> = {
    open: "var(--status-open)",
    blocked: "var(--status-blocked)",
    done: "var(--status-done)",
    reject: "var(--status-reject)",
};

const STATUS_HINTS: Record<IssueStatus, string> = {
    open: "issues/open — 作業待ち",
    blocked: "issues/blocked — 上流待ち",
    done: "issues/done — 完了",
    reject: "issues/reject — 不採用",
};

const READINESS_LANES: { id: Readiness; label: string; color: string; hint: string }[] = [
    { id: "ready", label: "Ready", color: "var(--status-ready)", hint: "open かつ依存がすべて done/reject" },
    { id: "waiting", label: "Waiting on deps", color: "var(--status-blocked)", hint: "open だが未完了の依存が残る" },
    { id: "blocked", label: "Blocked", color: "var(--doc-rejected)", hint: "issues/blocked に置かれている" },
    { id: "done", label: "Done", color: "var(--status-done)", hint: "issues/done" },
    { id: "reject", label: "Rejected", color: "var(--status-reject)", hint: "issues/reject" },
];

export function statusColor(status: IssueStatus): string {
    return STATUS_COLORS[status];
}

export function issueIndex(issues: IssueRecord[]): Map<string, IssueRecord> {
    return new Map(issues.map((issue) => [issue.id, issue]));
}

/** Dependencies that are neither done nor rejected, i.e. what actually gates work. */
export function unresolvedDeps(issue: IssueRecord, byId: Map<string, IssueRecord>): IssueRecord[] {
    return issue.dependsOn
        .map((id) => byId.get(id))
        .filter((dep): dep is IssueRecord => Boolean(dep) && dep!.status !== "done" && dep!.status !== "reject");
}

export function readinessOf(issue: IssueRecord, byId: Map<string, IssueRecord>): Readiness {
    if (issue.status === "done") return "done";
    if (issue.status === "reject") return "reject";
    if (issue.status === "blocked") return "blocked";
    return unresolvedDeps(issue, byId).length === 0 ? "ready" : "waiting";
}

function matchesQuery(issue: IssueRecord, query: string): boolean {
    const needle = query.trim().toLowerCase();
    if (!needle) return true;
    if (needle.startsWith("#")) return issue.id === needle.slice(1);
    return (
        issue.id.includes(needle) ||
        issue.title.toLowerCase().includes(needle) ||
        issue.summary.toLowerCase().includes(needle) ||
        issue.tracks.some((track) => track.includes(needle))
    );
}

export function applyFilters(
    issues: IssueRecord[],
    filters: BoardFilters,
    byId: Map<string, IssueRecord>,
): IssueRecord[] {
    return issues.filter((issue) => {
        if (!filters.statuses.includes(issue.status)) return false;
        if (filters.tracks.length && !issue.tracks.some((track) => filters.tracks.includes(track))) return false;
        if (filters.orchestrationClasses.length && !filters.orchestrationClasses.includes(issue.orchestrationClass)) {
            return false;
        }
        if (filters.onlyReady && readinessOf(issue, byId) !== "ready") return false;
        return matchesQuery(issue, filters.query);
    });
}

/** Lane accent for value-derived axes, cycled so adjacent lanes stay distinguishable. */
const LANE_PALETTE = [
    "var(--status-open)",
    "var(--status-ready)",
    "var(--doc-superseded)",
    "var(--status-blocked)",
    "var(--status-done)",
    "var(--doc-rejected)",
];

function paletteColor(index: number): string {
    return LANE_PALETTE[index % LANE_PALETTE.length];
}

function groupByKey(
    issues: IssueRecord[],
    keysOf: (issue: IssueRecord) => string[],
    labelOf: (key: string) => string,
): Lane[] {
    const buckets = new Map<string, IssueRecord[]>();
    for (const issue of issues) {
        for (const key of keysOf(issue)) {
            const bucket = buckets.get(key);
            if (bucket) bucket.push(issue);
            else buckets.set(key, [issue]);
        }
    }
    return [...buckets.entries()]
        .sort((a, b) => b[1].length - a[1].length || a[0].localeCompare(b[0]))
        .map(([key, laneIssues], index) => ({
            id: key,
            label: labelOf(key),
            color: paletteColor(index),
            hint: `${laneIssues.length} issue`,
            issues: laneIssues,
        }));
}

/**
 * Build lanes for the selected axis.
 *
 * Status lanes follow the status filter rather than the full vocabulary: a lane
 * for a status the user switched off is guaranteed empty and only steals width.
 * Readiness keeps all lanes so the pipeline shape stays legible while filtering.
 * Value-derived axes (track, priority, orchestration) show only non-empty lanes,
 * because their vocabularies run to dozens of rarely-used values.
 */
export function buildLanes(
    issues: IssueRecord[],
    axis: BoardAxis,
    byId: Map<string, IssueRecord>,
    enabledStatuses: IssueStatus[],
): Lane[] {
    if (axis === "status") {
        return (["open", "blocked", "done", "reject"] as IssueStatus[])
            .filter((status) => enabledStatuses.includes(status))
            .map((status) => ({
                id: status,
                label: status,
                color: STATUS_COLORS[status],
                hint: STATUS_HINTS[status],
                issues: issues.filter((issue) => issue.status === status),
            }));
    }

    if (axis === "readiness") {
        const readiness = new Map(issues.map((issue) => [issue.id, readinessOf(issue, byId)]));
        return READINESS_LANES.filter(
            (lane) => lane.id === "ready" || lane.id === "waiting" || enabledStatuses.includes(lane.id as IssueStatus),
        ).map((lane) => ({
            ...lane,
            issues: issues.filter((issue) => readiness.get(issue.id) === lane.id),
        }));
    }

    if (axis === "track") {
        return groupByKey(issues, (issue) => issue.tracks, (key) => key);
    }

    if (axis === "orchestration") {
        return groupByKey(issues, (issue) => [issue.orchestrationClass], (key) => key);
    }

    const lanes = groupByKey(
        issues,
        (issue) => [issue.priority === null ? "unset" : `P${issue.priority}`],
        (key) => (key === "unset" ? "Priority 未設定" : key),
    );
    return lanes.sort((a, b) => {
        if (a.id === "unset") return 1;
        if (b.id === "unset") return -1;
        return a.id.localeCompare(b.id, undefined, { numeric: true });
    });
}
