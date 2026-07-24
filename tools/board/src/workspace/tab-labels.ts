import type { BoardData } from "../../shared/types";
import type { TabView } from "./types";

export interface TabLabel {
    title: string;
    /** Short qualifier shown next to the title, e.g. `open` or `ADR`. */
    kicker: string;
    color: string;
    /** Full text for the tab's tooltip. */
    tooltip: string;
}

const DOC_KIND_LABELS: Record<string, string> = {
    adr: "ADR",
    rfc: "RFC",
    plan: "PLAN",
    research: "RESEARCH",
    process: "PROCESS",
    state: "STATE",
    guide: "DOC",
};

const DOC_STATUS_COLORS: Record<string, string> = {
    ACCEPTED: "var(--doc-accepted)",
    PROPOSED: "var(--doc-proposed)",
    SUPERSEDED: "var(--doc-superseded)",
    DRAFT: "var(--doc-draft)",
    REJECTED: "var(--doc-rejected)",
    UNKNOWN: "var(--doc-unknown)",
};

const STATUS_COLORS: Record<string, string> = {
    open: "var(--status-open)",
    blocked: "var(--status-blocked)",
    done: "var(--status-done)",
    reject: "var(--status-reject)",
};

export function docStatusColor(status: string): string {
    return DOC_STATUS_COLORS[status] ?? "var(--doc-unknown)";
}

export function docKindLabel(kind: string): string {
    return DOC_KIND_LABELS[kind] ?? kind.toUpperCase();
}

/** Resolve a tab's display identity from the dataset, falling back to the path. */
export function tabLabel(view: TabView, data: BoardData | null): TabLabel {
    if (view.kind === "board") {
        return { title: "Board", kicker: "", color: "var(--accent)", tooltip: "Kanban board" };
    }
    if (view.kind === "graph") {
        const suffix = view.focus ? ` #${view.focus}` : "";
        return { title: `Graph${suffix}`, kicker: "", color: "var(--doc-superseded)", tooltip: "Dependency graph" };
    }
    if (view.kind === "search") {
        return {
            title: view.query ? `Search: ${view.query}` : "Search",
            kicker: "",
            color: "var(--status-ready)",
            tooltip: "Full-text search",
        };
    }

    const issue = data?.issues.find((candidate) => candidate.path === view.path);
    if (issue) {
        return {
            title: `#${issue.id} ${issue.title}`,
            kicker: issue.status,
            color: STATUS_COLORS[issue.status] ?? "var(--fg-subtle)",
            tooltip: `${issue.path}\n${issue.title}`,
        };
    }

    const doc = data?.docs.find((candidate) => candidate.path === view.path);
    if (doc) {
        return {
            title: doc.title,
            kicker: docKindLabel(doc.kind),
            color: docStatusColor(doc.status),
            tooltip: `${doc.path}\n${doc.title}`,
        };
    }

    const name = view.path.slice(view.path.lastIndexOf("/") + 1);
    return { title: name, kicker: "", color: "var(--fg-subtle)", tooltip: view.path };
}
