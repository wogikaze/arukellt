import { createContext, useCallback, useContext, useEffect, useMemo, useReducer } from "react";
import type { ReactNode } from "react";

import type { TabView, WorkspaceState } from "./types";
import type { OpenTarget, WorkspaceAction } from "./reducer";
import { workspaceReducer } from "./reducer";
import { loadWorkspace, saveWorkspace } from "./persistence";

interface WorkspaceContextValue {
    state: WorkspaceState;
    dispatch: (action: WorkspaceAction) => void;
    /** Convenience wrapper; `event` picks the target the way editors do. */
    open: (view: TabView, event?: { metaKey: boolean; ctrlKey: boolean; shiftKey: boolean }) => void;
}

const WorkspaceContext = createContext<WorkspaceContextValue | null>(null);

function targetFor(event?: { metaKey: boolean; ctrlKey: boolean; shiftKey: boolean }): OpenTarget {
    if (!event) return "active";
    if (event.metaKey || event.ctrlKey) return "split-right";
    if (event.shiftKey) return "background";
    return "active";
}

export function WorkspaceProvider({ children }: { children: ReactNode }): ReactNode {
    const [state, dispatch] = useReducer(workspaceReducer, undefined, loadWorkspace);

    useEffect(() => {
        saveWorkspace(state);
    }, [state]);

    const open = useCallback<WorkspaceContextValue["open"]>(
        (view, event) => dispatch({ type: "open", view, target: targetFor(event) }),
        [],
    );

    const value = useMemo(() => ({ state, dispatch, open }), [state, open]);
    return <WorkspaceContext.Provider value={value}>{children}</WorkspaceContext.Provider>;
}

export function useWorkspace(): WorkspaceContextValue {
    const value = useContext(WorkspaceContext);
    if (!value) throw new Error("useWorkspace must be used inside WorkspaceProvider");
    return value;
}
