import type { BoardAxis, BoardFilters } from "../data/grouping";

/**
 * What a tab shows.
 *
 * Board axis/filters and graph depth live in the view rather than in component
 * state so that a split layout can hold two differently-filtered boards, and so
 * that a shared layout link reproduces exactly what the sender was looking at.
 * They are optional: layouts persisted by an older build stay loadable.
 */
export type TabView =
    | { kind: "board"; axis?: BoardAxis; filters?: BoardFilters }
    | { kind: "graph"; focus: string | null; depth?: number; showIsolated?: boolean }
    | { kind: "file"; path: string }
    | { kind: "search"; query: string };

export interface Tab {
    id: string;
    view: TabView;
    /** Pinned tabs survive "close others" and render without a close button. */
    pinned: boolean;
}

export type SplitAxis = "row" | "column";

export type PaneNode =
    | { kind: "leaf"; id: string; tabs: Tab[]; activeTabId: string | null }
    | { kind: "split"; id: string; axis: SplitAxis; sizes: number[]; children: PaneNode[] };

export interface WorkspaceState {
    root: PaneNode;
    activePaneId: string;
}

/**
 * Stable key for a view. Two views with the same identity are the same tab, so
 * clicking the same issue twice focuses it instead of stacking duplicates.
 * Search is excluded on purpose: each query edit mutates the tab in place.
 */
export function viewIdentity(view: TabView): string {
    switch (view.kind) {
        case "board":
            return "board";
        case "graph":
            return "graph";
        case "file":
            return `file:${view.path}`;
        case "search":
            return "search";
    }
}

export function isLeaf(node: PaneNode): node is Extract<PaneNode, { kind: "leaf" }> {
    return node.kind === "leaf";
}
