import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";

import type { BoardData, IssueRecord } from "../../shared/types";
import { fetchBoard } from "../api/client";

interface BoardContextValue {
    data: BoardData | null;
    error: string | null;
    isRefreshing: boolean;
    reload: () => void;
    /** Issue lookup by canonical id, rebuilt only when the dataset changes. */
    issuesById: Map<string, IssueRecord>;
}

const BoardContext = createContext<BoardContextValue | null>(null);

export function BoardProvider({ children }: { children: ReactNode }): ReactNode {
    const [data, setData] = useState<BoardData | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [isRefreshing, setRefreshing] = useState(true);
    const [reloadToken, setReloadToken] = useState(0);

    useEffect(() => {
        const controller = new AbortController();
        setRefreshing(true);
        fetchBoard({ refresh: reloadToken > 0, signal: controller.signal })
            .then((next) => {
                setData(next);
                setError(null);
            })
            .catch((cause: unknown) => {
                if (controller.signal.aborted) return;
                setError(cause instanceof Error ? cause.message : String(cause));
            })
            .finally(() => {
                if (!controller.signal.aborted) setRefreshing(false);
            });
        return () => controller.abort();
    }, [reloadToken]);

    const reload = useCallback(() => setReloadToken((token) => token + 1), []);
    const issuesById = useMemo(
        () => new Map((data?.issues ?? []).map((issue) => [issue.id, issue])),
        [data],
    );

    const value = useMemo(
        () => ({ data, error, isRefreshing, reload, issuesById }),
        [data, error, isRefreshing, reload, issuesById],
    );
    return <BoardContext.Provider value={value}>{children}</BoardContext.Provider>;
}

export function useBoard(): BoardContextValue {
    const value = useContext(BoardContext);
    if (!value) throw new Error("useBoard must be used inside BoardProvider");
    return value;
}

/**
 * Dataset accessor for views that cannot render without data. The shell blocks
 * on the first load, so by the time a view mounts the dataset exists.
 */
export function useBoardData(): BoardData {
    const { data } = useBoard();
    if (!data) throw new Error("board data is not loaded yet");
    return data;
}
