import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/shell.css";
import "./styles/board.css";
import "./styles/doc.css";
import "./styles/overlays.css";

import { App } from "./app/App";
import { ThemeProvider } from "./app/ThemeContext";
import { BoardProvider } from "./data/BoardContext";
import { ToastProvider } from "./components/Toast";
import { WorkspaceProvider } from "./workspace/WorkspaceContext";

const host = document.getElementById("root");
if (!host) throw new Error("#root element is missing from index.html");

createRoot(host).render(
    <StrictMode>
        <ThemeProvider>
            <ToastProvider>
                <BoardProvider>
                    <WorkspaceProvider>
                        <App />
                    </WorkspaceProvider>
                </BoardProvider>
            </ToastProvider>
        </ThemeProvider>
    </StrictMode>,
);
