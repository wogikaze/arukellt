import type { IncomingMessage, ServerResponse } from "node:http";

import { getDataset, getFile } from "./dataset";
import { search } from "./search";

export const API_PREFIX = "/api/";

interface ApiResult {
    status: number;
    body: unknown;
}

function route(pathname: string, params: URLSearchParams): ApiResult {
    if (pathname === "/api/board") {
        return { status: 200, body: getDataset(params.get("refresh") === "1") };
    }
    if (pathname === "/api/file") {
        const path = params.get("path") ?? "";
        const file = getFile(path);
        if (!file) return { status: 404, body: { error: `not a readable repository file: ${path}` } };
        return { status: 200, body: file };
    }
    if (pathname === "/api/search") {
        return { status: 200, body: search(params.get("q") ?? "") };
    }
    return { status: 404, body: { error: `unknown endpoint: ${pathname}` } };
}

/**
 * Serve one `/api/*` request. Shared by the Vite dev middleware and the
 * production server so both environments expose an identical contract.
 */
export function handleApiRequest(req: IncomingMessage, res: ServerResponse): void {
    const url = new URL(req.url ?? "/", "http://localhost");
    let result: ApiResult;
    try {
        result = route(url.pathname, url.searchParams);
    } catch (error) {
        result = { status: 500, body: { error: error instanceof Error ? error.message : String(error) } };
    }

    const payload = JSON.stringify(result.body);
    res.writeHead(result.status, {
        "Content-Type": "application/json; charset=utf-8",
        "Content-Length": Buffer.byteLength(payload),
        // The dataset mirrors files on disk; a stale response would show the
        // author their own edits missing after a reload.
        "Cache-Control": "no-store",
    });
    res.end(payload);
}
