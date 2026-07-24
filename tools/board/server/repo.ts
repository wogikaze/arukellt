import { execFileSync } from "node:child_process";
import { existsSync, statSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

/**
 * A repository checkout is recognized by the two directory trees the board
 * reads. Checking for `.git` alone is not enough: git worktrees store a file
 * there, and a stale parent match would silently serve the wrong repo.
 */
function isRepoRoot(candidate: string): boolean {
    return existsSync(join(candidate, "issues")) && existsSync(join(candidate, "docs"));
}

function findRepoRoot(): string {
    const override = process.env.ARUKELLT_BOARD_ROOT;
    if (override) {
        const abs = resolve(override);
        if (!isRepoRoot(abs)) {
            throw new Error(`ARUKELLT_BOARD_ROOT=${abs} has no issues/ and docs/ directories`);
        }
        return abs;
    }
    let dir = dirname(fileURLToPath(import.meta.url));
    while (true) {
        if (isRepoRoot(dir)) return dir;
        const parent = dirname(dir);
        if (parent === dir) {
            throw new Error("could not locate repository root above " + fileURLToPath(import.meta.url));
        }
        dir = parent;
    }
}

export const REPO_ROOT = findRepoRoot();

export const REPO_NAME = REPO_ROOT.split(sep).filter(Boolean).pop() ?? "repository";

export function currentBranch(): string {
    try {
        return execFileSync("git", ["rev-parse", "--abbrev-ref", "HEAD"], {
            cwd: REPO_ROOT,
            encoding: "utf8",
            stdio: ["ignore", "pipe", "ignore"],
        }).trim();
    } catch {
        return "";
    }
}

/**
 * Resolve a client-supplied repository-relative path.
 *
 * Returns null for anything that escapes the repo, is absolute, or is not a
 * regular file. The board is read-only, but path traversal would still leak
 * arbitrary host files through /api/file, so this guard is load-bearing.
 */
export function resolveRepoFile(relPath: string): string | null {
    if (!relPath || isAbsolute(relPath) || relPath.includes("\0")) return null;
    const abs = resolve(REPO_ROOT, relPath);
    const rel = relative(REPO_ROOT, abs);
    if (rel.startsWith("..") || isAbsolute(rel)) return null;
    if (!existsSync(abs) || !statSync(abs).isFile()) return null;
    return abs;
}

export function toRepoRelative(abs: string): string {
    return relative(REPO_ROOT, abs).split(sep).join("/");
}
