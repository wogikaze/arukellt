import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

import { boardApiPlugin } from "./server/dev-plugin";

const isStatic = process.env.BOARD_STATIC === "1";

export default defineConfig({
    plugins: [react(), boardApiPlugin()],
    base: isStatic ? "./" : "/",
    define: {
        __BOARD_STATIC__: JSON.stringify(isStatic),
        __BOARD_DATA_URL__: JSON.stringify(isStatic ? "./data.json" : "/api/board"),
    },
    server: {
        port: 8770,
        strictPort: false,
    },
    build: {
        outDir: isStatic ? process.env.BOARD_OUTDIR ?? "../../docs/board" : "dist/client",
        emptyOutDir: true,
        // mermaid is large and only needed once a graph or diagram is rendered;
        // keeping it in its own chunk keeps first paint fast.
        chunkSizeWarningLimit: 1200,
    },
});
