import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";

import { renderMermaid } from "./mermaid";
import { useTheme } from "../app/ThemeContext";

/**
 * Rendered mermaid diagram with pan/zoom.
 *
 * Dependency graphs get wide quickly, so the SVG is placed in a scroll
 * container with a zoom control rather than being scaled to fit and unreadable.
 */
export function Mermaid({
    source,
    onNodeClick,
}: {
    source: string;
    /** Receives the id encoded in a node's element id, e.g. `n714` -> `714`. */
    onNodeClick?: (id: string) => void;
}): ReactNode {
    const { theme } = useTheme();
    const [svg, setSvg] = useState("");
    const [error, setError] = useState<string | null>(null);
    const [zoom, setZoom] = useState(1);
    const hostRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        let active = true;
        setError(null);
        void renderMermaid(source, theme).then((result) => {
            if (!active) return;
            setSvg(result.svg);
            setError(result.error);
        });
        return () => {
            active = false;
        };
    }, [source, theme]);

    useEffect(() => {
        const host = hostRef.current;
        if (!host || !svg || !onNodeClick) return;
        const onClick = (event: MouseEvent): void => {
            const node = (event.target as Element).closest<SVGGElement>("g.node");
            const match = /(?:^|-)n(\d+[a-z]?)(?:-|$)/.exec(node?.id ?? "");
            if (!match) return;
            onNodeClick(match[1]);
        };
        host.addEventListener("click", onClick);
        return () => host.removeEventListener("click", onClick);
    }, [svg, onNodeClick]);

    if (error) {
        return (
            <div className="mermaid-error">
                <strong>図の描画に失敗しました</strong>
                <pre>{error}</pre>
            </div>
        );
    }

    return (
        <div className="mermaid">
            <div className="mermaid__controls">
                <button type="button" className="icon-button" title="縮小" onClick={() => setZoom((z) => Math.max(0.3, z - 0.15))}>
                    −
                </button>
                <span className="mermaid__zoom-label">{Math.round(zoom * 100)}%</span>
                <button type="button" className="icon-button" title="拡大" onClick={() => setZoom((z) => Math.min(3, z + 0.15))}>
                    +
                </button>
                <button type="button" className="icon-button" title="等倍に戻す" onClick={() => setZoom(1)}>
                    ⤢
                </button>
            </div>
            <div className="mermaid__scroll">
                <div
                    ref={hostRef}
                    className={onNodeClick ? "mermaid__canvas mermaid__canvas--interactive" : "mermaid__canvas"}
                    style={{ transform: `scale(${zoom})` }}
                    dangerouslySetInnerHTML={{ __html: svg }}
                />
            </div>
        </div>
    );
}
