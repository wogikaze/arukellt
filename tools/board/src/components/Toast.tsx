import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";

interface ToastMessage {
    id: number;
    text: string;
    tone: "info" | "error";
}

const ToastContext = createContext<((text: string, tone?: "info" | "error") => void) | null>(null);

const TOAST_LIFETIME_MS = 2200;

export function ToastProvider({ children }: { children: ReactNode }): ReactNode {
    const [messages, setMessages] = useState<ToastMessage[]>([]);
    const nextId = useRef(0);

    const notify = useCallback((text: string, tone: "info" | "error" = "info") => {
        nextId.current += 1;
        const message = { id: nextId.current, text, tone };
        setMessages((current) => [...current, message]);
        setTimeout(() => setMessages((current) => current.filter((item) => item.id !== message.id)), TOAST_LIFETIME_MS);
    }, []);

    const value = useMemo(() => notify, [notify]);
    return (
        <ToastContext.Provider value={value}>
            {children}
            <div className="toast-stack" role="status" aria-live="polite">
                {messages.map((message) => (
                    <div key={message.id} className={`toast toast--${message.tone}`}>
                        {message.text}
                    </div>
                ))}
            </div>
        </ToastContext.Provider>
    );
}

export function useToast(): (text: string, tone?: "info" | "error") => void {
    const notify = useContext(ToastContext);
    if (!notify) throw new Error("useToast must be used inside ToastProvider");
    return notify;
}

/**
 * Clipboard write with a textarea fallback.
 *
 * The board is normally served over plain HTTP on a LAN address, where
 * `navigator.clipboard` is unavailable because the origin is not secure — and
 * copying paths is the whole point of the tool, so the fallback is required.
 */
export async function writeClipboard(text: string): Promise<boolean> {
    if (navigator.clipboard && window.isSecureContext) {
        try {
            await navigator.clipboard.writeText(text);
            return true;
        } catch {
            // Fall through to the legacy path.
        }
    }
    const textarea = document.createElement("textarea");
    textarea.value = text;
    textarea.setAttribute("readonly", "");
    textarea.style.position = "fixed";
    textarea.style.opacity = "0";
    document.body.appendChild(textarea);
    textarea.select();
    let copied = false;
    try {
        copied = document.execCommand("copy");
    } catch {
        copied = false;
    }
    document.body.removeChild(textarea);
    return copied;
}

/** Copy handler shared by every copy affordance so feedback stays consistent. */
export function useCopy(): (text: string, label: string) => void {
    const notify = useToast();
    return useCallback(
        (text: string, label: string) => {
            void writeClipboard(text).then((copied) =>
                copied ? notify(`${label} をコピーしました`) : notify(`${label} のコピーに失敗しました`, "error"),
            );
        },
        [notify],
    );
}

/** Escape key handling shared by the palette and other transient overlays. */
export function useEscapeKey(enabled: boolean, onEscape: () => void): void {
    useEffect(() => {
        if (!enabled) return;
        const handler = (event: KeyboardEvent): void => {
            if (event.key !== "Escape") return;
            event.preventDefault();
            onEscape();
        };
        window.addEventListener("keydown", handler);
        return () => window.removeEventListener("keydown", handler);
    }, [enabled, onEscape]);
}
