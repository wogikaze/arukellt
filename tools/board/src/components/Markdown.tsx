import { useEffect, useId, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";

import type { Heading } from "./markdown";
import { renderMarkdown } from "./markdown";
import { Mermaid } from "./Mermaid";
import { useCopy } from "./Toast";

export interface MarkdownProps {
    text: string;
    /** Repository path of the source file; relative links resolve against it. */
    sourcePath: string;
    onOpenPath: (path: string, event: React.MouseEvent) => void;
    onHeadings?: (headings: Heading[]) => void;
}

/**
 * Rendered document body.
 *
 * The HTML is produced outside React (marked + sanitiser) and injected once,
 * then two kinds of interactive islands are grafted on: mermaid diagrams get
 * their own React roots, and code blocks get copy buttons. Doing it this way
 * keeps the markdown pipeline free of React-specific parsing.
 */
export function Markdown({ text, sourcePath, onOpenPath, onHeadings }: MarkdownProps): ReactNode {
    const containerRef = useRef<HTMLDivElement>(null);
    const diagramRoots = useRef<Root[]>([]);
    const copy = useCopy();
    const [copyTarget, setCopyTarget] = useState<{ top: number; left: number; code: string } | null>(null);
    const idPrefix = `${useId().replace(/:/g, "")}-`;

    const rendered = useMemo(() => renderMarkdown(text, sourcePath, idPrefix), [text, sourcePath, idPrefix]);

    useEffect(() => {
        onHeadings?.(rendered.headings);
    }, [rendered, onHeadings]);

    useEffect(() => {
        const container = containerRef.current;
        if (!container) return;
        container.innerHTML = rendered.html;

        const roots: Root[] = [];
        for (const holder of [...container.querySelectorAll<HTMLElement>(".mermaid-source")]) {
            const source = holder.textContent ?? "";
            holder.textContent = "";
            const root = createRoot(holder);
            root.render(<Mermaid source={source} />);
            roots.push(root);
        }
        diagramRoots.current = roots;

        return () => {
            // Unmount asynchronously: React forbids unmounting a root while the
            // parent component is still rendering.
            const stale = roots;
            queueMicrotask(() => stale.forEach((root) => root.unmount()));
        };
    }, [rendered]);

    useEffect(() => {
        const container = containerRef.current;
        if (!container) return;

        const onClick = (event: MouseEvent): void => {
            const anchor = (event.target as Element).closest<HTMLAnchorElement>("a[data-repo-path]");
            if (!anchor) return;
            event.preventDefault();
            onOpenPath(anchor.dataset.repoPath ?? "", event as unknown as React.MouseEvent);
        };

        const onPointerMove = (event: PointerEvent): void => {
            const pre = (event.target as Element).closest<HTMLPreElement>("pre");
            if (!pre) {
                setCopyTarget(null);
                return;
            }
            const containerBox = container.getBoundingClientRect();
            const preBox = pre.getBoundingClientRect();
            setCopyTarget({
                top: preBox.top - containerBox.top + container.scrollTop + 6,
                left: preBox.right - containerBox.left - 30,
                code: pre.textContent ?? "",
            });
        };

        container.addEventListener("click", onClick);
        container.addEventListener("pointermove", onPointerMove);
        container.addEventListener("pointerleave", () => setCopyTarget(null));
        return () => {
            container.removeEventListener("click", onClick);
            container.removeEventListener("pointermove", onPointerMove);
        };
    }, [onOpenPath, rendered]);

    return (
        <div className="markdown-host">
            <div ref={containerRef} className="markdown" />
            {copyTarget ? (
                <button
                    type="button"
                    className="markdown__code-copy"
                    style={{ top: copyTarget.top, left: copyTarget.left }}
                    title="コードブロックをコピー"
                    onClick={() => copy(copyTarget.code, "コードブロック")}
                >
                    copy
                </button>
            ) : null}
        </div>
    );
}
