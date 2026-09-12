/**
 * Compiler output panel for the playground shell (ADR-017 DOM surface).
 *
 * @module
 */

import type { ConsoleOutputSection } from "./compiler-output.js";

/** Options for creating an output panel. */
export interface OutputPanelOptions {
  /** Whether to auto-inject panel CSS. Default: `true`. */
  injectStyles?: boolean;
}

/** An output panel instance. */
export interface OutputPanel {
  /** Root DOM element. */
  readonly element: HTMLElement;

  /** Replace panel content with the given sections. */
  update(sections: ConsoleOutputSection[], status?: string, isError?: boolean): void;

  /** Clear output and hide the panel. */
  clear(): void;

  /** Destroy the panel and remove it from the DOM. */
  destroy(): void;
}

export const OUTPUT_CSS = `
.ark-output-panel {
  display: none;
  flex-direction: column;
  gap: 8px;
  padding: 12px 16px;
  background: var(--ark-output-bg, #181825);
  border: 1px solid var(--ark-output-border, #313244);
  border-radius: 10px;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.82rem;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 30vh;
  overflow-y: auto;
}

.ark-output-panel.visible {
  display: flex;
}

.ark-output-status {
  font-family: system-ui, sans-serif;
  font-size: 0.82rem;
  color: var(--ark-output-status, #9399b2);
  min-height: 1.2em;
}

.ark-output-status.error {
  color: var(--ark-output-status-error, #f38ba8);
}

.ark-output-section-title {
  font-family: system-ui, sans-serif;
  font-size: 0.72rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--ark-output-label, #9399b2);
}

.ark-output-section-body {
  margin: 0;
}
`;

let stylesInjected = false;

/** Inject output panel styles (idempotent). */
export function injectOutputStyles(): void {
  if (stylesInjected || typeof document === "undefined") return;
  const style = document.createElement("style");
  style.setAttribute("data-ark-output", "true");
  style.textContent = OUTPUT_CSS;
  document.head.appendChild(style);
  stylesInjected = true;
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/** Create a compiler output panel and mount it inside the given container. */
export function createOutputPanel(
  container: HTMLElement,
  options: OutputPanelOptions = {},
): OutputPanel {
  const { injectStyles = true } = options;
  if (injectStyles) {
    injectOutputStyles();
  }

  const root = document.createElement("section");
  root.className = "ark-output-panel";
  root.setAttribute("aria-label", "Compiler output");

  const statusEl = document.createElement("div");
  statusEl.className = "ark-output-status";
  statusEl.setAttribute("aria-live", "polite");
  root.appendChild(statusEl);

  const body = document.createElement("div");
  body.className = "ark-output-body";
  root.appendChild(body);

  container.appendChild(root);

  let destroyed = false;

  return {
    get element(): HTMLElement {
      return root;
    },

    update(sections: ConsoleOutputSection[], status = "", isError = false): void {
      if (destroyed) return;

      body.innerHTML = "";
      for (const section of sections) {
        const title = document.createElement("div");
        title.className = "ark-output-section-title";
        title.textContent = section.title;

        const sectionBody = document.createElement("pre");
        sectionBody.className = "ark-output-section-body";
        sectionBody.innerHTML = escapeHtml(section.body);

        body.appendChild(title);
        body.appendChild(sectionBody);
      }

      statusEl.textContent = status;
      statusEl.classList.toggle("error", isError);
      root.classList.toggle("visible", sections.length > 0 || status.length > 0);
    },

    clear(): void {
      if (destroyed) return;
      body.innerHTML = "";
      statusEl.textContent = "";
      statusEl.classList.remove("error");
      root.classList.remove("visible");
    },

    destroy(): void {
      if (destroyed) return;
      destroyed = true;
      if (root.parentNode) {
        root.parentNode.removeChild(root);
      }
    },
  };
}
