import type { BoardData, FileContent, SearchResponse } from "../../shared/types";

declare const __BOARD_STATIC__: boolean;
declare const __BOARD_DATA_URL__: string;

const STATIC = typeof __BOARD_STATIC__ !== "undefined" ? __BOARD_STATIC__ : false;
const DATA_URL = typeof __BOARD_DATA_URL__ !== "undefined" ? __BOARD_DATA_URL__ : "/api/board";

let staticData: BoardData | null = null;

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
    const url = STATIC ? DATA_URL : `${DATA_URL}${options.refresh ? "?refresh=1" : ""}`;
    return getJson<BoardData>(url, options.signal).then((data) => {
        if (STATIC) staticData = data;
        return data;
    });
}

export function fetchFile(path: string, _signal?: AbortSignal): Promise<FileContent> {
    if (STATIC) {
        if (!staticData) return Promise.reject(new Error("board data not loaded"));
        const text = staticData.files?.[path];
        if (text === undefined) return Promise.reject(new Error(`not a readable repository file: ${path}`));
        return Promise.resolve({
            path,
            absolutePath: `${staticData.repoRoot}/${path}`,
            text,
            issue: staticData.issues.find((issue) => issue.path === path) ?? null,
            doc: staticData.docs.find((doc) => doc.path === path) ?? null,
        });
    }
    return getJson<FileContent>(`/api/file?path=${encodeURIComponent(path)}`);
}

const MAX_HITS = 60;
const MAX_EXCERPTS_PER_FILE = 3;
const EXCERPT_LENGTH = 200;
const TITLE_WEIGHT = 40;
const ID_WEIGHT = 200;
const BODY_WEIGHT = 1;

function countOccurrences(haystack: string, needle: string): number {
    let count = 0;
    let from = 0;
    while (true) {
        const at = haystack.indexOf(needle, from);
        if (at === -1) return count;
        count += 1;
        from = at + needle.length;
    }
}

function collectExcerpts(lines: string[], needle: string): { line: number; text: string }[] {
    const excerpts: { line: number; text: string }[] = [];
    const loweredNeedle = needle.toLowerCase();
    for (let i = 0; i < lines.length && excerpts.length < MAX_EXCERPTS_PER_FILE; i += 1) {
        const at = lines[i].toLowerCase().indexOf(loweredNeedle);
        if (at === -1) continue;
        const start = Math.max(0, at - 60);
        const snippet = lines[i].slice(start, start + EXCERPT_LENGTH).trim();
        excerpts.push({ line: i + 1, text: (start > 0 ? "…" : "") + snippet });
    }
    return excerpts;
}

function staticSearch(data: BoardData, query: string): SearchResponse {
    const needle = query.trim().toLowerCase();
    if (needle.length < 2 || !data.files) return { query, total: 0, hits: [] };

    const bareId = /^#?(\d{1,4})$/.exec(needle)?.[1];
    const candidates: { path: string; title: string; kind: "issue" | "doc"; status: string; identity: string }[] = [
        ...data.issues.map((issue) => ({ path: issue.path, title: issue.title, kind: "issue" as const, status: issue.status, identity: issue.id })),
        ...data.docs.map((doc) => ({ path: doc.path, title: doc.title, kind: "doc" as const, status: doc.status, identity: doc.id })),
    ];

    const hits: SearchResponse["hits"] = [];
    for (const candidate of candidates) {
        const text = data.files[candidate.path];
        if (text === undefined) continue;
        const lowered = text.toLowerCase();
        const bodyMatches = countOccurrences(lowered, needle);
        const titleMatches = countOccurrences(candidate.title.toLowerCase(), needle);
        const identityMatch = bareId ? candidate.identity.toLowerCase() === bareId : false;
        if (!bodyMatches && !titleMatches && !identityMatch) continue;

        hits.push({
            path: candidate.path,
            title: candidate.title,
            kind: candidate.kind,
            status: candidate.status,
            score: (identityMatch ? ID_WEIGHT : 0) + titleMatches * TITLE_WEIGHT + Math.min(bodyMatches, 20) * BODY_WEIGHT,
            excerpts: collectExcerpts(text.split("\n"), needle),
        });
    }

    hits.sort((a, b) => b.score - a.score || a.path.localeCompare(b.path));
    return { query, total: hits.length, hits: hits.slice(0, MAX_HITS) };
}

export function fetchSearch(query: string, _signal?: AbortSignal): Promise<SearchResponse> {
    if (STATIC) {
        if (!staticData) return Promise.reject(new Error("board data not loaded"));
        return Promise.resolve(staticSearch(staticData, query));
    }
    return getJson<SearchResponse>(`/api/search?q=${encodeURIComponent(query)}`);
}
