import { useCallback, useRef } from "react";
import type { ReactNode } from "react";

import type { PaneNode, SplitAxis, Tab } from "./types";
import { isLeaf } from "./types";
import { useWorkspace } from "./WorkspaceContext";
import { TabBar } from "./TabBar";
import { ViewHost } from "../views/ViewHost";

/** Panes never collapse below this fraction, so a dragged splitter cannot hide one. */
const MIN_PANE_FRACTION = 0.1;

export function PaneTree({ node }: { node: PaneNode }): ReactNode {
    if (isLeaf(node)) return <PaneLeaf node={node} />;
    return <PaneSplit node={node} />;
}

function PaneSplit({ node }: { node: Extract<PaneNode, { kind: "split" }> }): ReactNode {
    const { dispatch } = useWorkspace();
    const containerRef = useRef<HTMLDivElement>(null);

    const startResize = useCallback(
        (index: number, event: React.PointerEvent<HTMLDivElement>) => {
            const container = containerRef.current;
            if (!container) return;
            const box = container.getBoundingClientRect();
            const extent = node.axis === "row" ? box.width : box.height;
            if (extent <= 0) return;

            const origin = node.axis === "row" ? event.clientX : event.clientY;
            const before = node.sizes[index];
            const after = node.sizes[index + 1];
            const handle = event.currentTarget;
            handle.setPointerCapture(event.pointerId);

            const onMove = (move: PointerEvent): void => {
                const position = node.axis === "row" ? move.clientX : move.clientY;
                const raw = (position - origin) / extent;
                const delta = Math.max(
                    MIN_PANE_FRACTION - before,
                    Math.min(after - MIN_PANE_FRACTION, raw),
                );
                const sizes = [...node.sizes];
                sizes[index] = before + delta;
                sizes[index + 1] = after - delta;
                dispatch({ type: "resize", splitId: node.id, sizes });
            };
            const onUp = (): void => {
                handle.removeEventListener("pointermove", onMove);
                handle.removeEventListener("pointerup", onUp);
            };
            handle.addEventListener("pointermove", onMove);
            handle.addEventListener("pointerup", onUp);
        },
        [dispatch, node],
    );

    return (
        <div ref={containerRef} className={`pane-split pane-split--${node.axis}`}>
            {node.children.map((child, index) => (
                <div key={child.id} className="pane-split__slot" style={{ flexGrow: node.sizes[index] }}>
                    <PaneTree node={child} />
                    {index < node.children.length - 1 ? (
                        <div
                            className={`splitter splitter--${node.axis}`}
                            role="separator"
                            aria-orientation={node.axis === "row" ? "vertical" : "horizontal"}
                            aria-label="ペインのサイズを変更"
                            onPointerDown={(event) => startResize(index, event)}
                            onDoubleClick={() => {
                                const even = 1 / node.children.length;
                                dispatch({ type: "resize", splitId: node.id, sizes: node.children.map(() => even) });
                            }}
                        />
                    ) : null}
                </div>
            ))}
        </div>
    );
}

function PaneLeaf({ node }: { node: Extract<PaneNode, { kind: "leaf" }> }): ReactNode {
    const { state, dispatch } = useWorkspace();
    const isActivePane = state.activePaneId === node.id;
    const activeTab: Tab | undefined = node.tabs.find((tab) => tab.id === node.activeTabId) ?? node.tabs[0];
    const canClosePane = state.root.kind === "split";

    const split = (axis: SplitAxis): void => dispatch({ type: "split-pane", paneId: node.id, axis });

    return (
        <section
            className={isActivePane ? "pane pane--active" : "pane"}
            // Capture phase so focusing a pane wins even when the click lands on
            // an inner control that stops propagation.
            onPointerDownCapture={() => {
                if (!isActivePane) dispatch({ type: "focus-pane", paneId: node.id });
            }}
            aria-label={isActivePane ? "アクティブなペイン" : "ペイン"}
        >
            <header className="pane__header">
                <TabBar paneId={node.id} tabs={node.tabs} activeTabId={node.activeTabId} isActivePane={isActivePane} />
                <div className="pane__actions">
                    <button type="button" className="icon-button" title="右に分割 (Ctrl+\\)" onClick={() => split("row")}>
                        <SplitRightIcon />
                    </button>
                    <button type="button" className="icon-button" title="下に分割 (Ctrl+Shift+\\)" onClick={() => split("column")}>
                        <SplitDownIcon />
                    </button>
                    {canClosePane ? (
                        <button
                            type="button"
                            className="icon-button"
                            title="ペインを閉じる"
                            onClick={() => dispatch({ type: "close-pane", paneId: node.id })}
                        >
                            ×
                        </button>
                    ) : null}
                </div>
            </header>
            <div className="pane__body">
                {activeTab ? (
                    <ViewHost paneId={node.id} tabId={activeTab.id} view={activeTab.view} />
                ) : (
                    <div className="empty-state">
                        <p className="empty-state__title">タブがありません</p>
                        <p>Ctrl+K でコマンドパレットを開く</p>
                    </div>
                )}
            </div>
        </section>
    );
}

function SplitRightIcon(): ReactNode {
    return (
        <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="1.3" aria-hidden="true">
            <rect x="1.5" y="2.5" width="13" height="11" rx="1.5" />
            <line x1="8" y1="2.5" x2="8" y2="13.5" />
        </svg>
    );
}

function SplitDownIcon(): ReactNode {
    return (
        <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="1.3" aria-hidden="true">
            <rect x="1.5" y="2.5" width="13" height="11" rx="1.5" />
            <line x1="1.5" y1="8" x2="14.5" y2="8" />
        </svg>
    );
}
