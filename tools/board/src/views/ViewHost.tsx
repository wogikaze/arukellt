import type { ReactNode } from "react";

import type { TabView } from "../workspace/types";
import { DEFAULT_FILTERS } from "../data/grouping";
import { BoardView } from "./BoardView";
import { DocView } from "./DocView";
import { GraphView } from "./GraphView";
import { SearchView } from "./SearchView";

const DEFAULT_GRAPH_DEPTH = 2;

/** Render the view a tab holds, applying defaults for fields older layouts omit. */
export function ViewHost({ paneId, tabId, view }: { paneId: string; tabId: string; view: TabView }): ReactNode {
    switch (view.kind) {
        case "board":
            return (
                <BoardView
                    paneId={paneId}
                    tabId={tabId}
                    axis={view.axis ?? "status"}
                    filters={view.filters ?? DEFAULT_FILTERS}
                />
            );
        case "graph":
            return (
                <GraphView
                    paneId={paneId}
                    tabId={tabId}
                    focus={view.focus}
                    depth={view.depth ?? DEFAULT_GRAPH_DEPTH}
                    showIsolated={view.showIsolated ?? false}
                />
            );
        case "file":
            return <DocView path={view.path} />;
        case "search":
            return <SearchView paneId={paneId} tabId={tabId} query={view.query} />;
    }
}
