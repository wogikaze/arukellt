import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

import { API_PREFIX, handleApiRequest } from "./api";
import { REPO_ROOT } from "./repo";

/** `dist/server/main.js` -> `dist/client`. */
const CLIENT_DIR = resolve(fileURLToPath(import.meta.url), "..", "..", "client");

const CONTENT_TYPES: Record<string, string> = {
    ".html": "text/html; charset=utf-8",
    ".js": "text/javascript; charset=utf-8",
    ".css": "text/css; charset=utf-8",
    ".json": "application/json; charset=utf-8",
    ".svg": "image/svg+xml",
    ".woff2": "font/woff2",
    ".png": "image/png",
    ".ico": "image/x-icon",
};

/** Hashed asset filenames are immutable; index.html must never be cached. */
function cacheControl(pathname: string): string {
    return pathname.startsWith("/assets/") ? "public, max-age=31536000, immutable" : "no-store";
}

function resolveStaticFile(pathname: string): string | null {
    const rel = normalize(decodeURIComponent(pathname)).replace(/^([/\\])+/, "");
    if (rel.split(/[/\\]/).includes("..")) return null;
    const abs = join(CLIENT_DIR, rel);
    if (!abs.startsWith(CLIENT_DIR + sep)) return null;
    if (!existsSync(abs) || !statSync(abs).isFile()) return null;
    return abs;
}

function serveFile(abs: string, pathname: string, res: import("node:http").ServerResponse): void {
    res.writeHead(200, {
        "Content-Type": CONTENT_TYPES[extname(abs)] ?? "application/octet-stream",
        "Content-Length": statSync(abs).size,
        "Cache-Control": cacheControl(pathname),
    });
    createReadStream(abs).pipe(res);
}

function parseCliOptions(): { port: number; host: string; open: boolean } {
    const { values } = parseArgs({
        options: {
            port: { type: "string", short: "p", default: process.env.PORT ?? "8770" },
            host: { type: "string", short: "H", default: "127.0.0.1" },
            open: { type: "boolean", default: false },
        },
    });
    return { port: Number(values.port), host: String(values.host), open: Boolean(values.open) };
}

function main(): void {
    if (!existsSync(join(CLIENT_DIR, "index.html"))) {
        console.error(`board: client bundle missing at ${CLIENT_DIR}. Run \`npm run build\` first.`);
        process.exitCode = 1;
        return;
    }
    const { port, host, open } = parseCliOptions();

    const server = createServer((req, res) => {
        const pathname = new URL(req.url ?? "/", "http://localhost").pathname;
        if (pathname.startsWith(API_PREFIX)) {
            handleApiRequest(req, res);
            return;
        }
        const abs = resolveStaticFile(pathname);
        if (abs) {
            serveFile(abs, pathname, res);
            return;
        }
        // Client-side routing lives in the URL hash, but an unknown path should
        // still boot the app rather than 404.
        serveFile(join(CLIENT_DIR, "index.html"), "/index.html", res);
    });

    server.listen(port, host, () => {
        const url = `http://${host}:${port}/`;
        console.log(`board: serving ${url}`);
        console.log(`board: repository ${REPO_ROOT}`);
        if (open) void import("node:child_process").then(({ spawn }) => {
            spawn("xdg-open", [url], { stdio: "ignore", detached: true }).unref();
        });
    });
}

main();
