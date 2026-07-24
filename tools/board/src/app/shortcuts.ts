import { useEffect } from "react";

import type { WorkspaceAction } from "../workspace/reducer";

interface ShortcutHandlers {
    openPalette: () => void;
    toggleSidebar: () => void;
    dispatch: (action: WorkspaceAction) => void;
    activePaneId: string;
}

/** Typing in a field must not trigger single-key or plain shortcuts. */
function isTextEntry(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false;
    return target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName);
}

/**
 * Global keyboard map.
 *
 * The bindings mirror editor conventions (Ctrl+K palette, Ctrl+\ split,
 * Ctrl+W close tab) because that is what the audience already has in muscle
 * memory. `Ctrl+W` is intercepted before the browser sees it, which browsers
 * permit only because the app is not a top-level tab-close target on Linux.
 */
export function useShortcuts({ openPalette, toggleSidebar, dispatch, activePaneId }: ShortcutHandlers): void {
    useEffect(() => {
        const onKeyDown = (event: KeyboardEvent): void => {
            const modifier = event.ctrlKey || event.metaKey;
            if (!modifier) return;

            switch (event.key.toLowerCase()) {
                case "k":
                    event.preventDefault();
                    openPalette();
                    return;
                case "b":
                    if (isTextEntry(event.target)) return;
                    event.preventDefault();
                    toggleSidebar();
                    return;
                case "\\":
                    event.preventDefault();
                    dispatch({ type: "split-pane", paneId: activePaneId, axis: event.shiftKey ? "column" : "row" });
                    return;
                case "w":
                    event.preventDefault();
                    dispatch({ type: "close-active-tab" });
                    return;
                case "tab":
                    event.preventDefault();
                    dispatch({ type: "cycle-tab", delta: event.shiftKey ? -1 : 1 });
                    return;
                case "]":
                    event.preventDefault();
                    dispatch({ type: "cycle-pane", delta: 1 });
                    return;
                case "[":
                    event.preventDefault();
                    dispatch({ type: "cycle-pane", delta: -1 });
                    return;
                default:
            }
        };

        window.addEventListener("keydown", onKeyDown);
        return () => window.removeEventListener("keydown", onKeyDown);
    }, [openPalette, toggleSidebar, dispatch, activePaneId]);
}
