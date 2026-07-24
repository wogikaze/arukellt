import type { PaneNode, SplitAxis, Tab, TabView, WorkspaceState } from "./types";
import { isLeaf, viewIdentity } from "./types";

export type OpenTarget = "active" | "split-right" | "split-down" | "background";

export type WorkspaceAction =
    | { type: "open"; view: TabView; target?: OpenTarget }
    | { type: "replace-view"; paneId: string; tabId: string; view: TabView }
    | { type: "activate-tab"; paneId: string; tabId: string }
    | { type: "close-tab"; paneId: string; tabId: string }
    | { type: "close-active-tab" }
    | { type: "close-other-tabs"; paneId: string; tabId: string }
    | { type: "toggle-pin"; paneId: string; tabId: string }
    | { type: "focus-pane"; paneId: string }
    | { type: "cycle-pane"; delta: number }
    | { type: "cycle-tab"; delta: number }
    | { type: "split-pane"; paneId: string; axis: SplitAxis }
    | { type: "close-pane"; paneId: string }
    | { type: "resize"; splitId: string; sizes: number[] }
    | { type: "move-tab"; tabId: string; fromPaneId: string; toPaneId: string; toIndex: number }
    | { type: "reset" };

let idCounter = 0;

function nextId(prefix: string): string {
    idCounter += 1;
    return `${prefix}-${Date.now().toString(36)}-${idCounter.toString(36)}`;
}

function makeTab(view: TabView): Tab {
    return { id: nextId("tab"), view, pinned: false };
}

export function makeLeaf(views: TabView[]): PaneNode {
    const tabs = views.map(makeTab);
    return { kind: "leaf", id: nextId("pane"), tabs, activeTabId: tabs[0]?.id ?? null };
}

export function initialWorkspace(): WorkspaceState {
    const root = makeLeaf([{ kind: "board" }]);
    return { root, activePaneId: root.id };
}

/* --- tree traversal ------------------------------------------------------ */

export function collectLeaves(node: PaneNode): Extract<PaneNode, { kind: "leaf" }>[] {
    if (isLeaf(node)) return [node];
    return node.children.flatMap(collectLeaves);
}

export function findLeaf(node: PaneNode, paneId: string): Extract<PaneNode, { kind: "leaf" }> | null {
    return collectLeaves(node).find((leaf) => leaf.id === paneId) ?? null;
}

/** Rebuild the tree with `paneId`'s leaf replaced by `update`'s result. */
function mapLeaf(
    node: PaneNode,
    paneId: string,
    update: (leaf: Extract<PaneNode, { kind: "leaf" }>) => PaneNode,
): PaneNode {
    if (isLeaf(node)) return node.id === paneId ? update(node) : node;
    return { ...node, children: node.children.map((child) => mapLeaf(child, paneId, update)) };
}

/**
 * Drop empty leaves and collapse splits that no longer branch.
 * Returns null when the whole subtree disappeared, which the caller replaces
 * with a fresh board pane so the workspace is never empty.
 */
function prune(node: PaneNode): PaneNode | null {
    if (isLeaf(node)) return node.tabs.length ? node : null;

    const kept: PaneNode[] = [];
    const keptSizes: number[] = [];
    node.children.forEach((child, index) => {
        const pruned = prune(child);
        if (!pruned) return;
        kept.push(pruned);
        keptSizes.push(node.sizes[index] ?? 1);
    });

    if (kept.length === 0) return null;
    if (kept.length === 1) return kept[0];

    const total = keptSizes.reduce((sum, size) => sum + size, 0);
    return { ...node, children: kept, sizes: keptSizes.map((size) => size / total) };
}

/** Insert `leaf` next to `paneId` along `axis`, nesting a new split if needed. */
function insertBeside(node: PaneNode, paneId: string, axis: SplitAxis, leaf: PaneNode): PaneNode {
    if (isLeaf(node)) {
        if (node.id !== paneId) return node;
        return { kind: "split", id: nextId("split"), axis, sizes: [0.5, 0.5], children: [node, leaf] };
    }

    const index = node.children.findIndex((child) => isLeaf(child) && child.id === paneId);
    // Same axis: widen the existing split instead of nesting, so three panes
    // side by side stay one row rather than a lopsided tree.
    if (index !== -1 && node.axis === axis) {
        const share = node.sizes[index] ?? 1 / node.children.length;
        const children = [...node.children];
        const sizes = [...node.sizes];
        children.splice(index + 1, 0, leaf);
        sizes.splice(index, 1, share / 2, share / 2);
        return { ...node, children, sizes };
    }

    return { ...node, children: node.children.map((child) => insertBeside(child, paneId, axis, leaf)) };
}

/* --- tab helpers --------------------------------------------------------- */

function withTabClosed(leaf: Extract<PaneNode, { kind: "leaf" }>, tabId: string): PaneNode {
    const index = leaf.tabs.findIndex((tab) => tab.id === tabId);
    if (index === -1) return leaf;
    const tabs = leaf.tabs.filter((tab) => tab.id !== tabId);
    if (leaf.activeTabId !== tabId) return { ...leaf, tabs };
    // Focus the neighbour on the right, matching editor conventions.
    const next = tabs[Math.min(index, tabs.length - 1)];
    return { ...leaf, tabs, activeTabId: next?.id ?? null };
}

function withViewOpened(leaf: Extract<PaneNode, { kind: "leaf" }>, view: TabView): PaneNode {
    const identity = viewIdentity(view);
    const existing = leaf.tabs.find((tab) => viewIdentity(tab.view) === identity);
    if (existing) {
        // Reusing the tab keeps the strip stable. The incoming view still wins
        // for graph/search, where reopening carries a new focus or query — but
        // not for the board, whose axis and filters are user state that a plain
        // "open Board" must not discard.
        const keepExisting = existing.view.kind === "board" && view.kind === "board";
        const tabs = keepExisting
            ? leaf.tabs
            : leaf.tabs.map((tab) => (tab.id === existing.id ? { ...tab, view } : tab));
        return { ...leaf, tabs, activeTabId: existing.id };
    }
    const tab = makeTab(view);
    return { ...leaf, tabs: [...leaf.tabs, tab], activeTabId: tab.id };
}

/* --- reducer -------------------------------------------------------------- */

function openView(state: WorkspaceState, view: TabView, target: OpenTarget): WorkspaceState {
    if (target === "background") {
        const root = mapLeaf(state.root, state.activePaneId, (leaf) => {
            const identity = viewIdentity(view);
            if (leaf.tabs.some((tab) => viewIdentity(tab.view) === identity)) return leaf;
            return { ...leaf, tabs: [...leaf.tabs, makeTab(view)] };
        });
        return { ...state, root };
    }

    if (target === "active") {
        return { ...state, root: mapLeaf(state.root, state.activePaneId, (leaf) => withViewOpened(leaf, view)) };
    }

    const leaf = makeLeaf([view]);
    const axis: SplitAxis = target === "split-right" ? "row" : "column";
    return { root: insertBeside(state.root, state.activePaneId, axis, leaf), activePaneId: leaf.id };
}

function shiftFocus(state: WorkspaceState, delta: number): WorkspaceState {
    const leaves = collectLeaves(state.root);
    if (leaves.length < 2) return state;
    const index = leaves.findIndex((leaf) => leaf.id === state.activePaneId);
    const next = leaves[(index + delta + leaves.length) % leaves.length];
    return { ...state, activePaneId: next.id };
}

function shiftTab(state: WorkspaceState, delta: number): WorkspaceState {
    const leaf = findLeaf(state.root, state.activePaneId);
    if (!leaf || leaf.tabs.length < 2) return state;
    const index = leaf.tabs.findIndex((tab) => tab.id === leaf.activeTabId);
    const next = leaf.tabs[(index + delta + leaf.tabs.length) % leaf.tabs.length];
    return { ...state, root: mapLeaf(state.root, leaf.id, (l) => ({ ...l, activeTabId: next.id })) };
}

function moveTab(state: WorkspaceState, action: Extract<WorkspaceAction, { type: "move-tab" }>): WorkspaceState {
    const source = findLeaf(state.root, action.fromPaneId);
    const tab = source?.tabs.find((candidate) => candidate.id === action.tabId);
    if (!source || !tab) return state;
    if (action.fromPaneId === action.toPaneId) {
        const remaining = source.tabs.filter((candidate) => candidate.id !== action.tabId);
        const from = source.tabs.findIndex((candidate) => candidate.id === action.tabId);
        const insertAt = Math.max(0, Math.min(action.toIndex > from ? action.toIndex - 1 : action.toIndex, remaining.length));
        remaining.splice(insertAt, 0, tab);
        return { ...state, root: mapLeaf(state.root, source.id, (leaf) => ({ ...leaf, tabs: remaining, activeTabId: tab.id })) };
    }

    let root = mapLeaf(state.root, action.fromPaneId, (leaf) => withTabClosed(leaf, action.tabId));
    root = mapLeaf(root, action.toPaneId, (leaf) => {
        const tabs = [...leaf.tabs];
        tabs.splice(Math.max(0, Math.min(action.toIndex, tabs.length)), 0, tab);
        return { ...leaf, tabs, activeTabId: tab.id };
    });
    return { root: prune(root) ?? initialWorkspace().root, activePaneId: action.toPaneId };
}

function afterPrune(state: WorkspaceState, root: PaneNode): WorkspaceState {
    const pruned = prune(root);
    if (!pruned) return initialWorkspace();
    const leaves = collectLeaves(pruned);
    const activeStillExists = leaves.some((leaf) => leaf.id === state.activePaneId);
    return { root: pruned, activePaneId: activeStillExists ? state.activePaneId : leaves[0].id };
}

export function workspaceReducer(state: WorkspaceState, action: WorkspaceAction): WorkspaceState {
    switch (action.type) {
        case "open":
            return openView(state, action.view, action.target ?? "active");

        case "replace-view":
            return {
                ...state,
                root: mapLeaf(state.root, action.paneId, (leaf) => ({
                    ...leaf,
                    tabs: leaf.tabs.map((tab) => (tab.id === action.tabId ? { ...tab, view: action.view } : tab)),
                })),
            };

        case "activate-tab":
            return {
                activePaneId: action.paneId,
                root: mapLeaf(state.root, action.paneId, (leaf) => ({ ...leaf, activeTabId: action.tabId })),
            };

        case "close-tab":
            return afterPrune(state, mapLeaf(state.root, action.paneId, (leaf) => withTabClosed(leaf, action.tabId)));

        case "close-active-tab": {
            const leaf = findLeaf(state.root, state.activePaneId);
            if (!leaf?.activeTabId) return state;
            const tabId = leaf.activeTabId;
            return afterPrune(state, mapLeaf(state.root, leaf.id, (target) => withTabClosed(target, tabId)));
        }

        case "close-other-tabs":
            return afterPrune(
                state,
                mapLeaf(state.root, action.paneId, (leaf) => ({
                    ...leaf,
                    tabs: leaf.tabs.filter((tab) => tab.id === action.tabId || tab.pinned),
                    activeTabId: action.tabId,
                })),
            );

        case "toggle-pin":
            return {
                ...state,
                root: mapLeaf(state.root, action.paneId, (leaf) => ({
                    ...leaf,
                    tabs: leaf.tabs.map((tab) => (tab.id === action.tabId ? { ...tab, pinned: !tab.pinned } : tab)),
                })),
            };

        case "focus-pane":
            return { ...state, activePaneId: action.paneId };

        case "cycle-pane":
            return shiftFocus(state, action.delta);

        case "cycle-tab":
            return shiftTab(state, action.delta);

        case "split-pane": {
            const source = findLeaf(state.root, action.paneId);
            const active = source?.tabs.find((tab) => tab.id === source.activeTabId);
            const leaf = makeLeaf([active ? active.view : { kind: "board" }]);
            return { root: insertBeside(state.root, action.paneId, action.axis, leaf), activePaneId: leaf.id };
        }

        case "close-pane":
            return afterPrune(state, mapLeaf(state.root, action.paneId, (leaf) => ({ ...leaf, tabs: [] })));

        case "resize": {
            const apply = (node: PaneNode): PaneNode => {
                if (isLeaf(node)) return node;
                if (node.id === action.splitId) return { ...node, sizes: action.sizes };
                return { ...node, children: node.children.map(apply) };
            };
            return { ...state, root: apply(state.root) };
        }

        case "move-tab":
            return moveTab(state, action);

        case "reset":
            return initialWorkspace();
    }
}
