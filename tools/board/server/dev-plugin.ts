import type { Plugin } from "vite";

import { API_PREFIX, handleApiRequest } from "./api";

/**
 * Mount the board API on the Vite dev server so `npm run dev` needs no second
 * process, and dev and production answer the same routes.
 */
export function boardApiPlugin(): Plugin {
    return {
        name: "arukellt-board-api",
        configureServer(server) {
            server.middlewares.use((req, res, next) => {
                if (!req.url?.startsWith(API_PREFIX)) return next();
                handleApiRequest(req, res);
            });
        },
    };
}
