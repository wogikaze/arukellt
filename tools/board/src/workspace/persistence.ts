import type { PaneNode, TabView, WorkspaceState } from "./types";
import { collectLeaves, initialWorkspace } from "./reducer";

const STORAGE_KEY = "arukellt-board:workspace:v1";
const LAYOUT_ROUTE = "#/layout/";

/* --- structural validation ------------------------------------------------
   Persisted layouts outlive code changes and can be pasted in from a shared
   link, so anything that fails validation falls back to a fresh workspace
   rather than rendering a half-broken pane tree. */

function isTabView(value: unknown): value is TabView {
    if (typeof value !== "object" || value === null) return false;
    const view = value as Record<string, unknown>;
    switch (view.kind) {
        case "board":
            return true;
        case "graph":
            return view.focus === null || typeof view.focus === "string";
        case "file":
            return typeof view.path === "string" && view.path.length > 0;
        case "search":
            return typeof view.query === "string";
        default:
            return false;
    }
}

function isPaneNode(value: unknown): value is PaneNode {
    if (typeof value !== "object" || value === null) return false;
    const node = value as Record<string, unknown>;
    if (typeof node.id !== "string") return false;

    if (node.kind === "leaf") {
        if (!Array.isArray(node.tabs) || node.tabs.length === 0) return false;
        return node.tabs.every((tab) => {
            if (typeof tab !== "object" || tab === null) return false;
            const record = tab as Record<string, unknown>;
            return typeof record.id === "string" && isTabView(record.view);
        });
    }

    if (node.kind === "split") {
        if (node.axis !== "row" && node.axis !== "column") return false;
        if (!Array.isArray(node.children) || node.children.length < 2) return false;
        if (!Array.isArray(node.sizes) || node.sizes.length !== node.children.length) return false;
        if (!node.sizes.every((size) => typeof size === "number" && size > 0)) return false;
        return node.children.every(isPaneNode);
    }

    return false;
}

function isWorkspaceState(value: unknown): value is WorkspaceState {
    if (typeof value !== "object" || value === null) return false;
    const state = value as Record<string, unknown>;
    if (typeof state.activePaneId !== "string" || !isPaneNode(state.root)) return false;
    return collectLeaves(state.root as PaneNode).some((leaf) => leaf.id === state.activePaneId);
}

/* --- shareable layout links ---------------------------------------------- */

function toBase64Url(text: string): string {
    const bytes = new TextEncoder().encode(text);
    let binary = "";
    for (const byte of bytes) binary += String.fromCharCode(byte);
    return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function fromBase64Url(encoded: string): string {
    const padded = encoded.replace(/-/g, "+").replace(/_/g, "/").padEnd(Math.ceil(encoded.length / 4) * 4, "=");
    const binary = atob(padded);
    const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
    return new TextDecoder().decode(bytes);
}

export function encodeLayoutLink(state: WorkspaceState): string {
    return `${location.origin}${location.pathname}${LAYOUT_ROUTE}${toBase64Url(JSON.stringify(state))}`;
}

/* --- deep links -----------------------------------------------------------
   `#/issue/714`, `#/file/docs/adr/ADR-013-primary-target.md`, `#/graph/714`,
   `#/search/wasm32-gc`, `#/board`. Issue links resolve to a path at open time,
   so the resolver is supplied by the caller that owns the dataset. */

export interface DeepLink {
    kind: "board" | "graph" | "search" | "file" | "issue";
    value: string;
}

export function parseDeepLink(hash: string): DeepLink | null {
    const route = hash.replace(/^#\/?/, "");
    if (!route) return null;
    const slash = route.indexOf("/");
    const head = slash === -1 ? route : route.slice(0, slash);
    const rest = slash === -1 ? "" : decodeURIComponent(route.slice(slash + 1));

    if (head === "board") return { kind: "board", value: "" };
    if (head === "graph") return { kind: "graph", value: rest };
    if (head === "search") return { kind: "search", value: rest };
    if (head === "file") return { kind: "file", value: rest };
    if (head === "issue") return { kind: "issue", value: rest };
    return null;
}

/* --- load / save ---------------------------------------------------------- */

export function loadWorkspace(): WorkspaceState {
    if (location.hash.startsWith(LAYOUT_ROUTE)) {
        try {
            const decoded: unknown = JSON.parse(fromBase64Url(location.hash.slice(LAYOUT_ROUTE.length)));
            if (isWorkspaceState(decoded)) return decoded;
        } catch {
            // Fall through to stored or fresh state.
        }
    }
    try {
        const stored: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
        if (isWorkspaceState(stored)) return stored;
    } catch {
        // Fall through to fresh state.
    }
    return initialWorkspace();
}

export function saveWorkspace(state: WorkspaceState): void {
    try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
    } catch {
        // Private-mode or quota failures must not break the session.
    }
}
