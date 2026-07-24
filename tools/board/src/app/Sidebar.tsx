import { useMemo, useState } from "react";
import type { ReactNode } from "react";

import type { DocKind, DocRecord, TreeNode } from "../../shared/types";
import { DOC_KINDS } from "../../shared/types";
import { useBoard } from "../data/BoardContext";
import { docKindLabel, docStatusColor } from "../workspace/tab-labels";
import { useWorkspace } from "../workspace/WorkspaceContext";

type SidebarSection = "decisions" | "files";

export function Sidebar(): ReactNode {
    const { data } = useBoard();
    const [section, setSection] = useState<SidebarSection>("decisions");
    const [query, setQuery] = useState("");

    if (!data) return <aside className="sidebar" />;

    return (
        <aside className="sidebar" aria-label="ナビゲーション">
            <StatsPanel />
            <div className="sidebar__tabs" role="tablist">
                <button
                    type="button"
                    role="tab"
                    aria-selected={section === "decisions"}
                    className={section === "decisions" ? "sidebar__tab sidebar__tab--active" : "sidebar__tab"}
                    onClick={() => setSection("decisions")}
                >
                    決定記録
                </button>
                <button
                    type="button"
                    role="tab"
                    aria-selected={section === "files"}
                    className={section === "files" ? "sidebar__tab sidebar__tab--active" : "sidebar__tab"}
                    onClick={() => setSection("files")}
                >
                    docs/
                </button>
            </div>
            <input
                type="search"
                className="sidebar__filter"
                placeholder={section === "decisions" ? "ADR / RFC を絞り込み" : "ファイル名を絞り込み"}
                value={query}
                onChange={(event) => setQuery(event.target.value)}
            />
            <div className="sidebar__scroll">
                {section === "decisions" ? <DecisionList docs={data.docs} query={query} /> : <FileTree nodes={data.tree} query={query} />}
            </div>
        </aside>
    );
}

function StatsPanel(): ReactNode {
    const { data } = useBoard();
    const { dispatch } = useWorkspace();
    if (!data) return null;

    const { issuesByStatus, readyCount } = data.stats;
    const cells = [
        { label: "open", value: issuesByStatus.open, color: "var(--status-open)" },
        { label: "ready", value: readyCount, color: "var(--status-ready)" },
        { label: "blocked", value: issuesByStatus.blocked, color: "var(--status-blocked)" },
        { label: "done", value: issuesByStatus.done, color: "var(--status-done)" },
    ];

    return (
        <div className="stats">
            {cells.map((cell) => (
                <button
                    key={cell.label}
                    type="button"
                    className="stats__cell"
                    style={{ ["--status-color" as string]: cell.color }}
                    title={
                        cell.label === "ready"
                            ? "依存がすべて解決済みの open issue。Board の Ready only と同じ集合"
                            : `issues/${cell.label}`
                    }
                    onClick={() =>
                        dispatch({
                            type: "open",
                            view:
                                cell.label === "ready"
                                    ? { kind: "board", axis: "readiness", filters: { statuses: ["open"], tracks: [], orchestrationClasses: [], query: "", onlyReady: true } }
                                    : { kind: "board", axis: "status", filters: { statuses: [cell.label as "open"], tracks: [], orchestrationClasses: [], query: "", onlyReady: false } },
                            target: "active",
                        })
                    }
                >
                    <span className="stats__value">{cell.value}</span>
                    <span className="stats__label">{cell.label}</span>
                </button>
            ))}
        </div>
    );
}

function DecisionList({ docs, query }: { docs: DocRecord[]; query: string }): ReactNode {
    const { open } = useWorkspace();
    const [collapsed, setCollapsed] = useState<Set<DocKind>>(new Set(["process", "state", "research"]));

    const grouped = useMemo(() => {
        const needle = query.trim().toLowerCase();
        const matching = needle
            ? docs.filter((doc) => doc.title.toLowerCase().includes(needle) || doc.id.toLowerCase().includes(needle))
            : docs;
        return DOC_KINDS.map((kind) => ({ kind, items: matching.filter((doc) => doc.kind === kind) })).filter(
            (group) => group.items.length > 0,
        );
    }, [docs, query]);

    return (
        <nav className="tree">
            {grouped.map((group) => {
                // Filtering implies intent to see the matches, so collapsed
                // groups expand automatically while a query is active.
                const isCollapsed = collapsed.has(group.kind) && !query;
                return (
                    <div key={group.kind} className="tree__group">
                        <button
                            type="button"
                            className="tree__group-header"
                            aria-expanded={!isCollapsed}
                            onClick={() =>
                                setCollapsed((current) => {
                                    const next = new Set(current);
                                    if (next.has(group.kind)) next.delete(group.kind);
                                    else next.add(group.kind);
                                    return next;
                                })
                            }
                        >
                            <span className="tree__caret">{isCollapsed ? "▸" : "▾"}</span>
                            {docKindLabel(group.kind)}
                            <span className="tree__count">{group.items.length}</span>
                        </button>
                        {isCollapsed
                            ? null
                            : group.items.map((doc) => (
                                  <button
                                      key={doc.path}
                                      type="button"
                                      className="tree__item"
                                      style={{ ["--status-color" as string]: docStatusColor(doc.status) }}
                                      title={`${doc.path}\n${doc.statusText || doc.status}`}
                                      onClick={(event) => open({ kind: "file", path: doc.path }, event)}
                                  >
                                      <span className="status-dot" />
                                      <span className="tree__label">{doc.title}</span>
                                  </button>
                              ))}
                    </div>
                );
            })}
            {grouped.length === 0 ? <p className="tree__empty">該当なし</p> : null}
        </nav>
    );
}

function FileTree({ nodes, query }: { nodes: TreeNode[]; query: string }): ReactNode {
    const needle = query.trim().toLowerCase();
    const filtered = useMemo(() => (needle ? filterTree(nodes, needle) : nodes), [nodes, needle]);
    return (
        <nav className="tree">
            {filtered.map((node) => (
                <TreeBranch key={node.path} node={node} depth={0} forceOpen={Boolean(needle)} />
            ))}
            {filtered.length === 0 ? <p className="tree__empty">該当なし</p> : null}
        </nav>
    );
}

function filterTree(nodes: TreeNode[], needle: string): TreeNode[] {
    const kept: TreeNode[] = [];
    for (const node of nodes) {
        if (node.type === "file") {
            if (node.path.toLowerCase().includes(needle)) kept.push(node);
            continue;
        }
        const children = filterTree(node.children ?? [], needle);
        if (children.length) kept.push({ ...node, children });
    }
    return kept;
}

function TreeBranch({ node, depth, forceOpen }: { node: TreeNode; depth: number; forceOpen: boolean }): ReactNode {
    const { open } = useWorkspace();
    // Only the top level starts expanded; docs/ is deep enough that expanding
    // everything would bury the structure.
    const [isOpen, setOpen] = useState(depth === 0);
    const expanded = forceOpen || isOpen;
    const indent = { paddingLeft: `${8 + depth * 12}px` };

    if (node.type === "file") {
        return (
            <button
                type="button"
                className="tree__item"
                style={indent}
                title={node.path}
                onClick={(event) => open({ kind: "file", path: node.path }, event)}
            >
                <span className="tree__label">{node.name}</span>
            </button>
        );
    }

    return (
        <div className="tree__group">
            <button
                type="button"
                className="tree__group-header"
                style={indent}
                aria-expanded={expanded}
                onClick={() => setOpen((current) => !current)}
            >
                <span className="tree__caret">{expanded ? "▾" : "▸"}</span>
                {node.name}
            </button>
            {expanded
                ? (node.children ?? []).map((child) => (
                      <TreeBranch key={child.path} node={child} depth={depth + 1} forceOpen={forceOpen} />
                  ))
                : null}
        </div>
    );
}
