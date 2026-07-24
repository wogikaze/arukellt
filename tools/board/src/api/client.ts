import type { BoardData, FileContent, SearchResponse } from "../../shared/types";

async function getJson<T>(url: string, signal?: AbortSignal): Promise<T> {
    const response = await fetch(url, { signal });
    if (!response.ok) {
        const detail = await response.json().catch(() => null);
        const message = detail && typeof detail === "object" && "error" in detail ? String(detail.error) : response.statusText;
        throw new Error(message);
    }
    return (await response.json()) as T;
}

export function fetchBoard(options: { refresh?: boolean; signal?: AbortSignal } = {}): Promise<BoardData> {
    return getJson<BoardData>(`/api/board${options.refresh ? "?refresh=1" : ""}`, options.signal);
}

export function fetchFile(path: string, signal?: AbortSignal): Promise<FileContent> {
    return getJson<FileContent>(`/api/file?path=${encodeURIComponent(path)}`, signal);
}

export function fetchSearch(query: string, signal?: AbortSignal): Promise<SearchResponse> {
    return getJson<SearchResponse>(`/api/search?q=${encodeURIComponent(query)}`, signal);
}
