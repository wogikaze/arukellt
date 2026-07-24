#!/usr/bin/env node
/** Generate a static `data.json` snapshot by talking to the board server. */
import { spawn } from "node:child_process";
import { createServer } from "node:net";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "../../..");
const SERVER = resolve(__dirname, "../dist/server/main.js");
const OUT = process.argv[2] ? resolve(process.argv[2]) : resolve(REPO_ROOT, "docs/board/data.json");

function getFreePort() {
    return new Promise((res, rej) => {
        const s = createServer();
        s.listen(0, () => {
            const port = s.address().port;
            s.close(() => res(port));
        });
        s.on("error", rej);
    });
}

function waitForServer(port) {
    return new Promise((res, rej) => {
        const url = `http://127.0.0.1:${port}/api/board`;
        let attempts = 0;
        const tryFetch = () => {
            attempts += 1;
            fetch(url)
                .then((r) => (r.ok ? res() : Promise.reject()))
                .catch(() => {
                    if (attempts > 60) return rej(new Error("server did not become ready"));
                    setTimeout(tryFetch, 250);
                });
        };
        tryFetch();
    });
}

async function getJson(url) {
    const r = await fetch(url);
    if (!r.ok) throw new Error(`${url}: ${r.status} ${r.statusText}`);
    return r.json();
}

async function main() {
    mkdirSync(dirname(OUT), { recursive: true });
    const port = await getFreePort();
    const proc = spawn("node", [SERVER, "-p", String(port)], {
        cwd: REPO_ROOT,
        stdio: "pipe",
    });

    try {
        await waitForServer(port);
        const data = await getJson(`http://127.0.0.1:${port}/api/board`);
        data.files = {};
        const paths = [...data.issues.map((i) => i.path), ...data.docs.map((d) => d.path)];
        for (const path of paths) {
            const file = await getJson(`http://127.0.0.1:${port}/api/file?path=${encodeURIComponent(path)}`);
            data.files[path] = file.text;
        }
        writeFileSync(OUT, JSON.stringify(data));
        console.log(`wrote ${OUT} (${Object.keys(data.files).length} files, ${Math.round(JSON.stringify(data).length / 1024)} KB)`);
    } finally {
        proc.kill();
    }
}

main().catch((err) => {
    console.error(err);
    process.exitCode = 1;
});
