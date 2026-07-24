import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

import { boardApiPlugin } from "./server/dev-plugin";

export default defineConfig({
    plugins: [react(), boardApiPlugin()],
    server: {
        port: 8770,
        strictPort: false,
    },
    build: {
        outDir: "dist/client",
        emptyOutDir: true,
        // mermaid is large and only needed once a graph or diagram is rendered;
        // keeping it in its own chunk keeps first paint fast.
        chunkSizeWarningLimit: 1200,
    },
});
