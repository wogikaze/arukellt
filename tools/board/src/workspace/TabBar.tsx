import { useRef, useState } from "react";
import type { ReactNode } from "react";

import type { Tab } from "./types";
import { useWorkspace } from "./WorkspaceContext";
import { tabLabel } from "./tab-labels";
import { useBoard } from "../data/BoardContext";

/** Payload moved between tab strips; panes are identified so cross-pane drops work. */
interface DragPayload {
    tabId: string;
    fromPaneId: string;
}

const DRAG_MIME = "application/x-arukellt-tab";

export function TabBar({
    paneId,
    tabs,
    activeTabId,
    isActivePane,
}: {
    paneId: string;
    tabs: Tab[];
    activeTabId: string | null;
    isActivePane: boolean;
}): ReactNode {
    const { dispatch } = useWorkspace();
    const { data } = useBoard();
    const [dropIndex, setDropIndex] = useState<number | null>(null);
    const stripRef = useRef<HTMLDivElement>(null);

    const readPayload = (event: React.DragEvent): DragPayload | null => {
        try {
            const raw = event.dataTransfer.getData(DRAG_MIME);
            return raw ? (JSON.parse(raw) as DragPayload) : null;
        } catch {
            return null;
        }
    };

    /** Insertion index from the pointer position relative to tab midpoints. */
    const indexFromPointer = (event: React.DragEvent): number => {
        const strip = stripRef.current;
        if (!strip) return tabs.length;
        const children = [...strip.querySelectorAll<HTMLElement>("[data-tab-id]")];
        for (let index = 0; index < children.length; index += 1) {
            const box = children[index].getBoundingClientRect();
            if (event.clientX < box.left + box.width / 2) return index;
        }
        return children.length;
    };

    return (
        <div
            className={isActivePane ? "tabbar tabbar--active" : "tabbar"}
            onDragOver={(event) => {
                if (!event.dataTransfer.types.includes(DRAG_MIME)) return;
                event.preventDefault();
                event.dataTransfer.dropEffect = "move";
                setDropIndex(indexFromPointer(event));
            }}
            onDragLeave={() => setDropIndex(null)}
            onDrop={(event) => {
                const payload = readPayload(event);
                setDropIndex(null);
                if (!payload) return;
                event.preventDefault();
                dispatch({
                    type: "move-tab",
                    tabId: payload.tabId,
                    fromPaneId: payload.fromPaneId,
                    toPaneId: paneId,
                    toIndex: indexFromPointer(event),
                });
            }}
        >
            <div className="tabbar__strip" ref={stripRef} role="tablist">
                {tabs.map((tab, index) => {
                    const label = tabLabel(tab.view, data);
                    const isActive = tab.id === activeTabId;
                    return (
                        <div
                            key={tab.id}
                            data-tab-id={tab.id}
                            className={
                                "tab" +
                                (isActive ? " tab--active" : "") +
                                (tab.pinned ? " tab--pinned" : "") +
                                (dropIndex === index ? " tab--drop-before" : "")
                            }
                            role="tab"
                            aria-selected={isActive}
                            tabIndex={isActive ? 0 : -1}
                            title={label.tooltip}
                            draggable
                            onDragStart={(event) => {
                                event.dataTransfer.effectAllowed = "move";
                                event.dataTransfer.setData(
                                    DRAG_MIME,
                                    JSON.stringify({ tabId: tab.id, fromPaneId: paneId } satisfies DragPayload),
                                );
                            }}
                            onMouseDown={(event) => {
                                // Middle click closes, matching browser tab behaviour.
                                if (event.button === 1) {
                                    event.preventDefault();
                                    dispatch({ type: "close-tab", paneId, tabId: tab.id });
                                }
                            }}
                            onClick={() => dispatch({ type: "activate-tab", paneId, tabId: tab.id })}
                            onDoubleClick={() => dispatch({ type: "toggle-pin", paneId, tabId: tab.id })}
                            onKeyDown={(event) => {
                                if (event.key !== "Enter" && event.key !== " ") return;
                                event.preventDefault();
                                dispatch({ type: "activate-tab", paneId, tabId: tab.id });
                            }}
                        >
                            <span className="status-dot" style={{ ["--status-color" as string]: label.color }} />
                            <span className="tab__title">{label.title}</span>
                            {tab.pinned ? <span className="tab__pin" title="固定中（ダブルクリックで解除）">▪</span> : null}
                            <button
                                type="button"
                                className="tab__close"
                                aria-label="タブを閉じる"
                                title="タブを閉じる"
                                onClick={(event) => {
                                    event.stopPropagation();
                                    dispatch({ type: "close-tab", paneId, tabId: tab.id });
                                }}
                            >
                                ×
                            </button>
                        </div>
                    );
                })}
                {dropIndex === tabs.length ? <div className="tab-drop-marker" /> : null}
            </div>
        </div>
    );
}
